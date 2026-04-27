<script setup>
const props = defineProps({
  visible: Boolean,
  error: {
    type: [String, Object],
    default: null,
  },
});

const emit = defineEmits(["close", "retry", "fallback", "skip"]);

const parsedError = computed(() => {
  if (!props.error) return null;

  // If error is already a parsed object
  if (typeof props.error === "object") {
    return props.error;
  }

  // Try to parse JSON string error response
  try {
    return JSON.parse(props.error);
  } catch {
    // Fallback for simple string errors
    return {
      message: props.error,
      code: "UNKNOWN_ERROR",
      can_retry: false,
      can_fallback: false,
      suggested_action: null,
    };
  }
});

const errorIcon = computed(() => {
  const code = parsedError.value?.code;
  if (code === "NETWORK_TIMEOUT" || code === "API_QUOTA_EXCEEDED") {
    return "network";
  } else if (code === "AUDIO_DECODE_FAILED" || code === "FILE_IO_ERROR") {
    return "file";
  } else if (code === "PYTHON_UNAVAILABLE") {
    return "code";
  } else if (code === "BACKEND_ERROR") {
    return "server";
  }
  return "warning";
});

const errorTitle = computed(() => {
  const code = parsedError.value?.code;
  const titles = {
    NETWORK_TIMEOUT: "Network Error",
    API_QUOTA_EXCEEDED: "API Limit Reached",
    AUDIO_DECODE_FAILED: "Audio File Error",
    FILE_IO_ERROR: "File Error",
    PYTHON_UNAVAILABLE: "Python Environment Error",
    BACKEND_ERROR: "Backend Error",
    PROCESSING_ERROR: "Processing Error",
    VALIDATION_ERROR: "Validation Error",
    INVALID_CONFIG: "Configuration Error",
  };
  return titles[code] || "Error";
});

const handleRetry = () => {
  emit("retry");
  emit("close");
};

const handleFallback = () => {
  emit("fallback");
  emit("close");
};

const handleSkip = () => {
  emit("skip");
  emit("close");
};
</script>

<template>
  <Transition name="scale">
    <div v-if="visible && parsedError" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog error-dialog">
        <div class="dialog-header error">
          <div class="error-icon">
            <svg v-if="errorIcon === 'network'" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12" />
            </svg>
            <svg v-else-if="errorIcon === 'file'" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
            </svg>
            <svg v-else-if="errorIcon === 'code'" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 18" />
            </svg>
            <svg v-else-if="errorIcon === 'server'" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12" />
            </svg>
            <svg v-else width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
            </svg>
          </div>
          <h2 class="dialog-title">{{ errorTitle }}</h2>
          <button class="close-btn" @click="emit('close')">&times;</button>
        </div>

        <div class="dialog-body">
          <div class="error-message">{{ parsedError.message }}</div>

          <div v-if="parsedError.suggested_action" class="error-suggestion">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z" />
            </svg>
            <span>{{ parsedError.suggested_action }}</span>
          </div>

          <div v-if="parsedError.details" class="error-details">
            <details>
              <summary>Technical Details</summary>
              <pre>{{ parsedError.details }}</pre>
            </details>
          </div>
        </div>

        <div class="dialog-footer">
          <button v-if="parsedError.can_fallback" class="btn btn-secondary" @click="handleFallback">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.53 16.122a3 3 0 00-5.78 1.128 2.25 2.25 0 01-2.4 2.245 4.5 4.5 0 008.4-2.245c0-.399-.077-.78-.22-1.128zm0 0a15.998 15.998 0 003.388-1.62m-5.043-.025a15.994 15.994 0 011.622-3.395m3.42 3.42a15.995 15.995 0 004.764-4.648l3.876-5.814a1.151 1.151 0 00-1.597-1.597L14.146 6.32a15.996 15.996 0 00-4.649 4.763m3.42 3.42a6.776 6.776 0 00-3.42-3.42" />
            </svg>
            Use Alternative Backend
          </button>
          <button v-if="parsedError.can_retry" class="btn btn-secondary" @click="handleRetry">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            Retry
          </button>
          <button class="btn btn-primary" @click="emit('close')">Close</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.error-dialog {
  width: 520px;
  max-width: 90vw;
}

.dialog-header.error {
  background: linear-gradient(135deg, rgba(248, 113, 113, 0.1) 0%, rgba(248, 113, 113, 0.05) 100%);
  border-bottom: 1px solid rgba(248, 113, 113, 0.3);
}

.error-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: rgba(248, 113, 113, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--danger);
}

.error-message {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text);
  margin-bottom: 16px;
}

.error-suggestion {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  border-radius: 10px;
  background: rgba(56, 189, 248, 0.08);
  border: 1px solid rgba(56, 189, 248, 0.2);
  font-size: 13px;
  line-height: 1.5;
  color: var(--cyan);
}

.error-suggestion svg {
  flex-shrink: 0;
  margin-top: 2px;
}

.error-details {
  margin-top: 16px;
}

.error-details details {
  cursor: pointer;
}

.error-details summary {
  font-size: 12px;
  color: var(--text-dim);
  padding: 8px 0;
  user-select: none;
}

.error-details summary:hover {
  color: var(--text);
}

.error-details pre {
  margin-top: 8px;
  padding: 12px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-dim);
  overflow-x: auto;
}

.dialog-footer {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  padding: 20px 24px;
  border-top: 1px solid var(--border-light);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  border: 1px solid transparent;
}

.btn-primary {
  background: linear-gradient(135deg, var(--cyan) 0%, var(--purple) 100%);
  color: #050816;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(56, 189, 248, 0.3);
}

.btn-secondary {
  background: transparent;
  border-color: var(--border-light);
  color: var(--text);
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: var(--border);
}

.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(5, 8, 22, 0.7);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.dialog {
  background: linear-gradient(135deg, rgba(15, 23, 42, 0.95) 0%, rgba(15, 23, 42, 0.98) 100%);
  border-radius: 16px;
  border: 1px solid var(--border-light);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-light);
}

.dialog-title {
  flex: 1;
  font-size: 18px;
  font-weight: 600;
  margin: 0;
  color: var(--text);
}

.close-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 24px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text);
}

.dialog-body {
  padding: 24px;
}

/* Transitions */
.scale-enter-active,
.scale-leave-active {
  transition: all 0.2s ease;
}

.scale-enter-from,
.scale-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.2s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>
