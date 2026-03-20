// @spec-tags: core,audio,decode
// @invariants: decode_audio decodes MP3/WAV/FLAC/OGG to PCM; returns FileNotFound for missing files, UnsupportedFormat for unknown extensions; SUPPORTED_FORMATS = ["mp3","wav","flac","ogg"]; DecodedAudio has sample_rate:u32, channels:u16, samples:Vec<f32>
// @build: 33

use rhythm_grid::audio::{decode_audio, AudioError, DecodedAudio, SUPPORTED_FORMATS};
use std::path::Path;

// ── SUPPORTED_FORMATS constant ───────────────────────────────────────────────

#[test]
fn supported_formats_contains_mp3() {
    assert!(SUPPORTED_FORMATS.contains(&"mp3"));
}

#[test]
fn supported_formats_contains_wav() {
    assert!(SUPPORTED_FORMATS.contains(&"wav"));
}

#[test]
fn supported_formats_contains_flac() {
    assert!(SUPPORTED_FORMATS.contains(&"flac"));
}

#[test]
fn supported_formats_contains_ogg() {
    assert!(SUPPORTED_FORMATS.contains(&"ogg"));
}

#[test]
fn supported_formats_has_exactly_four_entries() {
    assert_eq!(SUPPORTED_FORMATS.len(), 4);
}

// ── AudioError variants ───────────────────────────────────────────────────────

#[test]
fn decode_audio_nonexistent_file_returns_file_not_found() {
    let path = Path::new("/tmp/rhythmgrid_test_nonexistent_file_abc123.wav");
    let result = decode_audio(path);
    assert!(result.is_err());
    match result {
        Err(AudioError::FileNotFound) => {}
        other => panic!("expected AudioError::FileNotFound, got {:?}", other),
    }
}

#[test]
fn decode_audio_unsupported_extension_returns_unsupported_format() {
    let path = Path::new("/tmp/test_audio.xyz");
    let result = decode_audio(path);
    assert!(result.is_err());
    match result {
        Err(AudioError::UnsupportedFormat) => {}
        other => panic!("expected AudioError::UnsupportedFormat, got {:?}", other),
    }
}

#[test]
fn decode_audio_no_extension_returns_unsupported_format() {
    let path = Path::new("/tmp/audiofile_no_ext");
    let result = decode_audio(path);
    assert!(result.is_err());
    match result {
        Err(AudioError::UnsupportedFormat) => {}
        other => panic!("expected AudioError::UnsupportedFormat, got {:?}", other),
    }
}

// ── DecodedAudio struct fields ────────────────────────────────────────────────

#[test]
fn decoded_audio_has_sample_rate_field() {
    // Compile-time check: DecodedAudio must have sample_rate: u32.
    let _: u32 = DecodedAudio {
        sample_rate: 44100,
        channels: 2,
        samples: vec![0.0f32],
    }
    .sample_rate;
}

#[test]
fn decoded_audio_has_channels_field() {
    let _: u16 = DecodedAudio {
        sample_rate: 44100,
        channels: 2,
        samples: vec![0.0f32],
    }
    .channels;
}

#[test]
fn decoded_audio_has_samples_field() {
    let audio = DecodedAudio {
        sample_rate: 44100,
        channels: 1,
        samples: vec![0.5f32, -0.5f32],
    };
    assert_eq!(audio.samples.len(), 2);
}

// ── Unsupported-format check precedes file-existence check ───────────────────

#[test]
fn decode_audio_unsupported_ext_on_nonexistent_file_returns_unsupported_format() {
    // Extension check should fire before any filesystem access.
    let path = Path::new("/tmp/rhythmgrid_definitely_absent_file.zzz");
    match decode_audio(path) {
        Err(AudioError::UnsupportedFormat) => {}
        other => panic!(
            "expected AudioError::UnsupportedFormat for unknown extension, got {:?}",
            other
        ),
    }
}
