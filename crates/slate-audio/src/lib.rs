//! WAV, BWF and RF64 decoding plus resampling for the transcription path.
//!
//! Every file the review queue accepts passes through here before transcription: the
//! container is decoded, the channels are averaged, and the result is resampled to the
//! 16 kHz mono that Whisper expects.
//!
//! ```
//! use slate_audio::{AudioBuffer, TRANSCRIPTION_RATE};
//!
//! let buffer = AudioBuffer::new(48_000, 2, vec![0.5; 48_000 * 2]).expect("buffer");
//! let prepared = buffer.to_transcription_input().expect("prepared");
//!
//! assert_eq!(prepared.channels(), 1);
//! assert_eq!(prepared.sample_rate(), TRANSCRIPTION_RATE);
//! assert_eq!(prepared.frames(), 16_000);
//! ```

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod buffer;
mod error;
mod resample;
mod wav;

pub use buffer::AudioBuffer;
pub use error::AudioError;
pub use resample::{output_len, resample_channel};
pub use wav::{decode_file, encode_pcm16, parse, write_pcm16, BwfMetadata, WavFile};

/// The rate Whisper models are trained on, and the only rate the transcriber accepts.
pub const TRANSCRIPTION_RATE: u32 = 16_000;
