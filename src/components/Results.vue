<script setup lang="ts">
import { useDbStore } from '../stores/db';
import { Clock } from 'lucide-vue-next';

const dbStore = useDbStore();
</script>

<template>
  <div class="results-panel">
    <div class="results-header">
      <div class="stats" v-if="dbStore.queryResult">
        <Clock :size="14" /> {{ dbStore.queryResult.execution_time_ms }}ms | {{ dbStore.queryResult.rows.length }} rows
      </div>
      <div class="error" v-if="dbStore.error">
        {{ dbStore.error }}
      </div>
    </div>
    
    <div class="results-content" v-if="dbStore.queryResult">
      <table v-if="dbStore.queryResult.columns.length > 0">
        <thead>
          <tr>
            <th v-for="col in dbStore.queryResult.columns" :key="col">{{ col }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, i) in dbStore.queryResult.rows" :key="i">
            <td v-for="col in dbStore.queryResult.columns" :key="col">
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
  </div>
</template>

<style scoped>
.results-panel {
  height: 100%;
  background: #1e1e1e;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.results-header {
  padding: 8px 16px;
  background: #252526;
  border-top: 1px solid #333;
  border-bottom: 1px solid #333;
  font-size: 12px;
  color: #888;
  min-height: 32px;
}
.stats {
  display: flex;
  align-items: center;
  gap: 8px;
}
.error {
  color: #f48771;
}
.results-content {
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
}
td {
  padding: 6px 8px;
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
