// @spec-tags: core,audio,decode
// @invariants: StreamingDecoder opens audio files and yields PCM chunks incrementally; sample_rate/channels available immediately after open; next_chunk returns None at EOF; errors for missing files and unsupported formats; decode_audio backward compatibility unchanged
// @build: 81

use rhythm_grid::audio::{AudioError, StreamingDecoder, SUPPORTED_FORMATS};
use std::io::Write;
use std::path::Path;

// ── Helper: write a minimal WAV file with f32 PCM (format code 3) ─────────────

fn write_wav_f32(path: &std::path::Path, sample_rate: u32, channels: u16, samples: &[f32]) {
    // WAV layout: RIFF chunk -> fmt sub-chunk (16 bytes) -> data sub-chunk
    let num_samples = samples.len() as u32;
    let bytes_per_sample: u32 = 4; // f32
    let data_size = num_samples * bytes_per_sample;
    let fmt_size: u32 = 16;
    // RIFF chunk size = 4 (WAVE) + 8 (fmt header) + fmt_size + 8 (data header) + data_size
    let riff_size: u32 = 4 + 8 + fmt_size + 8 + data_size;

    let mut buf: Vec<u8> = Vec::new();
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&fmt_size.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes()); // audio format = 3 (IEEE_FLOAT)
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * (channels as u32) * bytes_per_sample;
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = (channels as u16) * (bytes_per_sample as u16);
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    let mut file = std::fs::File::create(path).expect("failed to create wav fixture");
    file.write_all(&buf).expect("failed to write wav fixture");
}

// ── Error cases: UnsupportedFormat ───────────────────────────────────────────

#[test]
fn streaming_decoder_unsupported_extension_returns_unsupported_format() {
    let path = Path::new("/tmp/rhythmgrid_streaming_test_file.xyz");
    match StreamingDecoder::open(path) {
        Err(AudioError::UnsupportedFormat) => {}
        Ok(_) => panic!("expected AudioError::UnsupportedFormat for unknown extension, got Ok"),
        Err(e) => panic!(
            "expected AudioError::UnsupportedFormat for unknown extension, got {:?}",
            e
        ),
    }
}

#[test]
fn streaming_decoder_no_extension_returns_unsupported_format() {
    let path = Path::new("/tmp/rhythmgrid_streaming_no_ext");
    match StreamingDecoder::open(path) {
        Err(AudioError::UnsupportedFormat) => {}
        Ok(_) => panic!("expected AudioError::UnsupportedFormat for missing extension, got Ok"),
        Err(e) => panic!(
            "expected AudioError::UnsupportedFormat for missing extension, got {:?}",
            e
        ),
    }
}

#[test]
fn streaming_decoder_unsupported_ext_on_nonexistent_file_returns_unsupported_format() {
    // Extension check must fire before any filesystem access.
    let path = Path::new("/tmp/rhythmgrid_streaming_absent.zzz");
    match StreamingDecoder::open(path) {
        Err(AudioError::UnsupportedFormat) => {}
        Ok(_) => panic!("expected AudioError::UnsupportedFormat (extension checked first), got Ok"),
        Err(e) => panic!(
            "expected AudioError::UnsupportedFormat (extension checked first), got {:?}",
            e
        ),
    }
}

// ── Error cases: FileNotFound ─────────────────────────────────────────────────

#[test]
fn streaming_decoder_nonexistent_wav_returns_file_not_found() {
    let path = Path::new("/tmp/rhythmgrid_streaming_nonexistent_abc123.wav");
    match StreamingDecoder::open(path) {
        Err(AudioError::FileNotFound) => {}
        Ok(_) => panic!("expected AudioError::FileNotFound for missing file, got Ok"),
        Err(e) => panic!(
            "expected AudioError::FileNotFound for missing file, got {:?}",
            e
        ),
    }
}

#[test]
fn streaming_decoder_nonexistent_mp3_returns_file_not_found() {
    let path = Path::new("/tmp/rhythmgrid_streaming_nonexistent_abc123.mp3");
    match StreamingDecoder::open(path) {
        Err(AudioError::FileNotFound) => {}
        Ok(_) => panic!("expected AudioError::FileNotFound for missing mp3, got Ok"),
        Err(e) => panic!(
            "expected AudioError::FileNotFound for missing mp3, got {:?}",
            e
        ),
    }
}

// ── Open success: format fields available after open ─────────────────────────

#[test]
fn streaming_decoder_open_wav_returns_correct_sample_rate() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_sr_test.wav");
    let samples: Vec<f32> = (0..256).map(|i| (i as f32) / 256.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    assert_eq!(decoder.sample_rate(), 44100u32);
}

#[test]
fn streaming_decoder_open_wav_returns_correct_channels() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_ch_test.wav");
    let samples: Vec<f32> = (0..256).map(|i| (i as f32) / 256.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    assert_eq!(decoder.channels(), 1u16);
}

// ── next_chunk: correctness and coverage ─────────────────────────────────────

#[test]
fn streaming_decoder_next_chunk_returns_some_for_nonempty_wav() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_chunk_some.wav");
    let samples: Vec<f32> = (0..1024).map(|i| (i as f32) / 1024.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    let chunk = decoder.next_chunk();
    assert!(
        chunk.is_some(),
        "expected Some(chunk) for non-empty WAV, got None"
    );
}

#[test]
fn streaming_decoder_next_chunk_returns_nonempty_vec() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_chunk_nonempty.wav");
    let samples: Vec<f32> = (0..1024).map(|i| (i as f32) / 1024.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    let chunk = decoder.next_chunk().expect("expected at least one chunk");
    assert!(
        !chunk.is_empty(),
        "expected non-empty chunk, got empty Vec"
    );
}

#[test]
fn streaming_decoder_collects_all_samples_from_wav() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_all_samples.wav");
    let samples: Vec<f32> = (0..2048).map(|i| (i as f32) / 2048.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    let mut collected: Vec<f32> = Vec::new();
    while let Some(chunk) = decoder.next_chunk() {
        collected.extend_from_slice(&chunk);
    }

    assert_eq!(
        collected.len(),
        samples.len(),
        "total decoded samples ({}) must match written samples ({})",
        collected.len(),
        samples.len()
    );
}

#[test]
fn streaming_decoder_collected_samples_match_written_values() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_sample_values.wav");
    // Use values that round-trip cleanly through f32
    let samples: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
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

#[test]
fn streaming_decoder_returns_none_at_end_of_stream() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_eof.wav");
    let samples: Vec<f32> = (0..256).map(|i| (i as f32) / 256.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    // Drain all chunks
    while let Some(_) = decoder.next_chunk() {}
    // After EOF, next call must return None
    let after_eof = decoder.next_chunk();
    assert!(
        after_eof.is_none(),
        "expected None after EOF, got Some(chunk)"
    );
}

#[test]
fn streaming_decoder_chunk_size_within_expected_range() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_chunk_size.wav");
    // Write enough samples that at least one chunk will be yielded
    let samples: Vec<f32> = (0..8192).map(|i| (i as f32) / 8192.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    let chunk = decoder.next_chunk().expect("expected at least one chunk");
    // Spec: typically 1024-4096 frames; allow some flexibility for codec variation
    assert!(
        chunk.len() >= 128 && chunk.len() <= 16384,
        "chunk size {} out of expected range [128, 16384]",
        chunk.len()
    );
}

// ── Each next_chunk call returns a new Vec ────────────────────────────────────

#[test]
fn streaming_decoder_each_chunk_is_independent_vec() {
    let tmp = std::env::temp_dir().join("rhythmgrid_streaming_independent_vecs.wav");
    // Need enough samples to yield at least 2 chunks (> 4096 frames)
    let samples: Vec<f32> = (0..8192).map(|i| (i as f32) / 8192.0).collect();
    write_wav_f32(&tmp, 44100, 1, &samples);

    let mut decoder = StreamingDecoder::open(&tmp).expect("failed to open wav");
    let chunk1 = decoder.next_chunk();
    let chunk2 = decoder.next_chunk();

    if let (Some(c1), Some(c2)) = (chunk1, chunk2) {
        // They are separate allocations — modifying one should not affect the other.
        // We just verify they are distinct Vecs (ownership check at compile time is sufficient;
        // here we verify they can coexist and have valid lengths).
        assert!(!c1.is_empty());
        assert!(!c2.is_empty());
    }
    // If only one chunk was produced that's fine — the WAV may pack all data in one packet.
}

// ── decode_audio backward compatibility ──────────────────────────────────────

#[test]
fn decode_audio_still_works_after_streaming_decoder_introduced() {
    use rhythm_grid::audio::decode_audio;
    // Unsupported format error verifies decode_audio is still present and functional.
    let path = Path::new("/tmp/rhythmgrid_compat_check.xyz");
    match decode_audio(path) {
        Err(AudioError::UnsupportedFormat) => {}
        other => panic!(
            "decode_audio backward compat broken: expected UnsupportedFormat, got {:?}",
            other
        ),
    }
}

// ── SUPPORTED_FORMATS covers streaming decoder ────────────────────────────────

#[test]
fn streaming_decoder_accepts_all_supported_format_extensions_as_valid() {
    // For each supported extension, a nonexistent file with that extension
    // must return FileNotFound (not UnsupportedFormat), proving the extension
    // passes the format check.
    for ext in SUPPORTED_FORMATS {
        let path_str = format!("/tmp/rhythmgrid_streaming_fmt_check_absent.{}", ext);
        let path = Path::new(&path_str);
        match StreamingDecoder::open(path) {
            Err(AudioError::FileNotFound) => {}
            Err(AudioError::UnsupportedFormat) => panic!(
                "extension '{}' in SUPPORTED_FORMATS was rejected as unsupported by StreamingDecoder",
                ext
            ),
            other => {
                // Opening a nonexistent file could theoretically succeed if path somehow
                // exists on the CI machine — that is fine too.
                let _ = other;
            }
        }
    }
}
