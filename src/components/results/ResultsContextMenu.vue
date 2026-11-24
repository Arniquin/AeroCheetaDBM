<script setup lang="ts">
import { Clipboard, Download } from 'lucide-vue-next';

defineProps<{
  pos: { x: number; y: number };
  selectedCol: string | null;
}>();

const emit = defineEmits(['copyColumn', 'copyFull']);
</script>

<template>
  <div class="context-menu" :style="{ top: pos.y + 'px', left: pos.x + 'px' }">
    <div v-if="selectedCol" class="menu-section">
      <div class="menu-label">Column: {{ selectedCol }}</div>
      <button @click="emit('copyColumn', true)"><Clipboard :size="12" /> Copy with Header</button>
      <button @click="emit('copyColumn', false)"><Clipboard :size="12" /> Copy without Header</button>
    </div>
    <div class="menu-section">
      <div class="menu-label">Full Results</div>
      <button @click="emit('copyFull', true)"><Download :size="12" /> Copy All with Headers</button>
      <button @click="emit('copyFull', false)"><Download :size="12" /> Copy All without Headers</button>
    </div>
  </div>
</template>

<style scoped>
.context-menu {
  position: fixed;
  background: #252526;
  border: 1px solid #444;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  z-index: 1000;
  border-radius: 4px;
  padding: 4px 0;
  min-width: 180px;
}
.menu-section {
  border-bottom: 1px solid #333;
  padding: 4px 0;
}
.menu-section:last-child {
  border-bottom: none;
}
.menu-label {
  padding: 4px 12px;
  font-size: 10px;
  color: #666;
  text-transform: uppercase;
  font-weight: bold;
}
button {
  width: 100%;
  padding: 6px 12px;
  background: transparent;
  border: none;
  color: #ccc;
  text-align: left;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
}
button:hover {
  background: #094771;
  color: #fff;
}
</style>
