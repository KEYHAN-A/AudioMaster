use anyhow::Result;
use std::path::Path;

use super::decode::DecodedAudio;
use crate::types::{AudioAnalysis, AudioMetadata, FrequencyBands};

/// Compute full audio analysis from decoded samples.
pub fn analyze(path: &Path, audio: &DecodedAudio) -> Result<AudioAnalysis> {
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    let metadata = AudioMetadata {
        path: path.to_path_buf(),
        sample_rate: audio.sample_rate,
        channels: audio.channels,
        duration_secs: audio.duration_secs(),
        bit_depth: None,
        format,
    };

    let rms_db = compute_rms_db(&audio.samples);
    let peak_db = compute_peak_db(&audio.samples);
    let true_peak_db = compute_true_peak_db(audio);
    let weighted = k_weight(audio);
    let lufs_integrated =
        compute_lufs_weighted(&weighted, audio.sample_rate, audio.channels as usize);
    let lufs_short_term_max = compute_windowed_loudness_max(
        &weighted,
        audio.sample_rate,
        audio.channels as usize,
        3.0,
        1.0,
    );
    let lufs_momentary_max = compute_windowed_loudness_max(
        &weighted,
        audio.sample_rate,
        audio.channels as usize,
        0.4,
        0.1,
    );
    let loudness_range_lu = compute_loudness_range(
        &weighted,
        audio.sample_rate,
        audio.channels as usize,
        lufs_integrated,
    );
    let dynamic_range_db = compute_dynamic_range(audio);
    let crest_factor_db = (peak_db - rms_db).max(0.0);
    let peak_to_loudness_ratio = (true_peak_db - lufs_integrated).max(0.0);
    let stereo_width = compute_stereo_width(audio);
    let stereo_correlation = compute_stereo_correlation(audio);
    let dc_offset = compute_dc_offset(audio);
    let clipped_samples = audio
        .samples
        .iter()
        .filter(|sample| sample.is_finite() && sample.abs() >= 1.0)
        .count() as u64;
    let frequency_bands = compute_frequency_bands(audio);

    Ok(AudioAnalysis {
        schema_version: 2,
        metadata,
        lufs_integrated,
        lufs_short_term_max,
        lufs_momentary_max,
        loudness_range_lu,
        rms_db,
        peak_db,
        true_peak_db,
        dynamic_range_db,
        crest_factor_db,
        peak_to_loudness_ratio,
        stereo_width,
        stereo_correlation,
        dc_offset,
        clipped_samples,
        frequency_bands,
    })
}

/// RMS level in dB.
fn compute_rms_db(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return -100.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    if rms < 1e-10 {
        -100.0
    } else {
        20.0 * rms.log10()
    }
}

/// Peak level in dB.
fn compute_peak_db(samples: &[f32]) -> f64 {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max) as f64;
    if peak < 1e-10 {
        -100.0
    } else {
        20.0 * peak.log10()
    }
}

/// Four-times oversampled peak estimate using a windowed-sinc interpolator.
///
/// This is intentionally independent from the render limiter so metering can
/// detect inter-sample overs even when the source samples remain below 0 dBFS.
fn compute_true_peak_db(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if channels == 0 || audio.samples.is_empty() {
        return -100.0;
    }

    let frames = audio.samples.len() / channels;
    let mut peak = audio
        .samples
        .iter()
        .filter(|sample| sample.is_finite())
        .map(|sample| sample.abs() as f64)
        .fold(0.0, f64::max);
    let half_taps = 4isize;

    for channel in 0..channels {
        for frame in 0..frames.saturating_sub(1) {
            for phase in 1..4 {
                let position = frame as f64 + phase as f64 / 4.0;
                let mut value = 0.0;
                let mut weight_sum = 0.0;

                for tap in -half_taps..=half_taps {
                    let sample_index = frame as isize + tap;
                    if sample_index < 0 || sample_index >= frames as isize {
                        continue;
                    }

                    let distance = position - sample_index as f64;
                    let normalized = distance / (half_taps as f64 + 1.0);
                    let window = if normalized.abs() <= 1.0 {
                        0.5 + 0.5 * (std::f64::consts::PI * normalized).cos()
                    } else {
                        0.0
                    };
                    let sinc = if distance.abs() < 1e-12 {
                        1.0
                    } else {
                        let angle = std::f64::consts::PI * distance;
                        angle.sin() / angle
                    };
                    let weight = sinc * window;
                    value +=
                        audio.samples[sample_index as usize * channels + channel] as f64 * weight;
                    weight_sum += weight;
                }

                if weight_sum.abs() > 1e-12 {
                    peak = peak.max((value / weight_sum).abs());
                }
            }
        }
    }

    if peak < 1e-10 {
        -100.0
    } else {
        20.0 * peak.log10()
    }
}

#[derive(Clone, Copy)]
struct BiquadCoefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

#[derive(Clone, Copy, Default)]
struct BiquadState {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadState {
    fn process(&mut self, input: f64, coefficients: BiquadCoefficients) -> f64 {
        let output =
            coefficients.b0 * input + coefficients.b1 * self.x1 + coefficients.b2 * self.x2
                - coefficients.a1 * self.y1
                - coefficients.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

/// Apply the two-stage K-weighting filter defined by ITU-R BS.1770.
fn k_weight(audio: &DecodedAudio) -> Vec<f32> {
    let channels = audio.channels as usize;
    if channels == 0 || audio.sample_rate == 0 {
        return Vec::new();
    }

    let shelf = k_weighting_shelf(audio.sample_rate as f64);
    let high_pass = k_weighting_high_pass(audio.sample_rate as f64);
    let mut shelf_state = vec![BiquadState::default(); channels];
    let mut high_pass_state = vec![BiquadState::default(); channels];
    let mut output = Vec::with_capacity(audio.samples.len());

    for frame in audio.samples.chunks(channels) {
        for (channel, sample) in frame.iter().enumerate() {
            let input = if sample.is_finite() {
                *sample as f64
            } else {
                0.0
            };
            let stage_one = shelf_state[channel].process(input, shelf);
            let filtered = high_pass_state[channel].process(stage_one, high_pass);
            output.push(filtered as f32);
        }
    }

    output
}

fn k_weighting_shelf(sample_rate: f64) -> BiquadCoefficients {
    let frequency = 1_681.974_450_955_533;
    let gain_db = 3.999_843_853_973_347;
    let q = 0.707_175_236_955_419_6;
    let k = (std::f64::consts::PI * frequency / sample_rate).tan();
    let vh = 10.0f64.powf(gain_db / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;

    BiquadCoefficients {
        b0: (vh + vb * k / q + k * k) / a0,
        b1: 2.0 * (k * k - vh) / a0,
        b2: (vh - vb * k / q + k * k) / a0,
        a1: 2.0 * (k * k - 1.0) / a0,
        a2: (1.0 - k / q + k * k) / a0,
    }
}

fn k_weighting_high_pass(sample_rate: f64) -> BiquadCoefficients {
    let frequency = 38.135_470_876_024_44;
    let q = 0.500_327_037_323_877_3;
    let k = (std::f64::consts::PI * frequency / sample_rate).tan();
    let a0 = 1.0 + k / q + k * k;

    BiquadCoefficients {
        b0: 1.0 / a0,
        b1: -2.0 / a0,
        b2: 1.0 / a0,
        a1: 2.0 * (k * k - 1.0) / a0,
        a2: (1.0 - k / q + k * k) / a0,
    }
}

fn channel_energy(samples: &[f32], channels: usize) -> f64 {
    if samples.is_empty() || channels == 0 {
        return 0.0;
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return 0.0;
    }
    samples
        .iter()
        .filter(|sample| sample.is_finite())
        .map(|sample| (*sample as f64).powi(2))
        .sum::<f64>()
        / frames as f64
}

fn energy_to_lufs(energy: f64) -> f64 {
    if !energy.is_finite() || energy <= 1e-20 {
        -100.0
    } else {
        -0.691 + 10.0 * energy.log10()
    }
}

/// ITU-R BS.1770 K-weighted integrated loudness for mono/stereo material.
#[cfg(test)]
fn compute_lufs(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if audio.samples.is_empty() || channels == 0 {
        return -100.0;
    }

    compute_lufs_weighted(&k_weight(audio), audio.sample_rate, channels)
}

fn compute_lufs_weighted(samples: &[f32], sample_rate: u32, channels: usize) -> f64 {
    let frame_count = samples.len() / channels;

    // Gating block size: 400ms
    let block_size = (sample_rate as f64 * 0.4) as usize;
    let hop_size = block_size / 4; // 75% overlap

    if frame_count < block_size {
        let energy = channel_energy(samples, channels);
        return energy_to_lufs(energy);
    }

    let mut block_loudness: Vec<(f64, f64)> = Vec::new();

    let mut pos = 0;
    while pos + block_size <= frame_count {
        let mut sum_sq = 0.0f64;
        for frame_idx in pos..pos + block_size {
            for ch in 0..channels {
                let sample = samples[frame_idx * channels + ch] as f64;
                sum_sq += sample * sample;
            }
        }

        // BS.1770 sums channel mean-square energies; mono/stereo weights are 1.0.
        let mean_sq = sum_sq / block_size as f64;
        if mean_sq > 0.0 {
            block_loudness.push((mean_sq, energy_to_lufs(mean_sq)));
        }

        pos += hop_size;
    }

    if block_loudness.is_empty() {
        return -100.0;
    }

    // Absolute gating threshold: -70 LUFS
    let above_abs_gate: Vec<(f64, f64)> = block_loudness
        .iter()
        .copied()
        .filter(|(_, loudness)| *loudness >= -70.0)
        .collect();

    if above_abs_gate.is_empty() {
        return -100.0;
    }

    // The relative gate is derived from mean energy, not an arithmetic mean of dB values.
    let mean_above =
        above_abs_gate.iter().map(|(energy, _)| energy).sum::<f64>() / above_abs_gate.len() as f64;
    let relative_gate = energy_to_lufs(mean_above) - 10.0;

    let gated: Vec<f64> = above_abs_gate
        .into_iter()
        .filter(|(_, loudness)| *loudness >= relative_gate)
        .map(|(energy, _)| energy)
        .collect();

    if gated.is_empty() {
        return -100.0;
    }

    energy_to_lufs(gated.iter().sum::<f64>() / gated.len() as f64)
}

fn compute_windowed_loudness_max(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    window_seconds: f64,
    hop_seconds: f64,
) -> f64 {
    let frame_count = samples.len() / channels;
    let window_size = (sample_rate as f64 * window_seconds) as usize;
    let hop_size = (sample_rate as f64 * hop_seconds) as usize;

    if frame_count < window_size {
        return energy_to_lufs(channel_energy(samples, channels));
    }

    let mut max_loudness = -100.0f64;
    let mut pos = 0;

    while pos + window_size <= frame_count {
        let mut sum_sq = 0.0f64;
        for frame_idx in pos..pos + window_size {
            for ch in 0..channels {
                let sample = samples[frame_idx * channels + ch] as f64;
                sum_sq += sample * sample;
            }
        }

        let mean_sq = sum_sq / window_size as f64;
        if mean_sq > 0.0 {
            let loudness = energy_to_lufs(mean_sq);
            if loudness > max_loudness {
                max_loudness = loudness;
            }
        }

        pos += hop_size;
    }

    max_loudness
}

fn compute_loudness_range(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    integrated_lufs: f64,
) -> f64 {
    if channels == 0 || samples.is_empty() {
        return 0.0;
    }
    let frames = samples.len() / channels;
    let window = sample_rate as usize * 3;
    let hop = sample_rate as usize;
    if frames < window || hop == 0 {
        return 0.0;
    }

    let relative_gate = integrated_lufs - 20.0;
    let mut values = Vec::new();
    let mut position = 0;
    while position + window <= frames {
        let start = position * channels;
        let end = (position + window) * channels;
        let loudness = energy_to_lufs(channel_energy(&samples[start..end], channels));
        if loudness >= -70.0 && loudness >= relative_gate {
            values.push(loudness);
        }
        position += hop;
    }

    if values.len() < 2 {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let low = percentile(&values, 0.10);
    let high = percentile(&values, 0.95);
    (high - low).max(0.0)
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    let index = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

/// Dynamic range: difference between peak loudness of loud and quiet sections.
fn compute_dynamic_range(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if audio.samples.is_empty() || channels == 0 {
        return 0.0;
    }

    let frame_count = audio.samples.len() / channels;
    let window = (audio.sample_rate as f64 * 0.5) as usize;

    if frame_count < window {
        return 0.0;
    }

    let mut window_rms: Vec<f64> = Vec::new();
    let mut pos = 0;

    while pos + window <= frame_count {
        let mut sum_sq = 0.0f64;
        let mut count = 0usize;

        for frame_idx in pos..pos + window {
            for ch in 0..channels {
                let s = audio.samples[frame_idx * channels + ch] as f64;
                sum_sq += s * s;
                count += 1;
            }
        }

        let rms = (sum_sq / count as f64).sqrt();
        if rms > 1e-10 {
            window_rms.push(20.0 * rms.log10());
        }

        pos += window;
    }

    if window_rms.len() < 2 {
        return 0.0;
    }

    window_rms.sort_by(f64::total_cmp);

    let top_10 = &window_rms[window_rms.len() * 9 / 10..];
    let bottom_10 = &window_rms[..window_rms.len() / 10];

    if top_10.is_empty() || bottom_10.is_empty() {
        return 0.0;
    }

    let top_avg: f64 = top_10.iter().sum::<f64>() / top_10.len() as f64;
    let bottom_avg: f64 = bottom_10.iter().sum::<f64>() / bottom_10.len() as f64;

    (top_avg - bottom_avg).abs()
}

/// Stereo width: 0.0 = mono, 1.0 = full stereo, >1.0 = out-of-phase content.
fn compute_stereo_width(audio: &DecodedAudio) -> f64 {
    if audio.channels < 2 {
        return 0.0;
    }

    let channels = audio.channels as usize;
    let frame_count = audio.samples.len() / channels;

    let mut sum_mid_sq = 0.0f64;
    let mut sum_side_sq = 0.0f64;

    for i in 0..frame_count {
        let left = audio.samples[i * channels] as f64;
        let right = audio.samples[i * channels + 1] as f64;

        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;

        sum_mid_sq += mid * mid;
        sum_side_sq += side * side;
    }

    if sum_mid_sq < 1e-20 {
        return if sum_side_sq > 1e-20 { 2.0 } else { 0.0 };
    }

    let ratio = sum_side_sq / sum_mid_sq;
    // Map to 0..1 range approximately: ratio of 1.0 means full stereo
    ratio.sqrt().min(2.0)
}

fn compute_stereo_correlation(audio: &DecodedAudio) -> f64 {
    if audio.channels < 2 {
        return 1.0;
    }
    let channels = audio.channels as usize;
    let frames = audio.samples.len() / channels;
    if frames == 0 {
        return 0.0;
    }

    let mut sum_left = 0.0;
    let mut sum_right = 0.0;
    for frame in audio.samples.chunks(channels) {
        sum_left += frame[0] as f64;
        sum_right += frame[1] as f64;
    }
    let mean_left = sum_left / frames as f64;
    let mean_right = sum_right / frames as f64;

    let mut covariance = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    for frame in audio.samples.chunks(channels) {
        let left = frame[0] as f64 - mean_left;
        let right = frame[1] as f64 - mean_right;
        covariance += left * right;
        left_energy += left * left;
        right_energy += right * right;
    }

    let denominator = (left_energy * right_energy).sqrt();
    if denominator <= 1e-20 {
        1.0
    } else {
        (covariance / denominator).clamp(-1.0, 1.0)
    }
}

fn compute_dc_offset(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if channels == 0 || audio.samples.is_empty() {
        return 0.0;
    }

    let mut sums = vec![0.0f64; channels];
    let mut frames = 0usize;
    for frame in audio.samples.chunks(channels) {
        if frame.len() != channels {
            break;
        }
        for (channel, sample) in frame.iter().enumerate() {
            if sample.is_finite() {
                sums[channel] += *sample as f64;
            }
        }
        frames += 1;
    }
    if frames == 0 {
        return 0.0;
    }

    sums.into_iter()
        .map(|sum| (sum / frames as f64).abs())
        .fold(0.0, f64::max)
}

/// Compute energy in 7 frequency bands using a basic DFT approach.
fn compute_frequency_bands(audio: &DecodedAudio) -> FrequencyBands {
    // Use mono mixdown
    let mono: Vec<f64> = if audio.channels >= 2 {
        let ch = audio.channels as usize;
        let frames = audio.samples.len() / ch;
        (0..frames)
            .map(|i| {
                let mut sum = 0.0f64;
                for c in 0..ch {
                    sum += audio.samples[i * ch + c] as f64;
                }
                sum / ch as f64
            })
            .collect()
    } else {
        audio.samples.iter().map(|&s| s as f64).collect()
    };

    if mono.is_empty() {
        return FrequencyBands {
            sub_bass: -100.0,
            bass: -100.0,
            low_mid: -100.0,
            mid: -100.0,
            upper_mid: -100.0,
            presence: -100.0,
            brilliance: -100.0,
        };
    }

    let sr = audio.sample_rate as f64;

    // Band boundaries in Hz
    let bands: [(f64, f64); 7] = [
        (20.0, 60.0),      // Sub-bass
        (60.0, 250.0),     // Bass
        (250.0, 500.0),    // Low-mid
        (500.0, 2000.0),   // Mid
        (2000.0, 4000.0),  // Upper-mid
        (4000.0, 6000.0),  // Presence
        (6000.0, 20000.0), // Brilliance
    ];

    // Use Goertzel-like energy estimation on overlapping windows
    let window_size = 4096.min(mono.len());
    let available_windows = (mono.len() / window_size).max(1);
    let num_windows = available_windows.min(64);
    let window_stride = (available_windows / num_windows).max(1);

    let mut band_energies = [0.0f64; 7];

    for w in 0..num_windows {
        let start = w * window_stride * window_size;
        let end = (start + window_size).min(mono.len());
        let segment = &mono[start..end];
        let n = segment.len();

        // Simple DFT energy for each band
        for (band_idx, &(f_low, f_high)) in bands.iter().enumerate() {
            let k_low = ((f_low * n as f64) / sr).round() as usize;
            let k_high = ((f_high * n as f64) / sr).round() as usize;
            let k_high = k_high.min(n / 2);

            if k_low >= k_high {
                continue;
            }

            // Compute energy at a few representative frequencies in the band
            let num_probes = 8.min(k_high - k_low);
            let step = ((k_high - k_low) as f64 / num_probes as f64).max(1.0) as usize;

            let mut energy = 0.0f64;
            let mut k = k_low;
            while k < k_high {
                // Goertzel algorithm for single DFT bin
                let omega = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
                let coeff = 2.0 * omega.cos();
                let mut s0 = 0.0f64;
                let mut s1 = 0.0f64;
                let mut s2;

                for &sample in segment.iter() {
                    s2 = s1;
                    s1 = s0;
                    s0 = sample + coeff * s1 - s2;
                }

                let power = s0 * s0 + s1 * s1 - coeff * s0 * s1;
                energy += power;

                k += step.max(1);
            }

            band_energies[band_idx] += energy;
        }
    }

    // Normalize and convert to dB
    let total: f64 = band_energies.iter().sum();
    let normalize = if total > 1e-20 { total } else { 1.0 };

    let to_db = |e: f64| -> f64 {
        let ratio = e / normalize;
        if ratio < 1e-20 {
            -100.0
        } else {
            10.0 * ratio.log10()
        }
    };

    FrequencyBands {
        sub_bass: to_db(band_energies[0]),
        bass: to_db(band_energies[1]),
        low_mid: to_db(band_energies[2]),
        mid: to_db(band_energies[3]),
        upper_mid: to_db(band_energies[4]),
        presence: to_db(band_energies[5]),
        brilliance: to_db(band_energies[6]),
    }
}

#[cfg(test)]
mod tests {
    use super::super::decode::DecodedAudio;
    use super::*;

    /// Helper to create test audio data.
    fn create_test_audio(samples: Vec<f32>, sample_rate: u32, channels: u16) -> DecodedAudio {
        let total_frames = samples.len() as u64 / channels as u64;
        DecodedAudio {
            samples,
            sample_rate,
            channels,
            total_frames,
        }
    }

    /// Helper to create sine wave samples.
    fn create_sine_wave(
        frequency: f32,
        duration_secs: f64,
        sample_rate: u32,
        amplitude: f32,
    ) -> Vec<f32> {
        let num_samples = (sample_rate as f64 * duration_secs) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin()
            })
            .collect()
    }

    /// Test RMS calculation with silent audio.
    #[test]
    fn test_rms_silent() {
        let audio = create_test_audio(vec![0.0; 1000], 48000, 2);
        let rms = compute_rms_db(&audio.samples);
        assert_eq!(rms, -100.0);
    }

    /// Test RMS calculation with full scale audio.
    #[test]
    fn test_rms_full_scale() {
        let samples = vec![1.0; 1000];
        let rms = compute_rms_db(&samples);
        assert!((rms - 0.0).abs() < 0.1);
    }

    /// Test RMS calculation with -6 dB audio.
    #[test]
    fn test_rms_minus_6db() {
        let samples = vec![0.5; 1000]; // 0.5 = -6 dB
        let rms = compute_rms_db(&samples);
        assert!((rms - (-6.02)).abs() < 0.1);
    }

    /// Test Peak calculation with silent audio.
    #[test]
    fn test_peak_silent() {
        let audio = create_test_audio(vec![0.0; 1000], 48000, 2);
        let peak = compute_peak_db(&audio.samples);
        assert_eq!(peak, -100.0);
    }

    /// Test Peak calculation with full scale audio.
    #[test]
    fn test_peak_full_scale() {
        let samples = vec![1.0; 1000];
        let peak = compute_peak_db(&samples);
        assert!((peak - 0.0).abs() < 0.1);
    }

    /// Test Peak calculation with -3 dB audio.
    #[test]
    fn test_peak_minus_3db() {
        let samples = vec![0.707; 1000]; // 0.707 ≈ -3 dB
        let peak = compute_peak_db(&samples);
        assert!((peak - (-3.01)).abs() < 0.1);
    }

    /// Test Peak calculation with clipped audio.
    #[test]
    fn test_peak_clipped() {
        let samples = vec![1.5; 1000]; // Clipped beyond full scale
        let peak = compute_peak_db(&samples);
        assert!(peak > 0.0); // Should be positive dB for clipped audio
    }

    /// Test stereo width calculation with mono audio.
    #[test]
    fn test_stereo_width_mono() {
        // Mono samples (L = R for each frame)
        let samples: Vec<f32> = (0..1000)
            .flat_map(|i| {
                let value = (i as f32 / 1000.0) * 0.5;
                [value, value] // L = R
            })
            .collect();

        let audio = create_test_audio(samples, 48000, 2);
        let width = compute_stereo_width(&audio);
        assert!(
            (width - 0.0).abs() < 0.01,
            "Mono audio should have 0 stereo width"
        );
    }

    /// Test stereo width calculation with wide stereo.
    #[test]
    fn test_stereo_width_wide() {
        // Wide stereo (L = -R for each frame)
        let samples: Vec<f32> = (0..1000)
            .flat_map(|i| {
                let value = (i as f32 / 1000.0) * 0.5;
                [value, -value] // L = -R (maximum width)
            })
            .collect();

        let audio = create_test_audio(samples, 48000, 2);
        let width = compute_stereo_width(&audio);
        assert!(width > 0.8, "Wide stereo should have high width value");
    }

    /// Test LUFS calculation with silent audio.
    #[test]
    fn test_lufs_silent() {
        let audio = create_test_audio(vec![0.0; 48000], 48000, 2);
        let lufs = compute_lufs(&audio);
        assert_eq!(lufs, -100.0);
    }

    /// Test dynamic range calculation.
    #[test]
    fn test_dynamic_range() {
        // Audio with varying levels for better dynamic range detection
        let mut samples = vec![0.001; 12000];
        samples.extend(std::iter::repeat_n(0.1, 12000));
        samples.extend(std::iter::repeat_n(0.5, 12000));

        // Interleaved stereo
        let stereo_samples: Vec<f32> = samples.iter().flat_map(|&s| [s, s]).collect();

        let audio = create_test_audio(stereo_samples, 48000, 2);
        let dr = compute_dynamic_range(&audio);

        // Dynamic range should be non-negative
        assert!(dr >= 0.0, "Dynamic range should be non-negative");
    }

    /// Test frequency band calculation.
    #[test]
    fn test_frequency_bands() {
        // Create 1 kHz tone at -10 dB with more samples for better DFT resolution
        let samples = create_sine_wave(1000.0, 2.0, 48000, 0.5);

        let audio = create_test_audio(samples, 48000, 1);
        let bands = compute_frequency_bands(&audio);

        // Just verify that the bands are calculated (not all -100)
        let total_energy = bands.sub_bass
            + bands.bass
            + bands.low_mid
            + bands.mid
            + bands.upper_mid
            + bands.presence
            + bands.brilliance;
        assert!(
            total_energy > -600.0,
            "Should have some energy in frequency bands"
        );
    }

    /// Test empty sample handling.
    #[test]
    fn test_empty_samples() {
        let audio = create_test_audio(vec![], 48000, 2);

        let rms = compute_rms_db(&audio.samples);
        let peak = compute_peak_db(&audio.samples);
        let lufs = compute_lufs(&audio);

        assert_eq!(rms, -100.0);
        assert_eq!(peak, -100.0);
        assert_eq!(lufs, -100.0);
    }

    /// Test single channel audio.
    #[test]
    fn test_single_channel() {
        let samples = create_sine_wave(440.0, 0.5, 48000, 0.5);
        let audio = create_test_audio(samples, 48000, 1);

        // Should not panic with single channel
        let lufs = compute_lufs(&audio);
        let dr = compute_dynamic_range(&audio);
        let bands = compute_frequency_bands(&audio);

        assert!(lufs > -100.0, "LUFS should be calculated");
        assert!(dr >= 0.0, "Dynamic range should be non-negative");
        // Just verify bands are calculated, not checking specific energy
        assert!(bands.bass > -100.0, "Bass band should be calculated");
    }
}
