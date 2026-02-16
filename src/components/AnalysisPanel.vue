<script setup>
const props = defineProps({
  analysis: Object,
  postAnalysis: Object,
});

function lufsClass(value) {
  if (value > -10) return "hot";
  if (value > -16) return "warm";
  return "cool";
}

function dbDisplay(value) {
  return value !== null && value !== undefined ? value.toFixed(1) : "--";
}
</script>

<template>
  <div class="analysis-panel">
    <div class="metrics-strip stagger-enter">
      <div class="metric-card">
        <span class="metric-label">LUFS Int.</span>
        <span class="metric-value" :class="lufsClass(analysis.lufs_integrated)">
          {{ dbDisplay(analysis.lufs_integrated) }}
        </span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.lufs_integrated) }}
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">LUFS ST Max</span>
        <span class="metric-value">{{ dbDisplay(analysis.lufs_short_term_max) }}</span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.lufs_short_term_max) }}
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">RMS</span>
        <span class="metric-value">{{ dbDisplay(analysis.rms_db) }} dB</span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.rms_db) }} dB
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Peak</span>
        <span class="metric-value" :class="{ hot: analysis.peak_db > -1 }">
          {{ dbDisplay(analysis.peak_db) }} dB
        </span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.peak_db) }} dB
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">True Peak</span>
        <span class="metric-value" :class="{ hot: analysis.true_peak_db > -0.3 }">
          {{ dbDisplay(analysis.true_peak_db) }} dBTP
        </span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.true_peak_db) }} dBTP
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">DR</span>
        <span class="metric-value cool">{{ dbDisplay(analysis.dynamic_range_db) }} dB</span>
        <span v-if="postAnalysis" class="metric-after">
          {{ dbDisplay(postAnalysis.dynamic_range_db) }} dB
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Width</span>
        <span class="metric-value">{{ (analysis.stereo_width * 100).toFixed(0) }}%</span>
        <span v-if="postAnalysis" class="metric-after">
          {{ (postAnalysis.stereo_width * 100).toFixed(0) }}%
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.analysis-panel {
  padding: 8px 12px;
  border-top: 1px solid var(--border-subtle);
  background: rgba(17, 24, 39, 0.5);
  overflow-x: auto;
  flex-shrink: 0;
}

.metrics-strip {
  display: flex;
  gap: 6px;
  min-width: min-content;
}

.metric-card {
  flex: 1;
  min-width: 90px;
  padding: 8px 10px;
  border-radius: 10px;
  background-color: var(--bg-input);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 3px;
  transition: border-color 0.2s ease;
}

.metric-card:hover { border-color: var(--border-light); }

.metric-label {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
}

.metric-value {
  font-size: 14px;
  font-weight: 700;
  font-family: var(--font-mono);
  color: var(--cyan);
}

.metric-value.hot { color: var(--danger); }
.metric-value.warm { color: var(--warning); }
.metric-value.cool { color: var(--success); }

.metric-after {
  font-size: 10px;
  font-weight: 600;
  font-family: var(--font-mono);
  color: var(--purple);
}

.metric-after::before {
  content: "After: ";
  font-weight: 500;
  color: var(--text-muted);
}
</style>
