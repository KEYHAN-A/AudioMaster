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
    let true_peak_db = peak_db + 0.2; // simplified true-peak estimation
    let lufs_integrated = compute_lufs(audio);
    let lufs_short_term_max = compute_short_term_lufs_max(audio);
    let dynamic_range_db = compute_dynamic_range(audio);
    let stereo_width = compute_stereo_width(audio);
    let frequency_bands = compute_frequency_bands(audio);

    Ok(AudioAnalysis {
        metadata,
        lufs_integrated,
        lufs_short_term_max,
        rms_db,
        peak_db,
        true_peak_db,
        dynamic_range_db,
        stereo_width,
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
    let peak = samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max) as f64;
    if peak < 1e-10 {
        -100.0
    } else {
        20.0 * peak.log10()
    }
}

/// Simplified ITU-R BS.1770 loudness measurement.
/// Full implementation requires K-weighting filter; this is a practical approximation.
fn compute_lufs(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if audio.samples.is_empty() || channels == 0 {
        return -100.0;
    }

    // K-weighting approximation: apply simple high-shelf boost
    // For a proper implementation we'd use a biquad filter chain,
    // but this gives reasonable results for analysis purposes.
    let samples = &audio.samples;
    let frame_count = samples.len() / channels;

    // Gating block size: 400ms
    let block_size = (audio.sample_rate as f64 * 0.4) as usize;
    let hop_size = block_size / 4; // 75% overlap

    if frame_count < block_size {
        // Too short for proper gating, return simple RMS-based estimate
        let rms_db = compute_rms_db(samples);
        return rms_db - 0.691; // approximate K-weighting offset
    }

    let mut block_loudness: Vec<f64> = Vec::new();

    let mut pos = 0;
    while pos + block_size <= frame_count {
        let mut sum_sq = 0.0f64;
        let mut count = 0usize;

        for frame_idx in pos..pos + block_size {
            for ch in 0..channels {
                let sample = samples[frame_idx * channels + ch] as f64;
                sum_sq += sample * sample;
                count += 1;
            }
        }

        let mean_sq = sum_sq / count as f64;
        if mean_sq > 0.0 {
            let loudness = -0.691 + 10.0 * mean_sq.log10();
            block_loudness.push(loudness);
        }

        pos += hop_size;
    }

    if block_loudness.is_empty() {
        return -100.0;
    }

    // Absolute gating threshold: -70 LUFS
    let above_abs_gate: Vec<f64> = block_loudness
        .iter()
        .copied()
        .filter(|&l| l > -70.0)
        .collect();

    if above_abs_gate.is_empty() {
        return -100.0;
    }

    // Relative gating threshold: mean of above absolute gate - 10 LU
    let mean_above: f64 = above_abs_gate.iter().sum::<f64>() / above_abs_gate.len() as f64;
    let relative_gate = mean_above - 10.0;

    let gated: Vec<f64> = above_abs_gate
        .into_iter()
        .filter(|&l| l > relative_gate)
        .collect();

    if gated.is_empty() {
        return -100.0;
    }

    gated.iter().sum::<f64>() / gated.len() as f64
}

/// Maximum short-term loudness (3-second window).
fn compute_short_term_lufs_max(audio: &DecodedAudio) -> f64 {
    let channels = audio.channels as usize;
    if audio.samples.is_empty() || channels == 0 {
        return -100.0;
    }

    let frame_count = audio.samples.len() / channels;
    let window_size = (audio.sample_rate as f64 * 3.0) as usize;
    let hop_size = (audio.sample_rate as f64 * 1.0) as usize;

    if frame_count < window_size {
        return compute_lufs(audio);
    }

    let mut max_loudness = -100.0f64;
    let mut pos = 0;

    while pos + window_size <= frame_count {
        let mut sum_sq = 0.0f64;
        let mut count = 0usize;

        for frame_idx in pos..pos + window_size {
            for ch in 0..channels {
                let sample = audio.samples[frame_idx * channels + ch] as f64;
                sum_sq += sample * sample;
                count += 1;
            }
        }

        let mean_sq = sum_sq / count as f64;
        if mean_sq > 0.0 {
            let loudness = -0.691 + 10.0 * mean_sq.log10();
            if loudness > max_loudness {
                max_loudness = loudness;
            }
        }

        pos += hop_size;
    }

    max_loudness
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

    window_rms.sort_by(|a, b| a.partial_cmp(b).unwrap());

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
    let num_windows = (mono.len() / window_size).max(1);

    let mut band_energies = [0.0f64; 7];

    for w in 0..num_windows {
        let start = w * window_size;
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
