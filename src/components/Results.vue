<script setup lang="ts">
import { ref } from 'vue';
import { useDbStore } from '../stores/db';
import ResultsHeader from './results/ResultsHeader.vue';
import ResultsTable from './results/ResultsTable.vue';
import StructureView from './results/StructureView.vue';
import ResultsContextMenu from './results/ResultsContextMenu.vue';

const dbStore = useDbStore();

const showCopyMenu = ref(false);
const copyMenuPos = ref({ x: 0, y: 0 });
const selectedColForCopy = ref<string | null>(null);

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
    <ResultsHeader />
    
    <div class="results-content">
      <div v-if="dbStore.activeResultTab?.type === 'results'" class="tab-content">
        <ResultsTable 
          @headerContextMenu="handleHeaderContextMenu"
          @copyFull="copyFullResults(true)"
        />
      </div>

      <StructureView 
        v-if="dbStore.activeResultTab?.type === 'structure'" 
        :activeResultTab="dbStore.activeResultTab"
      />
      
      <div v-if="dbStore.resultTabs.length === 0 && !dbStore.loading" class="no-results">
        Run a query or view table structure to see data here.
      </div>
      <div v-if="dbStore.loading && dbStore.resultTabs.length === 0" class="loading-state">
        Executing query...
      </div>
    </div>

    <ResultsContextMenu 
      v-if="showCopyMenu"
      :pos="copyMenuPos"
      :selectedCol="selectedColForCopy"
      @copyColumn="copyColumn"
      @copyFull="copyFullResults"
    />
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
.results-content {
  flex: 1;
  overflow: hidden;
}
.tab-content {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.no-results, .loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #555;
  font-size: 14px;
}
</style>
