//! Band-limited resampling with a Kaiser-windowed sinc kernel.
//!
//! Whisper wants 16 kHz mono. Recordings arrive at 44.1, 48, 96 or 192 kHz, so every file
//! crosses this module before transcription. The kernel is evaluated per output sample
//! rather than precomputed into a polyphase table: a 40-minute file is resampled once, and
//! the simpler code is easier to prove correct against the tests below.
//!
//! The weights are normalised by their own sum, which keeps a constant signal constant and
//! stops the truncated kernel from attenuating the first and last few milliseconds.

/// Zero crossings on each side of the kernel at unity cutoff.
///
/// Sixteen is the usual trade-off for 16-bit material: enough stopband rejection that a
/// 48 kHz to 16 kHz conversion does not fold audible content back into the passband, and
/// short enough that a long file resamples in seconds.
const ZERO_CROSSINGS: f64 = 16.0;

/// Kaiser window shape. 8.6 gives roughly 80 dB of stopband attenuation.
const KAISER_BETA: f64 = 8.6;

/// The output length for `input_len` frames moving from `from` to `to` hertz.
///
/// Rounds to the nearest frame so a 44.1 kHz file does not lose a frame to truncation.
#[must_use]
pub fn output_len(input_len: usize, from: u32, to: u32) -> usize {
    if input_len == 0 || from == 0 || to == 0 || from == to {
        return input_len;
    }
    let ratio = f64::from(to) / f64::from(from);
    (input_len as f64 * ratio).round() as usize
}

/// Resample one channel.
///
/// Returns the input unchanged when the rates match, and an empty vector for an empty
/// input or a zero rate, so callers never have to special-case those.
#[must_use]
pub fn resample_channel(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if input.is_empty() || from == 0 || to == 0 {
        return Vec::new();
    }
    if from == to {
        return input.to_vec();
    }

    let ratio = f64::from(to) / f64::from(from);
    let out_len = output_len(input.len(), from, to);
    let cutoff = ratio.min(1.0) * 0.95;
    // Widen the kernel as the cutoff falls so the transition band stays the same width in
    // hertz; without this a 192 kHz to 16 kHz conversion aliases badly.
    let half = (ZERO_CROSSINGS / cutoff).ceil() as isize;
    let mut out = Vec::with_capacity(out_len);

    for index in 0..out_len {
        let position = index as f64 / ratio;
        let base = position.floor() as isize;
        let mut weighted = 0.0_f64;
        let mut weight_sum = 0.0_f64;

        for offset in -half..=half {
            let source = base + offset;
            let distance = position - source as f64;
            let weight = kernel(distance, cutoff, half as f64);
            // Only taps that land inside the input contribute, and only their weights count
            // towards the normaliser. Summing the out-of-range weights too would attenuate
            // the first and last few milliseconds of every file.
            if let Some(sample) = usize::try_from(source)
                .ok()
                .and_then(|i| input.get(i))
                .copied()
            {
                weighted += f64::from(sample) * weight;
                weight_sum += weight;
            }
        }

        let value = if weight_sum.abs() > 1e-12 {
            weighted / weight_sum
        } else {
            0.0
        };
        out.push(value as f32);
    }

    out
}

/// The windowed sinc weight at `distance` input samples from the kernel centre.
fn kernel(distance: f64, cutoff: f64, half: f64) -> f64 {
    let window = kaiser(distance / half);
    if window == 0.0 {
        return 0.0;
    }
    sinc(distance * cutoff) * window
}

/// The normalised sinc function, `sin(pi x) / (pi x)`, with the removable singularity at zero.
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let pi_x = std::f64::consts::PI * x;
        pi_x.sin() / pi_x
    }
}

/// The Kaiser window over `[-1, 1]`, zero outside.
fn kaiser(u: f64) -> f64 {
    if u.abs() > 1.0 {
        return 0.0;
    }
    let argument = KAISER_BETA * (1.0 - u * u).max(0.0).sqrt();
    bessel_i0(argument) / bessel_i0(KAISER_BETA)
}

/// The zeroth-order modified Bessel function of the first kind, by its power series.
///
/// The series converges quickly for the arguments a window needs, and avoids pulling in a
/// numerics dependency for one function.
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let quarter_x_squared = x * x / 4.0;
    for k in 1..=64 {
        term *= quarter_x_squared / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn sine(frequency: f64, rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|n| {
                let t = n as f64 / f64::from(rate);
                (2.0 * std::f64::consts::PI * frequency * t).sin() as f32
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
        (sum / samples.len() as f64).sqrt()
    }

    #[test]
    fn output_length_rounds_to_the_nearest_frame() {
        assert_eq!(output_len(4_800, 48_000, 16_000), 1_600);
        assert_eq!(output_len(44_100, 44_100, 16_000), 16_000);
        assert_eq!(output_len(1_000, 8_000, 16_000), 2_000);
        assert_eq!(output_len(7, 48_000, 16_000), 2);
    }

    #[test]
    fn output_length_is_the_input_length_for_degenerate_inputs() {
        assert_eq!(output_len(0, 48_000, 16_000), 0);
        assert_eq!(output_len(100, 0, 16_000), 100);
        assert_eq!(output_len(100, 48_000, 0), 100);
        assert_eq!(output_len(100, 48_000, 48_000), 100);
    }

    #[test]
    fn matching_rates_return_the_input_unchanged() {
        let input = vec![0.1, -0.2, 0.3];

        assert_eq!(resample_channel(&input, 16_000, 16_000), input);
    }

    #[test]
    fn an_empty_input_resamples_to_nothing() {
        assert!(resample_channel(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn a_zero_rate_resamples_to_nothing() {
        assert!(resample_channel(&[0.5; 32], 0, 16_000).is_empty());
        assert!(resample_channel(&[0.5; 32], 48_000, 0).is_empty());
    }

    #[test]
    fn downsampling_produces_the_expected_frame_count() {
        let input = sine(440.0, 48_000, 4_800);

        let out = resample_channel(&input, 48_000, 16_000);

        assert_eq!(out.len(), 1_600);
    }

    #[test]
    fn upsampling_produces_the_expected_frame_count() {
        let input = sine(440.0, 8_000, 800);

        let out = resample_channel(&input, 8_000, 16_000);

        assert_eq!(out.len(), 1_600);
    }

    #[test]
    fn a_constant_signal_stays_constant() {
        let input = vec![0.25_f32; 4_800];

        let out = resample_channel(&input, 48_000, 16_000);

        assert!(
            out.iter().all(|s| (s - 0.25).abs() < 1e-4),
            "peak deviation {}",
            out.iter().map(|s| (s - 0.25).abs()).fold(0.0_f32, f32::max)
        );
    }

    #[test]
    fn a_passband_sine_keeps_its_amplitude() {
        let input = sine(440.0, 48_000, 48_000);

        let out = resample_channel(&input, 48_000, 16_000);
        let interior = &out[100..out.len() - 100];

        let amplitude = rms(interior) * std::f64::consts::SQRT_2;
        assert!(
            (amplitude - 1.0).abs() < 0.02,
            "amplitude {amplitude} should be within 2% of 1.0"
        );
    }

    #[test]
    fn a_passband_sine_keeps_its_frequency() {
        let input = sine(1_000.0, 48_000, 48_000);

        let out = resample_channel(&input, 48_000, 16_000);

        // Count rising zero crossings over the interior and compare with the expected rate.
        let interior = &out[100..out.len() - 100];
        let crossings = interior
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        let seconds = interior.len() as f64 / 16_000.0;
        let measured = crossings as f64 / seconds;
        assert!(
            (measured - 1_000.0).abs() < 20.0,
            "measured {measured} Hz should be within 20 Hz of 1000 Hz"
        );
    }

    #[test]
    fn content_above_the_new_nyquist_is_attenuated() {
        // 10 kHz cannot survive a 16 kHz target, whose Nyquist is 8 kHz.
        let input = sine(10_000.0, 48_000, 48_000);

        let out = resample_channel(&input, 48_000, 16_000);
        let interior = &out[200..out.len() - 200];

        assert!(
            rms(interior) < 0.05,
            "aliased energy {} should be below 0.05",
            rms(interior)
        );
    }

    #[test]
    fn the_output_is_always_finite() {
        let input = vec![f32::MAX; 64];

        let out = resample_channel(&input, 48_000, 16_000);

        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn a_single_sample_resamples_without_panicking() {
        let out = resample_channel(&[0.5], 48_000, 16_000);

        assert_eq!(out.len(), 0);
    }

    #[test]
    fn the_kernel_is_one_at_zero_and_zero_beyond_the_window() {
        assert!((kernel(0.0, 1.0, 16.0) - 1.0).abs() < 1e-12);
        assert_eq!(kernel(17.0, 1.0, 16.0), 0.0);
    }

    #[test]
    fn the_bessel_series_matches_known_values() {
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-12);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008_4).abs() < 1e-12);
        assert!((bessel_i0(2.0) - 2.279_585_302_336_067_3).abs() < 1e-12);
    }

    proptest! {
        #[test]
        fn resampling_preserves_the_length_formula(
            frames in 1_usize..2_000,
            from in prop_oneof![Just(8_000_u32), Just(44_100), Just(48_000), Just(96_000)],
            to in prop_oneof![Just(16_000_u32), Just(22_050), Just(48_000)],
        ) {
            let input = vec![0.1_f32; frames];
            let out = resample_channel(&input, from, to);
            prop_assert_eq!(out.len(), output_len(frames, from, to));
            prop_assert!(out.iter().all(|s| s.is_finite()));
        }

        #[test]
        fn resampling_never_exceeds_the_input_peak(
            frames in 1_usize..500,
            sample in -1.0_f32..1.0,
        ) {
            let input = vec![sample; frames];
            let out = resample_channel(&input, 48_000, 16_000);
            let bound = sample.abs() + 1e-3;
            prop_assert!(out.iter().all(|s| s.abs() <= bound), "peak {} > {bound}", out.iter().fold(0.0_f32, |a, s| a.max(s.abs())));
        }
    }
}
