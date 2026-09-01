import { reactive, computed, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { trackProcessing, trackError, trackFeature } from "./useAnalytics.js";

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
  lastWarnings: [],
  currentJobId: null,
  preview: null,

  // Master options
  selectedBackend: "auto",
  selectedPreset: "streaming",
  selectedProvider: "ollama",
  bitDepth: 24,
  outputFormat: "wav",
  targetLufs: -14.0,
  noLimiter: false,
  albumMode: true,
  albumMaxRelativeOffsetLu: 1.5,

  // LM Studio state
  selectedLmStudioModel: "",
});

let progressListener = null;

async function ensureProgressListener() {
  if (progressListener) return;
  progressListener = await listen("mastering-progress", ({ payload }) => {
    if (!payload || payload.job_id !== state.currentJobId) return;
    state.processingProgress = Math.round((payload.progress?.fraction || 0) * 100);
    state.processingMessage = payload.progress?.message || state.processingMessage;
  });
}

const hasTracks = computed(() => state.tracks.length > 0);
const selectedTrack = computed(() =>
  state.tracks.find((t) => t.id === state.selectedTrackId) || null
);
const analyzedTracks = computed(() =>
  state.tracks.filter((t) => t.status === "analyzed" || t.status === "done")
);
const allAnalyzed = computed(() =>
  state.tracks.length > 0 && state.tracks.every((t) => t.status === "analyzed" || t.status === "done")
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
    if (state.config?.general) {
      state.selectedBackend = state.config.general.default_backend || "auto";
      state.bitDepth = state.config.general.default_bit_depth || 24;
      state.outputFormat = state.config.general.default_format || "wav";
      state.targetLufs = state.config.general.target_lufs ?? -14.0;
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
  trackFeature("tracks_imported", `${newPaths.length} tracks`);
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
      albumOffsetLu: 0,
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
  if (path) trackFeature("reference_set");
}

async function analyzeTrack(track) {
  track.status = "analyzing";
  track.error = null;
  const start = Date.now();
  try {
    const [analysis, waveform] = await Promise.all([
      invoke("analyze_file", { path: track.path }),
      invoke("get_waveform_data", { path: track.path, numPoints: 2000 }),
    ]);
    track.analysis = analysis;
    track.waveform = waveform;
    track.status = "analyzed";
    trackProcessing("analysis", "native", Date.now() - start, true);
  } catch (e) {
    const error = parseTauriError(e, "analysis", track.id);
    track.status = "error";
    track.error = error;
    state.error = error;
    trackProcessing("analysis", "native", Date.now() - start, false);
    trackError("ANALYSIS_FAILED", e);
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
  const jobId = globalThis.crypto?.randomUUID?.() || `job-${Date.now()}-${track.id}`;
  state.currentJobId = jobId;
  return {
    job_id: jobId,
    overwrite: false,
    input_path: track.path,
    output_path: outputPath || null,
    reference_path: state.referenceFile || null,
    backend: state.selectedBackend,
    ai_provider: state.selectedBackend === "ai" ? state.selectedProvider : null,
    lmstudio_model: state.selectedProvider === "lmstudio" ? state.selectedLmStudioModel || null : null,
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
  const start = Date.now();
  try {
    await ensureProgressListener();
    const request = buildRequest(track, outputPath);
    let result;
    try {
      result = await invoke("master_file", { request });
    } catch (error) {
      if (String(error).includes("explicit overwrite confirmation is required") &&
          globalThis.confirm("The output file already exists. Replace it with the new verified master?")) {
        request.overwrite = true;
        result = await invoke("master_file", { request });
      } else {
        throw error;
      }
    }
    track.result = result;
    state.lastWarnings.push(
      ...(result.warnings || []).map((warning) => ({ ...warning, track: track.name }))
    );
    track.status = "done";
    trackProcessing("mastering", state.selectedBackend, Date.now() - start, true);
    trackFeature("mastering_complete", result.backend_used);
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
    const error = parseTauriError(e, "mastering", track.id);
    if (error.code === "JOB_CANCELLED") {
      track.status = track.analysis ? "analyzed" : "idle";
      track.error = null;
      return;
    }
    track.status = "error";
    track.error = error;
    state.error = error;
    trackProcessing("mastering", state.selectedBackend, Date.now() - start, false);
    trackError("MASTERING_FAILED", e, { backend: state.selectedBackend });
  }
}

async function cancelMastering() {
  if (!state.currentJobId) return;
  await invoke("cancel_mastering", { jobId: state.currentJobId });
  state.processingMessage = "Cancelling safely...";
}

async function createPreview(track = selectedTrack.value) {
  if (!track) return;
  state.processing = true;
  state.processingMessage = "Creating level-matched preview...";
  state.processingProgress = 0;
  try {
    const request = buildRequest(track, null);
    const preview = await invoke("create_mastering_preview", { request });
    state.preview = {
      ...preview,
      trackId: track.id,
      originalUrl: convertFileSrc(preview.original_path),
      masteredUrl: convertFileSrc(preview.mastered_path),
    };
  } catch (error) {
    state.error = parseTauriError(error, "preview", track.id);
  } finally {
    state.processing = false;
    state.processingMessage = "";
    state.processingProgress = 0;
  }
}

async function masterAll() {
  state.processing = true;
  state.error = null;
  state.lastWarnings = [];
  const targets = state.tracks.filter(
    (t) => t.status === "analyzed" || t.status === "done"
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

async function masterAlbum(outputDirectory) {
  const targets = state.tracks.filter(
    (track) => track.status === "analyzed" || track.status === "done"
  );
  if (targets.length < 2) return masterAll();
  state.processing = true;
  state.error = null;
  state.lastWarnings = [];
  state.processingMessage = `Mastering album (${targets.length} tracks)...`;
  state.processingProgress = 10;
  for (const track of targets) {
    track.status = "mastering";
    track.error = null;
  }
  try {
    const response = await invoke("master_album", {
      request: {
        input_paths: targets.map((track) => track.path),
        output_directory: outputDirectory,
        reference_path: state.referenceFile || null,
        backend: state.selectedBackend,
        ai_provider: state.selectedBackend === "ai" ? state.selectedProvider : null,
        bit_depth: state.bitDepth,
        format: state.outputFormat,
        target_lufs: state.targetLufs,
        preset: state.selectedPreset,
        no_limiter: state.noLimiter,
        max_relative_offset_lu: state.albumMaxRelativeOffsetLu,
        track_offsets_lu: targets.map((track) => track.albumOffsetLu || 0),
      },
    });
    for (const albumTrack of response.tracks) {
      const track = targets.find((candidate) => candidate.path === albumTrack.input_path);
      if (!track) continue;
      track.result = albumTrack.result;
      track.postAnalysis = albumTrack.result.post_analysis;
      track.assignedTargetLufs = albumTrack.assigned_target_lufs;
      track.status = "done";
      state.lastWarnings.push(
        ...(albumTrack.result.warnings || []).map((warning) => ({ ...warning, track: track.name }))
      );
    }
    state.albumResult = response;
    state.processingProgress = 100;
    trackFeature("album_mastering_complete", `${targets.length} tracks`);
  } catch (e) {
    const error = parseTauriError(e, "album", null);
    for (const track of targets.filter((candidate) => candidate.status === "mastering")) {
      track.status = "error";
      track.error = error;
    }
    state.error = error;
    trackError("ALBUM_MASTERING_FAILED", e, { backend: state.selectedBackend });
  } finally {
    state.processing = false;
    state.processingMessage = "";
    state.processingProgress = 0;
  }
}

function parseTauriError(error, operation, trackId) {
  if (error && typeof error === "object") {
    return { ...error, operation, trackId };
  }
  const text = String(error ?? "Unknown error");
  try {
    return { ...JSON.parse(text), operation, trackId };
  } catch {
    return {
      message: text,
      code: "UNKNOWN_ERROR",
      can_retry: true,
      can_fallback: operation === "mastering",
      suggested_action: null,
      operation,
      trackId,
    };
  }
}

async function masterSelected(outputPath) {
  const track = selectedTrack.value;
  if (!track) return;
  state.processing = true;
  state.lastWarnings = [];
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
    masterAlbum,
    masterSelected,
    cancelMastering,
    createPreview,
    clearAll,
  };
}
