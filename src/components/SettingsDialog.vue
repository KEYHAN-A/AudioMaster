<script setup>
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../composables/useToast.js";

const props = defineProps({
  visible: Boolean,
  config: Object,
});

const emit = defineEmits(["close"]);
const { showToast } = useToast();

const localConfig = ref(null);
const saving = ref(false);
const activeTab = ref("general");

// LM Studio state
const lmstudioTesting = ref(false);
const lmstudioConnectionStatus = ref(null);
const lmstudioModels = ref([]);

// VRAM state
const vramDetecting = ref(false);
const vramInfo = ref(null);

watch(
  () => props.config,
  (val) => {
    if (val) localConfig.value = JSON.parse(JSON.stringify(val));
  },
  { immediate: true, deep: true }
);

async function testLmStudioConnection() {
  lmstudioTesting.value = true;
  try {
    const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
    const result = await invoke("lmstudio_status", { endpoint });
    lmstudioConnectionStatus.value = result.running;
    if (result.running) {
      await refreshLmStudioModels();
      showToast("LM Studio connected", "success");
    } else {
      showToast("LM Studio is not running", "error");
    }
  } catch (e) {
    lmstudioConnectionStatus.value = false;
    showToast(`Connection failed: ${e}`, "error");
  } finally {
    lmstudioTesting.value = false;
  }
}

async function refreshLmStudioModels() {
  try {
    const endpoint = localConfig.value?.ai?.lmstudio?.endpoint || null;
    lmstudioModels.value = await invoke("lmstudio_models", { endpoint });
  } catch (e) {
    lmstudioModels.value = [];
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
</script>

<template>
  <Transition name="scale">
    <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog" style="width: 620px; max-height: 85vh;">
        <div class="dialog-header">
          <h2 class="dialog-title gradient-text">Settings</h2>
          <button class="close-btn" @click="emit('close')">&times;</button>
        </div>

        <div v-if="localConfig" class="settings-body">
          <div class="settings-tabs">
            <button
              v-for="tab in ['general', 'ai', 'lmstudio', 'hardware']"
              :key="tab"
              class="tab-btn"
              :class="{ active: activeTab === tab }"
              @click="activeTab = tab"
            >
              {{ tab === 'lmstudio' ? 'LM Studio' : tab }}
            </button>
          </div>

          <div class="settings-content">
            <!-- General -->
            <template v-if="activeTab === 'general'">
              <div class="form-group">
                <label class="form-label">Default Backend</label>
                <select v-model="localConfig.general.default_backend" class="form-input">
                  <option value="auto">Auto</option>
                  <option value="matchering">Matchering</option>
                  <option value="ai">AI</option>
                  <option value="local_ml">Local ML</option>
                </select>
              </div>
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
              </div>
              <div class="form-group">
                <label class="form-label">OpenAI API Key</label>
                <input type="password" class="form-input mono" v-model="localConfig.ai.openai.api_key" placeholder="sk-..." />
              </div>
              <div class="form-group">
                <label class="form-label">Anthropic API Key</label>
                <input type="password" class="form-input mono" v-model="localConfig.ai.anthropic.api_key" placeholder="sk-..." />
              </div>
            </template>

            <!-- LM Studio -->
            <template v-if="activeTab === 'lmstudio'">
              <div class="section-header">
                <span class="section-title">LM Studio Connection</span>
                <span
                  v-if="lmstudioConnectionStatus !== null"
                  class="status-badge"
                  :class="lmstudioConnectionStatus ? 'status-ok' : 'status-err'"
                >
                  {{ lmstudioConnectionStatus ? 'Connected' : 'Offline' }}
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
                    :disabled="lmstudioTesting"
                  >
                    {{ lmstudioTesting ? 'Testing...' : 'Test' }}
                  </button>
                </div>
              </div>

              <div class="form-group">
                <label class="form-label">Model</label>
                <div class="input-row">
                  <select v-model="localConfig.ai.lmstudio.model" class="form-input">
                    <option value="">-- Select Model --</option>
                    <option v-for="m in lmstudioModels" :key="m.id" :value="m.id">
                      {{ m.id }}
                    </option>
                  </select>
                  <button class="btn btn-ghost btn-sm" @click="refreshLmStudioModels">
                    Refresh
                  </button>
                </div>
                <p class="form-hint" v-if="lmstudioModels.length === 0 && lmstudioConnectionStatus === null">
                  Click "Test" to connect and load models from LM Studio.
                </p>
                <p class="form-hint" v-if="lmstudioModels.length === 0 && lmstudioConnectionStatus === false">
                  LM Studio is not running. Start it and load a model, then click "Test".
                </p>
              </div>

              <div class="info-box" v-if="lmstudioModels.length > 0">
                <strong>{{ lmstudioModels.length }}</strong> model(s) available
              </div>
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
