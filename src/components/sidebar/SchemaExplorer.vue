<script setup lang="ts">
import { ref, computed } from 'vue';
import { Info, RefreshCcw, ChevronRight, ChevronDown, Database, Folder, Table, List, FileCode, Plus, Columns } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
const emit = defineEmits(['viewDetails']);

const collapsedGroups = ref<Record<string, boolean>>({});

const currentDatabase = computed({
  get: () => {
    if (dbStore.connectionString) {
      if (dbStore.connectionString.toLowerCase().includes("initial catalog=")) {
        return dbStore.connectionString.match(/initial catalog=([^;]*)/i)?.[1] || '';
      }
      if (dbStore.connectionString.toLowerCase().includes("database=")) {
        return dbStore.connectionString.match(/database=([^;]*)/i)?.[1] || '';
      }
      if (dbStore.connectionString.includes("://")) {
        try {
          return new URL(dbStore.connectionString).pathname.substring(1);
        } catch { return ''; }
      }
    }
    return dbStore.currentConnection?.database || '';
  },
  set: (val) => {
    dbStore.switchDatabase(val);
  }
});

const groupedTables = computed(() => {
  const groups: Record<string, Record<string, any[]>> = {};
  for (const table of dbStore.tables) {
    const db = table.database || 'default';
    const schema = table.schema || 'default';
    if (!groups[db]) groups[db] = {};
    if (!groups[db][schema]) groups[db][schema] = [];
    groups[db][schema].push(table);
  }
  return groups;
});

function toggleGroup(path: string) {
  collapsedGroups.value[path] = !collapsedGroups.value[path];
}

function isCollapsed(path: string) {
  return collapsedGroups.value[path] === true;
}
</script>

<template>
  <div class="schema-explorer">
    <div class="explorer-header">
      <select v-model="currentDatabase" class="db-selector" title="Switch Database">
        <option v-for="db in dbStore.databases" :key="db" :value="db">{{ db }}</option>
        <option v-if="!dbStore.databases.includes(currentDatabase)" :value="currentDatabase">{{ currentDatabase }}</option>
      </select>
      <div class="explorer-actions">
        <button @click="emit('viewDetails')" class="icon-btn" title="Connection Details"><Info :size="14" /></button>
        <button @click="dbStore.fetchTables()" class="icon-btn" title="Refresh Tables"><RefreshCcw :size="14" /></button>
      </div>
    </div>

    <div class="tables-list">
      <div v-for="(schemas, db) in groupedTables" :key="db" class="db-group">
        <div class="group-header db" @click="toggleGroup(db)">
          <component :is="isCollapsed(db) ? ChevronRight : ChevronDown" :size="14" />
          <Database :size="14" /> {{ db }}
        </div>
        
        <div v-if="!isCollapsed(db)" class="db-content">
          <div v-for="(tables, schema) in schemas" :key="schema" class="schema-group">
            <div class="group-header schema" @click="toggleGroup(db + '.' + schema)">
              <component :is="isCollapsed(db + '.' + schema) ? ChevronRight : ChevronDown" :size="14" />
              <Folder :size="14" /> {{ schema }}
            </div>
            
            <div v-if="!isCollapsed(db + '.' + schema)" class="schema-content">
              <div v-for="table in tables" :key="table.name" class="table-item">
                <div class="table-header" :class="{ active: dbStore.selectedTable === table.name }" @click="dbStore.fetchColumns(table.name, table.schema)">
                  <div class="table-name">
                    <Table :size="14" /> {{ table.name }}
                  </div>
                  <div class="table-actions">
                    <button @click.stop="dbStore.viewTableStructure(table.name, table.schema)" class="action-btn" title="View Structure"><List :size="12" /></button>
                    <button @click.stop="dbStore.generateTableScript(false)" class="action-btn" title="Generate Script"><FileCode :size="12" /></button>
                    <button @click.stop="dbStore.generateTableScript(true)" class="action-btn" title="Generate Script with Data"><Plus :size="12" /></button>
                  </div>
                </div>
                <div v-if="dbStore.selectedTable === table.name" class="columns-list">
                  <div v-for="col in dbStore.columns[table.name]" :key="col.name" class="column-item">
                    <Columns :size="12" /> {{ col.name }} <span class="type">{{ col.data_type }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.schema-explorer {
  padding: 8px;
}
.explorer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  gap: 8px;
  border-bottom: 1px solid #333;
  margin-bottom: 8px;
}
.db-selector {
  flex: 1;
  background: #1e1e1e;
  border: 1px solid #3c3c3c;
  color: #fff;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  width: 0;
  min-width: 0;
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
.db-group, .schema-group {
  margin-bottom: 2px;
}
.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  cursor: pointer;
  font-size: 12px;
  border-radius: 4px;
  user-select: none;
}
.group-header:hover {
  background: #2a2d2e;
}
.group-header.db {
  color: #ce9178;
  font-weight: bold;
}
.group-header.schema {
  color: #9cdcfe;
  margin-left: 12px;
}
.db-content, .schema-content {
  border-left: 1px solid #333;
  margin-left: 14px;
}
.table-item {
  margin-left: 12px;
}
.table-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 8px;
  cursor: pointer;
  font-size: 13px;
  border-radius: 4px;
  gap: 8px;
}
.table-name {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.table-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}
.table-header:hover .table-actions, .table-header.active .table-actions {
  opacity: 1;
}
.table-header:hover {
  background: #2a2d2e;
}
.table-header.active {
  background: #37373d;
  color: #007acc;
}
.columns-list {
  padding-left: 24px;
  margin-top: 4px;
  font-size: 12px;
  color: #888;
}
.column-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}
.type {
  font-size: 10px;
  color: #555;
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
</style>
