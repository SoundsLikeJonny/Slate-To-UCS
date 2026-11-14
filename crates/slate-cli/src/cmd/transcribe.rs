//! `slate transcribe`: audio file in, timed transcript out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use clap::Args;
use slate_audio::{AudioBuffer, AudioError};
use thiserror::Error;

use crate::whisper::{self, TranscribeError, WhisperConfig};
use crate::{EXIT_ENGINE, EXIT_USER};

/// Counter that keeps two prepared files in one process from colliding.
static PREPARED_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Arguments for `slate transcribe`.
#[derive(Debug, Args)]
pub struct TranscribeArgs {
    /// The audio file to transcribe: WAV, BWF or RF64.
    pub path: PathBuf,

    /// Emit JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

/// A temporary 16 kHz mono WAV that removes itself when dropped.
///
/// Whisper reads a path, not a buffer, so the prepared audio has to touch the disk. The
/// guard means a failed transcription cannot leave a stray file in the temp directory.
#[derive(Debug)]
pub struct PreparedWav {
    path: PathBuf,
}

impl PreparedWav {
    /// Write `buffer` to a uniquely named file in the system temp directory.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::Io`] when the file cannot be written.
    pub fn create(buffer: &AudioBuffer) -> Result<Self, AudioError> {
        let unique = PREPARED_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("slate-{}-{unique}.wav", std::process::id()));
        slate_audio::write_pcm16(&path, buffer)?;
        Ok(Self { path })
    }

    /// The path Whisper should read.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PreparedWav {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Why a transcription run failed.
#[derive(Debug, Error)]
pub enum CommandError {
    /// The audio file could not be decoded.
    #[error("{0}")]
    Audio(#[from] AudioError),

    /// Whisper could not run or its output could not be read.
    #[error("{0}")]
    Transcribe(#[from] TranscribeError),
}

impl CommandError {
    /// The documented exit code for this failure.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Transcribe(TranscribeError::NotInstalled { .. }) => EXIT_ENGINE,
            Self::Audio(_) | Self::Transcribe(_) => EXIT_USER,
        }
    }
}

/// Run `slate transcribe` and return the process exit code.
pub fn run(args: &TranscribeArgs) -> u8 {
    match transcribe(args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("slate: {error}");
            error.exit_code()
        }
    }
}

fn transcribe(args: &TranscribeArgs) -> Result<u8, CommandError> {
    let file = slate_audio::decode_file(&args.path)?;
    let prepared = PreparedWav::create(&file.audio.to_transcription_input()?)?;
    let transcript = whisper::transcribe(&WhisperConfig::from_env(), prepared.path())?;

    if args.json {
        let segments: Vec<_> = transcript
            .segments
            .iter()
            .map(|segment| {
                serde_json::json!({
                    "start_ms": segment.start_ms,
                    "end_ms": segment.end_ms,
                    "text": segment.text,
                })
            })
            .collect();
        let document = serde_json::json!({
            "text": transcript.text(),
            "segments": segments,
        });
        println!("{document}");
    } else if transcript.is_empty() {
        eprintln!("slate: no speech recognised in {}", args.path.display());
    } else {
        for segment in &transcript.segments {
            println!(
                "{:>8}  {:>8}  {}",
                format_timestamp(segment.start_ms),
                format_timestamp(segment.end_ms),
                segment.text
            );
        }
    }

    Ok(0)
}

/// Render milliseconds as `MM:SS.mmm`, which is how the review table shows a span.
fn format_timestamp(milliseconds: u64) -> String {
    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1_000;
    let millis = milliseconds % 1_000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_render_as_minutes_seconds_and_millis() {
        assert_eq!(format_timestamp(0), "00:00.000");
        assert_eq!(format_timestamp(1_500), "00:01.500");
        assert_eq!(format_timestamp(61_001), "01:01.001");
        assert_eq!(format_timestamp(3_600_000), "60:00.000");
    }

    #[test]
    fn a_prepared_file_is_written_and_then_removed() {
        let buffer = AudioBuffer::new(16_000, 1, vec![0.1; 160]).expect("buffer");

        let path = {
            let prepared = PreparedWav::create(&buffer).expect("prepared");
            let path = prepared.path().to_path_buf();
            assert!(path.is_file(), "{} should exist", path.display());
            let decoded = slate_audio::decode_file(&path).expect("decoded");
            assert_eq!(decoded.audio.frames(), 160);
            path
        };

        assert!(!path.exists(), "{} should be removed", path.display());
    }

    #[test]
    fn two_prepared_files_get_different_names() {
        let buffer = AudioBuffer::new(16_000, 1, vec![0.0; 16]).expect("buffer");

        let first = PreparedWav::create(&buffer).expect("first");
        let second = PreparedWav::create(&buffer).expect("second");

        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn an_audio_failure_maps_to_exit_one() {
        let error = CommandError::Audio(AudioError::NoChannels);

        assert_eq!(error.exit_code(), EXIT_USER);
    }

    #[test]
    fn a_missing_whisper_maps_to_exit_two() {
        let error = CommandError::Transcribe(TranscribeError::NotInstalled {
            binary: "whisper-cli".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        });

        assert_eq!(error.exit_code(), EXIT_ENGINE);
    }

    #[test]
    fn a_bad_transcript_maps_to_exit_one() {
        let error = CommandError::Transcribe(TranscribeError::Json("bad".into()));

        assert_eq!(error.exit_code(), EXIT_USER);
    }

    #[test]
    fn a_missing_input_file_is_reported_as_an_audio_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let args = TranscribeArgs {
            path: dir.path().join("absent.wav"),
            json: false,
        };

        let error = transcribe(&args).expect_err("missing file");

        assert!(matches!(error, CommandError::Audio(AudioError::Io { .. })));
    }
}
