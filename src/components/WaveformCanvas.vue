<script setup>
import { ref, watch, onMounted, onUnmounted, computed } from "vue";

const props = defineProps({
  analysis: Object,
  masterResult: Object,
});

const canvas = ref(null);
const container = ref(null);
const showMode = ref("frequency");

const modes = [
  { id: "frequency", label: "Frequency" },
  { id: "levels", label: "Levels" },
  { id: "comparison", label: "Before / After" },
];

let animId = null;
let resizeObs = null;

const hasComparison = computed(
  () => props.masterResult?.pre_analysis && props.masterResult?.post_analysis
);

onMounted(() => {
  resizeObs = new ResizeObserver(() => drawVisualization());
  if (container.value) resizeObs.observe(container.value);
  drawVisualization();
});

onUnmounted(() => {
  if (animId) cancelAnimationFrame(animId);
  if (resizeObs) resizeObs.disconnect();
});

watch([() => props.analysis, () => props.masterResult, showMode], () => {
  drawVisualization();
});

function drawVisualization() {
  if (animId) cancelAnimationFrame(animId);
  const cvs = canvas.value;
  if (!cvs) return;
  const ctx = cvs.getContext("2d");
  const rect = container.value?.getBoundingClientRect();
  if (!rect) return;

  cvs.width = rect.width * window.devicePixelRatio;
  cvs.height = rect.height * window.devicePixelRatio;
  cvs.style.width = `${rect.width}px`;
  cvs.style.height = `${rect.height}px`;
  ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
  const w = rect.width;
  const h = rect.height;

  ctx.clearRect(0, 0, w, h);

  if (!props.analysis) {
    drawEmptyState(ctx, w, h);
    return;
  }

  if (showMode.value === "frequency") {
    drawFrequencyBars(ctx, w, h, props.analysis);
  } else if (showMode.value === "levels") {
    drawLevels(ctx, w, h, props.analysis);
  } else if (showMode.value === "comparison" && hasComparison.value) {
    drawComparison(ctx, w, h);
  } else {
    drawFrequencyBars(ctx, w, h, props.analysis);
  }
}

function drawEmptyState(ctx, w, h) {
  ctx.save();
  ctx.strokeStyle = "rgba(56, 189, 248, 0.08)";
  ctx.lineWidth = 1;
  const spacing = 40;
  for (let y = 0; y < h; y += spacing) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();
  }

  ctx.fillStyle = "rgba(139, 149, 184, 0.3)";
  ctx.font = "14px Inter, sans-serif";
  ctx.textAlign = "center";
  ctx.fillText("Import an audio file to begin", w / 2, h / 2 - 10);
  ctx.font = "12px Inter, sans-serif";
  ctx.fillStyle = "rgba(139, 149, 184, 0.2)";
  ctx.fillText("Drag & drop or press Cmd+O", w / 2, h / 2 + 15);
  ctx.restore();
}

function drawFrequencyBars(ctx, w, h, analysis) {
  const bands = analysis.frequency_bands;
  const labels = ["Sub", "Bass", "Low-mid", "Mid", "Hi-mid", "Presence", "Brilliance"];
  const values = [
    bands.sub_bass, bands.bass, bands.low_mid, bands.mid,
    bands.upper_mid, bands.presence, bands.brilliance,
  ];

  const maxVal = Math.max(...values, 0.01);
  const barCount = values.length;
  const gap = 12;
  const barWidth = Math.min((w - 80 - (barCount - 1) * gap) / barCount, 100);
  const startX = (w - (barWidth * barCount + gap * (barCount - 1))) / 2;
  const maxHeight = h - 80;
  const baseY = h - 50;

  const colors = [
    "#ef4444", "#f59e0b", "#22c55e", "#38bdf8",
    "#818cf8", "#a78bfa", "#ec4899",
  ];

  values.forEach((val, i) => {
    const x = startX + i * (barWidth + gap);
    const barH = (val / maxVal) * maxHeight * 0.85;

    const grad = ctx.createLinearGradient(x, baseY, x, baseY - barH);
    grad.addColorStop(0, colors[i] + "ee");
    grad.addColorStop(1, colors[i] + "55");
    ctx.fillStyle = grad;
    ctx.beginPath();
    roundRect(ctx, x, baseY - barH, barWidth, barH, 6);
    ctx.fill();

    ctx.shadowColor = colors[i];
    ctx.shadowBlur = 12;
    ctx.fillStyle = colors[i] + "30";
    ctx.fillRect(x, baseY - barH, barWidth, 4);
    ctx.shadowBlur = 0;

    ctx.fillStyle = "rgba(139, 149, 184, 0.6)";
    ctx.font = "10px Inter, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(labels[i], x + barWidth / 2, baseY + 16);

    ctx.fillStyle = colors[i];
    ctx.font = "600 10px 'JetBrains Mono', monospace";
    ctx.fillText(val.toFixed(2), x + barWidth / 2, baseY - barH - 8);
  });
}

function drawLevels(ctx, w, h, analysis) {
  const metrics = [
    { label: "LUFS (Int)", value: analysis.lufs_integrated, min: -60, max: 0, color: "#38bdf8" },
    { label: "LUFS (ST)", value: analysis.lufs_short_term_max, min: -60, max: 0, color: "#7dd3fc" },
    { label: "RMS", value: analysis.rms_db, min: -60, max: 0, color: "#a78bfa" },
    { label: "Peak", value: analysis.peak_db, min: -60, max: 0, color: "#f59e0b" },
    { label: "True Peak", value: analysis.true_peak_db, min: -60, max: 0, color: "#ef4444" },
    { label: "DR", value: analysis.dynamic_range_db, min: 0, max: 30, color: "#22c55e" },
  ];

  const barH = 22;
  const gap = 16;
  const startY = (h - metrics.length * (barH + gap)) / 2;
  const labelW = 80;
  const barMaxW = w - labelW - 100;

  metrics.forEach((m, i) => {
    const y = startY + i * (barH + gap);
    const pct = Math.max(0, Math.min(1, (m.value - m.min) / (m.max - m.min)));
    const barW = pct * barMaxW;

    ctx.fillStyle = "rgba(139, 149, 184, 0.5)";
    ctx.font = "600 11px Inter, sans-serif";
    ctx.textAlign = "right";
    ctx.fillText(m.label, labelW, y + barH / 2 + 4);

    ctx.fillStyle = "rgba(26, 34, 54, 0.6)";
    ctx.beginPath();
    roundRect(ctx, labelW + 12, y, barMaxW, barH, 6);
    ctx.fill();

    const grad = ctx.createLinearGradient(labelW + 12, 0, labelW + 12 + barW, 0);
    grad.addColorStop(0, m.color + "aa");
    grad.addColorStop(1, m.color);
    ctx.fillStyle = grad;
    ctx.beginPath();
    roundRect(ctx, labelW + 12, y, barW, barH, 6);
    ctx.fill();

    ctx.fillStyle = m.color;
    ctx.font = "700 11px 'JetBrains Mono', monospace";
    ctx.textAlign = "left";
    ctx.fillText(`${m.value.toFixed(1)} ${m.label.includes("DR") ? "dB" : ""}`, labelW + 16 + barW + 8, y + barH / 2 + 4);
  });
}

function drawComparison(ctx, w, h) {
  const pre = props.masterResult.pre_analysis;
  const post = props.masterResult.post_analysis;
  if (!pre || !post) return;

  const metrics = [
    { label: "LUFS", before: pre.lufs_integrated, after: post.lufs_integrated, unit: "" },
    { label: "Peak dB", before: pre.peak_db, after: post.peak_db, unit: "" },
    { label: "RMS dB", before: pre.rms_db, after: post.rms_db, unit: "" },
    { label: "DR dB", before: pre.dynamic_range_db, after: post.dynamic_range_db, unit: "" },
    { label: "Width", before: pre.stereo_width * 100, after: post.stereo_width * 100, unit: "%" },
  ];

  const halfW = w / 2;
  const rowH = 50;
  const startY = (h - metrics.length * rowH) / 2;

  ctx.fillStyle = "rgba(56, 189, 248, 0.1)";
  ctx.fillRect(0, 0, halfW, h);

  ctx.fillStyle = "rgba(167, 139, 250, 0.05)";
  ctx.fillRect(halfW, 0, halfW, h);

  ctx.fillStyle = "rgba(139, 149, 184, 0.4)";
  ctx.font = "600 12px Inter, sans-serif";
  ctx.textAlign = "center";
  ctx.fillText("BEFORE", halfW / 2, startY - 20);
  ctx.fillText("AFTER", halfW + halfW / 2, startY - 20);

  metrics.forEach((m, i) => {
    const y = startY + i * rowH + 30;

    ctx.fillStyle = "rgba(139, 149, 184, 0.5)";
    ctx.font = "600 11px Inter, sans-serif";
    ctx.textAlign = "left";
    ctx.fillText(m.label, 16, y);

    ctx.fillStyle = "#38bdf8";
    ctx.font = "700 14px 'JetBrains Mono', monospace";
    ctx.textAlign = "center";
    ctx.fillText(`${m.before.toFixed(1)}${m.unit}`, halfW / 2, y);

    const improved = m.label === "DR dB" || m.label === "Width"
      ? m.after > m.before
      : m.after > m.before;
    ctx.fillStyle = "#a78bfa";
    ctx.fillText(`${m.after.toFixed(1)}${m.unit}`, halfW + halfW / 2, y);

    const diff = m.after - m.before;
    if (Math.abs(diff) > 0.05) {
      ctx.fillStyle = diff > 0 ? "#34d399" : "#f87171";
      ctx.font = "600 10px 'JetBrains Mono', monospace";
      ctx.fillText(`${diff > 0 ? "+" : ""}${diff.toFixed(1)}`, w - 40, y);
    }
  });
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
}
</script>

<template>
  <div class="waveform-container" ref="container">
    <div v-if="analysis" class="mode-switcher">
      <button
        v-for="mode in modes"
        :key="mode.id"
        class="mode-btn"
        :class="{ active: showMode === mode.id, disabled: mode.id === 'comparison' && !hasComparison }"
        :disabled="mode.id === 'comparison' && !hasComparison"
        @click="showMode = mode.id"
      >
        {{ mode.label }}
      </button>
    </div>
    <canvas ref="canvas" class="viz-canvas"></canvas>
  </div>
</template>

<style scoped>
.waveform-container {
  flex: 1;
  position: relative;
  min-height: 200px;
  background: linear-gradient(180deg, var(--navy-deep), rgba(10, 14, 26, 0.8));
}

.viz-canvas {
  width: 100%;
  height: 100%;
  display: block;
}

.mode-switcher {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  gap: 2px;
  background-color: var(--bg-input);
  border-radius: 10px;
  padding: 3px;
  z-index: 10;
  border: 1px solid var(--border);
}

.mode-btn {
  padding: 5px 12px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-family: var(--font-sans);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.mode-btn.active {
  background-color: var(--cyan-subtle);
  color: var(--cyan);
}

.mode-btn:hover:not(.active):not(.disabled) {
  color: var(--text);
}

.mode-btn.disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
</style>
