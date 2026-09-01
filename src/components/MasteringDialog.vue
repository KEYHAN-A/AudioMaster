<script setup>
import { watch } from "vue";
import { useLmStudio } from "../composables/useLmStudio.js";

const props = defineProps({
  visible: Boolean,
  presets: Array,
  backends: Array,
  state: Object,
});

const emit = defineEmits(["close", "master"]);

const { state: lm, refresh } = useLmStudio();

watch(
  () => props.state?.selectedProvider,
  async (provider) => {
    if (provider === "lmstudio") {
      const endpoint = props.state?.config?.ai?.lmstudio?.endpoint || null;
      await refresh(endpoint);
    }
  }
);

function modelLabel(m) {
  let label = m.display_name || m.id;
  const parts = [];
  if (m.size_gb) parts.push(`${m.size_gb} GB`);
  if (m.quant) parts.push(m.quant);
  if (parts.length) label += ` (${parts.join(", ")})`;
  if (m.loaded) label += " [loaded]";
  return label;
}
</script>

<template>
  <Transition name="scale">
    <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog" style="width: 560px;" role="dialog" aria-modal="true" aria-labelledby="master-dialog-title">
        <div class="dialog-header">
          <h2 id="master-dialog-title" class="dialog-title gradient-text">Master {{ state.tracks?.length || 0 }} Track(s)</h2>
          <button class="close-btn" aria-label="Close mastering settings" @click="emit('close')">&times;</button>
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

          <div v-if="state.tracks?.length > 1" class="form-group">
            <label class="toggle-label">
              <input type="checkbox" v-model="state.albumMode" />
              <span class="toggle-text">Album continuity mode</span>
            </label>
            <p class="form-hint">Preserves intentional song-to-song loudness differences while keeping the release cohesive.</p>
            <label v-if="state.albumMode" class="form-label">Maximum relative offset: {{ state.albumMaxRelativeOffsetLu }} LU</label>
            <input
              v-if="state.albumMode"
              type="range"
              v-model.number="state.albumMaxRelativeOffsetLu"
              min="0"
              max="3"
              step="0.25"
            />
            <div v-if="state.albumMode" class="album-adjustments">
              <label v-for="track in state.tracks" :key="track.id" class="album-adjustment">
                <span>{{ track.name }}</span>
                <input v-model.number="track.albumOffsetLu" class="form-input" type="number" min="-3" max="3" step="0.25" aria-label="Per-track loudness adjustment in LU" />
                <span>LU</span>
              </label>
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

          <!-- AI Provider -->
          <Transition name="slide-up">
            <div v-if="state.selectedBackend === 'ai'" class="form-group">
              <label class="form-label">AI Provider</label>
              <select v-model="state.selectedProvider" class="form-input">
                <option value="ollama">Ollama (Local)</option>
                <option value="lmstudio">LM Studio (Local)</option>
                <option value="keyhanstudio">KeyhanStudio API</option>
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
              </select>
            </div>
          </Transition>

          <!-- LM Studio Model Selection -->
          <Transition name="slide-up">
            <div v-if="state.selectedBackend === 'ai' && state.selectedProvider === 'lmstudio'" class="form-group">
              <label class="form-label">LM Studio Model</label>
              <div class="lmstudio-row">
                <select v-model="state.selectedLmStudioModel" class="form-input">
                  <option value="">-- Select Model --</option>
                  <option v-for="m in lm.models" :key="m.id" :value="m.id">
                    {{ modelLabel(m) }}
                  </option>
                </select>
                <span
                  class="lmstudio-status"
                  :class="lm.online ? 'status-ok' : 'status-err'"
                >
                  {{ lm.online ? 'Online' : 'Offline' }}
                </span>
                <button class="btn btn-ghost btn-sm" @click="refresh(state?.config?.ai?.lmstudio?.endpoint || null)">
                  Refresh
                </button>
              </div>
              <p v-if="lm.online === false" class="form-hint">
                LM Studio is not running. Start it and load a model first.
              </p>
            </div>
          </Transition>

          <!-- Settings row -->
          <div class="form-row">
            <div class="form-group" style="flex: 1;">
              <label class="form-label">Target LUFS</label>
              <input type="number" class="form-input" v-model.number="state.targetLufs" min="-30" max="-5" step="0.5" />
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
                <option value="aiff">AIFF</option>
                <option value="flac">FLAC</option>
                <option value="mp3">MP3</option>
                <option value="aac">AAC (distribution)</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label class="toggle-label">
              <input type="checkbox" v-model="state.noLimiter" />
              <span class="toggle-text">Disable limiter</span>
            </label>
          </div>
        </div>

        <div class="dialog-footer">
          <button class="btn btn-ghost" @click="emit('close')">Cancel</button>
          <button class="btn btn-primary" @click="emit('master')">
            Start Mastering
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.dialog-body { display: flex; flex-direction: column; gap: 4px; }

.preset-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }

.preset-card {
  padding: 10px; border-radius: 10px; border: 1px solid var(--border-light);
  background: var(--bg-input); cursor: pointer; text-align: center;
  display: flex; flex-direction: column; gap: 4px; transition: all 0.2s ease;
}
.preset-card:hover { border-color: var(--cyan); }
.preset-card.active { border-color: var(--cyan); background-color: var(--cyan-subtle); }
.preset-name { font-size: 12px; font-weight: 700; color: var(--text-bright); text-transform: capitalize; }
.preset-lufs { font-size: 13px; font-weight: 700; font-family: var(--font-mono); color: var(--cyan); }
.preset-desc { font-size: 10px; color: var(--text-muted); }

.backend-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }

.backend-card {
  padding: 10px; border-radius: 10px; border: 1px solid var(--border-light);
  background: var(--bg-input); cursor: pointer; text-align: center;
  display: flex; flex-direction: column; gap: 4px; transition: all 0.2s ease;
}
.backend-card:hover:not(.unavailable) { border-color: var(--cyan); }
.backend-card.active { border-color: var(--cyan); background-color: var(--cyan-subtle); }
.backend-card.unavailable { opacity: 0.5; cursor: not-allowed; }
.backend-name { font-size: 12px; font-weight: 700; color: var(--text-bright); }
.backend-status { font-size: 10px; font-weight: 600; color: var(--danger); }
.backend-status.ok { color: var(--success); }
.backend-desc { font-size: 10px; color: var(--text-muted); }

.form-row { display: flex; gap: 12px; }

.toggle-label { display: flex; align-items: center; gap: 8px; cursor: pointer; }
.toggle-label input { accent-color: var(--cyan); }
.toggle-text { font-size: 12px; color: var(--text); }

.lmstudio-row { display: flex; gap: 8px; align-items: center; }
.lmstudio-row .form-input { flex: 1; }

.lmstudio-status {
  font-size: 10px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 20px;
  white-space: nowrap;
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
.album-adjustments { display: grid; gap: 6px; margin-top: 8px; max-height: 120px; overflow: auto; }
.album-adjustment { display: grid; grid-template-columns: minmax(0, 1fr) 80px 24px; gap: 8px; align-items: center; font-size: 11px; }
</style>
