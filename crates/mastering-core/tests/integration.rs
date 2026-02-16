use mastering_core::config::Config;
use mastering_core::types::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_test_wav() -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".wav").unwrap();

    let sample_rate: u32 = 44100;
    let channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let num_samples: u32 = sample_rate * 2; // 2 seconds
    let data_size: u32 = num_samples * channels as u32 * (bits_per_sample / 8) as u32;
    let file_size: u32 = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    let block_align = channels * (bits_per_sample / 8);
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&bits_per_sample.to_le_bytes()).unwrap();

    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write a 440Hz sine wave
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample_l = (16000.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as i16;
        let sample_r = (14000.0 * (2.0 * std::f64::consts::PI * 660.0 * t).sin()) as i16;
        file.write_all(&sample_l.to_le_bytes()).unwrap();
        file.write_all(&sample_r.to_le_bytes()).unwrap();
    }

    file.flush().unwrap();
    file
}

#[test]
fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.general.default_backend, Backend::Auto);
    assert_eq!(config.general.default_bit_depth, 24);
    assert_eq!(config.general.target_lufs, -14.0);
    assert_eq!(config.ai.default_provider, AiProvider::Ollama);
}

#[test]
fn test_config_roundtrip() {
    let config = Config::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.general.default_backend, Backend::Auto);
    assert_eq!(parsed.ai.ollama.model, "llama3");
}

#[test]
fn test_backend_parsing() {
    assert_eq!("auto".parse::<Backend>().unwrap(), Backend::Auto);
    assert_eq!("matchering".parse::<Backend>().unwrap(), Backend::Matchering);
    assert_eq!("ai".parse::<Backend>().unwrap(), Backend::Ai);
    assert_eq!("local-ml".parse::<Backend>().unwrap(), Backend::LocalMl);
    assert!("invalid".parse::<Backend>().is_err());
}

#[test]
fn test_ai_provider_parsing() {
    assert_eq!("ollama".parse::<AiProvider>().unwrap(), AiProvider::Ollama);
    assert_eq!(
        "keyhanstudio".parse::<AiProvider>().unwrap(),
        AiProvider::KeyhanStudio
    );
    assert_eq!("openai".parse::<AiProvider>().unwrap(), AiProvider::OpenAi);
    assert_eq!(
        "anthropic".parse::<AiProvider>().unwrap(),
        AiProvider::Anthropic
    );
    assert_eq!(
        "claude".parse::<AiProvider>().unwrap(),
        AiProvider::Anthropic
    );
}

#[test]
fn test_preset_values() {
    assert_eq!(Preset::Streaming.target_lufs(), -14.0);
    assert_eq!(Preset::Cd.target_lufs(), -9.0);
    assert_eq!(Preset::Vinyl.target_lufs(), -12.0);
    assert_eq!(Preset::Loud.target_lufs(), -6.0);
}

#[tokio::test]
async fn test_audio_analysis() {
    let wav_file = create_test_wav();

    let analysis = mastering_core::analysis::analyze_file(wav_file.path())
        .await
        .unwrap();

    assert_eq!(analysis.metadata.sample_rate, 44100);
    assert_eq!(analysis.metadata.channels, 2);
    assert!((analysis.metadata.duration_secs - 2.0).abs() < 0.1);

    // LUFS should be reasonable for our test tone
    assert!(analysis.lufs_integrated > -20.0);
    assert!(analysis.lufs_integrated < 0.0);

    // Peak should be below 0 dB
    assert!(analysis.peak_db < 0.0);

    // Stereo width should be ~1.0 for uncorrelated stereo
    assert!(analysis.stereo_width > 0.5);
}

#[test]
fn test_mastering_job_output_path() {
    use mastering_core::pipeline::MasteringJob;
    use std::path::PathBuf;

    let config = Config::default();
    let job = MasteringJob {
        input_path: PathBuf::from("/tmp/my_song.wav"),
        output_path: None,
        reference_path: None,
        backend: Backend::Auto,
        ai_provider: None,
        bit_depth: None,
        format: None,
        target_lufs: None,
        no_limiter: false,
        preset: None,
        dry_run: false,
    };

    let output = job.resolved_output_path(&config);
    assert_eq!(output, PathBuf::from("/tmp/my_song_mastered.wav"));
}

#[test]
fn test_mastering_job_auto_backend() {
    use mastering_core::pipeline::MasteringJob;
    use std::path::PathBuf;

    let job_no_ref = MasteringJob {
        input_path: PathBuf::from("song.wav"),
        output_path: None,
        reference_path: None,
        backend: Backend::Auto,
        ai_provider: None,
        bit_depth: None,
        format: None,
        target_lufs: None,
        no_limiter: false,
        preset: None,
        dry_run: false,
    };
    assert_eq!(job_no_ref.resolved_backend(), Backend::Ai);

    let job_with_ref = MasteringJob {
        input_path: PathBuf::from("song.wav"),
        output_path: None,
        reference_path: Some(PathBuf::from("ref.wav")),
        backend: Backend::Auto,
        ai_provider: None,
        bit_depth: None,
        format: None,
        target_lufs: None,
        no_limiter: false,
        preset: None,
        dry_run: false,
    };
    assert_eq!(job_with_ref.resolved_backend(), Backend::Matchering);
}
