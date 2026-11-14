//! The in-memory sample buffer every decoder produces and every consumer reads.

use crate::error::AudioError;
use crate::resample;

/// Interleaved audio at a known rate.
///
/// Samples are `f32` in the range `[-1.0, 1.0]`; integer formats are scaled on decode so
/// that a full-scale negative sample is exactly `-1.0` and a full-scale positive sample is
/// just under `1.0`. Interleaving keeps a decoded file in one allocation, which matters
/// when a 96 kHz 24-bit BWF is 40 minutes long.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl AudioBuffer {
    /// Build a buffer, rejecting a shape that cannot be indexed.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::NoChannels`] for a zero channel count,
    /// [`AudioError::NoSampleRate`] for a zero rate, and [`AudioError::RaggedFrames`] when
    /// the sample count is not a whole number of frames.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, AudioError> {
        if channels == 0 {
            return Err(AudioError::NoChannels);
        }
        if sample_rate == 0 {
            return Err(AudioError::NoSampleRate);
        }
        if samples.len() % usize::from(channels) != 0 {
            return Err(AudioError::RaggedFrames {
                samples: samples.len(),
                channels,
            });
        }
        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// The sample rate in hertz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The number of frames, which is the sample count divided by the channel count.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// The interleaved samples.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Whether the buffer holds no frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The duration in seconds.
    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }

    /// The largest absolute sample, which the UI shows as a level check.
    #[must_use]
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0_f32, |acc, s| acc.max(s.abs()))
    }

    /// One channel as a contiguous slice of frames.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::NoSuchChannel`] when `index` is past the last channel.
    pub fn channel(&self, index: u16) -> Result<Vec<f32>, AudioError> {
        if index >= self.channels {
            return Err(AudioError::NoSuchChannel {
                index,
                channels: self.channels,
            });
        }
        let stride = usize::from(self.channels);
        Ok(self
            .samples
            .iter()
            .skip(usize::from(index))
            .step_by(stride)
            .copied()
            .collect())
    }

    /// Average every channel into one, which is what the transcriber wants.
    ///
    /// A mono buffer is returned unchanged rather than copied through the averaging path,
    /// so the common case costs nothing.
    #[must_use]
    pub fn downmix(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }
        let stride = usize::from(self.channels);
        let divisor = f32::from(self.channels);
        let mut mono = Vec::with_capacity(self.frames());
        for frame in self.samples.chunks_exact(stride) {
            mono.push(frame.iter().sum::<f32>() / divisor);
        }
        Self {
            sample_rate: self.sample_rate,
            channels: 1,
            samples: mono,
        }
    }

    /// Resample every channel to `target_rate`, keeping the channel layout.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::BadTargetRate`] for a zero target.
    pub fn resample(&self, target_rate: u32) -> Result<Self, AudioError> {
        if target_rate == 0 {
            return Err(AudioError::BadTargetRate(target_rate));
        }
        if target_rate == self.sample_rate {
            return Ok(self.clone());
        }
        let mut out = Vec::with_capacity(
            resample::output_len(self.frames(), self.sample_rate, target_rate)
                * usize::from(self.channels),
        );
        let channels: Vec<Vec<f32>> = (0..self.channels)
            .map(|index| {
                self.channel(index).map(|channel| {
                    resample::resample_channel(&channel, self.sample_rate, target_rate)
                })
            })
            .collect::<Result<_, _>>()?;
        let frames = channels.first().map_or(0, Vec::len);
        for frame in 0..frames {
            for channel in &channels {
                out.push(channel[frame]);
            }
        }
        Self::new(target_rate, self.channels, out)
    }

    /// Downmix to mono and resample to 16 kHz in one step, the transcriber's input shape.
    ///
    /// # Errors
    ///
    /// As [`AudioBuffer::resample`].
    pub fn to_transcription_input(&self) -> Result<Self, AudioError> {
        self.downmix().resample(crate::TRANSCRIPTION_RATE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo() -> AudioBuffer {
        AudioBuffer::new(48_000, 2, vec![0.0, 1.0, 0.5, -1.0, -0.5, 0.25]).expect("buffer")
    }

    #[test]
    fn a_zero_channel_buffer_is_rejected() {
        let error = AudioBuffer::new(48_000, 0, vec![]).expect_err("no channels");

        assert!(matches!(error, AudioError::NoChannels));
    }

    #[test]
    fn a_zero_rate_buffer_is_rejected() {
        let error = AudioBuffer::new(0, 1, vec![0.0]).expect_err("no rate");

        assert!(matches!(error, AudioError::NoSampleRate));
    }

    #[test]
    fn a_ragged_sample_count_is_rejected() {
        let error = AudioBuffer::new(48_000, 2, vec![0.0; 3]).expect_err("ragged");

        assert!(matches!(
            error,
            AudioError::RaggedFrames {
                samples: 3,
                channels: 2
            }
        ));
    }

    #[test]
    fn frames_duration_and_peak_follow_the_samples() {
        let buffer = stereo();

        assert_eq!(buffer.frames(), 3);
        assert_eq!(buffer.channels(), 2);
        assert!((buffer.duration_seconds() - 3.0 / 48_000.0).abs() < 1e-12);
        assert!((buffer.peak() - 1.0).abs() < f32::EPSILON);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn an_empty_buffer_has_no_frames_and_no_peak() {
        let buffer = AudioBuffer::new(16_000, 1, vec![]).expect("buffer");

        assert!(buffer.is_empty());
        assert_eq!(buffer.frames(), 0);
        assert_eq!(buffer.peak(), 0.0);
        assert_eq!(buffer.duration_seconds(), 0.0);
    }

    #[test]
    fn channels_deinterleave_in_order() {
        let buffer = stereo();

        assert_eq!(buffer.channel(0).expect("left"), vec![0.0, 0.5, -0.5]);
        assert_eq!(buffer.channel(1).expect("right"), vec![1.0, -1.0, 0.25]);
    }

    #[test]
    fn a_channel_past_the_last_is_rejected() {
        let error = stereo().channel(2).expect_err("no third channel");

        assert!(matches!(
            error,
            AudioError::NoSuchChannel {
                index: 2,
                channels: 2
            }
        ));
    }

    #[test]
    fn downmix_averages_the_channels() {
        let mono = stereo().downmix();

        assert_eq!(mono.channels(), 1);
        assert_eq!(mono.sample_rate(), 48_000);
        assert_eq!(mono.samples(), &[0.5, -0.25, -0.125]);
    }

    #[test]
    fn downmix_of_mono_is_the_same_buffer() {
        let buffer = AudioBuffer::new(16_000, 1, vec![0.1, -0.2]).expect("buffer");

        assert_eq!(buffer.downmix(), buffer);
    }

    #[test]
    fn resampling_to_the_same_rate_is_a_copy() {
        let buffer = stereo();

        assert_eq!(buffer.resample(48_000).expect("same rate"), buffer);
    }

    #[test]
    fn resampling_to_zero_is_rejected() {
        let error = stereo().resample(0).expect_err("zero target");

        assert!(matches!(error, AudioError::BadTargetRate(0)));
    }

    #[test]
    fn resampling_keeps_the_channel_layout() {
        let buffer = AudioBuffer::new(48_000, 2, vec![0.5; 4_800 * 2]).expect("buffer");

        let out = buffer.resample(16_000).expect("resampled");

        assert_eq!(out.channels(), 2);
        assert_eq!(out.sample_rate(), 16_000);
        assert_eq!(out.frames(), 1_600);
        assert_eq!(out.samples().len(), 3_200);
    }

    #[test]
    fn transcription_input_is_mono_at_sixteen_kilohertz() {
        let buffer = AudioBuffer::new(96_000, 2, vec![0.25; 96_000 * 2]).expect("buffer");

        let out = buffer.to_transcription_input().expect("prepared");

        assert_eq!(out.channels(), 1);
        assert_eq!(out.sample_rate(), crate::TRANSCRIPTION_RATE);
        assert_eq!(out.frames(), 16_000);
        assert!(out.samples().iter().all(|s| (s - 0.25).abs() < 1e-3));
    }
}
