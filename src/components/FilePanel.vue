<script setup>
import { computed } from "vue";

const props = defineProps({
  inputFile: String,
  referenceFile: String,
  analysis: Object,
  masterResult: Object,
});

const emit = defineEmits(["openFile", "openReference"]);

const fileName = computed(() => {
  if (!props.inputFile) return null;
  return props.inputFile.split("/").pop().split("\\").pop();
});

const referenceName = computed(() => {
  if (!props.referenceFile) return null;
  return props.referenceFile.split("/").pop().split("\\").pop();
});

function formatDuration(seconds) {
  if (!seconds) return "--:--";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatSize(bytes) {
  if (!bytes) return "--";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}
</script>

<template>
  <div class="file-panel">
    <div class="panel-section">
      <h3 class="section-title">Input Track</h3>
      <div v-if="inputFile" class="file-card glass-card">
        <div class="file-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M9 9l10.5-3m0 6.553v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 11-.99-3.467l2.31-.66a2.25 2.25 0 001.632-2.163zm0 0V2.34A1.154 1.154 0 0017.882 1.2l-5.764 1.648A1.154 1.154 0 0011 3.996V14.5" />
          </svg>
        </div>
        <div class="file-info">
          <span class="file-name">{{ fileName }}</span>
          <div v-if="analysis" class="file-meta">
            <span>{{ analysis.metadata.sample_rate / 1000 }}kHz</span>
            <span>{{ analysis.metadata.channels }}ch</span>
            <span>{{ formatDuration(analysis.metadata.duration_secs) }}</span>
          </div>
        </div>
      </div>
      <button v-else class="btn btn-ghost import-btn" @click="emit('openFile')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        Import audio file
      </button>
    </div>

    <div class="panel-section">
      <h3 class="section-title">Reference Track</h3>
      <div v-if="referenceFile" class="file-card glass-card">
        <div class="file-icon reference">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M19.114 5.636a9 9 0 010 12.728M16.463 8.288a5.25 5.25 0 010 7.424M6.75 8.25l4.72-4.72a.75.75 0 011.28.53v15.88a.75.75 0 01-1.28.53l-4.72-4.72H4.51c-.88 0-1.704-.507-1.938-1.354A9.009 9.009 0 012.25 12c0-.83.112-1.633.322-2.396C2.806 8.756 3.63 8.25 4.51 8.25H6.75z" />
          </svg>
        </div>
        <div class="file-info">
          <span class="file-name">{{ referenceName }}</span>
          <span class="file-meta-text">Used for style matching</span>
        </div>
      </div>
      <button v-else class="btn btn-ghost import-btn" @click="emit('openReference')" :disabled="!inputFile">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        Add reference (optional)
      </button>
    </div>

    <Transition name="slide-up">
      <div v-if="masterResult" class="panel-section">
        <h3 class="section-title">Output</h3>
        <div class="file-card glass-card output-card">
          <div class="file-icon output">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="file-info">
            <span class="file-name">{{ masterResult.output_path.split('/').pop() }}</span>
            <span class="file-meta-text">via {{ masterResult.backend_used }}</span>
          </div>
        </div>
      </div>
    </Transition>

    <div v-if="analysis" class="panel-section quick-stats">
      <h3 class="section-title">Quick Stats</h3>
      <div class="stat-grid stagger-enter">
        <div class="stat-item">
          <span class="stat-label">LUFS</span>
          <span class="stat-value">{{ analysis.lufs_integrated.toFixed(1) }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">Peak</span>
          <span class="stat-value">{{ analysis.peak_db.toFixed(1) }} dB</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">DR</span>
          <span class="stat-value">{{ analysis.dynamic_range_db.toFixed(1) }} dB</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">Width</span>
          <span class="stat-value">{{ (analysis.stereo_width * 100).toFixed(0) }}%</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-panel { padding: 16px; display: flex; flex-direction: column; gap: 20px; }

.panel-section {}

.section-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.file-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 12px;
}

.file-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background-color: var(--cyan-subtle);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--cyan);
  flex-shrink: 0;
}

.file-icon.reference {
  background-color: var(--purple-subtle);
  color: var(--purple);
}

.file-icon.output {
  background-color: rgba(52, 211, 153, 0.15);
  color: var(--success);
}

.file-info { display: flex; flex-direction: column; gap: 2px; overflow: hidden; }

.file-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-bright);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: var(--text-dim);
  font-family: var(--font-mono);
}

.file-meta-text {
  font-size: 11px;
  color: var(--text-dim);
}

.import-btn {
  width: 100%;
  padding: 20px;
  gap: 10px;
  border-style: dashed;
  color: var(--text-muted);
}

.output-card {
  border-color: rgba(52, 211, 153, 0.25);
}

.stat-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.stat-item {
  padding: 10px;
  border-radius: 10px;
  background-color: var(--bg-input);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
}

.stat-value {
  font-size: 15px;
  font-weight: 700;
  color: var(--cyan);
  font-family: var(--font-mono);
}
</style>
