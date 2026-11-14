//! Errors raised while decoding or resampling audio.

use thiserror::Error;

/// Everything that can go wrong turning a file on disk into mono 16 kHz samples.
///
/// Each variant names the file and the reason, because the review UI shows the message
/// next to the row that failed rather than in a modal.
#[derive(Debug, Error)]
pub enum AudioError {
    /// The file could not be read.
    #[error("{path}: {source}")]
    Io {
        /// The path that was read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The file does not start with a RIFF or RF64 container.
    #[error("not a RIFF or RF64 file: magic {0:?}")]
    NotRiff([u8; 4]),

    /// The container is RIFF-shaped but does not hold WAVE data.
    #[error("RIFF container is not WAVE: form type {0:?}")]
    NotWave([u8; 4]),

    /// A required chunk is absent.
    #[error("missing the {0} chunk")]
    MissingChunk(&'static str),

    /// A chunk claims more bytes than the file holds.
    #[error("{chunk} chunk claims {claimed} bytes but only {available} remain")]
    Truncated {
        /// The chunk identifier.
        chunk: &'static str,
        /// The size the chunk declared.
        claimed: u64,
        /// The bytes actually available.
        available: u64,
    },

    /// The sample format is not one this decoder implements.
    #[error("unsupported WAVE format tag {0:#06x}")]
    UnsupportedFormat(u16),

    /// The bit depth is not one this decoder implements for that format.
    #[error("unsupported bit depth {bits} for format tag {format:#06x}")]
    UnsupportedBitDepth {
        /// The format tag the file declared.
        format: u16,
        /// The container width in bits.
        bits: u16,
    },

    /// An RF64 file has no `ds64` chunk, so its 64-bit sizes are unknown.
    #[error("RF64 file has no ds64 chunk")]
    Rf64WithoutDs64,

    /// The declared channel count is zero.
    #[error("channel count is zero")]
    NoChannels,

    /// The declared sample rate is zero.
    #[error("sample rate is zero")]
    NoSampleRate,

    /// The sample count is not a whole number of frames.
    #[error("{samples} samples is not a whole number of {channels}-channel frames")]
    RaggedFrames {
        /// The interleaved sample count.
        samples: usize,
        /// The channel count.
        channels: u16,
    },

    /// A channel index was out of range.
    #[error("channel {index} does not exist in a {channels}-channel buffer")]
    NoSuchChannel {
        /// The requested index.
        index: u16,
        /// The channel count.
        channels: u16,
    },

    /// A resample target rate was zero.
    #[error("cannot resample to {0} Hz")]
    BadTargetRate(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_file_and_the_reason() {
        let error = AudioError::Io {
            path: "take.wav".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };

        assert_eq!(error.to_string(), "take.wav: no such file");
    }

    #[test]
    fn a_truncated_chunk_reports_both_sizes() {
        let error = AudioError::Truncated {
            chunk: "data",
            claimed: 4096,
            available: 12,
        };

        assert_eq!(
            error.to_string(),
            "data chunk claims 4096 bytes but only 12 remain"
        );
    }

    #[test]
    fn a_ragged_frame_count_names_both_numbers() {
        let error = AudioError::RaggedFrames {
            samples: 7,
            channels: 2,
        };

        assert_eq!(
            error.to_string(),
            "7 samples is not a whole number of 2-channel frames"
        );
    }

    #[test]
    fn an_unsupported_bit_depth_names_the_format() {
        let error = AudioError::UnsupportedBitDepth {
            format: 1,
            bits: 12,
        };

        assert_eq!(
            error.to_string(),
            "unsupported bit depth 12 for format tag 0x0001"
        );
    }
}
