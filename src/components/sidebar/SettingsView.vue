<script setup lang="ts">
import { Folder } from 'lucide-vue-next';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
const emit = defineEmits(['close']);

async function selectWorkspace() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === 'string') {
      dbStore.setWorkspace(selected);
    }
  } catch (e) {
    console.error('Failed to select workspace', e);
  }
}
</script>

<template>
  <div class="settings-view">
    <div class="section-title">Settings</div>
    <div class="setting-item">
      <label>Default Workspace</label>
      <div class="workspace-picker">
        <div class="path-display" :title="dbStore.defaultWorkspace || 'Not set'">
          {{ dbStore.defaultWorkspace || 'Select a folder...' }}
        </div>
        <button @click="selectWorkspace" class="icon-btn"><Folder :size="14" /></button>
      </div>
      <p class="help-text">Queries will be stored in subfolders named after DB aliases.</p>
    </div>
    <div class="setting-item mt-16">
      <label>Idle Connection Timeout</label>
      <select v-model="dbStore.connectionTimeout" @change="dbStore.persistSettings">
        <option :value="0">Never</option>
        <option :value="300000">5 minutes</option>
        <option :value="600000">10 minutes</option>
        <option :value="900000">15 minutes</option>
        <option :value="1800000">30 minutes</option>
        <option :value="3600000">1 hour</option>
        <option :value="10800000">3 hours</option>
        <option :value="36000000">10 hours</option>
        <option :value="86400000">24 hours</option>
        <option :value="172800000">48 hours</option>
      </select>
      <p class="help-text">Auto-disconnect after period of inactivity.</p>
    </div>
    <div class="setting-item mt-16">
      <label>Connection Timeout (seconds)</label>
      <input type="number" v-model.number="dbStore.connectTimeout" @change="dbStore.persistSettings" min="1" max="300" />
      <p class="help-text">Timeout for initial connection (Default: 30).</p>
    </div>
    <div class="setting-item mt-16">
      <label>Max Displayed Rows</label>
      <input type="number" v-model.number="dbStore.resultLimit" @change="dbStore.persistSettings" min="1" max="10000" />
      <p class="help-text">Limit shown rows in results (Default: 400).</p>
    </div>
    <button @click="emit('close')" class="secondary-btn mt-16">Close Settings</button>
  </div>
</template>

<style scoped>
.settings-view {
  padding: 16px;
}
.section-title {
  padding: 8px;
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
}
.setting-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.workspace-picker {
  display: flex;
  gap: 8px;
  align-items: center;
}
.path-display {
  flex: 1;
  background: #1e1e1e;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border: 1px solid #333;
}
.help-text {
  font-size: 10px;
  color: #666;
  margin-top: 4px;
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
input, select {
  background: #1e1e1e;
  border: 1px solid #3c3c3c;
  color: #fff;
  padding: 8px;
  border-radius: 4px;
  font-size: 13px;
  transition: border-color 0.2s;
}
select option {
  background: #252526;
  color: #fff;
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
.mt-16 {
  margin-top: 16px;
}
</style>
