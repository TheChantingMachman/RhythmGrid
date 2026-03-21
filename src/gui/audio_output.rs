// Audio output via cpal — streams PCM samples to system audio device.
// Supports real music with auto-advance and streaming decode.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rhythm_grid::audio::{decode_audio, fft_bands, generate_procedural, BeatDetector, DEFAULT_BPM};
use rhythm_grid::music::{scan_folder, Playlist};

/// Shared state between the audio thread and the game loop.
pub struct AudioState {
    pub amplitude: f32,
    pub beat: bool,
    pub beat_intensity: f32,
    pub bass: f32,
    pub mids: f32,
    pub highs: f32,
    pub track_name: String,
    pub volume: f32,           // 0.0-1.0, applied in audio callback
    pub skip_requested: bool,  // set by game loop, consumed by decode thread
    pub shutdown: bool,        // set to stop audio thread
    beat_detector: BeatDetector,
    elapsed_secs: f64,
    fft_buffer: Vec<f32>,
    fft_sample_rate: u32,
}

impl AudioState {
    pub fn new() -> Self {
        AudioState {
            amplitude: 0.0,
            beat: false,
            beat_intensity: 0.0,
            bass: 0.0,
            mids: 0.0,
            highs: 0.0,
            track_name: String::new(),
            volume: 0.5,
            skip_requested: false,
            shutdown: false,
            beat_detector: BeatDetector::new(),
            elapsed_secs: 0.0,
            fft_buffer: Vec::with_capacity(2048),
            fft_sample_rate: 44100,
        }
    }

    pub fn update_from_samples(&mut self, samples: &[f32], sample_rate: u32) {
        if !samples.is_empty() {
            let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
            self.amplitude = self.amplitude * 0.7 + rms * 0.3;
        }
        let chunk_duration = samples.len() as f64 / sample_rate as f64;
        self.elapsed_secs += chunk_duration;

        self.beat = false;
        if let Some(_event) = self.beat_detector.detect(self.amplitude, self.elapsed_secs) {
            self.beat = true;
            self.beat_intensity = 1.0;
        }

        // Accumulate samples for FFT
        self.fft_sample_rate = sample_rate;
        self.fft_buffer.extend_from_slice(samples);
        const FFT_WINDOW: usize = 2048;
        if self.fft_buffer.len() >= FFT_WINDOW {
            let window: Vec<f32> = self.fft_buffer.drain(..FFT_WINDOW).collect();
            let (b, m, h) = fft_bands(&window, sample_rate);
            // Smooth toward new values
            self.bass = self.bass * 0.6 + b * 0.4;
            self.mids = self.mids * 0.6 + m * 0.4;
            self.highs = self.highs * 0.6 + h * 0.4;
            // Keep buffer from growing unbounded
            if self.fft_buffer.len() > FFT_WINDOW * 2 {
                self.fft_buffer.drain(..self.fft_buffer.len() - FFT_WINDOW);
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.beat_intensity = (self.beat_intensity - dt * 4.0).max(0.0);
        self.beat = false;
    }

    fn reset_for_new_track(&mut self, name: &str) {
        self.beat_detector = BeatDetector::new();
        self.elapsed_secs = 0.0;
        self.track_name = name.to_string();
    }
}

/// Ring buffer of decoded samples shared between decode thread and audio callback.
struct SampleBuffer {
    samples: VecDeque<f32>,
    channels: usize,
    finished: bool, // true when current track is fully decoded
}

impl SampleBuffer {
    fn new(channels: usize) -> Self {
        SampleBuffer {
            samples: VecDeque::with_capacity(44100 * 2 * 10), // ~10s buffer
            channels,
            finished: false,
        }
    }
}

/// Simple nearest-neighbor resampling.
fn resample(samples: &[f32], from_rate: u32, to_rate: u32, channels: u16) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let ch = channels as usize;
    let frame_count = samples.len() / ch;
    let out_frames = (frame_count as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_frames * ch);
    for i in 0..out_frames {
        let src_frame = ((i as f64 * ratio) as usize).min(frame_count - 1);
        for c in 0..ch {
            out.push(samples[src_frame * ch + c]);
        }
    }
    out
}

fn track_name_from_path(path: &PathBuf) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Decode a track and push samples into the shared buffer in chunks.
/// Returns false if decode failed or skip was requested.
fn stream_decode_track(
    path: &PathBuf,
    target_sample_rate: u32,
    buffer: &Arc<Mutex<SampleBuffer>>,
    state: &Arc<Mutex<AudioState>>,
) -> bool {
    let audio = match decode_audio(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to decode {}: {}", path.display(), e);
            return false;
        }
    };

    let samples = if audio.sample_rate == target_sample_rate {
        audio.samples
    } else {
        resample(&audio.samples, audio.sample_rate, target_sample_rate, audio.channels)
    };

    let channels = audio.channels as usize;

    // Push in chunks to avoid holding the lock too long
    let chunk_size = 44100 * channels; // ~1 second chunks
    for chunk in samples.chunks(chunk_size) {
        // Check for skip before pushing each chunk
        if let Ok(mut s) = state.try_lock() {
            if s.skip_requested {
                s.skip_requested = false;
                return false;
            }
        }
        // Wait if buffer is too full (back-pressure)
        loop {
            if let Ok(buf) = buffer.try_lock() {
                if buf.samples.len() < 44100 * channels * 20 {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if let Ok(mut buf) = buffer.lock() {
            buf.channels = channels;
            buf.samples.extend(chunk);
        }
    }

    if let Ok(mut buf) = buffer.lock() {
        buf.finished = true;
    }
    true
}

/// Starts the audio output stream. Returns shared state for the game loop.
pub fn start_audio(music_folder: Option<&str>) -> Arc<Mutex<AudioState>> {
    let state = Arc::new(Mutex::new(AudioState::new()));
    let state_clone = state.clone();
    let folder_owned = music_folder.map(|s| s.to_string());

    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => { eprintln!("No audio output device found"); return; }
        };
        let supported_config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => { eprintln!("No audio output config: {}", e); return; }
        };

        let sample_rate = supported_config.sample_rate().0;
        let out_channels = supported_config.channels() as usize;

        // Set up playlist or procedural fallback
        let mut playlist: Option<Playlist> = None;
        let mut use_procedural = true;

        if let Some(folder) = &folder_owned {
            let files = scan_folder(Path::new(folder));
            if !files.is_empty() {
                playlist = Some(Playlist::new(files));
                use_procedural = false;
            }
        }

        // Shared sample buffer between decode thread and audio callback
        let buffer = Arc::new(Mutex::new(SampleBuffer::new(2)));

        if use_procedural {
            // Load procedural immediately (it's fast)
            let audio = generate_procedural(DEFAULT_BPM, 300.0, sample_rate);
            if let Ok(mut buf) = buffer.lock() {
                buf.channels = 1;
                buf.samples.extend(&audio.samples);
                buf.finished = true; // will loop
            }
            if let Ok(mut s) = state_clone.lock() {
                s.reset_for_new_track("Procedural 120 BPM");
            }
        }

        // Decode thread — handles streaming decode and track advancement
        let buffer_decode = buffer.clone();
        let state_decode = state_clone.clone();
        let playlist_shared = Arc::new(Mutex::new(playlist));
        let playlist_decode = playlist_shared.clone();

        std::thread::spawn(move || {
            if use_procedural { return; } // nothing to decode

            loop {
                let track_path;
                let track_name;
                {
                    let pl = playlist_decode.lock().unwrap();
                    match pl.as_ref().and_then(|p| p.current()) {
                        Some(path) => {
                            track_path = path.clone();
                            track_name = track_name_from_path(&track_path);
                        }
                        None => return,
                    }
                }

                if let Ok(mut s) = state_decode.lock() {
                    s.reset_for_new_track(&track_name);
                }

                // Reset buffer for new track
                if let Ok(mut buf) = buffer_decode.lock() {
                    buf.samples.clear();
                    buf.finished = false;
                }

                let ok = stream_decode_track(&track_path, sample_rate, &buffer_decode, &state_decode);

                if !ok {
                    // Skip broken track
                }

                // Wait for playback to finish or skip signal
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    // Check for skip request
                    if let Ok(mut s) = state_decode.try_lock() {
                        if s.skip_requested {
                            s.skip_requested = false;
                            // Clear the buffer to stop current track
                            if let Ok(mut buf) = buffer_decode.try_lock() {
                                buf.samples.clear();
                                buf.finished = true;
                            }
                            break;
                        }
                    }
                    if let Ok(buf) = buffer_decode.try_lock() {
                        if buf.finished && buf.samples.is_empty() {
                            break;
                        }
                    }
                }

                // Advance to next track
                {
                    let mut pl = playlist_decode.lock().unwrap();
                    if let Some(p) = pl.as_mut() {
                        p.advance();
                    }
                }
            }
        });

        // Audio output callback
        let buffer_play = buffer.clone();
        let state_play = state_clone.clone();

        let config = cpal::StreamConfig {
            channels: out_channels as u16,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let is_procedural = use_procedural;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut chunk_mono = Vec::with_capacity(data.len() / out_channels);

                // Read volume from shared state
                let vol = state_play.try_lock().map(|s| s.volume).unwrap_or(0.5);

                if let Ok(mut buf) = buffer_play.try_lock() {
                    let src_ch = buf.channels.max(1);

                    for frame in data.chunks_mut(out_channels) {
                        if buf.samples.len() >= src_ch {
                            let mut mono = 0.0f32;
                            for out_s in frame.iter_mut() {
                                *out_s = 0.0;
                            }
                            for ch in 0..src_ch {
                                if let Some(sample) = buf.samples.pop_front() {
                                    mono += sample;
                                    let out_ch_idx = ch % out_channels;
                                    frame[out_ch_idx] += sample * vol;
                                }
                            }
                            chunk_mono.push(mono / src_ch as f32);
                        } else if is_procedural && buf.finished {
                            // Procedural: loop by re-reading (buffer was drained)
                            // This shouldn't happen with 5min of procedural, but safety:
                            for s in frame.iter_mut() { *s = 0.0; }
                        } else {
                            // Buffer underrun — silence while waiting for decode
                            for s in frame.iter_mut() { *s = 0.0; }
                        }
                    }
                } else {
                    // Couldn't get lock — silence
                    for s in data.iter_mut() { *s = 0.0; }
                }

                if let Ok(mut state) = state_play.try_lock() {
                    state.update_from_samples(&chunk_mono, sample_rate);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ).expect("build audio stream");

        stream.play().expect("play audio stream");

        // Keep stream alive until shutdown is requested
        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Ok(s) = state_clone.try_lock() {
                if s.shutdown { break; }
            }
        }
        // stream drops here, stopping audio output
    });

    state
}
