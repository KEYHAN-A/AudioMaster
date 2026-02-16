<script setup>
import { onMounted, onUnmounted, ref, computed } from "vue";
import { useMastering } from "../composables/useMastering.js";
import { useToast } from "../composables/useToast.js";
import WorkflowBar from "./WorkflowBar.vue";
import FilePanel from "./FilePanel.vue";
import AnalysisPanel from "./AnalysisPanel.vue";
import WaveformCanvas from "./WaveformCanvas.vue";
import MasteringDialog from "./MasteringDialog.vue";
import ProcessingDialog from "./ProcessingDialog.vue";
import SettingsDialog from "./SettingsDialog.vue";
import ToastNotification from "./ToastNotification.vue";

const {
  state,
  hasInput,
  isAnalyzed,
  isMastered,
  loadConfig,
  loadBackends,
  loadPresets,
  setInputFile,
  setReferenceFile,
  analyzeInput,
  masterTrack,
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
  if (meta && e.key === "o" && !e.shiftKey) {
    e.preventDefault();
    openFilePicker();
  } else if (meta && e.key === "r" && hasInput.value && !state.processing) {
    e.preventDefault();
    handleAnalyze();
  } else if (meta && e.key === "m" && isAnalyzed.value) {
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
    const path = await open({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["wav", "flac", "mp3", "ogg", "aiff"] }],
    });
    if (path) {
      setInputFile(path);
      showToast("File loaded", "success");
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

async function handleAnalyze() {
  if (!hasInput.value || state.processing) return;
  await analyzeInput();
  if (isAnalyzed.value) {
    showToast("Analysis complete", "success");
  }
}

function handleOpenMaster() {
  if (isAnalyzed.value) {
    showMasterDialog.value = true;
  }
}

async function handleMaster(outputPath) {
  showMasterDialog.value = false;
  await masterTrack(outputPath);
  if (isMastered.value) {
    showToast("Mastering complete!", "success");
  }
}

function handleDrop(e) {
  isDragOver.value = false;
  const files = e.dataTransfer?.files;
  if (files?.length > 0) {
    setInputFile(files[0].path || files[0].name);
    showToast("File loaded", "success");
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
          <p>Drop audio file to import</p>
        </div>
      </div>
    </Transition>

    <div class="app-content">
      <WorkflowBar
        :currentStep="state.currentStep"
        :canAnalyze="hasInput && !state.processing"
        :canMaster="isAnalyzed && !state.processing"
        :canExport="isMastered"
        @analyze="handleAnalyze"
        @master="handleOpenMaster"
      />

      <div class="toolbar">
        <div class="toolbar-group">
          <button class="btn btn-primary btn-sm" @click="openFilePicker" :disabled="state.processing">
            <span class="btn-icon">+</span> Import File
          </button>
          <button class="btn btn-ghost btn-sm" @click="openReferencePicker" :disabled="state.processing">
            Reference
          </button>
          <button class="btn btn-ghost btn-sm" @click="clearAll" :disabled="!hasInput">
            Clear
          </button>
        </div>
        <div class="toolbar-group">
          <button
            class="btn btn-accent btn-sm"
            @click="handleAnalyze"
            :disabled="!hasInput || state.processing"
          >
            Analyze
          </button>
          <button
            class="btn btn-success btn-sm"
            @click="handleOpenMaster"
            :disabled="!isAnalyzed || state.processing"
          >
            Master
          </button>
          <button
            class="btn btn-ghost btn-sm"
            @click="showSettings = true"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>

      <div class="main-area">
        <div class="side-panel">
          <FilePanel
            :inputFile="state.inputFile"
            :referenceFile="state.referenceFile"
            :analysis="state.analysis"
            :masterResult="state.masterResult"
            @openFile="openFilePicker"
            @openReference="openReferencePicker"
          />
        </div>
        <div class="center-area">
          <WaveformCanvas :analysis="state.analysis" :masterResult="state.masterResult" />
          <AnalysisPanel v-if="state.analysis" :analysis="state.analysis" :masterResult="state.masterResult" />
        </div>
      </div>

      <div class="status-bar">
        <span class="status-text">Mastering Pro v2.0.0</span>
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

    <!-- Dialogs -->
    <ProcessingDialog
      :visible="state.processing"
      :message="state.processingMessage"
    />

    <MasteringDialog
      :visible="showMasterDialog"
      :presets="state.presets"
      :backends="state.backends"
      :state="state"
      @close="showMasterDialog = false"
      @master="handleMaster"
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
  transition: border-color 0.2s ease;
}

.app-shell.drag-over {
  border: 2px solid var(--cyan);
}

.app-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background-color: var(--bg-panel);
  border-bottom: 1px solid var(--border-subtle);
  gap: 8px;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn-icon {
  font-weight: 700;
  margin-right: 2px;
}

.main-area {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.side-panel {
  width: 300px;
  min-width: 240px;
  border-right: 1px solid var(--border-subtle);
  overflow-y: auto;
  background-color: rgba(17, 24, 39, 0.5);
}

.center-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
