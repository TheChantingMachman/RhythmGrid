// @spec-tags: core,audio
// @invariants: StreamingDecoder::open_bytes decodes in-memory byte slices; sample_rate/channels available after open; next_chunk yields PCM; no UnsupportedFormat guard; invalid data returns DecodeError
// @build: 96

use rhythm_grid::audio::{AudioError, StreamingDecoder};

// ── Helper: build a minimal f32 WAV in memory ────────────────────────────────

fn build_wav_bytes(sample_rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let bytes_per_sample: u32 = 4;
    let num_samples = samples.len() as u32;
    let data_size = num_samples * bytes_per_sample;
    let fmt_size: u32 = 16;
    let riff_size: u32 = 4 + 8 + fmt_size + 8 + data_size;

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&fmt_size.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes()); // IEEE_FLOAT
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * (channels as u32) * bytes_per_sample;
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = (channels as u16) * (bytes_per_sample as u16);
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&32u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

/// Leak a Vec<u8> to obtain a `&'static [u8]` for use with open_bytes.
/// Memory is intentionally leaked — acceptable in test context.
fn leak_bytes(v: Vec<u8>) -> &'static [u8] {
    Box::leak(v.into_boxed_slice())
}

// ── Success: format fields available immediately after open_bytes ─────────────

#[test]
fn open_bytes_wav_returns_correct_sample_rate() {
    let samples: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed for valid WAV data");
    assert_eq!(
        decoder.sample_rate(),
        44100u32,
        "sample_rate must match the WAV header value"
    );
}

#[test]
fn open_bytes_wav_returns_correct_channels() {
    let samples: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
    let data = leak_bytes(build_wav_bytes(22050, 2, &samples));

    let decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed for valid 2-channel WAV data");
    assert_eq!(
        decoder.channels(),
        2u16,
        "channels must match the WAV header value"
    );
}

// ── Success: next_chunk yields PCM data ──────────────────────────────────────

#[test]
fn open_bytes_next_chunk_returns_some_for_nonempty_data() {
    let samples: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let mut decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed");
    let chunk = decoder.next_chunk();
    assert!(
        chunk.is_some(),
        "expected Some(chunk) from non-empty in-memory WAV, got None"
    );
}

#[test]
fn open_bytes_first_chunk_is_nonempty_vec() {
    let samples: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let mut decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed");
    let chunk = decoder.next_chunk().expect("expected at least one chunk");
    assert!(
        !chunk.is_empty(),
        "first chunk must be a non-empty Vec<f32>"
    );
}

#[test]
fn open_bytes_collects_all_samples_total_count_matches() {
    let samples: Vec<f32> = (0..2048).map(|i| i as f32 / 2048.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let mut decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed");

    let mut collected: Vec<f32> = Vec::new();
    while let Some(chunk) = decoder.next_chunk() {
        collected.extend_from_slice(&chunk);
    }

    assert_eq!(
        collected.len(),
        samples.len(),
        "total decoded samples ({}) must match written sample count ({})",
        collected.len(),
        samples.len()
    );
}

#[test]
fn open_bytes_decoded_sample_values_match_written_values() {
    let samples: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let mut decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed");

    let mut collected: Vec<f32> = Vec::new();
    while let Some(chunk) = decoder.next_chunk() {
        collected.extend_from_slice(&chunk);
    }

    assert_eq!(collected.len(), samples.len());
    for (i, (got, expected)) in collected.iter().zip(samples.iter()).enumerate() {
        assert!(
            (got - expected).abs() < 1e-6,
            "sample[{}]: expected {}, got {}",
            i,
            expected,
            got
        );
    }
}

// ── EOF behaviour ─────────────────────────────────────────────────────────────

#[test]
fn open_bytes_returns_none_after_all_chunks_consumed() {
    let samples: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let mut decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed");

    // Drain all chunks
    while let Some(_) = decoder.next_chunk() {}

    let after_eof = decoder.next_chunk();
    assert!(
        after_eof.is_none(),
        "expected None after EOF on in-memory stream, got Some(chunk)"
    );
}

// ── Error: invalid/corrupt bytes return DecodeError (not UnsupportedFormat) ──

#[test]
fn open_bytes_garbage_data_returns_decode_error() {
    // Random bytes that are not a valid WAV — probing will fail.
    static GARBAGE: &[u8] = b"this is not valid audio data at all xxxxxxxxxxxxxxxxxxxx";

    match StreamingDecoder::open_bytes(GARBAGE, "wav") {
        Err(AudioError::DecodeError(_)) => {}
        Err(AudioError::UnsupportedFormat) => panic!(
            "open_bytes must NOT return UnsupportedFormat — \
             it skips format validation and relies on the probe to fail with DecodeError"
        ),
        Err(AudioError::FileNotFound) => panic!(
            "open_bytes operates on in-memory data — FileNotFound is impossible"
        ),
        Err(e) => panic!(
            "expected AudioError::DecodeError for undecodable bytes, got {:?}",
            e
        ),
        Ok(_) => panic!("expected Err for garbage bytes, got Ok"),
    }
}

// ── Extension: bare "wav" (no leading dot) succeeds; verifies caller contract ─

#[test]
fn open_bytes_bare_extension_without_dot_succeeds() {
    // Spec: caller must supply bare extension like "wav", not ".wav".
    // Passing a bare extension to a valid WAV payload must succeed.
    let samples: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
    let data = leak_bytes(build_wav_bytes(44100, 1, &samples));

    let result = StreamingDecoder::open_bytes(data, "wav");
    assert!(
        result.is_ok(),
        "open_bytes with bare extension 'wav' and valid WAV data must return Ok, got {:?}",
        result.err()
    );
}

// ── No UnsupportedFormat guard: unknown extension does not fail fast ──────────

#[test]
fn open_bytes_unknown_extension_does_not_return_unsupported_format() {
    // The spec explicitly states: no format validation against SUPPORTED_FORMATS.
    // So an unknown extension must NOT produce UnsupportedFormat.
    // It will likely produce DecodeError when the prober finds no matching demuxer,
    // but it must not short-circuit with UnsupportedFormat.
    static GARBAGE: &[u8] = b"irrelevant content for unsupported extension probe test xxx";

    match StreamingDecoder::open_bytes(GARBAGE, "xyz") {
        Err(AudioError::UnsupportedFormat) => panic!(
            "open_bytes must NOT return UnsupportedFormat for unknown extensions — \
             no format validation is performed"
        ),
        Err(AudioError::FileNotFound) => panic!(
            "open_bytes operates on in-memory data — FileNotFound is impossible"
        ),
        // DecodeError or Ok are both acceptable outcomes here
        _ => {}
    }
}

// ── Consistent with open(): same struct fields produced ───────────────────────

#[test]
fn open_bytes_with_stereo_48khz_wav_returns_correct_fields() {
    // Verifies that all remaining setup steps (track discovery, format extraction)
    // behave identically to open() — i.e., the full decoder pipeline runs.
    let samples: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    let data = leak_bytes(build_wav_bytes(48000, 2, &samples));

    let decoder = StreamingDecoder::open_bytes(data, "wav")
        .expect("open_bytes should succeed for stereo 48kHz WAV");
    assert_eq!(decoder.sample_rate(), 48000u32);
    assert_eq!(decoder.channels(), 2u16);
}
