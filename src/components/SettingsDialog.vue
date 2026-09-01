<script setup>
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../composables/useToast.js";
import { useLmStudio } from "../composables/useLmStudio.js";
import { useCloud } from "../composables/useCloud.js";

const props = defineProps({
  visible: Boolean,
  config: Object,
});

const emit = defineEmits(["close"]);
const { showToast } = useToast();
const { state: lm, refresh: refreshLmStudio, loadModel, unloadModel } = useLmStudio();
const {
  state: cloud,
  refreshStatus: refreshCloudStatus,
  beginLogin,
  logout: cloudLogout,
  pullSync,
  pushSync,
  submitFeedback,
  setEarlyAccess,
} = useCloud();

const localConfig = ref(null);
const saving = ref(false);
const activeTab = ref("general");

// VRAM state
const vramDetecting = ref(false);
const vramInfo = ref(null);

// Recommendations state
const recommendations = ref(null);
const loadingRecommendations = ref(false);
const feedbackCategory = ref("general");
const feedbackMessage = ref("");
const diagnosticsOptIn = ref(false);

watch(
  () => props.config,
  (val) => {
    if (val) localConfig.value = JSON.parse(JSON.stringify(val));
  },
  { immediate: true, deep: true }
);

watch(
  () => props.visible,
  (visible) => {
    if (visible) refreshCloudStatus();
  }
);

function syncableSettings() {
  return {
    schema_version: 1,
    general: localConfig.value.general,
    ai: {
      default_provider: localConfig.value.ai.default_provider,
      ollama: localConfig.value.ai.ollama,
      lmstudio: localConfig.value.ai.lmstudio,
    },
  };
}

async function handleCloudLogin() {
  try {
    await beginLogin();
    const document = await pullSync();
    if (document.settings?.schema_version === 1) applyCloudSettings(document.settings);
    showToast("Signed in to KeyhanStudio", "success");
  } catch (e) {
    showToast(`Sign-in failed: ${e}`, "error");
  }
}

function applyCloudSettings(settings) {
  if (settings.general) Object.assign(localConfig.value.general, settings.general);
  if (settings.ai?.default_provider) localConfig.value.ai.default_provider = settings.ai.default_provider;
  if (settings.ai?.ollama) Object.assign(localConfig.value.ai.ollama, settings.ai.ollama);
  if (settings.ai?.lmstudio) Object.assign(localConfig.value.ai.lmstudio, settings.ai.lmstudio);
}

async function handlePullSync() {
  try {
    const document = await pullSync();
    applyCloudSettings(document.settings || {});
    showToast("Cloud settings downloaded", "success");
  } catch (e) {
    showToast(`Cloud download failed: ${e}`, "error");
  }
}

async function handlePushSync() {
  try {
    await pushSync(syncableSettings());
    showToast("Cloud settings updated", "success");
  } catch (e) {
    showToast(`Cloud update failed: ${e}`, "error");
  }
}

async function handleFeedback() {
  try {
    await submitFeedback(feedbackCategory.value, feedbackMessage.value, diagnosticsOptIn.value);
    feedbackMessage.value = "";
    showToast("Feedback sent—thank you", "success");
  } catch (e) {
    showToast(`Feedback failed: ${e}`, "error");
  }
}

async function handleEarlyAccess(event) {
  try {
    await setEarlyAccess(event.target.checked);
    showToast(cloud.earlyAccess ? "Early access enabled" : "Early access disabled", "success");
  } catch (e) {
    showToast(`Could not update early access: ${e}`, "error");
  }
}

async function testLmStudioConnection() {
  const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
  await refreshLmStudio(endpoint);
  if (lm.online) {
    showToast("LM Studio connected", "success");
  } else {
    showToast("LM Studio is not running", "error");
  }
}

async function handleLoadModel(modelId) {
  try {
    const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
    await loadModel(modelId, endpoint);
    showToast(`Model "${modelId}" loaded`, "success");
  } catch (e) {
    showToast(`Failed to load model: ${e}`, "error");
  }
}

async function handleUnloadModel(modelId) {
  try {
    const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
    await unloadModel(modelId, endpoint);
    showToast(`Model "${modelId}" unloaded`, "success");
  } catch (e) {
    showToast(`Failed to unload model: ${e}`, "error");
  }
}

async function detectGpu() {
  vramDetecting.value = true;
  try {
    vramInfo.value = await invoke("detect_vram");
    showToast("GPU detected", "success");
  } catch (e) {
    showToast(`GPU detection failed: ${e}`, "error");
  } finally {
    vramDetecting.value = false;
  }
}

async function exportDiagnostics() {
  try {
    const path = await invoke("export_diagnostic_bundle");
    showToast(`Diagnostics exported to ${path}`, "success", 8000);
  } catch (error) {
    showToast(`Diagnostics export failed: ${error}`, "error");
  }
}

async function loadRecommendations() {
  loadingRecommendations.value = true;
  try {
    const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
    recommendations.value = await invoke("lmstudio_recommend_models", { endpoint });
  } catch (e) {
    recommendations.value = null;
    showToast(`Failed to get recommendations: ${e}`, "error");
  } finally {
    loadingRecommendations.value = false;
  }
}

async function saveSettings() {
  if (!localConfig.value) return;
  saving.value = true;
  try {
    await invoke("save_config", { configJson: localConfig.value });
    showToast("Settings saved", "success");
    emit("close");
  } catch (e) {
    showToast(`Save failed: ${e}`, "error");
  } finally {
    saving.value = false;
  }
}

async function clearCredential(provider) {
  try {
    await invoke("clear_provider_credential", { provider });
    localConfig.value.ai[provider].api_key = "";
    showToast(`${provider} credential removed`, "success");
  } catch (error) {
    showToast(`Could not remove credential: ${error}`, "error");
  }
}

function modelLabel(m) {
  let label = m.display_name || m.id;
  const parts = [];
  if (m.size_gb) parts.push(`${m.size_gb} GB`);
  if (m.quant) parts.push(m.quant);
  if (m.architecture) parts.push(m.architecture);
  if (parts.length) label += ` (${parts.join(", ")})`;
  return label;
}
</script>

<template>
  <Transition name="scale">
    <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog" style="width: 620px; max-height: 85vh;" role="dialog" aria-modal="true" aria-labelledby="settings-dialog-title">
        <div class="dialog-header">
          <h2 id="settings-dialog-title" class="dialog-title gradient-text">Settings</h2>
          <button class="close-btn" aria-label="Close settings" @click="emit('close')">&times;</button>
        </div>

        <div v-if="localConfig" class="settings-body">
          <div class="settings-tabs">
            <button
            v-for="tab in ['general', 'account', 'ai', 'lmstudio', 'hardware']"
              :key="tab"
              class="tab-btn"
              :class="{ active: activeTab === tab }"
              @click="activeTab = tab"
            >
              {{ tab === 'lmstudio' ? 'LM Studio' : tab === 'hardware' ? 'Hardware' : tab }}
            </button>
          </div>

          <div class="settings-content">
            <!-- General -->
            <template v-if="activeTab === 'general'">
              <div class="form-group">
                <label class="form-label">Default Backend</label>
                <select v-model="localConfig.general.default_backend" class="form-input">
                  <option value="auto">Auto</option>
                  <option value="native">Native</option>
                  <option value="matchering">Matchering</option>
                  <option value="ai">AI</option>
                  <option value="local_ml">Local ML</option>
                </select>
              </div>
              <button class="btn btn-ghost btn-sm" type="button" @click="exportDiagnostics">Export local diagnostics</button>
              <div class="form-group">
                <label class="form-label">Default Bit Depth</label>
                <select v-model.number="localConfig.general.default_bit_depth" class="form-input">
                  <option :value="16">16</option>
                  <option :value="24">24</option>
                  <option :value="32">32</option>
                </select>
              </div>
              <div class="form-group">
                <label class="form-label">Default Target LUFS</label>
                <input type="number" class="form-input" v-model.number="localConfig.general.target_lufs" step="0.5" />
              </div>
              <div class="form-group">
                <label class="toggle-label">
                  <input type="checkbox" v-model="localConfig.privacy.telemetry_consent" />
                  <span class="toggle-text">Share anonymized crash diagnostics</span>
                </label>
                <p class="form-hint">Opt-in only. Audio, filenames, and local paths are never included.</p>
              </div>
            </template>

            <!-- KeyhanStudio Account -->
            <template v-if="activeTab === 'account'">
              <div class="section-header">
                <span class="section-title">KeyhanStudio Cloud</span>
                <span class="status-badge" :class="cloud.signedIn ? 'status-ok' : 'status-err'">
                  {{ cloud.signedIn ? 'Signed in' : 'Signed out' }}
                </span>
              </div>
              <p class="form-hint">
                Audio never leaves this computer. Cloud sync includes app settings, presets, early-access state, and feedback only.
              </p>
              <div v-if="cloud.signedIn" class="info-box">
                Signed in as <strong>{{ cloud.user?.name || cloud.user?.email }}</strong>
                <span v-if="cloud.earlyAccess" class="model-tag tag-loaded">Early access</span>
              </div>
              <label v-if="cloud.signedIn" class="form-hint" style="display:block; margin-top: 12px;">
                <input :checked="cloud.earlyAccess" type="checkbox" @change="handleEarlyAccess" />
                Join the AudioMaster early-access program
              </label>
              <div v-if="cloud.loginState === 'pending'" class="info-box warn">
                Enter code <strong class="mono">{{ cloud.userCode }}</strong> in the browser to finish signing in.
              </div>
              <div class="input-row" style="margin-top: 12px;">
                <button v-if="!cloud.signedIn" class="btn" :disabled="cloud.loading" @click="handleCloudLogin">
                  {{ cloud.loading ? 'Waiting for authorization...' : 'Sign in with KeyhanStudio' }}
                </button>
                <template v-else>
                  <button class="btn btn-sm" @click="handlePullSync">Download settings</button>
                  <button class="btn btn-sm" @click="handlePushSync">Upload settings</button>
                  <button class="btn btn-ghost btn-sm" @click="cloudLogout">Sign out</button>
                </template>
              </div>

              <template v-if="cloud.signedIn">
                <div class="section-header" style="margin-top: 24px;">
                  <span class="section-title">Feedback</span>
                </div>
                <div class="form-group">
                  <label class="form-label">Category</label>
                  <select v-model="feedbackCategory" class="form-input">
                    <option value="general">General</option>
                    <option value="audio-quality">Audio quality</option>
                    <option value="bug">Bug</option>
                    <option value="feature">Feature request</option>
                  </select>
                </div>
                <div class="form-group">
                  <label class="form-label">Message</label>
                  <textarea v-model="feedbackMessage" class="form-input" rows="4" maxlength="12000"></textarea>
                </div>
                <label class="form-hint">
                  <input v-model="diagnosticsOptIn" type="checkbox" /> Include diagnostic consent flag (logs are not uploaded automatically)
                </label>
                <button class="btn btn-sm" :disabled="!feedbackMessage.trim()" @click="handleFeedback">Send feedback</button>
              </template>
            </template>

            <!-- AI -->
            <template v-if="activeTab === 'ai'">
              <div class="form-group">
                <label class="form-label">Default AI Provider</label>
                <select v-model="localConfig.ai.default_provider" class="form-input">
                  <option value="ollama">Ollama</option>
                  <option value="lmstudio">LM Studio</option>
                  <option value="keyhanstudio">KeyhanStudio</option>
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                </select>
              </div>
              <div class="form-group">
                <label class="form-label">Ollama URL</label>
                <input type="text" class="form-input mono" v-model="localConfig.ai.ollama.endpoint" />
              </div>
              <div class="form-group">
                <label class="form-label">Ollama Model</label>
                <input type="text" class="form-input mono" v-model="localConfig.ai.ollama.model" />
              </div>
              <div class="form-group">
                <label class="form-label">KeyhanStudio URL</label>
                <input type="text" class="form-input mono" v-model="localConfig.ai.keyhanstudio.endpoint" />
              </div>
              <div class="form-group">
                <label class="form-label">KeyhanStudio API Key</label>
                <input type="password" class="form-input mono" v-model="localConfig.ai.keyhanstudio.api_key" placeholder="sk-..." />
                <button class="btn btn-ghost btn-sm" type="button" @click="clearCredential('keyhanstudio')">Remove stored key</button>
              </div>
              <div class="form-group">
                <label class="form-label">OpenAI API Key</label>
                <input type="password" class="form-input mono" v-model="localConfig.ai.openai.api_key" placeholder="sk-..." />
                <button class="btn btn-ghost btn-sm" type="button" @click="clearCredential('openai')">Remove stored key</button>
              </div>
              <div class="form-group">
                <label class="form-label">Anthropic API Key</label>
                <input type="password" class="form-input mono" v-model="localConfig.ai.anthropic.api_key" placeholder="sk-..." />
                <button class="btn btn-ghost btn-sm" type="button" @click="clearCredential('anthropic')">Remove stored key</button>
              </div>
            </template>

            <!-- LM Studio -->
            <template v-if="activeTab === 'lmstudio'">
              <div class="section-header">
                <span class="section-title">LM Studio Connection</span>
                <span
                  v-if="lm.online !== null"
                  class="status-badge"
                  :class="lm.online ? 'status-ok' : 'status-err'"
                >
                  {{ lm.online ? 'Connected' : 'Offline' }}
                </span>
              </div>

              <div class="form-group">
                <label class="form-label">Endpoint URL</label>
                <div class="input-row">
                  <input
                    type="text"
                    class="form-input mono"
                    v-model="localConfig.ai.lmstudio.endpoint"
                    placeholder="http://localhost:1234/v1"
                  />
                  <button
                    class="btn btn-sm"
                    @click="testLmStudioConnection"
                    :disabled="lm.loading"
                  >
                    {{ lm.loading ? 'Testing...' : 'Test' }}
                  </button>
                </div>
              </div>

              <!-- Model List with Load/Unload -->
              <div class="form-group" v-if="lm.models.length > 0">
                <label class="form-label">Available Models ({{ lm.models.length }})</label>
                <div class="model-list">
                  <div v-for="m in lm.models" :key="m.id" class="model-card">
                    <div class="model-info">
                      <span class="model-id">{{ modelLabel(m) }}</span>
                      <div class="model-meta">
                        <span v-if="m.architecture" class="model-tag">{{ m.architecture }}</span>
                        <span v-if="m.loaded" class="model-tag tag-loaded">Loaded</span>
                      </div>
                    </div>
                    <div class="model-actions">
                      <button
                        v-if="!m.loaded"
                        class="btn btn-sm btn-load"
                        :disabled="lm.loadingModel === m.id"
                        @click="handleLoadModel(m.id)"
                      >
                        {{ lm.loadingModel === m.id ? 'Loading...' : 'Load' }}
                      </button>
                      <button
                        v-if="m.loaded"
                        class="btn btn-sm btn-unload"
                        :disabled="lm.unloadingModel === m.id"
                        @click="handleUnloadModel(m.id)"
                      >
                        {{ lm.unloadingModel === m.id ? 'Unloading...' : 'Unload' }}
                      </button>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Default model selection -->
              <div class="form-group" v-if="lm.models.length > 0">
                <label class="form-label">Default Model</label>
                <div class="input-row">
                  <select v-model="localConfig.ai.lmstudio.model" class="form-input">
                    <option value="">-- Select Model --</option>
                    <option v-for="m in lm.models" :key="m.id" :value="m.id">
                      {{ modelLabel(m) }}
                    </option>
                  </select>
                  <button class="btn btn-ghost btn-sm" @click="testLmStudioConnection">
                    Refresh
                  </button>
                </div>
              </div>

              <p class="form-hint" v-if="lm.models.length === 0 && lm.online === null">
                Click "Test" to connect and load models from LM Studio.
              </p>
              <p class="form-hint" v-if="lm.models.length === 0 && lm.online === false">
                LM Studio is not running. Start it and load a model, then click "Test".
              </p>

              <!-- Recommendations -->
              <div class="section-header" style="margin-top: 16px;">
                <span class="section-title">GPU Recommendations</span>
                <button
                  class="btn btn-sm"
                  @click="loadRecommendations"
                  :disabled="loadingRecommendations"
                >
                  {{ loadingRecommendations ? 'Checking...' : 'Check Fit' }}
                </button>
              </div>

              <div v-if="recommendations" class="rec-section">
                <div class="info-box" v-if="recommendations.tier">
                  GPU Tier: <strong>{{ recommendations.tier }}</strong>
                  ({{ recommendations.vram_mb ? Math.round(recommendations.vram_mb / 1024) : '?' }} GB VRAM)
                </div>

                <div v-if="recommendations.recommended.length > 0" class="rec-list">
                  <h4 class="rec-title">Matching models (recommended & available)</h4>
                  <div v-for="rec in recommendations.recommended" :key="rec.model_id" class="rec-card">
                    <div class="rec-name">{{ rec.display_name }}</div>
                    <div class="rec-meta">
                      <span>{{ rec.size_gb }} GB ({{ rec.quant }})</span>
                      <span class="rec-notes">{{ rec.notes }}</span>
                    </div>
                    <code class="rec-id">{{ rec.model_id }}</code>
                    <button
                      class="btn btn-sm btn-load"
                      style="margin-top: 4px;"
                      @click="handleLoadModel(rec.model_id)"
                    >
                      Load This Model
                    </button>
                  </div>
                </div>

                <div v-if="recommendations.recommended.length === 0 && recommendations.available_models.length > 0" class="info-box warn">
                  None of the VRAM-recommended models match your installed models.
                  Consider downloading one from LM Studio.
                </div>

                <div v-if="recommendations.available_models.length === 0" class="info-box warn">
                  No models found in LM Studio. Download models first.
                </div>
              </div>

              <p class="form-hint" v-else>
                Click "Check Fit" to cross-reference your GPU with available LM Studio models.
              </p>
            </template>

            <!-- Hardware -->
            <template v-if="activeTab === 'hardware'">
              <div class="section-header">
                <span class="section-title">GPU & VRAM Detection</span>
                <button
                  class="btn btn-sm"
                  @click="detectGpu"
                  :disabled="vramDetecting"
                >
                  {{ vramDetecting ? 'Detecting...' : 'Detect GPU' }}
                </button>
              </div>

              <div v-if="vramInfo" class="vram-results">
                <div v-for="gpu in vramInfo.gpus" :key="gpu.gpu_name" class="gpu-card">
                  <div class="gpu-name">{{ gpu.gpu_name }}</div>
                  <div class="gpu-detail">
                    <span>VRAM: <strong>{{ gpu.vram_total_mb }} MB</strong></span>
                    <span v-if="gpu.is_apple_silicon" class="badge-silicon">Apple Silicon (Unified)</span>
                    <span v-if="gpu.metal_support">{{ gpu.metal_support }}</span>
                  </div>
                </div>

                <div v-if="vramInfo.tier" class="tier-badge">
                  Recommended tier: <strong>{{ vramInfo.tier }}</strong>
                </div>

                <div v-if="vramInfo.recommendations.length > 0" class="rec-section">
                  <h4 class="rec-title">Recommended Models</h4>
                  <div
                    v-for="rec in vramInfo.recommendations"
                    :key="rec.model_id"
                    class="rec-card"
                  >
                    <div class="rec-name">{{ rec.display_name }}</div>
                    <div class="rec-meta">
                      <span>{{ rec.size_gb }} GB ({{ rec.quant }})</span>
                      <span class="rec-notes">{{ rec.notes }}</span>
                    </div>
                    <code class="rec-id">{{ rec.model_id }}</code>
                  </div>
                </div>
              </div>

              <p class="form-hint" v-else>
                Click "Detect GPU" to check your system's VRAM and get model recommendations.
              </p>
            </template>
          </div>
        </div>

        <div class="dialog-footer">
          <button class="btn btn-ghost" @click="emit('close')">Cancel</button>
          <button class="btn btn-primary" @click="saveSettings" :disabled="saving">
            {{ saving ? 'Saving...' : 'Save Settings' }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.settings-body { display: flex; flex-direction: column; gap: 16px; }

.settings-tabs {
  display: flex;
  gap: 2px;
  background-color: var(--bg-input);
  border-radius: 10px;
  padding: 3px;
  border: 1px solid var(--border);
}

.tab-btn {
  flex: 1;
  padding: 7px 16px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-family: var(--font-sans);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  text-transform: capitalize;
  transition: all 0.2s ease;
}

.tab-btn.active {
  background-color: var(--cyan-subtle);
  color: var(--cyan);
}

.tab-btn:hover:not(.active) { color: var(--text); }

.settings-content {
  max-height: 400px;
  overflow-y: auto;
  padding-right: 4px;
}

.mono { font-family: var(--font-mono); font-size: 11px; }

.input-row { display: flex; gap: 8px; align-items: center; }
.input-row .form-input { flex: 1; }

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.section-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-bright);
}

.status-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 20px;
}

.status-ok {
  background: rgba(34, 197, 94, 0.15);
  color: var(--success);
}

.status-err {
  background: rgba(239, 68, 68, 0.15);
  color: var(--danger);
}

.form-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 4px;
}

.info-box {
  font-size: 11px;
  color: var(--cyan);
  background: var(--cyan-subtle);
  border-radius: 8px;
  padding: 8px 12px;
}

.info-box.warn {
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.1);
}

/* Model list */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 200px;
  overflow-y: auto;
}

.model-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 8px 12px;
}

.model-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }

.model-id {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-bright);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-meta { display: flex; gap: 6px; }

.model-tag {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(148, 163, 184, 0.15);
  color: var(--text-muted);
}

.model-tag.tag-loaded {
  background: rgba(34, 197, 94, 0.15);
  color: var(--success);
  font-weight: 600;
}

.model-actions { flex-shrink: 0; }

.btn-load {
  background: var(--cyan-subtle);
  color: var(--cyan);
}

.btn-unload {
  background: rgba(239, 68, 68, 0.1);
  color: var(--danger);
}

/* VRAM & Recommendations */
.vram-results { display: flex; flex-direction: column; gap: 10px; }

.gpu-card {
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 12px;
}

.gpu-name {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-bright);
  margin-bottom: 4px;
}

.gpu-detail {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
  align-items: center;
}

.badge-silicon {
  background: rgba(168, 85, 247, 0.15);
  color: #a78bfa;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 10px;
  font-weight: 600;
}

.tier-badge {
  font-size: 12px;
  color: var(--text);
  background: var(--bg-input);
  border-radius: 8px;
  padding: 8px 12px;
  border: 1px solid var(--border-light);
}

.rec-section { margin-top: 4px; }
.rec-list { margin-top: 8px; }

.rec-title {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-bright);
  margin-bottom: 8px;
}

.rec-card {
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  padding: 10px 12px;
  margin-bottom: 6px;
}

.rec-name {
  font-size: 12px;
  font-weight: 700;
  color: var(--cyan);
}

.rec-meta {
  font-size: 11px;
  color: var(--text-muted);
  display: flex;
  gap: 12px;
  margin: 2px 0;
}

.rec-notes { font-style: italic; }

.rec-id {
  font-size: 10px;
  color: var(--text-muted);
  background: rgba(0,0,0,0.2);
  padding: 2px 6px;
  border-radius: 4px;
  display: inline-block;
  margin-top: 4px;
}
</style>
