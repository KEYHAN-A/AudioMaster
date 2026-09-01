<script setup>
import { computed, ref, watch } from "vue";

const props = defineProps({ preview: Object, busy: Boolean });
defineEmits(["render"]);

const original = ref(null);
const mastered = ref(null);
const active = ref("mastered");
const position = ref(0);
const playing = ref(false);
const duration = computed(() => props.preview?.duration_seconds || 30);

watch(() => props.preview, () => {
  position.value = 0;
  playing.value = false;
});

async function audition(which) {
  const from = active.value === "original" ? original.value : mastered.value;
  const to = which === "original" ? original.value : mastered.value;
  if (!to) return;
  if (from) {
    position.value = from.currentTime || position.value;
    from.pause();
  }
  original.value?.pause();
  mastered.value?.pause();
  to.currentTime = position.value;
  active.value = which;
  await to.play();
  playing.value = true;
}

function pause() {
  const player = active.value === "original" ? original.value : mastered.value;
  if (player) position.value = player.currentTime;
  original.value?.pause();
  mastered.value?.pause();
  playing.value = false;
}

function seek() {
  if (original.value) original.value.currentTime = position.value;
  if (mastered.value) mastered.value.currentTime = position.value;
}
</script>

<template>
  <section class="preview-panel glass-card" aria-label="Level-matched mastering preview">
    <div>
      <strong>Level-matched A/B preview</strong>
      <p>Switch at the same playhead position without loudness bias.</p>
    </div>
    <button class="btn btn-ghost btn-sm" :disabled="busy" @click="$emit('render')">Render 30s preview</button>
    <template v-if="preview">
      <audio ref="original" :src="preview.originalUrl" preload="auto" @timeupdate="active === 'original' && (position = $event.target.currentTime || 0)" @ended="playing = false"></audio>
      <audio ref="mastered" :src="preview.masteredUrl" preload="auto" @timeupdate="active === 'mastered' && (position = $event.target.currentTime || 0)" @ended="playing = false"></audio>
      <div class="ab-controls">
        <button class="btn btn-sm" :aria-pressed="active === 'original'" @click="audition('original')">A · Original</button>
        <button class="btn btn-sm" :aria-pressed="active === 'mastered'" @click="audition('mastered')">B · Mastered</button>
        <button v-if="playing" class="btn btn-ghost btn-sm" @click="pause">Pause</button>
        <input v-model.number="position" aria-label="Preview playhead" type="range" min="0" :max="duration" step="0.05" @input="seek" />
      </div>
    </template>
  </section>
</template>

<style scoped>
.ab-controls { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.ab-controls input { min-width: 180px; flex: 1; }
</style>
