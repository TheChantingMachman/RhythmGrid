use rustfft::{FftPlanner, num_complex::Complex};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const SUPPORTED_FORMATS: &[&str] = &["mp3", "wav", "flac", "ogg"];
pub const DEFAULT_BPM: u32 = 120;

pub const DEFAULT_VOLUME: f32 = 0.8;
pub const MIN_VOLUME: f32 = 0.0;
pub const MAX_VOLUME: f32 = 1.0;
pub const DEFAULT_SPEED: f32 = 1.0;
pub const MIN_SPEED: f32 = 0.5;
pub const MAX_SPEED: f32 = 2.0;

#[derive(Debug)]
pub enum AudioError {
    FileNotFound,
    IoError(std::io::Error),
    DecodeError(String),
    UnsupportedFormat,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::FileNotFound => write!(f, "File not found"),
            AudioError::IoError(e) => write!(f, "IO error: {}", e),
            AudioError::DecodeError(s) => write!(f, "Decode error: {}", s),
            AudioError::UnsupportedFormat => write!(f, "Unsupported format"),
        }
    }
}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        AudioError::IoError(e)
    }
}

#[derive(Debug)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

pub fn decode_audio(path: &Path) -> Result<DecodedAudio, AudioError> {
    // Validate extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if !SUPPORTED_FORMATS.contains(&ext.as_str()) {
        return Err(AudioError::UnsupportedFormat);
    }

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(AudioError::FileNotFound),
        Err(e) => return Err(AudioError::IoError(e)),
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(&ext);

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioError::DecodeError(e.to_string()))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError::DecodeError("No audio track found".into()))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::DecodeError(e.to_string()))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(AudioError::DecodeError(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(AudioError::DecodeError(e.to_string())),
        };

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    Ok(DecodedAudio {
        sample_rate,
        channels,
        samples: all_samples,
    })
}

// --- Playback State Machine ---

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub struct AudioPlayer {
    state: PlaybackState,
    position: usize,
    audio: DecodedAudio,
    volume: f32,
    speed: f32,
}

impl AudioPlayer {
    pub fn new(audio: DecodedAudio) -> Self {
        AudioPlayer {
            state: PlaybackState::Stopped,
            position: 0,
            audio,
            volume: DEFAULT_VOLUME,
            speed: DEFAULT_SPEED,
        }
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(MIN_VOLUME, MAX_VOLUME);
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_speed(&mut self, s: f32) {
        self.speed = s.clamp(MIN_SPEED, MAX_SPEED);
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn amplitude(&self) -> f32 {
        let samples = &self.audio.samples;
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    pub fn play(&mut self) {
        if self.state != PlaybackState::Playing {
            self.state = PlaybackState::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.position = 0;
    }

    pub fn state(&self) -> PlaybackState {
        self.state.clone()
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

// --- Beat Detection ---

#[derive(Debug, Clone, PartialEq)]
pub struct BeatEvent {
    pub timestamp_secs: f64,
}

pub struct BeatDetector {
    window: [f32; 43],
    window_pos: usize,
    window_count: usize,
    last_beat_secs: f64,
}

impl BeatDetector {
    pub fn new() -> Self {
        BeatDetector {
            window: [0.0; 43],
            window_pos: 0,
            window_count: 0,
            last_beat_secs: f64::NEG_INFINITY,
        }
    }

    pub fn detect(&mut self, amplitude: f32, timestamp_secs: f64) -> Option<BeatEvent> {
        self.window[self.window_pos] = amplitude;
        self.window_pos = (self.window_pos + 1) % 43;
        if self.window_count < 43 {
            self.window_count += 1;
        }

        let count = self.window_count;
        let mean: f32 = self.window[..count].iter().sum::<f32>() / count as f32;

        if amplitude > mean * 1.5 && (timestamp_secs - self.last_beat_secs) >= 0.3 {
            self.last_beat_secs = timestamp_secs;
            Some(BeatEvent { timestamp_secs })
        } else {
            None
        }
    }
}

// --- FFT Frequency Band Decomposition ---

const BASS_LOW: u32 = 20;
const BASS_HIGH: u32 = 250;
const MIDS_LOW: u32 = 250;
const MIDS_HIGH: u32 = 4000;
const HIGHS_LOW: u32 = 4000;
const HIGHS_HIGH: u32 = 20000;

pub fn fft_bands(samples: &[f32], sample_rate: u32) -> (f32, f32, f32) {
    if samples.is_empty() || sample_rate == 0 {
        return (0.0, 0.0, 0.0);
    }

    let n = samples.len();

    // Apply Hann window and convert to complex
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1).max(1) as f32).cos());
            Complex { re: s * window, im: 0.0 }
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buffer);

    // Only use first half (positive frequencies)
    let num_bins = n / 2 + 1;
    let bin_hz = sample_rate as f32 / n as f32;

    let bin_energy = |bin: usize| -> f32 {
        let c = buffer[bin];
        c.re * c.re + c.im * c.im
    };

    let band_power = |low_hz: u32, high_hz: u32| -> f32 {
        let low_bin = ((low_hz as f32 / bin_hz).floor() as usize).min(num_bins - 1);
        let high_bin = ((high_hz as f32 / bin_hz).ceil() as usize).min(num_bins - 1);
        (low_bin..=high_bin).map(bin_energy).sum()
    };

    let total_power: f32 = (0..num_bins).map(bin_energy).sum();

    if total_power == 0.0 {
        return (0.0, 0.0, 0.0);
    }

    let bass = band_power(BASS_LOW, BASS_HIGH) / total_power;
    let mids = band_power(MIDS_LOW, MIDS_HIGH) / total_power;
    let highs = band_power(HIGHS_LOW, HIGHS_HIGH) / total_power;

    (bass, mids, highs)
}

pub fn generate_procedural(bpm: u32, duration_secs: f32, sample_rate: u32) -> DecodedAudio {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let beat_period = 60.0 / bpm as f32; // seconds per beat
    let samples_per_beat = beat_period * sample_rate as f32;

    let mut samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let beat_phase = (i as f32 % samples_per_beat) / samples_per_beat;
        // Simple click on each beat: decaying sine burst
        let amplitude = if beat_phase < 0.1 {
            (1.0 - beat_phase / 0.1) * (2.0 * std::f32::consts::PI * 440.0 * beat_phase * beat_period).sin()
        } else {
            0.0
        };
        samples.push(amplitude);
    }

    DecodedAudio {
        sample_rate,
        channels: 1,
        samples,
    }
}
