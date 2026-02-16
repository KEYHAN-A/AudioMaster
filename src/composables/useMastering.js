import { reactive, computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

let trackIdCounter = 0;

const state = reactive({
  tracks: [],
  selectedTrackId: null,
  referenceFile: null,
  processing: false,
  processingMessage: "",
  processingProgress: 0,
  backends: [],
  presets: [],
  config: null,
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

const hasTracks = computed(() => state.tracks.length > 0);
const selectedTrack = computed(() =>
  state.tracks.find((t) => t.id === state.selectedTrackId) || null
);
const analyzedTracks = computed(() =>
  state.tracks.filter((t) => t.status === "analyzed" || t.status === "done")
);
const allAnalyzed = computed(() =>
  state.tracks.length > 0 && state.tracks.every((t) => t.status !== "idle" && t.status !== "analyzing")
);
const hasAnyResult = computed(() =>
  state.tracks.some((t) => t.status === "done")
);

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

function addTracks(paths) {
  const newPaths = Array.isArray(paths) ? paths : [paths];
  for (const p of newPaths) {
    if (state.tracks.some((t) => t.path === p)) continue;
    const name = p.split("/").pop().split("\\").pop();
    state.tracks.push({
      id: ++trackIdCounter,
      path: p,
      name,
      status: "idle",
      analysis: null,
      waveform: null,
      result: null,
      error: null,
    });
  }
  if (!state.selectedTrackId && state.tracks.length > 0) {
    state.selectedTrackId = state.tracks[0].id;
  }
}

function removeTrack(id) {
  const idx = state.tracks.findIndex((t) => t.id === id);
  if (idx !== -1) state.tracks.splice(idx, 1);
  if (state.selectedTrackId === id) {
    state.selectedTrackId = state.tracks.length > 0 ? state.tracks[0].id : null;
  }
}

function selectTrack(id) {
  state.selectedTrackId = id;
}

function setReferenceFile(path) {
  state.referenceFile = path;
}

async function analyzeTrack(track) {
  track.status = "analyzing";
  track.error = null;
  try {
    const [analysis, waveform] = await Promise.all([
      invoke("analyze_file", { path: track.path }),
      invoke("get_waveform_data", { path: track.path, numPoints: 2000 }),
    ]);
    track.analysis = analysis;
    track.waveform = waveform;
    track.status = "analyzed";
  } catch (e) {
    track.status = "error";
    track.error = `Analysis failed: ${e}`;
  }
}

async function analyzeAll() {
  state.processing = true;
  state.error = null;
  const pending = state.tracks.filter((t) => t.status === "idle" || t.status === "error");
  for (let i = 0; i < pending.length; i++) {
    state.processingMessage = `Analyzing ${i + 1} of ${pending.length}...`;
    state.processingProgress = ((i + 1) / pending.length) * 100;
    await analyzeTrack(pending[i]);
  }
  state.processing = false;
  state.processingMessage = "";
  state.processingProgress = 0;
}

async function analyzeSelected() {
  const track = selectedTrack.value;
  if (!track) return;
  state.processing = true;
  state.processingMessage = `Analyzing ${track.name}...`;
  await analyzeTrack(track);
  state.processing = false;
  state.processingMessage = "";
}

function buildRequest(track, outputPath) {
  return {
    input_path: track.path,
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
}

async function masterTrack(track, outputPath) {
  track.status = "mastering";
  track.error = null;
  try {
    const request = buildRequest(track, outputPath);
    const result = await invoke("master_file", { request });
    track.result = result;
    track.status = "done";
    // Update analysis with post if available
    if (result.post_analysis) {
      track.postAnalysis = result.post_analysis;
      try {
        track.postWaveform = await invoke("get_waveform_data", {
          path: result.output_path,
          numPoints: 2000,
        });
      } catch (_) {}
    }
  } catch (e) {
    track.status = "error";
    track.error = `Mastering failed: ${e}`;
  }
}

async function masterAll() {
  state.processing = true;
  state.error = null;
  const targets = state.tracks.filter(
    (t) => t.status === "analyzed" || t.status === "error"
  );
  for (let i = 0; i < targets.length; i++) {
    state.processingMessage = `Mastering ${i + 1} of ${targets.length}: ${targets[i].name}`;
    state.processingProgress = ((i + 1) / targets.length) * 100;
    await masterTrack(targets[i]);
  }
  state.processing = false;
  state.processingMessage = "";
  state.processingProgress = 0;
}

async function masterSelected(outputPath) {
  const track = selectedTrack.value;
  if (!track) return;
  state.processing = true;
  state.processingMessage = `Mastering ${track.name}...`;
  await masterTrack(track, outputPath);
  state.processing = false;
  state.processingMessage = "";
}

function clearAll() {
  state.tracks.splice(0);
  state.selectedTrackId = null;
  state.referenceFile = null;
  state.error = null;
}

export function useMastering() {
  return {
    state,
    hasTracks,
    selectedTrack,
    analyzedTracks,
    allAnalyzed,
    hasAnyResult,
    loadConfig,
    loadBackends,
    loadPresets,
    addTracks,
    removeTrack,
    selectTrack,
    setReferenceFile,
    analyzeTrack,
    analyzeAll,
    analyzeSelected,
    masterTrack,
    masterAll,
    masterSelected,
    clearAll,
  };
}
