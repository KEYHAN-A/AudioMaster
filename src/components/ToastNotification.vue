<script setup>
import { useToast } from "../composables/useToast.js";

const { toasts } = useToast();
</script>

<template>
  <div class="toast-container">
    <TransitionGroup name="slide-up">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast"
        :class="toast.type"
      >
        <svg v-if="toast.type === 'success'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
        </svg>
        <svg v-else-if="toast.type === 'error'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" />
        </svg>
        <span>{{ toast.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 40px;
  right: 16px;
  z-index: 500;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
}

.toast {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 12px;
  background: rgba(21, 28, 46, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid var(--border-light);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.toast.success {
  border-color: rgba(52, 211, 153, 0.30);
  color: var(--success);
}

.toast.error {
  border-color: rgba(248, 113, 113, 0.30);
  color: var(--danger);
}

.toast.info {
  border-color: var(--border-light);
  color: var(--cyan);
}
</style>
