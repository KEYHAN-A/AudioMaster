<script setup>
import { ref, watch } from "vue";

const props = defineProps({
  visible: Boolean,
  presets: Array,
  backends: Array,
  state: Object,
});

const emit = defineEmits(["close", "master"]);

const outputPath = ref("");

watch(() => props.visible, (val) => {
  if (val && props.state?.inputFile) {
    const base = props.state.inputFile.replace(/\.[^.]+$/, "");
    outputPath.value = `${base}_mastered.wav`;
  }
});

function handleMaster() {
  emit("master", outputPath.value || null);
}
</script>

<template>
  <Transition name="scale">
    <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog" style="width: 560px;">
        <div class="dialog-header">
          <h2 class="dialog-title gradient-text">Mastering Configuration</h2>
          <button class="close-btn" @click="emit('close')">&times;</button>
        </div>

        <div class="dialog-body">
          <!-- Preset -->
          <div class="form-group">
            <label class="form-label">Preset</label>
            <div class="preset-grid">
              <button
                v-for="preset in presets"
                :key="preset.name"
                class="preset-card"
                :class="{ active: state.selectedPreset === preset.name }"
                @click="state.selectedPreset = preset.name; state.targetLufs = preset.target_lufs"
              >
                <span class="preset-name">{{ preset.name }}</span>
                <span class="preset-lufs">{{ preset.target_lufs }} LUFS</span>
                <span class="preset-desc">{{ preset.description }}</span>
              </button>
            </div>
          </div>

          <!-- Backend -->
          <div class="form-group">
            <label class="form-label">Backend</label>
            <div class="backend-grid">
              <button
                class="backend-card"
                :class="{ active: state.selectedBackend === 'auto' }"
                @click="state.selectedBackend = 'auto'"
              >
                <span class="backend-name">Auto</span>
                <span class="backend-desc">Best available</span>
              </button>
              <button
                v-for="b in backends"
                :key="b.name"
                class="backend-card"
                :class="{
                  active: state.selectedBackend === b.name.toLowerCase(),
                  unavailable: !b.available,
                }"
                @click="b.available ? (state.selectedBackend = b.name.toLowerCase()) : null"
              >
                <span class="backend-name">{{ b.name }}</span>
                <span class="backend-status" :class="{ ok: b.available }">
                  {{ b.available ? 'Ready' : 'N/A' }}
                </span>
              </button>
            </div>
          </div>

          <!-- AI Provider (if AI backend) -->
          <Transition name="slide-up">
            <div v-if="state.selectedBackend === 'ai'" class="form-group">
              <label class="form-label">AI Provider</label>
              <select v-model="state.selectedProvider" class="form-input">
                <option value="ollama">Ollama (Local)</option>
                <option value="keyhanstudio">KeyhanStudio API</option>
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
              </select>
            </div>
          </Transition>

          <!-- Target LUFS & bit depth -->
          <div class="form-row">
            <div class="form-group" style="flex: 1;">
              <label class="form-label">Target LUFS</label>
              <input
                type="number"
                class="form-input"
                v-model.number="state.targetLufs"
                min="-30"
                max="-5"
                step="0.5"
              />
            </div>
            <div class="form-group" style="flex: 1;">
              <label class="form-label">Bit Depth</label>
              <select v-model.number="state.bitDepth" class="form-input">
                <option :value="16">16-bit</option>
                <option :value="24">24-bit</option>
                <option :value="32">32-bit float</option>
              </select>
            </div>
            <div class="form-group" style="flex: 1;">
              <label class="form-label">Format</label>
              <select v-model="state.outputFormat" class="form-input">
                <option value="wav">WAV</option>
                <option value="flac">FLAC</option>
                <option value="mp3">MP3</option>
              </select>
            </div>
          </div>

          <!-- Options -->
          <div class="form-group">
            <label class="toggle-label">
              <input type="checkbox" v-model="state.noLimiter" />
              <span class="toggle-text">Disable limiter</span>
            </label>
          </div>

          <!-- Output path -->
          <div class="form-group">
            <label class="form-label">Output Path</label>
            <input
              type="text"
              class="form-input mono"
              v-model="outputPath"
              placeholder="Auto-generated"
            />
          </div>
        </div>

        <div class="dialog-footer">
          <button class="btn btn-ghost" @click="emit('close')">Cancel</button>
          <button class="btn btn-primary" @click="handleMaster">
            Start Mastering
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.dialog-body { display: flex; flex-direction: column; gap: 4px; }

.preset-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.preset-card {
  padding: 10px;
  border-radius: 10px;
  border: 1px solid var(--border-light);
  background: var(--bg-input);
  cursor: pointer;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 4px;
  transition: all 0.2s ease;
}

.preset-card:hover { border-color: var(--cyan); }
.preset-card.active {
  border-color: var(--cyan);
  background-color: var(--cyan-subtle);
}

.preset-name {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-bright);
  text-transform: capitalize;
}

.preset-lufs {
  font-size: 13px;
  font-weight: 700;
  font-family: var(--font-mono);
  color: var(--cyan);
}

.preset-desc {
  font-size: 10px;
  color: var(--text-muted);
}

.backend-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.backend-card {
  padding: 10px;
  border-radius: 10px;
  border: 1px solid var(--border-light);
  background: var(--bg-input);
  cursor: pointer;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 4px;
  transition: all 0.2s ease;
}

.backend-card:hover:not(.unavailable) { border-color: var(--cyan); }
.backend-card.active {
  border-color: var(--cyan);
  background-color: var(--cyan-subtle);
}
.backend-card.unavailable {
  opacity: 0.5;
  cursor: not-allowed;
}

.backend-name {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-bright);
}

.backend-status {
  font-size: 10px;
  font-weight: 600;
  color: var(--danger);
}

.backend-status.ok { color: var(--success); }
.backend-desc { font-size: 10px; color: var(--text-muted); }

.form-row {
  display: flex;
  gap: 12px;
}

.toggle-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.toggle-label input {
  accent-color: var(--cyan);
}

.toggle-text {
  font-size: 12px;
  color: var(--text);
}

.mono { font-family: var(--font-mono); font-size: 11px; }
</style>
