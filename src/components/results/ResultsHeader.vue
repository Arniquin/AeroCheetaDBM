<script setup lang="ts">
import { Clock, Info, Pin, PinOff, X } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
</script>

<template>
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
</template>

<style scoped>
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
</style>
