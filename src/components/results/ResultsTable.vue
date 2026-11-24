<script setup lang="ts">
import { computed } from 'vue';
import { Square } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();

const displayedRows = computed(() => {
  if (!dbStore.queryResult) return [];
  return dbStore.queryResult.rows.slice(0, dbStore.resultLimit);
});

function copyValue(val: any) {
  const text = typeof val === 'object' ? JSON.stringify(val) : String(val);
  navigator.clipboard.writeText(text);
}

const emit = defineEmits(['headerContextMenu', 'copyFull']);
</script>

<template>
  <div v-if="dbStore.queryResult" class="table-container">
    <table v-if="dbStore.queryResult.columns.length > 0">
      <thead>
        <tr>
          <th class="select-all-cell" @click="emit('copyFull')" title="Select all & copy with headers">
            <Square :size="12" fill="currentColor" />
          </th>
          <th v-for="col in dbStore.queryResult.columns" :key="col" @contextmenu="emit('headerContextMenu', $event, col)">
            {{ col }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, i) in displayedRows" :key="i">
          <td class="row-number-cell">{{ i + 1 }}</td>
          <td v-for="col in dbStore.queryResult.columns" :key="col" @dblclick="copyValue(row[col])" title="Double-click to copy">
            {{ typeof row[col] === 'object' ? JSON.stringify(row[col]) : row[col] }}
          </td>
        </tr>
      </tbody>
    </table>
    <div v-else class="no-data">Query executed successfully, no data returned.</div>
  </div>
  <div v-else-if="dbStore.loading" class="loading-state">
    Executing query...
  </div>
  <div v-else class="no-results">
    Run a query to see results here.
  </div>
</template>

<style scoped>
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
th:hover {
  background: #3c3c3c;
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
.select-all-cell:hover {
  background: #007acc;
  color: #fff;
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
tr:hover {
  background: #2a2d2e;
}
.no-results, .loading-state, .no-data {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #555;
  font-size: 14px;
}
</style>
