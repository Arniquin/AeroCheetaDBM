<script setup lang="ts">
import { ref } from 'vue';
import { useDbStore, type ConnectionInfo } from '../stores/db';
import SidebarHeader from './sidebar/SidebarHeader.vue';
import ExitView from './sidebar/ExitView.vue';
import SettingsView from './sidebar/SettingsView.vue';
import ConnectionForm from './sidebar/ConnectionForm.vue';
import SavedConnections from './sidebar/SavedConnections.vue';
import SchemaExplorer from './sidebar/SchemaExplorer.vue';
import ConnectionDetails from './sidebar/ConnectionDetails.vue';

const dbStore = useDbStore();

type ViewMode = 'form' | 'list' | 'explorer' | 'settings' | 'exit' | 'details';
const viewMode = ref<ViewMode>('list');

// Form state
const editingId = ref<string | null>(null);
const initialAlias = ref('');
const initialInfo = ref<ConnectionInfo | string>('');

function openNewConnectionForm() {
  editingId.value = null;
  initialAlias.value = '';
  initialInfo.value = {
    db_type: 'postgres',
    host: 'localhost',
    port: 5432,
    user: '',
    password: '',
    database: '',
    use_entra_id: false,
    trust_server_certificate: false
  };
  viewMode.value = 'form';
}

function editConnection(saved: any) {
  editingId.value = saved.id;
  initialAlias.value = saved.alias;
  initialInfo.value = saved.info;
  viewMode.value = 'form';
}

function handleDisconnect() {
  dbStore.disconnect();
  viewMode.value = 'list';
}
</script>

<template>
  <div class="sidebar">
    <SidebarHeader 
      v-model:viewMode="viewMode" 
      @openNewForm="openNewConnectionForm"
    />

    <ExitView 
      v-if="viewMode === 'exit'"
      @viewExplorer="viewMode = 'explorer'"
      @viewList="viewMode = 'list'"
      @disconnect="handleDisconnect"
    />

    <SettingsView 
      v-if="viewMode === 'settings'"
      @close="viewMode = 'list'"
    />

    <ConnectionForm 
      v-if="viewMode === 'form'"
      :editingId="editingId"
      :initialAlias="initialAlias"
      :initialInfo="initialInfo"
      @connected="viewMode = 'explorer'"
      @cancel="viewMode = 'list'"
    />

    <SavedConnections 
      v-else-if="viewMode === 'list'"
      @edit="editConnection"
      @connected="viewMode = 'explorer'"
    />

    <SchemaExplorer 
      v-else-if="viewMode === 'explorer'"
      @viewDetails="viewMode = 'details'"
    />

    <ConnectionDetails 
      v-else-if="viewMode === 'details'"
      @back="viewMode = 'explorer'"
    />
  </div>
</template>

<style scoped>
.sidebar {
  width: 260px;
  background: #252526;
  border-right: 1px solid #333;
  display: flex;
  flex-direction: column;
  color: #ccc;
  overflow-y: auto;
}
</style>
