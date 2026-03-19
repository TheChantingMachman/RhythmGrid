// Audio output via cpal — streams PCM samples to system audio device.
// Loads real music from a folder if configured, falls back to procedural.

use std::path::Path;
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rhythm_grid::audio::{decode_audio, generate_procedural, BeatDetector, DEFAULT_BPM};
use rhythm_grid::music::{scan_folder, Playlist};

/// Shared state between the audio thread and the game loop.
pub struct AudioState {
    pub amplitude: f32,
    pub beat: bool,
    pub beat_intensity: f32,
    pub track_name: String,
    beat_detector: BeatDetector,
    elapsed_secs: f64,
}

impl AudioState {
    pub fn new() -> Self {
        AudioState {
            amplitude: 0.0,
            beat: false,
            beat_intensity: 0.0,
            track_name: String::new(),
            beat_detector: BeatDetector::new(),
            elapsed_secs: 0.0,
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

/// Load audio samples — tries music folder first, falls back to procedural.
fn load_audio(music_folder: Option<&str>, sample_rate: u32) -> (Vec<f32>, u16, Option<Playlist>, String) {
    if let Some(folder) = music_folder {
        let path = Path::new(folder);
        let files = scan_folder(path);
        if !files.is_empty() {
            let mut playlist = Playlist::new(files);
            if let Some(track_path) = playlist.current() {
                let track_name = track_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                match decode_audio(track_path) {
                    Ok(audio) => {
                        // Resample if needed (simple nearest-neighbor for now)
                        let samples = if audio.sample_rate == sample_rate {
                            audio.samples
                        } else {
                            resample(&audio.samples, audio.sample_rate, sample_rate, audio.channels)
                        };
                        return (samples, audio.channels, Some(playlist), track_name);
                    }
                    Err(e) => {
                        eprintln!("Failed to decode {}: {}", track_path.display(), e);
                        playlist.advance();
                    }
                }
            }
        }
    }

    // Fallback: procedural
    let audio = generate_procedural(DEFAULT_BPM, 300.0, sample_rate);
    (audio.samples, 1, None, "Procedural 120 BPM".to_string())
}

/// Simple nearest-neighbor resampling (mono or interleaved).
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

        let (samples, src_channels, playlist, track_name) =
            load_audio(folder_owned.as_deref(), sample_rate);

        // Set initial track name
        if let Ok(mut s) = state_clone.lock() {
            s.reset_for_new_track(&track_name);
        }

        let samples = Arc::new(samples);
        let src_channels = src_channels as usize;
        let position = Arc::new(Mutex::new(0usize));
        let playlist = Arc::new(Mutex::new(playlist));

        let samples_c = samples.clone();
        let position_c = position.clone();
        let state_c = state_clone.clone();
        let _playlist_c = playlist.clone();

        let config = cpal::StreamConfig {
            channels: out_channels as u16,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut pos = position_c.lock().unwrap();
                let src = &samples_c;
                let mut chunk_mono = Vec::with_capacity(data.len() / out_channels);

                for frame in data.chunks_mut(out_channels) {
                    // Read from source (may be mono or stereo)
                    let sample_offset = *pos;

                    if sample_offset < src.len() {
                        // Mix source channels to mono for beat detection
                        let mut mono = 0.0f32;
                        for ch in 0..src_channels {
                            let idx = sample_offset + ch;
                            if idx < src.len() {
                                mono += src[idx];
                            }
                        }
                        mono /= src_channels as f32;
                        chunk_mono.push(mono);

                        // Write to output channels
                        for (out_ch, s) in frame.iter_mut().enumerate() {
                            let src_ch = out_ch % src_channels;
                            let idx = sample_offset + src_ch;
                            *s = if idx < src.len() { src[idx] * 0.5 } else { 0.0 };
                        }
                        *pos += src_channels;
                    } else {
                        // Loop back to start
                        *pos = 0;
                        for s in frame.iter_mut() {
                            *s = 0.0;
                        }
                    }
                }

                if let Ok(mut state) = state_c.try_lock() {
                    state.update_from_samples(&chunk_mono, sample_rate);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ).expect("build audio stream");

        stream.play().expect("play audio stream");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    state
}
