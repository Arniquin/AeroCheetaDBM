<script setup lang="ts">
import Sidebar from './components/Sidebar.vue';
import Editor from './components/Editor.vue';
import Results from './components/Results.vue';
import { useDbStore } from './stores/db';

const dbStore = useDbStore();
dbStore.init();

async function handleExecute(query: string) {
  await dbStore.executeQuery(query);
}
</script>

<template>
  <div class="app-container">
    <Sidebar />
    <div class="main-content">
      <div class="editor-section">
        <Editor @execute="handleExecute" />
      </div>
      <div class="results-section">
        <Results />
      </div>
    </div>
  </div>
</template>

<style>
body, html, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background: #1e1e1e;
  color: #fff;
  overflow: hidden;
}

.app-container {
  display: flex;
  height: 100vh;
  width: 100vw;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.editor-section {
  flex: 3;
  min-height: 200px;
}

.results-section {
  flex: 2;
  border-top: 1px solid #333;
  min-height: 150px;
}

/* Global scrollbar styling */
::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
::-webkit-scrollbar-track {
  background: #1e1e1e;
}
::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 5px;
}
::-webkit-scrollbar-thumb:hover {
  background: #444;
}
</style>
