// Audio output via cpal — streams PCM samples to system audio device.
// Feeds pipeline-generated procedural audio and tracks playback position
// for beat detection and amplitude reactivity.

use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rhythm_grid::audio::{generate_procedural, BeatDetector};

/// Shared state between the audio thread and the game loop.
pub struct AudioState {
    pub amplitude: f32,       // current RMS amplitude (0.0-1.0)
    pub beat: bool,           // true on frames where a beat was detected
    pub beat_intensity: f32,  // decays from 1.0 on beat to 0.0 over time
    pub _bpm: u32,
    beat_detector: BeatDetector,
    elapsed_secs: f64,
}

impl AudioState {
    pub fn new(bpm: u32) -> Self {
        AudioState {
            amplitude: 0.0,
            beat: false,
            beat_intensity: 0.0,
            _bpm: bpm,
            beat_detector: BeatDetector::new(),
            elapsed_secs: 0.0,
        }
    }

    /// Called from the audio thread with a chunk of samples just played.
    pub fn update_from_samples(&mut self, samples: &[f32], sample_rate: u32) {
        // RMS amplitude of this chunk
        if !samples.is_empty() {
            let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
            // Smooth toward new value
            self.amplitude = self.amplitude * 0.7 + rms * 0.3;
        }

        let chunk_duration = samples.len() as f64 / sample_rate as f64;
        self.elapsed_secs += chunk_duration;

        // Beat detection
        self.beat = false;
        if let Some(_event) = self.beat_detector.detect(self.amplitude, self.elapsed_secs) {
            self.beat = true;
            self.beat_intensity = 1.0;
        }
    }

    /// Called each frame from the game loop to decay beat intensity.
    pub fn tick(&mut self, dt: f32) {
        self.beat_intensity = (self.beat_intensity - dt * 4.0).max(0.0);
        self.beat = false; // reset per-frame flag
    }
}

/// Starts the audio output stream with procedural audio. Returns shared state.
pub fn start_audio(bpm: u32) -> Arc<Mutex<AudioState>> {
    let state = Arc::new(Mutex::new(AudioState::new(bpm)));
    let state_clone = state.clone();

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
        let channels = supported_config.channels() as usize;

        // Generate procedural audio — long enough for a full session
        let duration_secs = 300.0; // 5 minutes
        let audio = generate_procedural(bpm, duration_secs, sample_rate);
        let samples = Arc::new(audio.samples);
        let position = Arc::new(Mutex::new(0usize));

        let samples_c = samples.clone();
        let position_c = position.clone();
        let state_c = state_clone;

        let config = cpal::StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut pos = position_c.lock().unwrap();
                let src = &samples_c;
                let mut chunk_samples = Vec::with_capacity(data.len() / channels);

                for frame in data.chunks_mut(channels) {
                    let sample = if *pos < src.len() {
                        src[*pos]
                    } else {
                        *pos = 0; // loop
                        src[0]
                    };
                    *pos += 1;
                    chunk_samples.push(sample);

                    // Write same sample to all channels (mono → stereo)
                    for s in frame.iter_mut() {
                        *s = sample * 0.5; // reduce volume
                    }
                }

                // Update shared audio state
                if let Ok(mut state) = state_c.try_lock() {
                    state.update_from_samples(&chunk_samples, sample_rate);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ).expect("build audio stream");

        stream.play().expect("play audio stream");

        // Keep the thread alive (stream drops when thread exits)
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    state
}
