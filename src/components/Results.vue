<script setup lang="ts">
import { useDbStore } from '../stores/db';
import { Clock, Info, Table, FileCode, Download, Clipboard, Pin, X, PinOff, Square } from 'lucide-vue-next';
import { ref, computed } from 'vue';

const dbStore = useDbStore();

const showCopyMenu = ref(false);
const copyMenuPos = ref({ x: 0, y: 0 });
const selectedColForCopy = ref<string | null>(null);

function copyValue(val: any) {
  const text = typeof val === 'object' ? JSON.stringify(val) : String(val);
  navigator.clipboard.writeText(text);
}

const displayedRows = computed(() => {
  if (!dbStore.queryResult) return [];
  return dbStore.queryResult.rows.slice(0, dbStore.resultLimit);
});

function handleHeaderContextMenu(e: MouseEvent, col: string) {
  e.preventDefault();
  selectedColForCopy.value = col;
  copyMenuPos.value = { x: e.clientX, y: e.clientY };
  showCopyMenu.value = true;
}

function handleTableContextMenu(e: MouseEvent) {
  if (e.target instanceof HTMLElement && e.target.tagName === 'TH') return;
  e.preventDefault();
  selectedColForCopy.value = null;
  copyMenuPos.value = { x: e.clientX, y: e.clientY };
  showCopyMenu.value = true;
}

function copyFullResults(includeHeaders: boolean) {
  if (!dbStore.queryResult) return;
  let text = '';
  if (includeHeaders) {
    text += dbStore.queryResult.columns.join('\t') + '\n';
  }
  text += dbStore.queryResult.rows.map(row => 
    dbStore.queryResult!.columns.map(col => row[col]).join('\t')
  ).join('\n');
  navigator.clipboard.writeText(text);
  showCopyMenu.value = false;
}

function copyColumn(includeHeaders: boolean) {
  if (!dbStore.queryResult || !selectedColForCopy.value) return;
  let text = '';
  if (includeHeaders) {
    text += selectedColForCopy.value + '\n';
  }
  text += dbStore.queryResult.rows.map(row => row[selectedColForCopy.value!]).join('\n');
  navigator.clipboard.writeText(text);
  showCopyMenu.value = false;
}

window.addEventListener('click', () => {
  showCopyMenu.value = false;
});
</script>

<template>
  <div class="results-panel" @contextmenu="handleTableContextMenu">
    <div class="results-header">
      <div class="tab-selectors">
        <div 
          v-for="tab in dbStore.resultTabs" 
          :key="tab.id"
          class="result-tab"
          :class="{ active: dbStore.activeResultTabId === tab.id, pinned: tab.pinned }"
          @click="dbStore.activeResultTabId = tab.id"
        >
          <component :is="tab.type === 'results' ? Clock : Info" :size="14" />
          <span class="tab-name">{{ tab.name }}</span>
          <div class="tab-actions">
            <button class="tab-action-btn" @click.stop="dbStore.togglePinResultTab(tab.id)" :title="tab.pinned ? 'Unpin' : 'Pin'">
              <Pin v-if="!tab.pinned" :size="10" />
              <PinOff v-else :size="10" />
            </button>
            <button class="tab-action-btn close" @click.stop="dbStore.closeResultTab(tab.id)" title="Close">
              <X :size="10" />
            </button>
          </div>
        </div>
      </div>
      
      <div class="header-info" v-if="dbStore.queryResult">
        {{ dbStore.queryResult.execution_time_ms }}ms | 
        {{ dbStore.queryResult.rows.length }} rows 
        <span v-if="dbStore.queryResult.rows.length > dbStore.resultLimit" class="limit-warning">
          (Showing first {{ dbStore.resultLimit }})
        </span>
      </div>
      <div class="error" v-if="dbStore.error">
        {{ dbStore.error }}
      </div>
    </div>
    
    <div class="results-content">
      <!-- QUERY RESULTS -->
      <div v-if="dbStore.activeResultTab?.type === 'results'" class="tab-content">
        <div v-if="dbStore.queryResult" class="table-container">
          <table v-if="dbStore.queryResult.columns.length > 0">
            <thead>
              <tr>
                <th class="select-all-cell" @click="copyFullResults(true)" title="Select all & copy with headers">
                  <Square :size="12" fill="currentColor" />
                </th>
                <th v-for="col in dbStore.queryResult.columns" :key="col" @contextmenu="handleHeaderContextMenu($event, col)">
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
      </div>

      <!-- STRUCTURE TAB CONTENT -->
      <div v-if="dbStore.activeResultTab?.type === 'structure'" class="tab-content">
        <div class="structure-header">
          <div class="table-info">
            <Table :size="16" /> {{ dbStore.activeResultTab.schema ? dbStore.activeResultTab.schema + '.' : '' }}{{ dbStore.activeResultTab.table }}
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
      
      <div v-if="dbStore.resultTabs.length === 0 && !dbStore.loading" class="no-results">
        Run a query or view table structure to see data here.
      </div>
      <div v-if="dbStore.loading && dbStore.resultTabs.length === 0" class="loading-state">
        Executing query...
      </div>
    </div>

    <!-- CONTEXT MENU -->
    <div v-if="showCopyMenu" class="context-menu" :style="{ top: copyMenuPos.y + 'px', left: copyMenuPos.x + 'px' }">
      <div v-if="selectedColForCopy" class="menu-section">
        <div class="menu-label">Column: {{ selectedColForCopy }}</div>
        <button @click="copyColumn(true)"><Clipboard :size="12" /> Copy with Header</button>
        <button @click="copyColumn(false)"><Clipboard :size="12" /> Copy without Header</button>
      </div>
      <div class="menu-section">
        <div class="menu-label">Full Results</div>
        <button @click="copyFullResults(true)"><Download :size="12" /> Copy All with Headers</button>
        <button @click="copyFullResults(false)"><Download :size="12" /> Copy All without Headers</button>
      </div>
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
  position: relative;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-right: 16px;
  background: #252526;
  border-top: 1px solid #333;
  border-bottom: 1px solid #333;
  min-height: 35px;
}

.tab-selectors {
  display: flex;
  height: 100%;
  overflow-x: auto;
}

.result-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  background: #2d2d2d;
  color: #888;
  border-right: 1px solid #333;
  cursor: pointer;
  font-size: 12px;
  height: 35px;
  min-width: 120px;
  max-width: 200px;
  position: relative;
}

.result-tab:hover {
  background: #333;
  color: #ccc;
}

.result-tab.active {
  background: #1e1e1e;
  color: #007acc;
  border-bottom: 2px solid #007acc;
}

.result-tab.pinned {
  border-top: 2px solid #ce9178;
}

.tab-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
}

.result-tab:hover .tab-actions, .result-tab.active .tab-actions {
  opacity: 1;
}

.tab-action-btn {
  background: transparent;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.tab-action-btn:hover {
  background: #3c3c3c;
  color: #fff;
}

.tab-action-btn.close:hover {
  background: #e81123;
}

.header-info {
  font-size: 11px;
  color: #666;
  white-space: nowrap;
}

.limit-warning {
  color: #ce9178;
  margin-left: 8px;
}

.error {
  color: #f48771;
  font-size: 12px;
  padding: 0 8px;
}

.results-content {
  flex: 1;
  overflow: hidden;
}

.tab-content {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.table-container {
  flex: 1;
  overflow: auto;
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

.no-results, .loading-state, .no-data {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #555;
  font-size: 14px;
}

.context-menu {
  position: fixed;
  background: #252526;
  border: 1px solid #444;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  z-index: 1000;
  border-radius: 4px;
  padding: 4px 0;
  min-width: 180px;
}

.menu-section {
  border-bottom: 1px solid #333;
  padding: 4px 0;
}

.menu-section:last-child {
  border-bottom: none;
}

.menu-label {
  padding: 4px 12px;
  font-size: 10px;
  color: #666;
  text-transform: uppercase;
  font-weight: bold;
}

.context-menu button {
  width: 100%;
  padding: 6px 12px;
  background: transparent;
  border: none;
  color: #ccc;
  text-align: left;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
}

.context-menu button:hover {
  background: #094771;
  color: #fff;
}
</style>
