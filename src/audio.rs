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
