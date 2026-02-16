<script setup>
defineProps({
  visible: Boolean,
  message: String,
  progress: { type: Number, default: 0 },
});
</script>

<template>
  <Transition name="fade">
    <div v-if="visible" class="dialog-overlay">
      <div class="processing-card glass-card">
        <div class="processing-anim">
          <div class="ring-outer">
            <div class="ring-inner spin"></div>
          </div>
        </div>
        <h3 class="processing-title gradient-text">Processing</h3>
        <p class="processing-message">{{ message || "Working..." }}</p>
        <div class="progress-bar" style="width: 220px;">
          <div
            class="progress-bar-fill"
            :style="{ width: progress > 0 ? progress + '%' : '100%' }"
          ></div>
        </div>
        <span v-if="progress > 0" class="progress-pct">{{ Math.round(progress) }}%</span>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.processing-card {
  padding: 40px 48px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  text-align: center;
}

.processing-anim { width: 64px; height: 64px; position: relative; }

.ring-outer {
  width: 100%; height: 100%;
  border-radius: 50%;
  border: 2px solid var(--border-subtle);
  display: flex; align-items: center; justify-content: center;
}

.ring-inner {
  width: 48px; height: 48px;
  border-radius: 50%;
  border: 3px solid transparent;
  border-top-color: var(--cyan);
  border-right-color: var(--purple);
}

.processing-title { font-size: 18px; font-weight: 800; }
.processing-message { color: var(--text-dim); font-size: 13px; }
.progress-pct { font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); }
</style>
