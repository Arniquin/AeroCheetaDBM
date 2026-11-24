<script setup lang="ts">
import { Star, Pencil, Trash2 } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
const emit = defineEmits(['edit', 'connected']);

async function connectToSaved(saved: any) {
  try {
    const isCurrent = typeof saved.info === 'string' 
      ? dbStore.connectionString === saved.info 
      : JSON.stringify(dbStore.currentConnection) === JSON.stringify(saved.info);

    if (isCurrent && dbStore.connectionStatus === 'connected') {
      emit('connected');
      return;
    }

    await dbStore.connect(saved.info, saved.alias);
    emit('connected');
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <div class="saved-connections">
    <div class="section-title">Saved Connections</div>
    <div v-if="dbStore.savedConnections.length === 0" class="empty-msg">
      No saved connections. Click + to add one.
    </div>
    <div v-for="saved in dbStore.savedConnections" :key="saved.id" class="saved-item" :class="{ active: dbStore.currentConnection === saved.info || dbStore.connectionString === saved.info }">
      <div class="saved-info" @click="connectToSaved(saved)">
        <Star :size="14" :fill="saved.id === 'demo-db' ? '#ce9178' : 'none'" :stroke="saved.id === 'demo-db' ? '#ce9178' : 'currentColor'" />
        <div class="alias">{{ saved.alias }}</div>
      </div>
      <div class="item-actions">
        <button @click="emit('edit', saved)" class="action-btn" title="Edit connection"><Pencil :size="12" /></button>
        <button @click="dbStore.deleteConnection(saved.id)" class="action-btn delete" title="Remove saved connection"><Trash2 :size="12" /></button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.saved-connections {
  padding: 8px;
}
.section-title {
  padding: 8px;
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
}
.empty-msg {
  padding: 16px;
  font-size: 12px;
  color: #555;
  text-align: center;
}
.saved-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  border-radius: 4px;
  cursor: pointer;
}
.saved-item:hover {
  background: #2a2d2e;
}
.saved-item.active {
  background: #37373d;
  border-left: 2px solid #007acc;
}
.saved-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}
.alias {
  font-size: 13px;
}
.item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}
.saved-item:hover .item-actions {
  opacity: 1;
}
.action-btn {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
}
.action-btn:hover {
  color: #fff;
  background: #3c3c3c;
}
.action-btn.delete:hover {
  color: #f48771;
}
</style>
