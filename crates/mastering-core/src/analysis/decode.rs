use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Temporary safety ceiling until the decode and DSP graph are block-streamed.
/// This bounds a single f32 sample buffer to 256 MiB.
const MAX_DECODED_SAMPLES: usize = 64 * 1024 * 1024;

/// Decoded audio data: interleaved f32 samples with metadata.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub total_frames: u64,
}

impl DecodedAudio {
    /// Get samples for a single channel (0-indexed).
    pub fn channel_samples(&self, ch: u16) -> Vec<f32> {
        self.samples
            .iter()
            .skip(ch as usize)
            .step_by(self.channels as usize)
            .copied()
            .collect()
    }

    pub fn duration_secs(&self) -> f64 {
        self.total_frames as f64 / self.sample_rate as f64
    }
}

/// Decode an audio file into interleaved f32 samples using symphonia.
pub fn decode_audio(path: &Path) -> Result<DecodedAudio> {
    decode_audio_with_limit(path, MAX_DECODED_SAMPLES, false)
}

/// Decode a bounded prefix for secondary metrics when the full job is handled
/// by the streaming path.
pub fn decode_audio_prefix(path: &Path, max_samples: usize) -> Result<DecodedAudio> {
    decode_audio_with_limit(path, max_samples, true)
}

fn decode_audio_with_limit(
    path: &Path,
    max_samples: usize,
    truncate: bool,
) -> Result<DecodedAudio> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening audio file: {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("Probing audio format: {}", path.display()))?;

    let mut format_reader = probed.format;

    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .context("No supported audio track found")?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let sample_rate = codec_params.sample_rate.context("Missing sample rate")?;
    let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
    anyhow::ensure!(
        channels == 1 || channels == 2,
        "Only mono and stereo sources are supported (found {channels} channels)"
    );
    anyhow::ensure!(
        (8_000..=384_000).contains(&sample_rate),
        "Unsupported sample rate: {sample_rate} Hz"
    );
    if !truncate {
        if let Some(frames) = codec_params.n_frames {
            let estimated_samples = usize::try_from(frames)
                .ok()
                .and_then(|value| value.checked_mul(channels as usize))
                .context("Decoded audio is too large for this build")?;
            anyhow::ensure!(
                estimated_samples <= max_samples,
                "Decoded audio exceeds the current 256 MiB processing limit"
            );
        }
    }

    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &dec_opts)
        .context("Creating audio decoder")?;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut total_frames: u64 = 0;

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e).context("Reading packet"),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("Decoding packet"),
        };

        let spec = *decoded.spec();
        let num_frames = decoded.frames();
        total_frames += num_frames as u64;

        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let remaining = max_samples.saturating_sub(all_samples.len());
        if sample_buf.samples().len() > remaining {
            if truncate {
                let aligned = remaining - remaining % channels as usize;
                all_samples.extend_from_slice(&sample_buf.samples()[..aligned]);
                total_frames = (all_samples.len() / channels as usize) as u64;
                break;
            }
            anyhow::bail!("Decoded audio exceeds the current 256 MiB processing limit");
        }
        all_samples.extend_from_slice(sample_buf.samples());
    }

    anyhow::ensure!(
        !all_samples.is_empty(),
        "Audio stream contains no decoded samples"
    );

    Ok(DecodedAudio {
        samples: all_samples,
        sample_rate,
        channels,
        total_frames,
    })
}
