// @spec-tags: audio.decode
// @invariants: AudioData struct exposes sample_rate(u32), channels(u16), samples(Vec<f32>); SUPPORTED_FORMATS contains exactly [mp3,wav,flac,ogg]; decode returns Err for missing path or unsupported extension
// @build: 30

use rhythm_grid::audio::{decode, AudioData, SUPPORTED_FORMATS};
use std::path::Path;

/// Writes a minimal valid mono 44100 Hz 16-bit PCM WAV file with `num_samples` silent samples.
fn write_minimal_wav(path: &Path, num_samples: u32) {
    let sample_rate: u32 = 44100;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let data_size = num_samples * 2; // 16-bit = 2 bytes per sample

    let mut bytes: Vec<u8> = Vec::new();
    // RIFF header
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    // fmt chunk
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());      // chunk size
    bytes.extend_from_slice(&1u16.to_le_bytes());        // PCM = 1
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data chunk
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.extend(vec![0u8; data_size as usize]);

    std::fs::write(path, bytes).expect("helper: failed to write test WAV file");
}

#[test]
fn supported_formats_includes_mp3_wav_flac_ogg() {
    let formats: &[&str] = &SUPPORTED_FORMATS;
    assert!(formats.contains(&"mp3"),  "mp3 missing from supported formats");
    assert!(formats.contains(&"wav"),  "wav missing from supported formats");
    assert!(formats.contains(&"flac"), "flac missing from supported formats");
    assert!(formats.contains(&"ogg"),  "ogg missing from supported formats");
    assert_eq!(formats.len(), 4, "expected exactly 4 supported formats, got {}", formats.len());
}

#[test]
fn decoded_audio_has_sample_rate() {
    let path = std::env::temp_dir().join("rg_test_sample_rate_b30.wav");
    write_minimal_wav(&path, 100);
    let result: AudioData = decode(&path).expect("decode should succeed for valid WAV");
    assert!(result.sample_rate > 0, "sample_rate must be non-zero, got {}", result.sample_rate);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn decoded_audio_has_channel_count() {
    let path = std::env::temp_dir().join("rg_test_channels_b30.wav");
    write_minimal_wav(&path, 100);
    let result: AudioData = decode(&path).expect("decode should succeed for valid WAV");
    assert!(result.channels >= 1, "channels must be >= 1, got {}", result.channels);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn decoded_audio_has_samples() {
    let path = std::env::temp_dir().join("rg_test_has_samples_b30.wav");
    write_minimal_wav(&path, 100);
    let result: AudioData = decode(&path).expect("decode should succeed for valid WAV");
    assert!(!result.samples.is_empty(), "samples must be non-empty for valid audio");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn decode_invalid_path_returns_error() {
    let path = Path::new("/nonexistent/path/file_does_not_exist_rg_b30.wav");
    let result = decode(path);
    assert!(result.is_err(), "decoding a nonexistent path must return Err");
}

#[test]
fn decode_unsupported_format_returns_error() {
    let path = std::env::temp_dir().join("rg_test_unsupported_b30.txt");
    std::fs::write(&path, b"not audio data").expect("helper: failed to write .txt file");
    let result = decode(&path);
    assert!(result.is_err(), "decoding a .txt file must return Err (unsupported format)");
    let _ = std::fs::remove_file(&path);
}
