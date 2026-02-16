<script setup>
const props = defineProps({
  currentStep: { type: Number, default: 0 },
  canAnalyze: Boolean,
  canMaster: Boolean,
  canExport: Boolean,
});

const emit = defineEmits(["analyze", "master"]);

const steps = [
  { label: "Import", icon: "M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" },
  { label: "Analyze", icon: "M3.75 3v11.25A2.25 2.25 0 006 16.5h2.25M3.75 3h-1.5m1.5 0h16.5m0 0h1.5m-1.5 0v11.25A2.25 2.25 0 0118 16.5h-2.25m-7.5 0h7.5m-7.5 0l-1 3m8.5-3l1 3m0 0l.5 1.5m-.5-1.5h-9.5m0 0l-.5 1.5" },
  { label: "Configure", icon: "M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-9.75 0h9.75" },
  { label: "Master", icon: "M9 9l10.5-3m0 6.553v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 11-.99-3.467l2.31-.66a2.25 2.25 0 001.632-2.163zm0 0V2.34A1.154 1.154 0 0017.882 1.2l-5.764 1.648A1.154 1.154 0 0011 3.996V14.5" },
];
</script>

<template>
  <div class="workflow-bar">
    <div
      v-for="(step, i) in steps"
      :key="step.label"
      class="workflow-step"
      :class="{
        active: i === currentStep,
        done: i < currentStep,
        clickable: (i === 1 && canAnalyze) || (i === 3 && canMaster),
      }"
      @click="
        i === 1 && canAnalyze ? emit('analyze') :
        i === 3 && canMaster ? emit('master') : null
      "
    >
      <div class="step-indicator">
        <svg v-if="i < currentStep" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" :d="step.icon" />
        </svg>
      </div>
      <span class="step-label">{{ step.label }}</span>
      <div v-if="i < steps.length - 1" class="step-connector" :class="{ filled: i < currentStep }"></div>
    </div>
  </div>
</template>

<style scoped>
.workflow-bar {
  display: flex;
  align-items: center;
  padding: 10px 20px;
  background: linear-gradient(180deg, var(--bg-panel), rgba(17, 24, 39, 0.8));
  border-bottom: 1px solid var(--border-subtle);
  gap: 0;
}

.workflow-step {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  position: relative;
  padding: 6px 0;
  cursor: default;
}

.workflow-step.clickable { cursor: pointer; }
.workflow-step.clickable:hover .step-indicator { border-color: var(--cyan); }
.workflow-step.clickable:hover .step-label { color: var(--text-bright); }

.step-indicator {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1.5px solid var(--border-light);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  transition: all 0.3s ease;
  flex-shrink: 0;
}

.workflow-step.done .step-indicator {
  background: var(--cyan);
  border-color: var(--cyan);
  color: var(--navy-deep);
}

.workflow-step.active .step-indicator {
  border-color: var(--cyan);
  color: var(--cyan);
  box-shadow: 0 0 12px var(--glow-cyan);
}

.step-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.6px;
  transition: color 0.2s ease;
}

.workflow-step.done .step-label { color: var(--cyan); }
.workflow-step.active .step-label { color: var(--text-bright); }

.step-connector {
  flex: 1;
  height: 1.5px;
  background-color: var(--border-subtle);
  margin: 0 12px;
  transition: background-color 0.3s ease;
}

.step-connector.filled {
  background-color: var(--cyan);
}
</style>
