<script setup lang="ts">
import { computed } from 'vue';
import { Database, Play, Save } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();

const currentConnectionId = computed(() => {
  if (dbStore.connectionStatus !== 'connected') return '';
  
  if (dbStore.currentAlias) {
    const byAlias = dbStore.savedConnections.find(c => c.alias === dbStore.currentAlias);
    if (byAlias) return byAlias.id;
  }

  const active = dbStore.savedConnections.find(c => {
    if (typeof c.info === 'string') {
      return c.info === dbStore.connectionString;
    }
    
    const curr = dbStore.currentConnection;
    if (!curr || typeof c.info !== 'object') return false;
    
    return c.info.db_type === curr.db_type &&
           c.info.host === curr.host &&
           c.info.port === curr.port &&
           c.info.user === curr.user &&
           c.info.database === curr.database;
  });

  return active ? active.id : 'temp';
});

async function handleConnectionChange(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  const conn = dbStore.savedConnections.find(c => c.id === id);
  if (conn) {
    try {
      await dbStore.connect(conn.info, conn.alias);
    } catch (e) {
      console.error('Connection failed', e);
    }
  }
}

const emit = defineEmits(['execute', 'format', 'open', 'save']);
</script>

<template>
  <div class="toolbar">
    <div class="left-tools">
      <div class="connection-selector">
        <Database :size="14" />
        <select :value="currentConnectionId" @change="handleConnectionChange" class="conn-select">
          <option value="" disabled>No Connection</option>
          <option v-if="currentConnectionId === 'temp'" value="temp">Active (Unsaved)</option>
          <option v-for="conn in dbStore.savedConnections" :key="conn.id" :value="conn.id">
            {{ conn.alias }}
          </option>
        </select>
      </div>
      <button @click="emit('execute')" class="run-btn" title="Run (Ctrl+Enter)" :disabled="!dbStore.connectionStatus || dbStore.connectionStatus !== 'connected'">
        <Play :size="14" fill="currentColor" /> Run
      </button>
      <button @click="emit('format')" title="Format (Shift+Alt+F)">Format</button>
    </div>
    <div class="right-tools">
      <button @click="emit('open')" title="Open File">Open</button>
      <button @click="emit('save')" title="Save File (Ctrl+S)">
        <Save :size="14" /> Save
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  padding: 6px 12px;
  background: #1e1e1e;
  border-bottom: 1px solid #333;
  display: flex;
  justify-content: space-between;
  gap: 8px;
}
.left-tools, .right-tools {
  display: flex;
  gap: 8px;
  align-items: center;
}
.connection-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #2d2d2d;
  padding: 0 8px;
  border-radius: 4px;
  border: 1px solid #444;
  color: #888;
}
.conn-select {
  background: transparent;
  border: none;
  color: #ccc;
  font-size: 12px;
  padding: 4px 0;
  cursor: pointer;
  outline: none;
  max-width: 150px;
}
.conn-select option {
  background: #252526;
  color: #fff;
}
button {
  background: #333;
  color: #ccc;
  border: 1px solid #444;
  padding: 3px 10px;
  cursor: pointer;
  border-radius: 4px;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}
button:hover {
  background: #444;
  color: #fff;
}
.run-btn {
  background: #2d4a2e;
  color: #4ec9b0;
  border-color: #3e5a3e;
}
.run-btn:hover {
  background: #3e5a3e;
}
</style>
