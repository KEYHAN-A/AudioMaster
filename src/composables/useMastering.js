import { reactive, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

const state = reactive({
  inputFile: null,
  referenceFile: null,
  analysis: null,
  masterResult: null,
  processing: false,
  processingMessage: "",
  backends: [],
  presets: [],
  config: null,
  currentStep: 0,
  error: null,

  // Master options
  selectedBackend: "auto",
  selectedPreset: "streaming",
  selectedProvider: "ollama",
  bitDepth: 24,
  outputFormat: "wav",
  targetLufs: -14.0,
  noLimiter: false,
});

const hasInput = computed(() => state.inputFile !== null);
const hasReference = computed(() => state.referenceFile !== null);
const isAnalyzed = computed(() => state.analysis !== null);
const isMastered = computed(() => state.masterResult !== null);

async function loadConfig() {
  try {
    state.config = await invoke("get_config");
    if (state.config?.ai?.default_provider) {
      state.selectedProvider = state.config.ai.default_provider;
    }
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

async function loadBackends() {
  try {
    state.backends = await invoke("check_backends");
  } catch (e) {
    console.error("Failed to check backends:", e);
  }
}

async function loadPresets() {
  try {
    state.presets = await invoke("get_presets");
  } catch (e) {
    console.error("Failed to load presets:", e);
  }
}

function setInputFile(path) {
  state.inputFile = path;
  state.analysis = null;
  state.masterResult = null;
  state.currentStep = 0;
  state.error = null;
}

function setReferenceFile(path) {
  state.referenceFile = path;
}

async function analyzeInput() {
  if (!state.inputFile) return;

  state.processing = true;
  state.processingMessage = "Analyzing audio...";
  state.error = null;

  try {
    state.analysis = await invoke("analyze_file", { path: state.inputFile });
    state.currentStep = 1;
  } catch (e) {
    state.error = `Analysis failed: ${e}`;
  } finally {
    state.processing = false;
    state.processingMessage = "";
  }
}

async function masterTrack(outputPath) {
  if (!state.inputFile) return;

  state.processing = true;
  state.processingMessage = "Mastering in progress...";
  state.error = null;

  try {
    const request = {
      input_path: state.inputFile,
      output_path: outputPath || null,
      reference_path: state.referenceFile || null,
      backend: state.selectedBackend,
      ai_provider: state.selectedBackend === "ai" ? state.selectedProvider : null,
      bit_depth: state.bitDepth,
      format: state.outputFormat,
      target_lufs: state.targetLufs,
      preset: state.selectedPreset,
      no_limiter: state.noLimiter,
    };

    state.masterResult = await invoke("master_file", { request });
    state.currentStep = 3;
  } catch (e) {
    state.error = `Mastering failed: ${e}`;
  } finally {
    state.processing = false;
    state.processingMessage = "";
  }
}

function clearAll() {
  state.inputFile = null;
  state.referenceFile = null;
  state.analysis = null;
  state.masterResult = null;
  state.currentStep = 0;
  state.error = null;
}

export function useMastering() {
  return {
    state,
    hasInput,
    hasReference,
    isAnalyzed,
    isMastered,
    loadConfig,
    loadBackends,
    loadPresets,
    setInputFile,
    setReferenceFile,
    analyzeInput,
    masterTrack,
    clearAll,
  };
}
