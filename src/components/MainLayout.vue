<script setup>
import { onMounted, onUnmounted, ref } from "vue";
import { useMastering } from "../composables/useMastering.js";
import { useToast } from "../composables/useToast.js";
import TrackList from "./TrackList.vue";
import AnalysisPanel from "./AnalysisPanel.vue";
import WaveformCanvas from "./WaveformCanvas.vue";
import MasteringDialog from "./MasteringDialog.vue";
import ProcessingDialog from "./ProcessingDialog.vue";
import SettingsDialog from "./SettingsDialog.vue";
import ToastNotification from "./ToastNotification.vue";

const {
  state,
  hasTracks,
  selectedTrack,
  allAnalyzed,
  hasAnyResult,
  loadConfig,
  loadBackends,
  loadPresets,
  addTracks,
  removeTrack,
  selectTrack,
  setReferenceFile,
  analyzeAll,
  analyzeSelected,
  masterAll,
  masterSelected,
  clearAll,
} = useMastering();

const { showToast } = useToast();

const showMasterDialog = ref(false);
const showSettings = ref(false);
const isDragOver = ref(false);

onMounted(async () => {
  await loadConfig();
  await loadPresets();
  loadBackends();
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});

function handleKeydown(e) {
  const meta = e.metaKey || e.ctrlKey;
  if (meta && e.key === "o") {
    e.preventDefault();
    openFilePicker();
  } else if (meta && e.key === "r" && hasTracks.value && !state.processing) {
    e.preventDefault();
    handleAnalyzeAll();
  } else if (meta && e.key === "m" && allAnalyzed.value) {
    e.preventDefault();
    showMasterDialog.value = true;
  } else if (e.key === "Escape") {
    showMasterDialog.value = false;
    showSettings.value = false;
  }
}

async function openFilePicker() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const paths = await open({
      multiple: true,
      filters: [{ name: "Audio", extensions: ["wav", "flac", "mp3", "ogg", "aiff"] }],
    });
    if (paths) {
      const list = Array.isArray(paths) ? paths : [paths];
      addTracks(list);
      showToast(`${list.length} file(s) added`, "success");
    }
  } catch (e) {
    showToast(`Import failed: ${e}`, "error");
  }
}

async function openReferencePicker() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["wav", "flac", "mp3", "ogg", "aiff"] }],
    });
    if (path) {
      setReferenceFile(path);
      showToast("Reference loaded", "success");
    }
  } catch (e) {
    showToast(`Import failed: ${e}`, "error");
  }
}

async function handleAnalyzeAll() {
  if (!hasTracks.value || state.processing) return;
  await analyzeAll();
  showToast("Analysis complete", "success");
}

async function handleMasterAll() {
  showMasterDialog.value = false;
  await masterAll();
  if (hasAnyResult.value) {
    showToast("Mastering complete!", "success");
  }
}

function handleDrop(e) {
  isDragOver.value = false;
  const files = e.dataTransfer?.files;
  if (files?.length > 0) {
    const paths = [];
    for (let i = 0; i < files.length; i++) {
      paths.push(files[i].path || files[i].name);
    }
    addTracks(paths);
    showToast(`${paths.length} file(s) added`, "success");
  }
}
</script>

<template>
  <div
    class="app-shell"
    :class="{ 'drag-over': isDragOver }"
    @dragover.prevent="isDragOver = true"
    @dragleave="isDragOver = false"
    @drop.prevent="handleDrop"
  >
    <div class="bg-grid"></div>
    <div class="glow-orb glow-orb-cyan orb-1"></div>
    <div class="glow-orb glow-orb-purple orb-2"></div>

    <!-- Drag overlay -->
    <Transition name="fade">
      <div v-if="isDragOver" class="drag-overlay">
        <div class="drag-content">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 16v-8m0 0l-3 3m3-3l3 3M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5" />
          </svg>
          <p>Drop audio files to import</p>
        </div>
      </div>
    </Transition>

    <div class="app-content">
      <!-- ============ EMPTY STATE ============ -->
      <Transition name="fade" mode="out-in">
        <div v-if="!hasTracks" class="empty-state" key="empty">
          <div class="empty-hero">
            <div class="empty-icon float">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 9l10.5-3m0 6.553v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 11-.99-3.467l2.31-.66a2.25 2.25 0 001.632-2.163zm0 0V2.34A1.154 1.154 0 0017.882 1.2l-5.764 1.648A1.154 1.154 0 0011 3.996V14.5" />
              </svg>
            </div>
            <h1 class="empty-title gradient-text">AudioMaster</h1>
            <p class="empty-subtitle">AI-powered audio mastering</p>

            <button class="btn btn-primary empty-import" @click="openFilePicker">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
              Import Audio Files
            </button>

            <p class="empty-hint">
              Drag & drop files here or press
              <kbd>Cmd+O</kbd>
            </p>

            <div class="empty-features stagger-enter">
              <div class="feature-chip">
                <span class="feature-dot" style="background: var(--cyan);"></span>
                Batch mastering
              </div>
              <div class="feature-chip">
                <span class="feature-dot" style="background: var(--purple);"></span>
                AI-assisted EQ & compression
              </div>
              <div class="feature-chip">
                <span class="feature-dot" style="background: var(--success);"></span>
                Reference matching
              </div>
            </div>
          </div>

          <div class="empty-bottom">
            <button class="btn btn-ghost btn-sm" @click="showSettings = true">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              Settings
            </button>
          </div>
        </div>

        <!-- ============ LOADED STATE ============ -->
        <div v-else class="loaded-state" key="loaded">
          <!-- Toolbar -->
          <div class="toolbar">
            <div class="toolbar-group">
              <button class="btn btn-primary btn-sm" @click="openFilePicker" :disabled="state.processing">
                + Add Files
              </button>
              <button class="btn btn-ghost btn-sm" @click="openReferencePicker" :disabled="state.processing">
                Reference
              </button>
              <span v-if="state.referenceFile" class="ref-badge" :title="state.referenceFile">
                REF: {{ state.referenceFile.split('/').pop() }}
              </span>
            </div>
            <div class="toolbar-group">
              <button
                class="btn btn-accent btn-sm"
                @click="handleAnalyzeAll"
                :disabled="!hasTracks || state.processing"
              >
                Analyze All
              </button>
              <button
                class="btn btn-success btn-sm"
                @click="showMasterDialog = true"
                :disabled="!allAnalyzed || state.processing"
              >
                Master All
              </button>
              <button class="btn btn-ghost btn-sm" @click="clearAll" :disabled="state.processing">
                Clear
              </button>
              <button class="btn btn-ghost btn-sm" @click="showSettings = true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28z" />
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Track cards -->
          <TrackList
            :tracks="state.tracks"
            :selectedId="state.selectedTrackId"
            @select="selectTrack"
            @remove="removeTrack"
            @addMore="openFilePicker"
          />

          <!-- Visualization -->
          <div class="viz-area">
            <WaveformCanvas
              :track="selectedTrack"
            />
          </div>

          <!-- Analysis metrics -->
          <AnalysisPanel
            v-if="selectedTrack?.analysis"
            :analysis="selectedTrack.analysis"
            :postAnalysis="selectedTrack?.postAnalysis || selectedTrack?.result?.post_analysis"
          />

          <!-- Status bar -->
          <div class="status-bar">
            <span class="status-text">AudioMaster v1.0.0</span>
            <span class="status-text">
              {{ state.tracks.length }} track{{ state.tracks.length !== 1 ? 's' : '' }}
            </span>
            <span v-if="state.error" class="status-text status-error" @click="state.error = null">
              {{ state.error }}
            </span>
            <span v-else class="status-text status-dim">
              {{ state.processing ? state.processingMessage : 'Ready' }}
            </span>
            <span class="status-text status-dim">
              Backend: {{ state.selectedBackend }}
            </span>
          </div>
        </div>
      </Transition>
    </div>

    <!-- Dialogs -->
    <ProcessingDialog
      :visible="state.processing"
      :message="state.processingMessage"
      :progress="state.processingProgress"
    />

    <MasteringDialog
      :visible="showMasterDialog"
      :presets="state.presets"
      :backends="state.backends"
      :state="state"
      @close="showMasterDialog = false"
      @master="handleMasterAll"
    />

    <SettingsDialog
      :visible="showSettings"
      :config="state.config"
      @close="showSettings = false"
    />

    <ToastNotification />
  </div>
</template>

<style scoped>
.app-shell {
  width: 100vw;
  height: 100vh;
  position: relative;
  overflow: hidden;
  background-color: var(--navy-deep);
}

.app-shell.drag-over { border: 2px solid var(--cyan); }

.app-content {
  position: relative;
  z-index: 1;
  height: 100vh;
}

/* ---------- Empty state ---------- */
.empty-state {
  height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.empty-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  text-align: center;
}

.empty-icon {
  color: var(--cyan);
  opacity: 0.7;
  margin-bottom: 8px;
}

.empty-title {
  font-size: 36px;
  font-weight: 800;
  letter-spacing: -1px;
}

.empty-subtitle {
  font-size: 15px;
  color: var(--text-dim);
  margin-top: -8px;
}

.empty-import {
  margin-top: 16px;
  padding: 14px 36px;
  font-size: 15px;
  border-radius: 18px;
  gap: 10px;
}

.empty-hint {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 8px;
}

.empty-hint kbd {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-dim);
}

.empty-features {
  display: flex;
  gap: 12px;
  margin-top: 32px;
}

.feature-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 20px;
  border: 1px solid var(--border);
  background: var(--glass);
  font-size: 12px;
  color: var(--text-dim);
}

.feature-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.empty-bottom {
  position: absolute;
  bottom: 16px;
}

/* ---------- Loaded state ---------- */
.loaded-state {
  height: 100vh;
  display: flex;
  flex-direction: column;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background-color: var(--bg-panel);
  border-bottom: 1px solid var(--border-subtle);
  gap: 8px;
  flex-shrink: 0;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ref-badge {
  display: inline-block;
  padding: 3px 10px;
  border-radius: 8px;
  background: var(--purple-subtle);
  color: var(--purple);
  font-size: 10px;
  font-weight: 600;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.viz-area {
  flex: 1;
  overflow: hidden;
  min-height: 200px;
}

.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  background-color: var(--bg-panel);
  border-top: 1px solid var(--border-subtle);
  font-size: 11px;
  gap: 16px;
  flex-shrink: 0;
}

.status-text { color: var(--text-dim); }
.status-dim { opacity: 0.6; }
.status-error { color: var(--danger); opacity: 1; cursor: pointer; }
.status-error:hover { text-decoration: underline; }

.orb-1 { width: 500px; height: 500px; top: -100px; right: -100px; opacity: 0.12; }
.orb-2 { width: 400px; height: 400px; bottom: 50px; left: -150px; opacity: 0.08; }

.drag-overlay {
  position: fixed; inset: 0; z-index: 100;
  background: rgba(6, 12, 28, 0.85);
  backdrop-filter: blur(8px);
  display: flex; align-items: center; justify-content: center;
}
.drag-content { display: flex; flex-direction: column; align-items: center; gap: 16px; color: var(--cyan); }
.drag-content p { font-size: 16px; font-weight: 600; color: var(--text-bright); }
</style>
