// Audio output via cpal — streams PCM samples to system audio device.
// Supports real music with auto-advance and streaming decode.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rhythm_grid::audio::{fft_bands, spectral_centroid, generate_procedural, BeatDetector, MultiBeatDetector, SpectralFluxDetector, StreamingDecoder, DEFAULT_BPM};
use rhythm_grid::music::{scan_folder, Playlist};

/// Shared state between the audio thread and the game loop.
pub struct AudioState {
    pub amplitude: f32,
    pub beat: bool,
    pub beat_intensity: f32,
    pub bass: f32,
    pub mids: f32,
    pub highs: f32,
    pub bands: [f32; 7], // all 7 FFT bands
    pub band_beats: [bool; 7], // per-band beat this frame
    pub centroid: f32,         // spectral centroid 0.0 (dark) to 1.0 (bright)
    pub flux: f32,             // spectral flux (rate of spectral change)
    pub track_name: String,
    pub track_list: Vec<String>,       // all tracks in playlist (filenames only)
    pub current_track_index: usize,    // index into track_list
    pub volume: f32,           // 0.0-1.0, applied in audio callback
    pub skip_requested: bool,  // set by game loop, consumed by decode thread
    pub back_requested: bool,
    pub shuffle_requested: bool,
    pub shuffled: bool,            // mirrors Playlist::is_shuffled() for GUI display
    pub jump_to_requested: Option<usize>, // jump to track index in playlist
    pub paused: bool,
    pub shutdown: bool,        // set to stop audio thread
    beat_detector: BeatDetector,
    multi_beat_detector: MultiBeatDetector,
    flux_detector: SpectralFluxDetector,
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
            bands: [0.0; 7],
            band_beats: [false; 7],
            centroid: 0.0,
            flux: 0.0,
            track_name: String::new(),
            track_list: Vec::new(),
            current_track_index: 0,
            volume: 0.5,
            skip_requested: false,
            back_requested: false,
            shuffle_requested: false,
            shuffled: false,
            jump_to_requested: None,
            paused: false,
            shutdown: false,
            beat_detector: BeatDetector::new(),
            multi_beat_detector: MultiBeatDetector::new(),
            flux_detector: SpectralFluxDetector::new(),
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
        const FFT_WINDOW: usize = 1024; // ~23ms at 44.1kHz — faster beat response
        if self.fft_buffer.len() >= FFT_WINDOW {
            let window: Vec<f32> = self.fft_buffer.drain(..FFT_WINDOW).collect();
            let raw = fft_bands(&window, sample_rate);
            // Smooth all 7 bands
            for i in 0..7 {
                self.bands[i] = self.bands[i] * 0.6 + raw[i] * 0.4;
            }
            // Keep legacy 3-band summary for compatibility
            self.bass = self.bands[0] + self.bands[1];
            self.mids = self.bands[2] + self.bands[3] + self.bands[4];
            self.highs = self.bands[5] + self.bands[6];
            // Multi-band beat detection
            let events = self.multi_beat_detector.detect_bands(&self.bands, self.elapsed_secs);
            self.band_beats = [false; 7];
            for e in &events {
                self.band_beats[e.band] = true;
            }
            // Spectral centroid + flux
            self.centroid = self.centroid * 0.7 + spectral_centroid(&window, sample_rate) * 0.3;
            self.flux = self.flux_detector.detect(&self.bands);
            // Keep buffer from growing unbounded
            if self.fft_buffer.len() > FFT_WINDOW * 2 {
                self.fft_buffer.drain(..self.fft_buffer.len() - FFT_WINDOW);
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.beat_intensity = (self.beat_intensity - dt * 4.0).max(0.0);
        self.beat = false;
        // Note: band_beats cleared by game loop after reading, not here
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
            samples: VecDeque::with_capacity(44100 * 2 * 2), // ~2s buffer
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

/// Stream-decode a track, pushing PCM chunks to the buffer as they decode.
/// Playback starts within ~100ms instead of waiting for full file decode.
fn stream_decode_track(
    path: &PathBuf,
    target_sample_rate: u32,
    buffer: &Arc<Mutex<SampleBuffer>>,
    state: &Arc<Mutex<AudioState>>,
) -> bool {
    let mut decoder = match StreamingDecoder::open(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open {}: {:?}", path.display(), e);
            return false;
        }
    };

    let src_rate = decoder.sample_rate();
    let channels = decoder.channels() as usize;
    let needs_resample = src_rate != target_sample_rate;

    if let Ok(mut buf) = buffer.lock() {
        buf.channels = channels;
    }

    while let Some(chunk) = decoder.next_chunk() {
        // Check for skip/back/shuffle — abort decode so control loop can handle it
        if let Ok(s) = state.try_lock() {
            if s.skip_requested || s.back_requested || s.shuffle_requested || s.jump_to_requested.is_some() {
                return false;
            }
        }

        let out = if needs_resample {
            resample(&chunk, src_rate, target_sample_rate, channels as u16)
        } else {
            chunk
        };

        // Back-pressure: wait if buffer is too full
        loop {
            if let Ok(buf) = buffer.try_lock() {
                if buf.samples.len() < 44100 * channels * 2 { // ~2s back-pressure cap
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        if let Ok(mut buf) = buffer.lock() {
            buf.samples.extend(&out);
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
                // Populate track list for GUI display
                if let Ok(mut s) = state_clone.lock() {
                    s.track_list = files.iter()
                        .map(|p| track_name_from_path(p))
                        .collect();
                }
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
                    // Find current track index by matching name
                    if let Some(idx) = s.track_list.iter().position(|n| *n == track_name) {
                        s.current_track_index = idx;
                    }
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

                // Wait for playback to finish or control signal
                let mut go_back = false;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Ok(mut s) = state_decode.try_lock() {
                        if s.skip_requested || s.back_requested {
                            go_back = s.back_requested;
                            s.skip_requested = false;
                            s.back_requested = false;
                            if let Ok(mut buf) = buffer_decode.try_lock() {
                                buf.samples.clear();
                                buf.finished = true;
                            }
                            break;
                        }
                        if let Some(target) = s.jump_to_requested.take() {
                            if let Some(p) = playlist_decode.lock().unwrap().as_mut() {
                                p.jump_to(target);
                                s.current_track_index = target;
                            }
                            if let Ok(mut buf) = buffer_decode.lock() {
                                buf.samples.clear();
                                buf.finished = true;
                            }
                            go_back = false;
                            break;
                        }
                        if s.shuffle_requested {
                            s.shuffle_requested = false;
                            if let Some(p) = playlist_decode.lock().unwrap().as_mut() {
                                p.toggle_shuffle();
                                s.shuffled = p.is_shuffled();
                                s.track_list = p.files().iter()
                                    .map(|path| track_name_from_path(path))
                                    .collect();
                                s.current_track_index = 0;
                            }
                            if let Ok(mut buf) = buffer_decode.lock() {
                                buf.samples.clear();
                                buf.finished = true;
                            }
                            go_back = false;
                            break;
                        }
                    }
                    if let Ok(buf) = buffer_decode.try_lock() {
                        if buf.finished && buf.samples.is_empty() {
                            break;
                        }
                    }
                }

                // Navigate playlist
                {
                    let mut pl = playlist_decode.lock().unwrap();
                    if let Some(p) = pl.as_mut() {
                        if go_back {
                            p.prev_track();
                        } else {
                            p.advance();
                        }
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

                // Read volume and pause state
                let (vol, is_paused) = state_play.try_lock()
                    .map(|s| (s.volume, s.paused))
                    .unwrap_or((0.5, false));

                if is_paused {
                    for s in data.iter_mut() { *s = 0.0; }
                    return;
                }

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
