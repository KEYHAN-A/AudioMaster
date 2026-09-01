//! Deterministic native mastering DSP.
//!
//! The module owns the safety boundary between generated/user parameters and
//! audio processing. Backends may propose settings, but only validated values
//! reach the render graph.

use std::collections::VecDeque;
use std::path::Path;

use anyhow::{Context, Result};

use crate::analysis::{self, decode::DecodedAudio};
use crate::types::{EqBandType, MasteringParams};

#[derive(Debug, Clone)]
pub struct ValidatedParams {
    pub params: MasteringParams,
    pub warnings: Vec<String>,
}

/// Validate and clamp an externally supplied mastering plan.
pub fn validate_params(
    mut params: MasteringParams,
    sample_rate: u32,
    requested_target_lufs: f64,
    no_limiter: bool,
) -> Result<ValidatedParams> {
    anyhow::ensure!(sample_rate >= 8_000, "Sample rate is too low for mastering");
    ensure_finite(requested_target_lufs, "target_lufs")?;

    let mut warnings = Vec::new();
    let nyquist_limit = sample_rate as f64 * 0.49;
    for (index, band) in params.eq.iter_mut().enumerate() {
        ensure_finite(band.frequency, &format!("eq[{index}].frequency"))?;
        ensure_finite(band.gain_db, &format!("eq[{index}].gain_db"))?;
        ensure_finite(band.q, &format!("eq[{index}].q"))?;
        clamp_with_warning(
            &mut band.frequency,
            15.0,
            nyquist_limit,
            &format!("eq[{index}].frequency"),
            &mut warnings,
        );
        clamp_with_warning(
            &mut band.gain_db,
            -12.0,
            12.0,
            &format!("eq[{index}].gain_db"),
            &mut warnings,
        );
        clamp_with_warning(
            &mut band.q,
            0.1,
            12.0,
            &format!("eq[{index}].q"),
            &mut warnings,
        );
    }

    let compression = &mut params.compression;
    for (value, name) in [
        (compression.threshold_db, "compression.threshold_db"),
        (compression.ratio, "compression.ratio"),
        (compression.attack_ms, "compression.attack_ms"),
        (compression.release_ms, "compression.release_ms"),
        (compression.knee_db, "compression.knee_db"),
        (compression.makeup_gain_db, "compression.makeup_gain_db"),
    ] {
        ensure_finite(value, name)?;
    }
    clamp_with_warning(
        &mut compression.threshold_db,
        -60.0,
        0.0,
        "compression.threshold_db",
        &mut warnings,
    );
    clamp_with_warning(
        &mut compression.ratio,
        1.0,
        20.0,
        "compression.ratio",
        &mut warnings,
    );
    clamp_with_warning(
        &mut compression.attack_ms,
        0.1,
        200.0,
        "compression.attack_ms",
        &mut warnings,
    );
    clamp_with_warning(
        &mut compression.release_ms,
        5.0,
        2_000.0,
        "compression.release_ms",
        &mut warnings,
    );
    clamp_with_warning(
        &mut compression.knee_db,
        0.0,
        24.0,
        "compression.knee_db",
        &mut warnings,
    );
    clamp_with_warning(
        &mut compression.makeup_gain_db,
        -12.0,
        12.0,
        "compression.makeup_gain_db",
        &mut warnings,
    );

    ensure_finite(params.limiter.ceiling_db, "limiter.ceiling_db")?;
    ensure_finite(params.limiter.release_ms, "limiter.release_ms")?;
    clamp_with_warning(
        &mut params.limiter.ceiling_db,
        -6.0,
        -0.1,
        "limiter.ceiling_db",
        &mut warnings,
    );
    clamp_with_warning(
        &mut params.limiter.release_ms,
        5.0,
        1_000.0,
        "limiter.release_ms",
        &mut warnings,
    );

    ensure_finite(params.stereo.width, "stereo.width")?;
    ensure_finite(params.stereo.balance, "stereo.balance")?;
    clamp_with_warning(
        &mut params.stereo.width,
        0.0,
        2.0,
        "stereo.width",
        &mut warnings,
    );
    clamp_with_warning(
        &mut params.stereo.balance,
        -1.0,
        1.0,
        "stereo.balance",
        &mut warnings,
    );

    let target = requested_target_lufs.clamp(-24.0, -5.0);
    if (params.target_lufs - target).abs() > 1e-9 {
        warnings.push("Advisor target LUFS was replaced by the requested target".into());
    }
    params.target_lufs = target;
    if no_limiter && params.limiter.enabled {
        warnings.push("Limiter disabled by the mastering request".into());
        params.limiter.enabled = false;
    }

    Ok(ValidatedParams { params, warnings })
}

fn ensure_finite(value: f64, field: &str) -> Result<()> {
    anyhow::ensure!(value.is_finite(), "{field} must be finite");
    Ok(())
}

fn clamp_with_warning(
    value: &mut f64,
    minimum: f64,
    maximum: f64,
    field: &str,
    warnings: &mut Vec<String>,
) {
    let clamped = value.clamp(minimum, maximum);
    if (*value - clamped).abs() > f64::EPSILON {
        warnings.push(format!("{field} was clamped to {clamped}"));
        *value = clamped;
    }
}

/// Render validated mastering parameters to a WAV intermediate.
pub fn render_wav(
    input_path: &Path,
    output_path: &Path,
    params: &MasteringParams,
    bit_depth: u16,
) -> Result<Vec<String>> {
    render_wav_with_control(
        input_path,
        output_path,
        params,
        bit_depth,
        &crate::control::ProcessingControl::default(),
    )
}

pub fn render_wav_with_control(
    input_path: &Path,
    output_path: &Path,
    params: &MasteringParams,
    bit_depth: u16,
    control: &crate::control::ProcessingControl,
) -> Result<Vec<String>> {
    control.check_cancelled()?;
    control.report("render.decode", 0.28, 0, None, "Decoding source audio");
    let mut audio = match analysis::decode_audio(input_path) {
        Ok(audio) => audio,
        Err(error) if error.to_string().contains("256 MiB processing limit") => {
            return render_wav_streaming_ffmpeg(
                input_path,
                output_path,
                params,
                bit_depth,
                control,
            );
        }
        Err(error) => return Err(error),
    };
    anyhow::ensure!(
        audio.channels == 1 || audio.channels == 2,
        "Only mono and stereo audio are supported"
    );

    let total_frames = audio.total_frames;
    control.check_cancelled()?;
    sanitize(&mut audio.samples);
    control.report(
        "render.eq",
        0.35,
        0,
        Some(total_frames),
        "Applying tonal shaping",
    );
    apply_eq(&mut audio, params);
    apply_resonance_and_sibilance_guard(&mut audio);
    apply_low_end_mono_guard(&mut audio, 100.0);
    control.check_cancelled()?;
    control.report(
        "render.dynamics",
        0.5,
        total_frames,
        Some(total_frames),
        "Applying dynamics",
    );
    apply_compression(&mut audio, params);
    apply_stereo(&mut audio, params);

    let mut warnings = Vec::new();
    let intermediate = analysis::analyze(Path::new("intermediate.wav"), &audio)?;
    let loudness_gain_db = (params.target_lufs - intermediate.lufs_integrated).clamp(-24.0, 24.0);
    if (params.target_lufs - intermediate.lufs_integrated).abs() > 24.0 {
        warnings.push("Loudness correction was limited to 24 dB".into());
    }
    apply_gain(&mut audio.samples, loudness_gain_db);

    if params.limiter.enabled {
        control.check_cancelled()?;
        control.report(
            "render.limiter",
            0.65,
            0,
            Some(total_frames),
            "Applying true-peak limiting",
        );
        apply_lookahead_limiter(
            &mut audio,
            params.limiter.ceiling_db,
            params.limiter.release_ms,
        );
        enforce_true_peak_ceiling(&mut audio, params.limiter.ceiling_db)?;
    }

    control.check_cancelled()?;
    control.report(
        "render.write",
        0.77,
        total_frames,
        Some(total_frames),
        "Writing lossless master",
    );
    write_wav(output_path, &audio, bit_depth)?;
    Ok(warnings)
}

fn render_wav_streaming_ffmpeg(
    input_path: &Path,
    output_path: &Path,
    params: &MasteringParams,
    bit_depth: u16,
    control: &crate::control::ProcessingControl,
) -> Result<Vec<String>> {
    use std::io::BufRead;

    control.check_cancelled()?;

    let mut filters = Vec::new();
    for band in &params.eq {
        let filter = match band.band_type {
            EqBandType::Peak => format!(
                "equalizer=f={}:t=q:w={}:g={}",
                band.frequency, band.q, band.gain_db
            ),
            EqBandType::LowShelf => format!(
                "bass=f={}:width_type=q:w={}:g={}",
                band.frequency, band.q, band.gain_db
            ),
            EqBandType::HighShelf => format!(
                "treble=f={}:width_type=q:w={}:g={}",
                band.frequency, band.q, band.gain_db
            ),
            EqBandType::LowPass => format!("lowpass=f={}", band.frequency),
            EqBandType::HighPass => format!("highpass=f={}", band.frequency),
        };
        filters.push(filter);
    }
    let compression = &params.compression;
    let threshold = 10.0f64.powf(compression.threshold_db / 20.0);
    let makeup = 10.0f64.powf(compression.makeup_gain_db / 20.0).max(1.0);
    filters.push(format!(
        "acompressor=threshold={threshold}:ratio={}:attack={}:release={}:makeup={makeup}",
        compression.ratio, compression.attack_ms, compression.release_ms
    ));
    if params.limiter.enabled {
        filters.push(format!(
            "loudnorm=I={}:TP={}:LRA=20",
            params.target_lufs, params.limiter.ceiling_db
        ));
    } else {
        let measured = measure_streaming_lufs(input_path)?;
        filters.push(format!(
            "volume={}dB",
            (params.target_lufs - measured).clamp(-24.0, 24.0)
        ));
    }

    let codec = match bit_depth {
        16 => "pcm_s16le",
        24 => "pcm_s24le",
        32 => "pcm_f32le",
        _ => anyhow::bail!("Unsupported WAV bit depth: {bit_depth}"),
    };
    let (sample_rate, total_frames) = probe_streaming_timing(input_path).unwrap_or((48_000, None));
    let mut child = std::process::Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(input_path)
        .args([
            "-af",
            &filters.join(","),
            "-codec:a",
            codec,
            "-progress",
            "pipe:1",
            "-nostats",
        ])
        .arg(output_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Starting FFmpeg bounded-memory render")?;
    let stdout = child
        .stdout
        .take()
        .context("FFmpeg progress pipe is unavailable")?;
    for line in std::io::BufReader::new(stdout).lines() {
        control.check_cancelled().inspect_err(|_| {
            let _ = child.kill();
        })?;
        let line = line?;
        if let Some(microseconds) = line
            .strip_prefix("out_time_us=")
            .and_then(|value| value.parse::<u64>().ok())
        {
            let processed_frames = microseconds.saturating_mul(sample_rate as u64) / 1_000_000;
            let fraction = total_frames
                .map(|total| 0.28 + 0.49 * processed_frames as f64 / total.max(1) as f64)
                .unwrap_or(0.55);
            control.report(
                "render.streaming",
                fraction,
                processed_frames,
                total_frames,
                "Streaming bounded-memory render",
            );
        }
    }
    let status = child.wait()?;
    anyhow::ensure!(status.success(), "FFmpeg bounded-memory render failed");
    Ok(vec![
        "Long-form source used the bounded-memory FFmpeg render graph".into(),
        "Stereo width processing is bypassed in the long-form compatibility path".into(),
    ])
}

fn measure_streaming_lufs(input_path: &Path) -> Result<f64> {
    let output = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(input_path)
        .args([
            "-af",
            "loudnorm=I=-23:TP=-1:LRA=7:print_format=json",
            "-f",
            "null",
            "-",
        ])
        .output()?;
    anyhow::ensure!(output.status.success(), "FFmpeg loudness prepass failed");
    let report = String::from_utf8_lossy(&output.stderr);
    let start = report
        .rfind('{')
        .context("FFmpeg loudness prepass omitted JSON")?;
    let end = report
        .rfind('}')
        .context("FFmpeg loudness prepass returned incomplete JSON")?;
    let value: serde_json::Value = serde_json::from_str(&report[start..=end])?;
    value["input_i"]
        .as_str()
        .context("FFmpeg loudness prepass omitted input_i")?
        .parse()
        .context("FFmpeg loudness prepass returned invalid input_i")
}

fn probe_streaming_timing(input_path: &Path) -> Result<(u32, Option<u64>)> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=sample_rate:format=duration",
            "-of",
            "json",
        ])
        .arg(input_path)
        .output()?;
    anyhow::ensure!(output.status.success(), "ffprobe failed");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let sample_rate = value["streams"][0]["sample_rate"]
        .as_str()
        .and_then(|value| value.parse::<u32>().ok())
        .context("ffprobe omitted sample rate")?;
    let total_frames = value["format"]["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|duration| (duration * sample_rate as f64).round() as u64);
    Ok((sample_rate, total_frames))
}

fn sanitize(samples: &mut [f32]) {
    for sample in samples {
        if !sample.is_finite() {
            *sample = 0.0;
        }
    }
}

#[derive(Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

#[derive(Clone, Copy, Default)]
struct FilterState {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl FilterState {
    fn process(&mut self, input: f64, filter: Biquad) -> f64 {
        let output = filter.b0 * input + filter.b1 * self.x1 + filter.b2 * self.x2
            - filter.a1 * self.y1
            - filter.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

fn apply_eq(audio: &mut DecodedAudio, params: &MasteringParams) {
    let channels = audio.channels as usize;
    for band in &params.eq {
        let filter = make_eq_filter(band, audio.sample_rate as f64);
        let mut states = vec![FilterState::default(); channels];
        for frame in audio.samples.chunks_mut(channels) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = states[channel].process(*sample as f64, filter) as f32;
            }
        }
    }
}

fn make_eq_filter(band: &crate::types::EqBand, sample_rate: f64) -> Biquad {
    let omega = 2.0 * std::f64::consts::PI * band.frequency / sample_rate;
    let cos = omega.cos();
    let sin = omega.sin();
    let alpha = sin / (2.0 * band.q);
    let amplitude = 10.0f64.powf(band.gain_db / 40.0);

    let (b0, b1, b2, a0, a1, a2) = match band.band_type {
        EqBandType::Peak => (
            1.0 + alpha * amplitude,
            -2.0 * cos,
            1.0 - alpha * amplitude,
            1.0 + alpha / amplitude,
            -2.0 * cos,
            1.0 - alpha / amplitude,
        ),
        EqBandType::LowPass => (
            (1.0 - cos) / 2.0,
            1.0 - cos,
            (1.0 - cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        EqBandType::HighPass => (
            (1.0 + cos) / 2.0,
            -(1.0 + cos),
            (1.0 + cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        EqBandType::LowShelf => {
            let two_sqrt_a_alpha = 2.0 * amplitude.sqrt() * alpha;
            (
                amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos + two_sqrt_a_alpha),
                2.0 * amplitude * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos - two_sqrt_a_alpha),
                (amplitude + 1.0) + (amplitude - 1.0) * cos + two_sqrt_a_alpha,
                -2.0 * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                (amplitude + 1.0) + (amplitude - 1.0) * cos - two_sqrt_a_alpha,
            )
        }
        EqBandType::HighShelf => {
            let two_sqrt_a_alpha = 2.0 * amplitude.sqrt() * alpha;
            (
                amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos + two_sqrt_a_alpha),
                -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos - two_sqrt_a_alpha),
                (amplitude + 1.0) - (amplitude - 1.0) * cos + two_sqrt_a_alpha,
                2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                (amplitude + 1.0) - (amplitude - 1.0) * cos - two_sqrt_a_alpha,
            )
        }
    };

    Biquad {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn apply_compression(audio: &mut DecodedAudio, params: &MasteringParams) {
    let compression = &params.compression;
    if compression.ratio <= 1.000_1 {
        apply_gain(&mut audio.samples, compression.makeup_gain_db);
        return;
    }
    let channels = audio.channels as usize;
    let attack = coefficient(compression.attack_ms, audio.sample_rate);
    let release = coefficient(compression.release_ms, audio.sample_rate);
    let mut gain_reduction_db = 0.0;

    for frame in audio.samples.chunks_mut(channels) {
        let detector = frame
            .iter()
            .map(|sample| sample.abs() as f64)
            .fold(0.0, f64::max);
        let level_db = if detector <= 1e-12 {
            -120.0
        } else {
            20.0 * detector.log10()
        };
        let desired = compressor_reduction(
            level_db,
            compression.threshold_db,
            compression.ratio,
            compression.knee_db,
        );
        let smoothing = if desired < gain_reduction_db {
            attack
        } else {
            release
        };
        gain_reduction_db = smoothing * gain_reduction_db + (1.0 - smoothing) * desired;
        let gain = 10.0f64.powf((gain_reduction_db + compression.makeup_gain_db) / 20.0) as f32;
        for sample in frame {
            *sample *= gain;
        }
    }
}

fn coefficient(time_ms: f64, sample_rate: u32) -> f64 {
    (-1.0 / (time_ms * 0.001 * sample_rate as f64)).exp()
}

fn compressor_reduction(level: f64, threshold: f64, ratio: f64, knee: f64) -> f64 {
    let over = level - threshold;
    if knee <= 0.0 {
        return if over > 0.0 {
            -(over - over / ratio)
        } else {
            0.0
        };
    }
    if over <= -knee / 2.0 {
        0.0
    } else if over >= knee / 2.0 {
        -(over - over / ratio)
    } else {
        let position = over + knee / 2.0;
        -(1.0 - 1.0 / ratio) * position * position / (2.0 * knee)
    }
}

fn apply_stereo(audio: &mut DecodedAudio, params: &MasteringParams) {
    if audio.channels != 2 {
        return;
    }
    let balance = params.stereo.balance;
    let left_gain = if balance > 0.0 { 1.0 - balance } else { 1.0 };
    let right_gain = if balance < 0.0 { 1.0 + balance } else { 1.0 };
    for frame in audio.samples.chunks_mut(2) {
        let left = frame[0] as f64;
        let right = frame[1] as f64;
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5 * params.stereo.width;
        frame[0] = ((mid + side) * left_gain) as f32;
        frame[1] = ((mid - side) * right_gain) as f32;
    }
}

/// Remove only the low-frequency component of the side signal. This preserves
/// stereo ambience while keeping sub/bass translation stable on vinyl, clubs,
/// mono playback, and lossy codecs.
fn apply_low_end_mono_guard(audio: &mut DecodedAudio, cutoff_hz: f64) {
    if audio.channels != 2 || audio.samples.len() < 2 {
        return;
    }
    let coefficient =
        1.0 - (-2.0 * std::f64::consts::PI * cutoff_hz / audio.sample_rate as f64).exp();
    let mut low_side = 0.0f64;
    for frame in audio.samples.as_chunks_mut::<2>().0 {
        let left = frame[0] as f64;
        let right = frame[1] as f64;
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;
        low_side += coefficient * (side - low_side);
        let guarded_side = side - low_side;
        frame[0] = (mid + guarded_side) as f32;
        frame[1] = (mid - guarded_side) as f32;
    }
}

/// A deliberately subtle dynamic high-band guard. It attenuates the extracted
/// >5.5 kHz component by at most 2 dB during sustained sibilant/resonant peaks.
fn apply_resonance_and_sibilance_guard(audio: &mut DecodedAudio) {
    let channels = audio.channels as usize;
    if channels == 0 {
        return;
    }
    let low_pass_coefficient =
        1.0 - (-2.0 * std::f64::consts::PI * 5_500.0 / audio.sample_rate as f64).exp();
    let attack = coefficient(2.0, audio.sample_rate);
    let release = coefficient(70.0, audio.sample_rate);
    let mut low = vec![0.0f64; channels];
    let mut envelope = 0.0f64;
    for frame in audio.samples.chunks_exact_mut(channels) {
        let mut detector = 0.0f64;
        let mut high = [0.0f64; 2];
        for (channel, sample) in frame.iter().enumerate() {
            low[channel] += low_pass_coefficient * (*sample as f64 - low[channel]);
            high[channel] = *sample as f64 - low[channel];
            detector = detector.max(high[channel].abs());
        }
        let smoothing = if detector > envelope { attack } else { release };
        envelope = smoothing * envelope + (1.0 - smoothing) * detector;
        let level_db = 20.0 * envelope.max(1e-12).log10();
        let attenuation_db = ((level_db + 18.0) * 0.2).clamp(0.0, 2.0);
        let high_gain = 10.0f64.powf(-attenuation_db / 20.0);
        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = (low[channel] + high[channel] * high_gain) as f32;
        }
    }
}

fn apply_gain(samples: &mut [f32], gain_db: f64) {
    let gain = 10.0f64.powf(gain_db / 20.0) as f32;
    for sample in samples {
        *sample *= gain;
    }
}

fn apply_lookahead_limiter(audio: &mut DecodedAudio, ceiling_db: f64, release_ms: f64) {
    let channels = audio.channels as usize;
    let frames = audio.samples.len() / channels;
    let lookahead = ((audio.sample_rate as f64 * 0.005).round() as usize).max(1);
    let ceiling = 10.0f64.powf(ceiling_db / 20.0) as f32;
    let mut deque: VecDeque<(usize, f32)> = VecDeque::new();

    // Maintain a bounded monotonic look-ahead window. Earlier versions built
    // two arrays proportional to track duration; this uses O(lookahead)
    // auxiliary memory and predicts four-times oversampled inter-sample peaks.
    for index in 0..frames.min(lookahead + 1) {
        let peak = oversampled_frame_peak(audio, index);
        while deque
            .back()
            .is_some_and(|(_, candidate_peak)| *candidate_peak <= peak)
        {
            deque.pop_back();
        }
        deque.push_back((index, peak));
    }

    let release = coefficient(release_ms, audio.sample_rate) as f32;
    let mut gain = 1.0f32;
    for index in 0..frames {
        let future_peak = deque
            .front()
            .map(|(_, candidate_peak)| *candidate_peak)
            .unwrap_or(0.0);
        let desired = if future_peak > ceiling {
            ceiling / future_peak
        } else {
            1.0
        };
        gain = if desired < gain {
            desired
        } else {
            release * gain + (1.0 - release) * desired
        };
        let frame_start = index * channels;
        for sample in &mut audio.samples[frame_start..frame_start + channels] {
            *sample *= gain;
        }

        if deque
            .front()
            .is_some_and(|(candidate, _)| *candidate == index)
        {
            deque.pop_front();
        }
        let entering = index + lookahead + 1;
        if entering < frames {
            let peak = oversampled_frame_peak(audio, entering);
            while deque
                .back()
                .is_some_and(|(_, candidate_peak)| *candidate_peak <= peak)
            {
                deque.pop_back();
            }
            deque.push_back((entering, peak));
        }
    }
}

fn oversampled_frame_peak(audio: &DecodedAudio, frame: usize) -> f32 {
    let channels = audio.channels as usize;
    let frames = audio.samples.len() / channels;
    let next = (frame + 1).min(frames.saturating_sub(1));
    let previous = frame.saturating_sub(1);
    let after_next = (frame + 2).min(frames.saturating_sub(1));
    let mut peak = 0.0f32;
    for channel in 0..channels {
        let p0 = audio.samples[previous * channels + channel];
        let p1 = audio.samples[frame * channels + channel];
        let p2 = audio.samples[next * channels + channel];
        let p3 = audio.samples[after_next * channels + channel];
        peak = peak.max(p1.abs());
        for phase in 1..4 {
            let t = phase as f32 * 0.25;
            // Catmull-Rom interpolation provides a conservative, inexpensive
            // four-times oversampled detector for the limiter side-chain.
            let value = 0.5
                * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t);
            peak = peak.max(value.abs());
        }
    }
    peak
}

fn enforce_true_peak_ceiling(audio: &mut DecodedAudio, ceiling_db: f64) -> Result<()> {
    let report = analysis::analyze(Path::new("limited.wav"), audio)?;
    if report.true_peak_db > ceiling_db {
        apply_gain(&mut audio.samples, ceiling_db - report.true_peak_db);
    }
    Ok(())
}

fn write_wav(path: &Path, audio: &DecodedAudio, bit_depth: u16) -> Result<()> {
    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: bit_depth,
        sample_format: if bit_depth == 32 {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        },
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("Creating WAV output: {}", path.display()))?;
    let mut random = 0x4d59_5df4_d0f3_3173u64;

    match bit_depth {
        16 => {
            for sample in &audio.samples {
                let dither = tpdf(&mut random) / i16::MAX as f64;
                let value =
                    ((*sample as f64 + dither).clamp(-1.0, 1.0) * i16::MAX as f64).round() as i16;
                writer.write_sample(value)?;
            }
        }
        24 => {
            let maximum = 8_388_607.0;
            for sample in &audio.samples {
                let dither = tpdf(&mut random) / maximum;
                let value = ((*sample as f64 + dither).clamp(-1.0, 1.0) * maximum).round() as i32;
                writer.write_sample(value)?;
            }
        }
        32 => {
            for sample in &audio.samples {
                writer.write_sample(*sample)?;
            }
        }
        _ => anyhow::bail!("Unsupported WAV bit depth: {bit_depth}"),
    }
    writer.finalize()?;
    Ok(())
}

fn tpdf(state: &mut u64) -> f64 {
    fn uniform(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((*state >> 11) as f64) / ((1u64 << 53) as f64)
    }
    uniform(state) - uniform(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CompressionParams, EqBand, LimiterParams, StereoParams};

    fn params() -> MasteringParams {
        MasteringParams {
            eq: vec![EqBand {
                frequency: 1_000.0,
                gain_db: 0.0,
                q: 0.707,
                band_type: EqBandType::Peak,
            }],
            compression: CompressionParams {
                threshold_db: -20.0,
                ratio: 2.0,
                attack_ms: 10.0,
                release_ms: 100.0,
                knee_db: 6.0,
                makeup_gain_db: 0.0,
            },
            limiter: LimiterParams {
                enabled: true,
                ceiling_db: -1.0,
                release_ms: 50.0,
            },
            stereo: StereoParams {
                width: 1.0,
                balance: 0.0,
            },
            target_lufs: -14.0,
        }
    }

    #[test]
    fn validation_enforces_request_and_safety_bounds() {
        let mut proposed = params();
        proposed.eq[0].gain_db = 50.0;
        proposed.stereo.width = 5.0;
        proposed.target_lufs = -1.0;
        let validated = validate_params(proposed, 48_000, -14.0, true).unwrap();
        assert_eq!(validated.params.eq[0].gain_db, 12.0);
        assert_eq!(validated.params.stereo.width, 2.0);
        assert_eq!(validated.params.target_lufs, -14.0);
        assert!(!validated.params.limiter.enabled);
        assert!(!validated.warnings.is_empty());
    }

    #[test]
    fn limiter_keeps_samples_below_ceiling() {
        let mut audio = DecodedAudio {
            samples: vec![1.5; 4_800],
            sample_rate: 48_000,
            channels: 1,
            total_frames: 4_800,
        };
        apply_lookahead_limiter(&mut audio, -1.0, 50.0);
        let ceiling = 10.0f32.powf(-1.0 / 20.0);
        assert!(audio
            .samples
            .iter()
            .all(|sample| sample.abs() <= ceiling + 1e-6));
    }

    #[test]
    fn low_end_guard_collapses_sub_side_without_collapsing_all_stereo() {
        let mut samples = Vec::new();
        for frame in 0..48_000 {
            let low = (2.0 * std::f32::consts::PI * 50.0 * frame as f32 / 48_000.0).sin();
            let high = (2.0 * std::f32::consts::PI * 2_000.0 * frame as f32 / 48_000.0).sin();
            samples.extend([low + high, -low - high]);
        }
        let mut audio = DecodedAudio {
            samples,
            sample_rate: 48_000,
            channels: 2,
            total_frames: 48_000,
        };
        apply_low_end_mono_guard(&mut audio, 100.0);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
        assert!(audio.samples.iter().any(|sample| sample.abs() > 0.1));
    }
}
