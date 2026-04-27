use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub gpu_name: String,
    pub vram_total_mb: u64,
    pub metal_support: Option<String>,
    pub is_apple_silicon: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model_id: String,
    pub display_name: String,
    pub size_gb: f64,
    pub quant: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramTier {
    pub tier_name: String,
    pub vram_mb: u64,
    pub recommended_models: Vec<ModelRecommendation>,
}

pub fn detect_vram() -> Result<Vec<GpuInfo>> {
    let mut gpus = Vec::new();

    let output = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut current_chipset = String::new();
    let mut current_metal = String::new();
    let mut is_apple_silicon = false;

    for line in stdout.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Chipset Model:") {
            current_chipset = trimmed
                .trim_start_matches("Chipset Model:")
                .trim()
                .to_string();
            is_apple_silicon = current_chipset.contains("Apple")
                || current_chipset.contains("M1")
                || current_chipset.contains("M2")
                || current_chipset.contains("M3")
                || current_chipset.contains("M4");
        }

        if trimmed.starts_with("Metal:") {
            current_metal = trimmed
                .trim_start_matches("Metal:")
                .trim()
                .to_string();
        }

        if trimmed.contains("VRAM (Total):") {
            let vram_str = trimmed
                .split(':')
                .last()
                .unwrap_or("")
                .trim();
            let vram_mb = parse_vram_string(vram_str);

            if !current_chipset.is_empty() {
                gpus.push(GpuInfo {
                    gpu_name: current_chipset.clone(),
                    vram_total_mb: vram_mb,
                    metal_support: if current_metal.is_empty() {
                        None
                    } else {
                        Some(current_metal.clone())
                    },
                    is_apple_silicon,
                });
            }
        }
    }

    // Apple Silicon: no VRAM line in system_profiler, use total RAM
    if gpus.is_empty() && is_apple_silicon && !current_chipset.is_empty() {
        let total_ram_mb = detect_total_ram_mb().unwrap_or(0);
        // On Apple Silicon, GPU shares unified memory; ~70% available for GPU work
        let vram_mb = (total_ram_mb as f64 * 0.70) as u64;

        gpus.push(GpuInfo {
            gpu_name: current_chipset,
            vram_total_mb: vram_mb,
            metal_support: if current_metal.is_empty() {
                None
            } else {
                Some(current_metal)
            },
            is_apple_silicon: true,
        });
    }

    if gpus.is_empty() {
        warn!("No GPU detected via system_profiler");
    }

    Ok(gpus)
}

fn parse_vram_string(s: &str) -> u64 {
    let s = s.trim();
    if s.ends_with("GB") {
        s.trim_end_matches("GB")
            .trim()
            .parse::<f64>()
            .map(|v| (v * 1024.0) as u64)
            .unwrap_or(0)
    } else if s.ends_with("MB") {
        s.trim_end_matches("MB")
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        0
    }
}

fn detect_total_ram_mb() -> Result<u64> {
    let output = Command::new("sysctl").arg("-n").arg("hw.memsize").output()?;
    let mem_bytes: u64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    Ok(mem_bytes / (1024 * 1024))
}

pub fn get_vram_tiers() -> Vec<VramTier> {
    vec![
        VramTier {
            tier_name: "4GB".into(),
            vram_mb: 4096,
            recommended_models: vec![
                ModelRecommendation {
                    model_id: "lmstudio-community/qwen3-4b".into(),
                    display_name: "Qwen 3 4B".into(),
                    size_gb: 2.5,
                    quant: "Q4_K_M".into(),
                    notes: "Fast, good structured output for its size".into(),
                },
                ModelRecommendation {
                    model_id: "lmstudio-community/phi-4-mini-instruct".into(),
                    display_name: "Phi-4 Mini".into(),
                    size_gb: 2.5,
                    quant: "Q4_K_M".into(),
                    notes: "Excellent reasoning, tiny footprint".into(),
                },
            ],
        },
        VramTier {
            tier_name: "8GB".into(),
            vram_mb: 8192,
            recommended_models: vec![
                ModelRecommendation {
                    model_id: "lmstudio-community/qwen3-8b".into(),
                    display_name: "Qwen 3 8B".into(),
                    size_gb: 5.0,
                    quant: "Q4_K_M".into(),
                    notes: "Best quality/speed ratio for JSON tasks".into(),
                },
                ModelRecommendation {
                    model_id: "lmstudio-community/gemma-3-8b".into(),
                    display_name: "Gemma 3 8B".into(),
                    size_gb: 5.5,
                    quant: "Q4_K_M".into(),
                    notes: "Strong instruction following".into(),
                },
            ],
        },
        VramTier {
            tier_name: "16GB".into(),
            vram_mb: 16384,
            recommended_models: vec![
                ModelRecommendation {
                    model_id: "lmstudio-community/qwen/qwen3.6-27b".into(),
                    display_name: "Qwen 3.5 27B".into(),
                    size_gb: 17.5,
                    quant: "Q4_K_M".into(),
                    notes: "Outstanding reasoning, excellent JSON output".into(),
                },
                ModelRecommendation {
                    model_id: "lmstudio-community/google/gemma-4-26b-a4b".into(),
                    display_name: "Gemma 4 26B (MoE)".into(),
                    size_gb: 18.0,
                    quant: "Q4_K_M".into(),
                    notes: "Active-parameters MoE, fast inference".into(),
                },
            ],
        },
        VramTier {
            tier_name: "24GB".into(),
            vram_mb: 24576,
            recommended_models: vec![
                ModelRecommendation {
                    model_id: "lmstudio-community/qwen/qwen3.6-35b-a3b".into(),
                    display_name: "Qwen 3.5 35B-A3B (MoE)".into(),
                    size_gb: 22.1,
                    quant: "Q4_K_M".into(),
                    notes: "Near-cloud-quality reasoning, MoE efficiency".into(),
                },
                ModelRecommendation {
                    model_id: "lmstudio-community/zai-org/glm-4.7-flash".into(),
                    display_name: "GLM 4.7 Flash 30B".into(),
                    size_gb: 24.4,
                    quant: "6bit".into(),
                    notes: "Strong multilingual, great structured output".into(),
                },
            ],
        },
        VramTier {
            tier_name: "32GB+".into(),
            vram_mb: 32768,
            recommended_models: vec![
                ModelRecommendation {
                    model_id: "mlx-community/deepseek-r1-distill-qwen-32b".into(),
                    display_name: "DeepSeek R1 Distill Qwen 32B".into(),
                    size_gb: 18.4,
                    quant: "4bit".into(),
                    notes: "Deep reasoning chain, excellent analysis".into(),
                },
                ModelRecommendation {
                    model_id: "lmstudio-community/qwen/qwen3-coder-30b".into(),
                    display_name: "Qwen 3 Coder 30B".into(),
                    size_gb: 17.2,
                    quant: "4bit".into(),
                    notes: "Top-tier structured output quality".into(),
                },
            ],
        },
    ]
}

pub fn get_recommendations_for_vram(vram_mb: u64) -> Vec<ModelRecommendation> {
    let tiers = get_vram_tiers();
    tiers
        .iter()
        .filter(|t| vram_mb >= t.vram_mb)
        .last()
        .map(|t| t.recommended_models.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vram_gb() {
        assert_eq!(parse_vram_string("8 GB"), 8192);
        assert_eq!(parse_vram_string("16384 MB"), 16384);
        assert_eq!(parse_vram_string("16GB"), 16384);
    }

    #[test]
    fn test_get_tiers() {
        let tiers = get_vram_tiers();
        assert!(!tiers.is_empty());
        assert!(tiers[0].vram_mb < tiers[1].vram_mb);
    }

    #[test]
    fn test_recommendations_for_vram() {
        let recs = get_recommendations_for_vram(16384);
        assert!(!recs.is_empty());
        let recs_small = get_recommendations_for_vram(2048);
        assert!(recs_small.is_empty());
    }
}
