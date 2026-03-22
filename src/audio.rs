use rustfft::{FftPlanner, num_complex::Complex};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, Decoder};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
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

const BAND_COUNT: usize = 7;
const FREQ_MIN: u32 = 20;
const FREQ_MAX: u32 = 20000;
const BAND0_LOW: u32 = 20;
const BAND0_HIGH: u32 = 60;
const BAND1_LOW: u32 = 61;
const BAND1_HIGH: u32 = 250;
const BAND2_LOW: u32 = 251;
const BAND2_HIGH: u32 = 500;
const BAND3_LOW: u32 = 501;
const BAND3_HIGH: u32 = 2000;
const BAND4_LOW: u32 = 2001;
const BAND4_HIGH: u32 = 4000;
const BAND5_LOW: u32 = 4001;
const BAND5_HIGH: u32 = 8000;
const BAND6_LOW: u32 = 8001;
const BAND6_HIGH: u32 = 20000;

pub fn fft_bands(samples: &[f32], sample_rate: u32) -> [f32; 7] {
    if samples.is_empty() || sample_rate == 0 {
        return [0.0; BAND_COUNT];
    }

    let n = samples.len();

    // Convert to complex (rectangular window — preserves exact bin energy for pure tones)
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
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

    let bin_freq = |bin: usize| -> f32 {
        bin as f32 * bin_hz
    };

    // Band boundaries: contiguous, exclusive-low / inclusive-high (except band 0 includes low).
    // Use previous band's HIGH as the exclusive lower bound for bands 1-6 so there are no gaps.
    let band_highs: [f32; BAND_COUNT] = [
        BAND0_HIGH as f32,
        BAND1_HIGH as f32,
        BAND2_HIGH as f32,
        BAND3_HIGH as f32,
        BAND4_HIGH as f32,
        BAND5_HIGH as f32,
        BAND6_HIGH as f32,
    ];

    let mut bands = [0.0f32; BAND_COUNT];
    for bin in 0..num_bins {
        let f = bin_freq(bin);
        if f < FREQ_MIN as f32 || f > FREQ_MAX as f32 {
            continue;
        }
        let energy = bin_energy(bin);
        // Band 0: f >= 20.0 && f <= 60.0
        // Band N (1-6): f > previous_band_high && f <= band_N_high
        if f <= band_highs[0] {
            bands[0] += energy;
        } else if f <= band_highs[1] {
            bands[1] += energy;
        } else if f <= band_highs[2] {
            bands[2] += energy;
        } else if f <= band_highs[3] {
            bands[3] += energy;
        } else if f <= band_highs[4] {
            bands[4] += energy;
        } else if f <= band_highs[5] {
            bands[5] += energy;
        } else {
            bands[6] += energy;
        }
    }

    // Normalize by total_power. Use the sum of band energies (which equals total_power
    // for contiguous bands) to avoid floating-point divergence that could push sum > 1.0.
    let band_total: f32 = bands.iter().sum();
    if band_total == 0.0 {
        return [0.0; BAND_COUNT];
    }
    for b in &mut bands {
        *b /= band_total;
    }

    bands
}

// --- Multi-Band Beat Detection ---

#[derive(Debug, Clone, PartialEq)]
pub struct BandBeatEvent {
    pub band: usize,
    pub timestamp_secs: f64,
}

pub struct MultiBeatDetector {
    detectors: [BeatDetector; 7],
}

impl MultiBeatDetector {
    pub fn new() -> Self {
        MultiBeatDetector {
            detectors: std::array::from_fn(|_| BeatDetector::new()),
        }
    }

    pub fn detect_bands(&mut self, bands: &[f32; 7], timestamp_secs: f64) -> Vec<BandBeatEvent> {
        let mut events = Vec::new();
        for (i, &energy) in bands.iter().enumerate() {
            if self.detectors[i].detect(energy, timestamp_secs).is_some() {
                events.push(BandBeatEvent { band: i, timestamp_secs });
            }
        }
        events
    }
}

// --- Streaming Decoder ---

pub struct StreamingDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
}

impl StreamingDecoder {
    pub fn open(path: &Path) -> Result<StreamingDecoder, AudioError> {
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

        let format = probed.format;
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

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        Ok(StreamingDecoder {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn next_chunk(&mut self) -> Option<Vec<f32>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => return None,
                Err(_) => return None,
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
                Err(_) => continue,
            };

            let spec = *decoded.spec();
            let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            return Some(sample_buf.samples().to_vec());
        }
    }
}

// --- Spectral Centroid ---

pub fn spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    if samples.is_empty() || sample_rate == 0 {
        return 0.0;
    }

    let n = samples.len();
    // Apply Hann window to reduce spectral leakage, which otherwise biases
    // the centroid of low-frequency tones toward higher frequencies.
    let denom = if n > 1 { (n - 1) as f32 } else { 1.0 };
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / denom).cos());
            Complex { re: s * w, im: 0.0 }
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buffer);

    let num_bins = n / 2 + 1;
    let bin_hz = sample_rate as f32 / n as f32;

    let mut weighted_sum = 0.0f32;
    let mut magnitude_sum = 0.0f32;

    for bin in 0..num_bins {
        let freq = bin as f32 * bin_hz;
        if freq < 20.0 || freq > 20000.0 {
            continue;
        }
        let c = buffer[bin];
        let mag = (c.re * c.re + c.im * c.im).sqrt();
        weighted_sum += freq * mag;
        magnitude_sum += mag;
    }

    if magnitude_sum == 0.0 {
        return 0.0;
    }

    let centroid_hz = weighted_sum / magnitude_sum;
    ((centroid_hz - 20.0) / (20000.0 - 20.0)).clamp(0.0, 1.0)
}

// --- Spectral Flux Detector ---

#[derive(Debug)]
pub struct SpectralFluxDetector {
    prev: [f32; 7],
}

impl SpectralFluxDetector {
    pub fn new() -> Self {
        SpectralFluxDetector { prev: [0.0f32; 7] }
    }

    pub fn detect(&mut self, bands: &[f32; 7]) -> f32 {
        let flux = (0..7).map(|i| (bands[i] - self.prev[i]).max(0.0)).sum::<f32>();
        self.prev = *bands;
        flux
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
