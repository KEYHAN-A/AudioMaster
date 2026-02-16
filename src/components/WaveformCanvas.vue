<script setup>
import { ref, watch, onMounted, onUnmounted, computed } from "vue";

const props = defineProps({
  track: Object,
});

const container = ref(null);
const waveCanvas = ref(null);
const lufsCanvas = ref(null);
const specCanvas = ref(null);

let resizeObs = null;

const hasAnalysis = computed(() => props.track?.analysis);
const hasWaveform = computed(() => props.track?.waveform?.length > 0);
const hasDone = computed(() => props.track?.status === "done");

onMounted(() => {
  resizeObs = new ResizeObserver(() => drawAll());
  if (container.value) resizeObs.observe(container.value);
  drawAll();
});

onUnmounted(() => {
  if (resizeObs) resizeObs.disconnect();
});

watch(
  () => [props.track?.waveform, props.track?.analysis, props.track?.postWaveform, props.track?.postAnalysis, props.track?.result],
  () => drawAll(),
  { deep: true }
);

function drawAll() {
  drawWaveform();
  drawLufs();
  drawSpectrum();
}

// ---- WAVEFORM ----
function drawWaveform() {
  const cvs = waveCanvas.value;
  if (!cvs) return;
  const parent = cvs.parentElement;
  if (!parent) return;
  const rect = parent.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  cvs.width = rect.width * dpr;
  cvs.height = rect.height * dpr;
  cvs.style.width = `${rect.width}px`;
  cvs.style.height = `${rect.height}px`;
  const ctx = cvs.getContext("2d");
  ctx.scale(dpr, dpr);
  const w = rect.width;
  const h = rect.height;
  ctx.clearRect(0, 0, w, h);

  if (!hasWaveform.value) {
    drawEmptyWaveform(ctx, w, h);
    return;
  }

  const data = props.track.waveform;
  const postData = props.track?.postWaveform;
  const midY = h / 2;

  // Grid lines
  ctx.strokeStyle = "rgba(56, 189, 248, 0.06)";
  ctx.lineWidth = 1;
  for (const frac of [0.25, 0.5, 0.75]) {
    ctx.beginPath();
    ctx.moveTo(0, h * frac);
    ctx.lineTo(w, h * frac);
    ctx.stroke();
  }

  // Draw post waveform (if mastered) behind in purple
  if (postData?.length > 0) {
    drawWaveformData(ctx, postData, w, h, midY, "rgba(167, 139, 250, 0.3)", "rgba(167, 139, 250, 0.08)");
  }

  // Draw main waveform
  drawWaveformData(ctx, data, w, h, midY, "rgba(56, 189, 248, 0.6)", "rgba(56, 189, 248, 0.12)");

  // Center line
  ctx.strokeStyle = "rgba(56, 189, 248, 0.15)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, midY);
  ctx.lineTo(w, midY);
  ctx.stroke();

  // Labels
  ctx.fillStyle = "rgba(139, 149, 184, 0.4)";
  ctx.font = "10px Inter, sans-serif";
  ctx.textAlign = "left";
  ctx.fillText("Waveform", 8, 16);

  if (postData?.length > 0) {
    ctx.fillStyle = "rgba(167, 139, 250, 0.5)";
    ctx.fillText("After (purple)", 8, 28);
  }
}

function drawWaveformData(ctx, data, w, h, midY, strokeColor, fillColor) {
  const len = data.length;
  const step = w / len;

  // Filled area
  ctx.beginPath();
  ctx.moveTo(0, midY);
  for (let i = 0; i < len; i++) {
    const x = i * step;
    ctx.lineTo(x, midY + data[i][0] * midY);
  }
  for (let i = len - 1; i >= 0; i--) {
    const x = i * step;
    ctx.lineTo(x, midY + data[i][1] * midY);
  }
  ctx.closePath();
  ctx.fillStyle = fillColor;
  ctx.fill();

  // Outline top
  ctx.beginPath();
  for (let i = 0; i < len; i++) {
    const x = i * step;
    const y = midY + data[i][1] * midY;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = 1;
  ctx.stroke();

  // Outline bottom
  ctx.beginPath();
  for (let i = 0; i < len; i++) {
    const x = i * step;
    const y = midY + data[i][0] * midY;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();
}

function drawEmptyWaveform(ctx, w, h) {
  ctx.strokeStyle = "rgba(56, 189, 248, 0.06)";
  ctx.lineWidth = 1;
  const midY = h / 2;
  ctx.beginPath();
  ctx.moveTo(0, midY);
  ctx.lineTo(w, midY);
  ctx.stroke();

  ctx.fillStyle = "rgba(139, 149, 184, 0.2)";
  ctx.font = "13px Inter, sans-serif";
  ctx.textAlign = "center";
  ctx.fillText("Select a track and click Analyze to see the waveform", w / 2, midY - 6);
}

// ---- LUFS METER ----
function drawLufs() {
  const cvs = lufsCanvas.value;
  if (!cvs) return;
  const parent = cvs.parentElement;
  if (!parent) return;
  const rect = parent.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  cvs.width = rect.width * dpr;
  cvs.height = rect.height * dpr;
  cvs.style.width = `${rect.width}px`;
  cvs.style.height = `${rect.height}px`;
  const ctx = cvs.getContext("2d");
  ctx.scale(dpr, dpr);
  const w = rect.width;
  const h = rect.height;
  ctx.clearRect(0, 0, w, h);

  if (!hasAnalysis.value) return;

  const a = props.track.analysis;
  const post = props.track?.postAnalysis || props.track?.result?.post_analysis;

  const metrics = [
    { label: "Integrated", value: a.lufs_integrated, post: post?.lufs_integrated },
    { label: "Short-Term Max", value: a.lufs_short_term_max, post: post?.lufs_short_term_max },
    { label: "Peak", value: a.peak_db, post: post?.peak_db },
    { label: "True Peak", value: a.true_peak_db, post: post?.true_peak_db },
  ];

  const meterH = 14;
  const gap = 6;
  const labelW = 90;
  const valueW = 55;
  const meterLeft = labelW;
  const meterW = w - labelW - valueW - 24;
  const startY = (h - metrics.length * (meterH + gap)) / 2;

  // Target zone
  const targetLufs = -14;
  const rangeMin = -60;
  const rangeMax = 0;

  for (let i = 0; i < metrics.length; i++) {
    const m = metrics[i];
    const y = startY + i * (meterH + gap);
    const pct = Math.max(0, Math.min(1, (m.value - rangeMin) / (rangeMax - rangeMin)));

    // Label
    ctx.fillStyle = "rgba(139, 149, 184, 0.6)";
    ctx.font = "600 10px Inter, sans-serif";
    ctx.textAlign = "right";
    ctx.fillText(m.label, labelW - 8, y + meterH / 2 + 3);

    // Track bg
    ctx.fillStyle = "rgba(26, 34, 54, 0.8)";
    roundRectFill(ctx, meterLeft, y, meterW, meterH, 4);

    // Target zone indicator (green band around -14 LUFS)
    if (m.label.includes("Integrated") || m.label.includes("Short")) {
      const tPct1 = Math.max(0, (-16 - rangeMin) / (rangeMax - rangeMin));
      const tPct2 = Math.min(1, (-12 - rangeMin) / (rangeMax - rangeMin));
      ctx.fillStyle = "rgba(52, 211, 153, 0.08)";
      ctx.fillRect(meterLeft + tPct1 * meterW, y, (tPct2 - tPct1) * meterW, meterH);
    }

    // Fill bar
    const color = lufsColor(m.value, m.label);
    const grad = ctx.createLinearGradient(meterLeft, 0, meterLeft + pct * meterW, 0);
    grad.addColorStop(0, color + "88");
    grad.addColorStop(1, color);
    ctx.fillStyle = grad;
    roundRectFill(ctx, meterLeft, y, pct * meterW, meterH, 4);

    // Post value marker
    if (m.post != null) {
      const postPct = Math.max(0, Math.min(1, (m.post - rangeMin) / (rangeMax - rangeMin)));
      const markerX = meterLeft + postPct * meterW;
      ctx.strokeStyle = "#a78bfa";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(markerX, y - 2);
      ctx.lineTo(markerX, y + meterH + 2);
      ctx.stroke();
    }

    // Value text
    ctx.fillStyle = color;
    ctx.font = "700 10px 'JetBrains Mono', monospace";
    ctx.textAlign = "left";
    const valueStr = `${m.value.toFixed(1)}`;
    ctx.fillText(valueStr, meterLeft + meterW + 8, y + meterH / 2 + 3);
  }
}

function lufsColor(value, label) {
  if (label.includes("Peak")) {
    return value > -1 ? "#f87171" : value > -3 ? "#fbbf24" : "#38bdf8";
  }
  if (value > -8) return "#f87171";
  if (value > -12) return "#fbbf24";
  if (value > -16) return "#34d399";
  return "#38bdf8";
}

// ---- SPECTRUM ----
function drawSpectrum() {
  const cvs = specCanvas.value;
  if (!cvs) return;
  const parent = cvs.parentElement;
  if (!parent) return;
  const rect = parent.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  cvs.width = rect.width * dpr;
  cvs.height = rect.height * dpr;
  cvs.style.width = `${rect.width}px`;
  cvs.style.height = `${rect.height}px`;
  const ctx = cvs.getContext("2d");
  ctx.scale(dpr, dpr);
  const w = rect.width;
  const h = rect.height;
  ctx.clearRect(0, 0, w, h);

  if (!hasAnalysis.value) return;

  const bands = props.track.analysis.frequency_bands;
  const post = props.track?.postAnalysis?.frequency_bands ||
    props.track?.result?.post_analysis?.frequency_bands;

  const labels = ["Sub", "Bass", "Low-mid", "Mid", "Hi-mid", "Presence", "Brilliance"];
  const values = [bands.sub_bass, bands.bass, bands.low_mid, bands.mid, bands.upper_mid, bands.presence, bands.brilliance];
  const postValues = post
    ? [post.sub_bass, post.bass, post.low_mid, post.mid, post.upper_mid, post.presence, post.brilliance]
    : null;

  const colors = ["#ef4444", "#f59e0b", "#22c55e", "#38bdf8", "#818cf8", "#a78bfa", "#ec4899"];
  const maxVal = Math.max(...values, ...(postValues || []), 0.01);
  const barCount = values.length;
  const gap = 8;
  const padX = 16;
  const padBottom = 24;
  const barWidth = Math.min((w - 2 * padX - (barCount - 1) * gap) / barCount, 80);
  const startX = (w - (barWidth * barCount + gap * (barCount - 1))) / 2;
  const maxH = h - padBottom - 24;
  const baseY = h - padBottom;

  for (let i = 0; i < barCount; i++) {
    const x = startX + i * (barWidth + gap);

    // Post bar (behind, translucent purple)
    if (postValues) {
      const postH = (postValues[i] / maxVal) * maxH * 0.85;
      ctx.fillStyle = "rgba(167, 139, 250, 0.2)";
      ctx.beginPath();
      roundRectPath(ctx, x - 2, baseY - postH, barWidth + 4, postH, 4);
      ctx.fill();
    }

    // Main bar
    const barH = (values[i] / maxVal) * maxH * 0.85;
    const grad = ctx.createLinearGradient(x, baseY, x, baseY - barH);
    grad.addColorStop(0, colors[i] + "dd");
    grad.addColorStop(1, colors[i] + "44");
    ctx.fillStyle = grad;
    ctx.beginPath();
    roundRectPath(ctx, x, baseY - barH, barWidth, barH, 4);
    ctx.fill();

    // Glow top
    ctx.shadowColor = colors[i];
    ctx.shadowBlur = 8;
    ctx.fillStyle = colors[i] + "20";
    ctx.fillRect(x, baseY - barH, barWidth, 3);
    ctx.shadowBlur = 0;

    // Label
    ctx.fillStyle = "rgba(139, 149, 184, 0.5)";
    ctx.font = "600 9px Inter, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(labels[i], x + barWidth / 2, baseY + 14);

    // Value
    ctx.fillStyle = colors[i];
    ctx.font = "600 9px 'JetBrains Mono', monospace";
    ctx.fillText(values[i].toFixed(2), x + barWidth / 2, baseY - barH - 6);
  }
}

function roundRectFill(ctx, x, y, w, h, r) {
  ctx.beginPath();
  roundRectPath(ctx, x, y, w, h, r);
  ctx.fill();
}

function roundRectPath(ctx, x, y, w, h, r) {
  if (w < 0) { x += w; w = -w; }
  if (h < 0) { y += h; h = -h; }
  r = Math.min(r, w / 2, h / 2);
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
  <div class="viz-container" ref="container">
    <div class="viz-row">
      <div class="viz-panel waveform-panel">
        <canvas ref="waveCanvas"></canvas>
      </div>
    </div>
    <div class="viz-row viz-row-bottom">
      <div class="viz-panel lufs-panel">
        <canvas ref="lufsCanvas"></canvas>
      </div>
      <div class="viz-panel spectrum-panel">
        <canvas ref="specCanvas"></canvas>
      </div>
    </div>
  </div>
</template>

<style scoped>
.viz-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--navy-deep);
}

.viz-row {
  flex: 1;
  display: flex;
  gap: 1px;
  min-height: 0;
}

.viz-row-bottom {
  flex: 0.6;
}

.viz-panel {
  position: relative;
  flex: 1;
  background: linear-gradient(180deg, rgba(10, 14, 26, 0.9), rgba(5, 8, 22, 0.95));
  overflow: hidden;
}

.viz-panel canvas {
  display: block;
  width: 100%;
  height: 100%;
}

.lufs-panel { flex: 0.45; }
.spectrum-panel { flex: 0.55; }
</style>
