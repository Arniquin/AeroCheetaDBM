<script setup lang="ts">
import { Table, FileCode, Square } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();

defineProps<{
  activeResultTab: any;
}>();
</script>

<template>
  <div class="tab-content">
    <div class="structure-header">
      <div class="table-info">
        <Table :size="16" /> {{ activeResultTab.schema ? activeResultTab.schema + '.' : '' }}{{ activeResultTab.table }}
      </div>
      <div class="structure-actions">
        <button @click="dbStore.generateTableScript(false)" class="action-btn">
          <FileCode :size="14" /> Generate DDL
        </button>
        <button @click="dbStore.generateTableScript(true)" class="action-btn">
          <FileCode :size="14" /> DDL + Data
        </button>
      </div>
    </div>
    
    <div class="table-container">
      <table>
        <thead>
          <tr>
            <th class="select-all-cell">
              <Square :size="12" />
            </th>
            <th>Column Name</th>
            <th>Type</th>
            <th>Nullable</th>
            <th>Default</th>
            <th>Max Length</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(col, i) in dbStore.tableStructure" :key="col.name">
            <td class="row-number-cell">{{ i + 1 }}</td>
            <td class="bold">{{ col.name }}</td>
            <td class="type-cell">{{ col.data_type }}</td>
            <td>{{ col.is_nullable ? 'Yes' : 'No' }}</td>
            <td class="default-cell">{{ col.default_value || '-' }}</td>
            <td>{{ col.max_length || '-' }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.tab-content {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.structure-header {
  padding: 12px 16px;
  background: #2d2d2d;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #333;
}
.table-info {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: bold;
  font-size: 14px;
  color: #9cdcfe;
}
.structure-actions {
  display: flex;
  gap: 8px;
}
.action-btn {
  background: #3c3c3c;
  border: 1px solid #444;
  color: #ccc;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
}
.action-btn:hover {
  background: #444;
  color: #fff;
}
.table-container {
  flex: 1;
  overflow: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
  color: #ccc;
}
th {
  position: sticky;
  top: 0;
  background: #2d2d2d;
  text-align: left;
  padding: 8px;
  border-bottom: 1px solid #444;
  border-right: 1px solid #444;
  z-index: 10;
  font-size: 11px;
  color: #888;
  text-transform: uppercase;
  user-select: none;
}
.select-all-cell {
  width: 30px;
  min-width: 30px;
  text-align: center;
  background: #2d2d2d;
  cursor: pointer;
  color: #555;
  padding: 4px;
}
.row-number-cell {
  width: 30px;
  text-align: center;
  color: #555;
  font-size: 10px;
  background: #252526;
  border-right: 1px solid #333;
}
td {
  padding: 4px 8px;
  border-bottom: 1px solid #333;
  border-right: 1px solid #333;
  white-space: nowrap;
}
.bold {
  font-weight: bold;
  color: #fff;
}
.type-cell {
  color: #ce9178;
  font-family: monospace;
}
.default-cell {
  color: #b5cea8;
}
</style>
