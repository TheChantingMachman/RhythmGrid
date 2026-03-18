use std::path::Path;

pub const SUPPORTED_FORMATS: [&str; 4] = ["mp3", "wav", "flac", "ogg"];

pub const DEFAULT_BPM: u32 = 120;

#[derive(Debug, Clone)]
pub struct AudioData {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

pub fn decode(path: &Path) -> Result<AudioData, Box<dyn std::error::Error>> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format.default_track().ok_or("no default audio track")?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let sample_rate = codec_params.sample_rate.ok_or("missing sample rate")?;
    let channels = codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    let mut decoder =
        symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(sample_buf.samples());
    }

    Ok(AudioData {
        sample_rate,
        channels,
        samples,
    })
}

pub fn generate_procedural(bpm: u32) -> AudioData {
    let sample_rate: u32 = 44100;
    let channels: u16 = 1;

    let beat_samples = (60.0 / bpm as f64 * sample_rate as f64) as usize;
    let total_samples = beat_samples * 4;
    let pulse_len = (0.050 * sample_rate as f64) as usize;
    let freq = 440.0_f64;

    let mut samples = vec![0.0_f32; total_samples];

    for beat in 0..4 {
        let onset = beat * beat_samples;
        let end = (onset + pulse_len).min(total_samples);
        for i in onset..end {
            let t = ((i - onset) as f64 + 0.5) / sample_rate as f64;
            samples[i] = (2.0 * std::f64::consts::PI * freq * t).sin() as f32;
        }
    }

    AudioData {
        sample_rate,
        channels,
        samples,
    }
}
