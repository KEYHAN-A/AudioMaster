<script setup>
const props = defineProps({
  tracks: { type: Array, default: () => [] },
  selectedId: { type: Number, default: null },
});

const emit = defineEmits(["select", "remove", "addMore"]);

const statusColors = {
  idle: "var(--text-muted)",
  analyzing: "var(--warning)",
  analyzed: "var(--cyan)",
  mastering: "var(--purple)",
  done: "var(--success)",
  error: "var(--danger)",
};

const statusLabels = {
  idle: "Pending",
  analyzing: "Analyzing...",
  analyzed: "Ready",
  mastering: "Mastering...",
  done: "Done",
  error: "Error",
};
</script>

<template>
  <div class="track-list">
    <div class="track-scroll">
      <div
        v-for="track in tracks"
        :key="track.id"
        class="track-card"
        :class="{
          selected: track.id === selectedId,
          processing: track.status === 'analyzing' || track.status === 'mastering',
          done: track.status === 'done',
          error: track.status === 'error',
        }"
        @click="emit('select', track.id)"
      >
        <button
          class="track-remove"
          @click.stop="emit('remove', track.id)"
          title="Remove"
        >&times;</button>

        <div class="track-icon">
          <svg v-if="track.status === 'done'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
          </svg>
          <svg v-else-if="track.status === 'error'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
          </svg>
          <svg v-else-if="track.status === 'analyzing' || track.status === 'mastering'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="spin">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182" />
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M9 9l10.5-3m0 6.553v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 11-.99-3.467l2.31-.66a2.25 2.25 0 001.632-2.163zm0 0V2.34A1.154 1.154 0 0017.882 1.2l-5.764 1.648A1.154 1.154 0 0011 3.996V14.5" />
          </svg>
        </div>

        <div class="track-info">
          <span class="track-name">{{ track.name }}</span>
          <div class="track-meta">
            <span
              class="track-status"
              :style="{ color: statusColors[track.status] }"
            >{{ statusLabels[track.status] }}</span>
            <span v-if="track.analysis" class="track-lufs">
              {{ track.analysis.lufs_integrated.toFixed(1) }} LUFS
            </span>
          </div>
        </div>
      </div>

      <button class="add-card" @click="emit('addMore')">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        <span>Add</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.track-list {
  border-bottom: 1px solid var(--border-subtle);
  background: rgba(17, 24, 39, 0.6);
  padding: 8px 12px;
}

.track-scroll {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.track-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  padding-right: 28px;
  min-width: 180px;
  max-width: 240px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg-input);
  cursor: pointer;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.track-card:hover {
  border-color: var(--border-light);
}

.track-card.selected {
  border-color: var(--cyan);
  background: var(--cyan-subtle);
}

.track-card.processing {
  border-color: var(--warning);
}

.track-card.done {
  border-color: rgba(52, 211, 153, 0.25);
}

.track-card.error {
  border-color: rgba(248, 113, 113, 0.25);
}

.track-remove {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  opacity: 0;
  transition: all 0.15s ease;
}

.track-card:hover .track-remove { opacity: 1; }
.track-remove:hover { background: rgba(239, 68, 68, 0.15); color: var(--danger); }

.track-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: var(--bg-card);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--text-dim);
}

.track-card.selected .track-icon { color: var(--cyan); }
.track-card.done .track-icon { color: var(--success); }
.track-card.error .track-icon { color: var(--danger); }

.track-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.track-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-bright);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.track-meta {
  display: flex;
  gap: 8px;
  align-items: center;
}

.track-status {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.track-lufs {
  font-size: 10px;
  font-weight: 700;
  font-family: var(--font-mono);
  color: var(--cyan);
}

.add-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  min-width: 72px;
  padding: 10px;
  border-radius: 12px;
  border: 1px dashed var(--border-light);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.add-card span { font-size: 10px; font-weight: 600; }
.add-card:hover { border-color: var(--cyan); color: var(--cyan); background: var(--cyan-subtle); }
</style>
