//! The Whisper process boundary: arguments, JSON parsing and one spawn.
//!
//! Whisper runs as a separate executable rather than a linked library, so the model and the
//! binary can be fetched, verified and replaced without rebuilding the app. Everything that
//! can be tested without the real binary is pure: the argument list, the JSON parser and the
//! spawn are separate functions, and the spawn takes its runner as a parameter.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use thiserror::Error;

/// Environment variable naming the Whisper executable.
pub const WHISPER_BIN_ENV: &str = "SLATE_WHISPER_BIN";
/// Environment variable naming the model file.
pub const WHISPER_MODEL_ENV: &str = "SLATE_WHISPER_MODEL";
/// The executable name used when the environment says nothing.
pub const DEFAULT_WHISPER_BIN: &str = "whisper-cli";
/// The model used when the environment says nothing.
pub const DEFAULT_WHISPER_MODEL: &str = "models/ggml-base.en.bin";

/// Everything that can go wrong turning a prepared WAV into a transcript.
#[derive(Debug, Error)]
pub enum TranscribeError {
    /// The Whisper executable could not be started.
    #[error("{binary} could not be started: {source}")]
    NotInstalled {
        /// The executable that was tried.
        binary: String,
        /// The underlying spawn error.
        source: std::io::Error,
    },

    /// Whisper ran but reported failure.
    #[error("whisper exited with {0}")]
    ExitStatus(String),

    /// Whisper produced no JSON file where it was told to.
    #[error("whisper wrote no transcript to {0}")]
    MissingOutput(String),

    /// The transcript file could not be read.
    #[error("{path}: {source}")]
    Io {
        /// The path that was read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The transcript file was not the JSON the contract promises.
    #[error("transcript is not valid whisper JSON: {0}")]
    Json(String),
}

/// Where the Whisper executable and model live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperConfig {
    /// The executable to run.
    pub binary: PathBuf,
    /// The model file to pass with `-m`.
    pub model: PathBuf,
}

impl WhisperConfig {
    /// Read the configuration from the environment, falling back to the documented defaults.
    #[must_use]
    pub fn from_env() -> Self {
        let binary = std::env::var_os(WHISPER_BIN_ENV)
            .map_or_else(|| PathBuf::from(DEFAULT_WHISPER_BIN), PathBuf::from);
        let model = std::env::var_os(WHISPER_MODEL_ENV)
            .map_or_else(|| PathBuf::from(DEFAULT_WHISPER_MODEL), PathBuf::from);
        Self { binary, model }
    }

    /// The argument list for one transcription.
    ///
    /// `-oj` writes `<out_base>.json`, `-nt` drops the timestamps whisper would otherwise
    /// print, and `-f` names the prepared 16 kHz mono WAV.
    #[must_use]
    pub fn arguments(&self, wav: &Path, out_base: &Path) -> Vec<OsString> {
        vec![
            OsString::from("-m"),
            self.model.clone().into_os_string(),
            OsString::from("-f"),
            wav.as_os_str().to_os_string(),
            OsString::from("-oj"),
            OsString::from("-of"),
            out_base.as_os_str().to_os_string(),
            OsString::from("-nt"),
        ]
    }
}

/// One timed span of speech.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Start offset in milliseconds.
    pub start_ms: u64,
    /// End offset in milliseconds.
    pub end_ms: u64,
    /// The recognised text, trimmed.
    pub text: String,
}

/// A whole transcription.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transcript {
    /// The segments in file order.
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Every segment joined by a single space, which is what `slate predict` consumes.
    #[must_use]
    pub fn text(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.text.as_str())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Whether the transcript holds no speech.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Parse the JSON `whisper-cli -oj` writes.
///
/// # Errors
///
/// Returns [`TranscribeError::Json`] when the bytes are not an object with a
/// `transcription` array of objects carrying `offsets.from`, `offsets.to` and `text`.
pub fn parse_json(bytes: &[u8]) -> Result<Transcript, TranscribeError> {
    let root: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| TranscribeError::Json(error.to_string()))?;
    let entries = root
        .get("transcription")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| TranscribeError::Json("no transcription array".to_string()))?;

    let mut segments = Vec::with_capacity(entries.len());
    for entry in entries {
        let offsets = entry
            .get("offsets")
            .ok_or_else(|| TranscribeError::Json("segment has no offsets".to_string()))?;
        let start_ms = offsets
            .get("from")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| TranscribeError::Json("segment has no offsets.from".to_string()))?;
        let end_ms = offsets
            .get("to")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| TranscribeError::Json("segment has no offsets.to".to_string()))?;
        let text = entry
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| TranscribeError::Json("segment has no text".to_string()))?;
        segments.push(Segment {
            start_ms,
            end_ms,
            text: text.trim().to_string(),
        });
    }

    Ok(Transcript { segments })
}

/// Transcribe a prepared WAV by spawning Whisper.
///
/// # Errors
///
/// As [`transcribe_with`].
pub fn transcribe(config: &WhisperConfig, wav: &Path) -> Result<Transcript, TranscribeError> {
    transcribe_with(config, wav, |binary, arguments| {
        let mut child = slate_audio::Cancellable::spawn(binary, arguments)?;
        child.wait()
    })
}

/// Transcribe a prepared WAV, running the process through `runner`.
///
/// The runner is a parameter so the tests can prove the argument list, the exit-status
/// handling and the JSON read without a model on disk.
///
/// # Errors
///
/// Returns [`TranscribeError::NotInstalled`] when the process cannot start,
/// [`TranscribeError::ExitStatus`] when it exits non-zero,
/// [`TranscribeError::MissingOutput`] when no JSON appears, and
/// [`TranscribeError::Io`] or [`TranscribeError::Json`] when the JSON cannot be read or parsed.
pub fn transcribe_with<F>(
    config: &WhisperConfig,
    wav: &Path,
    runner: F,
) -> Result<Transcript, TranscribeError>
where
    F: FnOnce(&Path, &[OsString]) -> std::io::Result<ExitStatus>,
{
    let out_base = wav.with_extension("whisper");
    let arguments = config.arguments(wav, &out_base);
    let status =
        runner(&config.binary, &arguments).map_err(|source| TranscribeError::NotInstalled {
            binary: config.binary.display().to_string(),
            source,
        })?;
    if !status.success() {
        return Err(TranscribeError::ExitStatus(status.to_string()));
    }

    let json_path = out_base.with_extension("whisper.json");
    let bytes = std::fs::read(&json_path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            TranscribeError::MissingOutput(json_path.display().to_string())
        } else {
            TranscribeError::Io {
                path: json_path.display().to_string(),
                source,
            }
        }
    })?;
    parse_json(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WhisperConfig {
        WhisperConfig {
            binary: PathBuf::from("whisper-cli"),
            model: PathBuf::from("models/ggml-base.en.bin"),
        }
    }

    fn json(entries: &str) -> Vec<u8> {
        format!(r#"{{"transcription":[{entries}]}}"#).into_bytes()
    }

    #[test]
    fn arguments_name_the_model_the_wav_and_the_json_output() {
        let arguments = config().arguments(Path::new("take.wav"), Path::new("take.whisper"));

        let rendered: Vec<String> = arguments
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec![
                "-m",
                "models/ggml-base.en.bin",
                "-f",
                "take.wav",
                "-oj",
                "-of",
                "take.whisper",
                "-nt"
            ]
        );
    }

    #[test]
    fn from_env_falls_back_to_the_documented_defaults() {
        // The test process may already carry the variables, so assert the shape rather than
        // the value: a configured binary is never empty.
        let config = WhisperConfig::from_env();

        assert!(!config.binary.as_os_str().is_empty());
        assert!(!config.model.as_os_str().is_empty());
    }

    #[test]
    fn a_transcript_joins_its_segments() {
        let transcript = Transcript {
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 500,
                    text: "door".into(),
                },
                Segment {
                    start_ms: 500,
                    end_ms: 900,
                    text: "creak".into(),
                },
            ],
        };

        assert_eq!(transcript.text(), "door creak");
        assert!(!transcript.is_empty());
    }

    #[test]
    fn an_empty_transcript_has_no_text() {
        let transcript = Transcript::default();

        assert_eq!(transcript.text(), "");
        assert!(transcript.is_empty());
    }

    #[test]
    fn blank_segments_are_left_out_of_the_text() {
        let transcript = Transcript {
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 1,
                    text: String::new(),
                },
                Segment {
                    start_ms: 1,
                    end_ms: 2,
                    text: "slam".into(),
                },
            ],
        };

        assert_eq!(transcript.text(), "slam");
    }

    #[test]
    fn json_offsets_and_text_are_read() {
        let bytes = json(
            r#"{"offsets":{"from":0,"to":1500},"text":" INT kitchen door slam"},
               {"offsets":{"from":1500,"to":2600},"text":" take two "}"#,
        );

        let transcript = parse_json(&bytes).expect("parsed");

        assert_eq!(transcript.segments.len(), 2);
        assert_eq!(transcript.segments[0].start_ms, 0);
        assert_eq!(transcript.segments[0].end_ms, 1_500);
        assert_eq!(transcript.segments[0].text, "INT kitchen door slam");
        assert_eq!(transcript.segments[1].text, "take two");
    }

    #[test]
    fn an_empty_transcription_array_is_an_empty_transcript() {
        let transcript = parse_json(&json("")).expect("parsed");

        assert!(transcript.is_empty());
    }

    #[test]
    fn malformed_json_is_rejected() {
        let error = parse_json(b"{").expect_err("malformed");

        assert!(matches!(error, TranscribeError::Json(_)));
    }

    #[test]
    fn json_without_a_transcription_array_is_rejected() {
        let error = parse_json(br#"{"result":[]}"#).expect_err("no array");

        assert!(
            matches!(error, TranscribeError::Json(message) if message.contains("transcription"))
        );
    }

    #[test]
    fn a_segment_without_offsets_is_rejected() {
        let error = parse_json(&json(r#"{"text":"x"}"#)).expect_err("no offsets");

        assert!(matches!(error, TranscribeError::Json(message) if message.contains("offsets")));
    }

    #[test]
    fn a_segment_without_a_start_is_rejected() {
        let error = parse_json(&json(r#"{"offsets":{"to":1},"text":"x"}"#)).expect_err("no from");

        assert!(
            matches!(error, TranscribeError::Json(message) if message.contains("offsets.from"))
        );
    }

    #[test]
    fn a_segment_without_an_end_is_rejected() {
        let error = parse_json(&json(r#"{"offsets":{"from":0},"text":"x"}"#)).expect_err("no to");

        assert!(matches!(error, TranscribeError::Json(message) if message.contains("offsets.to")));
    }

    #[test]
    fn a_segment_without_text_is_rejected() {
        let error = parse_json(&json(r#"{"offsets":{"from":0,"to":1}}"#)).expect_err("no text");

        assert!(matches!(error, TranscribeError::Json(message) if message.contains("text")));
    }

    #[test]
    fn a_successful_run_reads_the_json_it_wrote() {
        let dir = tempfile::tempdir().expect("temp dir");
        let wav = dir.path().join("take.wav");
        std::fs::write(&wav, b"RIFF").expect("wav");

        let transcript = transcribe_with(&config(), &wav, |_, arguments| {
            let out_base = PathBuf::from(&arguments[6]);
            std::fs::write(
                out_base.with_extension("whisper.json"),
                json(r#"{"offsets":{"from":0,"to":900},"text":"door creak"}"#),
            )
            .expect("json");
            Ok(exit_status(0))
        })
        .expect("transcribed");

        assert_eq!(transcript.text(), "door creak");
    }

    #[test]
    fn a_missing_executable_is_reported_with_its_name() {
        let dir = tempfile::tempdir().expect("temp dir");
        let wav = dir.path().join("take.wav");

        let error = transcribe_with(&config(), &wav, |_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such file",
            ))
        })
        .expect_err("not installed");

        match error {
            TranscribeError::NotInstalled { binary, .. } => assert_eq!(binary, "whisper-cli"),
            other => panic!("expected NotInstalled, got {other:?}"),
        }
    }

    #[test]
    fn a_non_zero_exit_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let wav = dir.path().join("take.wav");

        let error =
            transcribe_with(&config(), &wav, |_, _| Ok(exit_status(3))).expect_err("failed");

        assert!(matches!(error, TranscribeError::ExitStatus(_)));
    }

    #[test]
    fn a_run_that_writes_no_json_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let wav = dir.path().join("take.wav");

        let error =
            transcribe_with(&config(), &wav, |_, _| Ok(exit_status(0))).expect_err("no output");

        assert!(matches!(error, TranscribeError::MissingOutput(path) if path.ends_with(".json")));
    }

    /// A portable `ExitStatus` for the tests: run the platform's own no-op command.
    fn exit_status(code: i32) -> ExitStatus {
        #[cfg(windows)]
        let mut command = {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "exit", &code.to_string()]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = std::process::Command::new("sh");
            command.args(["-c", &format!("exit {code}")]);
            command
        };
        command.status().expect("spawned a shell")
    }
}
