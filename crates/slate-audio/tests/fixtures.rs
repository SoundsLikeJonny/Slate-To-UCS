//! Round-trips the four container shapes a field recorder produces.
//!
//! The fixtures are synthesised here rather than committed as binary blobs: a WAV is a
//! header plus samples, so building one in the test keeps the repository free of opaque
//! files while still exercising the same bytes a recorder writes. Each case asserts the
//! duration and sample rate the review table displays, which is what a decode regression
//! would break first.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use slate_audio::{decode_file, encode_pcm16, parse, AudioBuffer, TRANSCRIPTION_RATE};

/// Build a RIFF container around the given chunks, padding odd lengths.
fn riff(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::from(*b"WAVE");
    for (id, payload) in chunks {
        body.extend_from_slice(id);
        body.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        body.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            body.push(0);
        }
    }
    let mut out = Vec::from(*b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// A `fmt ` chunk for integer PCM.
fn fmt(channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let block_align = channels * bits / 8;
    let mut out = Vec::new();
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out
}

/// A `bext` chunk carrying a slate and a time reference.
fn bext(description: &str, time_reference: u64) -> Vec<u8> {
    let mut out = vec![0_u8; 602];
    out[..description.len()].copy_from_slice(description.as_bytes());
    out[338..346].copy_from_slice(&time_reference.to_le_bytes());
    out
}

#[test]
fn a_forty_eight_kilohertz_stereo_slate_round_trips() {
    let frames = 48_000;
    let mut data = Vec::with_capacity(frames * 4);
    for frame in 0..frames {
        let value = ((frame % 200) as i16 - 100) * 300;
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&(-value).to_le_bytes());
    }
    let bytes = riff(&[(*b"fmt ", fmt(2, 48_000, 16)), (*b"data", data)]);

    let file = parse(&bytes).expect("decoded");

    assert_eq!(file.audio.sample_rate(), 48_000);
    assert_eq!(file.audio.channels(), 2);
    assert_eq!(file.audio.frames(), frames);
    assert!((file.audio.duration_seconds() - 1.0).abs() < 1e-9);
    assert!(file.bwf.is_none());
}

#[test]
fn a_ninety_six_kilohertz_twenty_four_bit_bwf_keeps_its_slate() {
    let frames = 96_000;
    let mut data = Vec::with_capacity(frames * 3);
    for frame in 0..frames {
        let value = (frame % 1_000) as i32 * 100;
        let bytes = value.to_le_bytes();
        data.extend_from_slice(&bytes[..3]);
    }
    let bytes = riff(&[
        (*b"fmt ", fmt(1, 96_000, 24)),
        (*b"bext", bext("INT kitchen, door slam", 96_000)),
        (*b"data", data),
    ]);

    let file = parse(&bytes).expect("decoded");
    let bwf = file.bwf.expect("bext");

    assert_eq!(file.audio.sample_rate(), 96_000);
    assert_eq!(file.audio.frames(), frames);
    assert!((file.audio.duration_seconds() - 1.0).abs() < 1e-9);
    assert_eq!(bwf.description, "INT kitchen, door slam");
    assert_eq!(bwf.time_reference, 96_000);
}

#[test]
fn an_rf64_header_decodes_the_same_audio_as_its_riff_twin() {
    let frames = 4_800;
    let data: Vec<u8> = (0..frames)
        .flat_map(|frame| (((frame % 100) as i16) * 200).to_le_bytes())
        .collect();
    let riff_bytes = riff(&[(*b"fmt ", fmt(1, 48_000, 16)), (*b"data", data.clone())]);

    let mut ds64 = Vec::new();
    ds64.extend_from_slice(&0_u64.to_le_bytes());
    ds64.extend_from_slice(&(data.len() as u64).to_le_bytes());
    ds64.extend_from_slice(&(frames as u64).to_le_bytes());
    ds64.extend_from_slice(&0_u32.to_le_bytes());

    let mut body = Vec::from(*b"WAVE");
    for (id, payload, size) in [
        (*b"ds64", ds64, None),
        (*b"fmt ", fmt(1, 48_000, 16), None),
        (*b"data", data, Some(u32::MAX)),
    ] {
        body.extend_from_slice(&id);
        body.extend_from_slice(&size.unwrap_or(payload.len() as u32).to_le_bytes());
        body.extend_from_slice(&payload);
    }
    let mut rf64 = Vec::from(*b"RF64");
    rf64.extend_from_slice(&u32::MAX.to_le_bytes());
    rf64.extend_from_slice(&body);

    let from_riff = parse(&riff_bytes).expect("riff");
    let from_rf64 = parse(&rf64).expect("rf64");

    assert_eq!(from_riff.audio.samples(), from_rf64.audio.samples());
    assert_eq!(from_rf64.audio.frames(), frames);
}

#[test]
fn a_prepared_file_is_sixteen_kilohertz_mono_and_readable_from_disk() {
    let buffer = AudioBuffer::new(96_000, 2, vec![0.2; 96_000 * 2]).expect("buffer");
    let prepared = buffer.to_transcription_input().expect("prepared");

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("prepared.wav");
    std::fs::write(&path, encode_pcm16(&prepared)).expect("written");
    let file = decode_file(&path).expect("decoded");

    assert_eq!(file.audio.sample_rate(), TRANSCRIPTION_RATE);
    assert_eq!(file.audio.channels(), 1);
    assert_eq!(file.audio.frames(), 16_000);
    assert!((file.audio.duration_seconds() - 1.0).abs() < 1e-9);
    assert!(file.audio.samples().iter().all(|s| (s - 0.2).abs() < 1e-3));
}
