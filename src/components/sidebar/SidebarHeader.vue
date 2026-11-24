<script setup lang="ts">
import { Server, ArrowLeft, LogOut, Star, Plus, Settings } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();

defineProps<{
  viewMode: string;
}>();

const emit = defineEmits(['update:viewMode', 'openNewForm', 'disconnect']);

function getStatusColor(status: string) {
  switch (status) {
    case 'connected': return '#4ec9b0';
    case 'connecting': return '#ce9178';
    case 'error': return '#f48771';
    default: return '#888';
  }
}
</script>

<template>
  <div class="header">
    <div class="title"><Server :size="16" /> AeroCheetahDBM</div>
    <div class="status-indicator" :style="{ backgroundColor: getStatusColor(dbStore.connectionStatus) }" :title="dbStore.connectionStatus"></div>
    
    <div class="header-actions">
      <button v-if="viewMode === 'explorer'" @click="emit('update:viewMode', 'list')" class="icon-btn" title="Back to list"><ArrowLeft :size="14" /></button>
      <button v-if="viewMode === 'explorer'" @click="emit('update:viewMode', 'exit')" class="icon-btn danger" title="Exit"><LogOut :size="14" /></button>
      <button v-if="viewMode === 'exit'" @click="emit('update:viewMode', 'explorer')" class="icon-btn" title="Back"><ArrowLeft :size="14" /></button>
      <button v-if="viewMode === 'form'" @click="emit('update:viewMode', 'list')" class="icon-btn" title="Back to list"><Star :size="14" /></button>
      <button v-if="viewMode === 'list'" @click="emit('openNewForm')" class="icon-btn" title="New Connection"><Plus :size="14" /></button>
      <button v-if="viewMode !== 'settings'" @click="emit('update:viewMode', 'settings')" class="icon-btn" title="Settings"><Settings :size="14" /></button>
    </div>
  </div>
</template>

<style scoped>
.header {
  padding: 12px;
  background: #2d2d2d;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #333;
}
.header-actions {
  display: flex;
  gap: 8px;
}
.title {
  font-weight: bold;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.icon-btn {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
}
.icon-btn:hover {
  color: #fff;
  background: #3c3c3c;
}
.icon-btn.danger:hover {
  color: #f48771;
}
.status-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-left: 8px;
}
</style>
