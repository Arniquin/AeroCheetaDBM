<script setup lang="ts">
import { Database, Server, LogOut } from 'lucide-vue-next';

const emit = defineEmits(['viewExplorer', 'viewList', 'disconnect']);
</script>

<template>
  <div class="exit-view">
    <div class="section-title">Manage Connection</div>
    <div class="exit-options">
      <button @click="emit('viewExplorer')" class="secondary-btn">
        <Database :size="14" /> Return to Explorer
      </button>
      <button @click="emit('viewList')" class="secondary-btn">
        <Server :size="14" /> Browse Other Connections
      </button>
      <button @click="emit('disconnect')" class="danger-btn mt-16">
        <LogOut :size="14" /> Disconnect
      </button>
    </div>
  </div>
</template>

<style scoped>
.exit-view {
  padding: 16px;
  display: flex;
  flex-direction: column;
  height: 100%;
}
.section-title {
  padding: 8px;
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
}
.exit-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 16px;
}
.exit-options button {
  justify-content: flex-start;
  padding: 10px;
}
.secondary-btn {
  background: #3c3c3c;
  color: white;
  border: 1px solid #444;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s;
}
.secondary-btn:hover {
  background: #444;
}
.danger-btn {
  background: #8c3333;
  color: white;
  border: none;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s;
}
.danger-btn:hover {
  background: #a33c3c;
}
.mt-16 {
  margin-top: 16px;
}
</style>
