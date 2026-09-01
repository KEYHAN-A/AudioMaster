import { reactive, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

const state = reactive({
  models: [],
  online: null,
  loadedModels: [],
  loading: false,
  loadingModel: null,
  unloadingModel: null,
  error: null,
  endpoint: null,
});

const hasModels = computed(() => state.models.length > 0);

function resolveEndpoint(config) {
  return config?.ai?.lmstudio?.endpoint || state.endpoint || null;
}

async function checkStatus(endpoint) {
  const ep = endpoint || state.endpoint || undefined;
  try {
    const result = await invoke("lmstudio_status", { endpoint: ep });
    state.online = result.running;
    state.endpoint = result.endpoint || state.endpoint;
    return result.running;
  } catch (_) {
    state.online = false;
    return false;
  }
}

async function loadModels(endpoint) {
  const ep = endpoint || state.endpoint || undefined;
  state.loading = true;
  state.error = null;
  try {
    state.models = await invoke("lmstudio_models", { endpoint: ep });
    state.loadedModels = state.models.filter((m) => m.loaded === true);
  } catch (e) {
    state.error = String(e);
    state.models = [];
    state.loadedModels = [];
  } finally {
    state.loading = false;
  }
}

async function refresh(endpoint) {
  const running = await checkStatus(endpoint);
  if (running) {
    await loadModels(endpoint || state.endpoint);
  } else {
    state.models = [];
    state.loadedModels = [];
  }
}

async function loadModel(modelId, endpoint) {
  const ep = endpoint || state.endpoint || undefined;
  state.loadingModel = modelId;
  try {
    await invoke("lmstudio_load_model", { endpoint: ep, modelId });
    await loadModels(ep);
  } catch (e) {
    state.error = String(e);
    throw e;
  } finally {
    state.loadingModel = null;
  }
}

async function unloadModel(modelId, endpoint) {
  const ep = endpoint || state.endpoint || undefined;
  state.unloadingModel = modelId;
  try {
    await invoke("lmstudio_unload_model", { endpoint: ep, modelId });
    await loadModels(ep);
  } catch (e) {
    state.error = String(e);
    throw e;
  } finally {
    state.unloadingModel = null;
  }
}

async function getLoadedModels(endpoint) {
  const ep = endpoint || state.endpoint || undefined;
  try {
    state.loadedModels = await invoke("lmstudio_loaded_models", { endpoint: ep });
  } catch (_) {
    state.loadedModels = [];
  }
}

export function useLmStudio() {
  return {
    state,
    hasModels,
    resolveEndpoint,
    checkStatus,
    loadModels,
    refresh,
    loadModel,
    unloadModel,
    getLoadedModels,
  };
}
