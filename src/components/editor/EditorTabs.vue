<script setup lang="ts">
import { FileCode, X, Plus } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
</script>

<template>
  <div class="tabs-bar">
    <div 
      v-for="file in dbStore.files" 
      :key="file.id" 
      class="tab"
      :class="{ active: dbStore.activeFileId === file.id, dirty: file.isDirty }"
      @click="dbStore.activeFileId = file.id"
    >
      <FileCode :size="14" />
      <span class="tab-name">{{ file.name }}</span>
      <span v-if="file.isDirty" class="dirty-dot"></span>
      <button class="close-tab" @click.stop="dbStore.removeFile(file.id)"><X :size="12" /></button>
    </div>
    <button class="add-tab" @click="dbStore.addFile()" title="New Tab (Ctrl+N)"><Plus :size="16" /></button>
  </div>
</template>

<style scoped>
.tabs-bar {
  display: flex;
  background: #252526;
  height: 35px;
  overflow-x: auto;
  overflow-y: hidden;
}
.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  background: #2d2d2d;
  color: #969696;
  border-right: 1px solid #252526;
  cursor: pointer;
  font-size: 12px;
  min-width: 100px;
  max-width: 200px;
  position: relative;
}
.tab.active {
  background: #1e1e1e;
  color: #fff;
}
.tab.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: #007acc;
}
.tab-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dirty-dot {
  width: 8px;
  height: 8px;
  background-color: #fff;
  border-radius: 50%;
  margin: 0 4px;
  flex-shrink: 0;
}
.close-tab {
  background: transparent;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  opacity: 0;
}
.tab:hover .close-tab {
  opacity: 1;
}
.close-tab:hover {
  background: #3c3c3c;
}
.add-tab {
  background: transparent;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 0 10px;
}
.add-tab:hover {
  background: #3c3c3c;
}
</style>
