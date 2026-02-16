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

watch(() => props.config, (val) => {
  if (val) localConfig.value = JSON.parse(JSON.stringify(val));
}, { immediate: true, deep: true });

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
      <div class="dialog" style="width: 580px; max-height: 80vh;">
        <div class="dialog-header">
          <h2 class="dialog-title gradient-text">Settings</h2>
          <button class="close-btn" @click="emit('close')">&times;</button>
        </div>

        <div v-if="localConfig" class="settings-body">
          <div class="settings-tabs">
            <button
              v-for="tab in ['general', 'ai', 'paths']"
              :key="tab"
              class="tab-btn"
              :class="{ active: activeTab === tab }"
              @click="activeTab = tab"
            >
              {{ tab }}
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

            <!-- Paths -->
            <template v-if="activeTab === 'paths'">
              <div class="form-group">
                <label class="form-label">Matchering Python Path</label>
                <input type="text" class="form-input mono" v-model="localConfig.backends.matchering.python_path" />
              </div>
              <div class="form-group">
                <label class="form-label">Local ML Python Path</label>
                <input type="text" class="form-input mono" v-model="localConfig.backends.local_ml.python_path" />
              </div>
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
</style>
