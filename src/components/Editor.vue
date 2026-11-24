<script setup lang="ts">
import { ref, computed } from 'vue';
import { format } from 'sql-formatter';
import { useDbStore } from '../stores/db';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeTextFile, readTextFile, mkdir } from '@tauri-apps/plugin-fs';
import EditorTabs from './editor/EditorTabs.vue';
import EditorToolbar from './editor/EditorToolbar.vue';
import MonacoEditor from './editor/MonacoEditor.vue';

const dbStore = useDbStore();
const monacoEditorRef = ref<any>(null);

const activeFile = computed(() => 
  dbStore.files.find(f => f.id === dbStore.activeFileId) || dbStore.files[0]
);

function execute() {
  const selectedText = monacoEditorRef.value?.getSelection();
  const queryToExecute = selectedText || monacoEditorRef.value?.getValue() || '';
  if (queryToExecute.trim()) {
    dbStore.executeQuery(queryToExecute);
  }
}

function formatQuery() {
  if (!monacoEditorRef.value) return;
  try {
    const formatted = format(monacoEditorRef.value.getValue(), { language: 'sql', keywordCase: 'upper' });
    monacoEditorRef.value.setValue(formatted);
  } catch (e) {
    console.error('Formatting failed', e);
  }
}

async function saveToFile() {
  if (!monacoEditorRef.value) return;
  try {
    let defaultPath = (activeFile.value?.name || 'query') + '.sql';
    
    if (dbStore.defaultWorkspace && dbStore.currentAlias) {
      const workspaceSubfolder = `${dbStore.defaultWorkspace}/${dbStore.currentAlias}`;
      try {
        await mkdir(workspaceSubfolder, { recursive: true });
        defaultPath = `${workspaceSubfolder}/${activeFile.value?.name}.sql`;
      } catch (e) {
        console.warn('Failed to create workspace subfolder, falling back to default save', e);
      }
    }

    const path = await save({
      filters: [{ name: 'SQL', extensions: ['sql'] }],
      defaultPath
    });
    if (path) {
      await writeTextFile(path, monacoEditorRef.value.getValue());
      if (dbStore.activeFileId) {
        const fileName = path.split('/').pop() || 'Query';
        dbStore.renameFile(dbStore.activeFileId, fileName);
        dbStore.setFileDirty(dbStore.activeFileId, false);
      }
    }
  } catch (e) {
    console.error('Save failed', e);
  }
}

async function openFile() {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: 'SQL', extensions: ['sql'] }]
    });
    if (path && typeof path === 'string') {
      const content = await readTextFile(path);
      const name = path.split('/').pop() || 'Opened Query';
      dbStore.addFile(name, content, false);
    }
  } catch (e) {
    console.error('Open failed', e);
  }
}

function updateContent(content: string) {
  if (dbStore.activeFileId) {
    dbStore.updateFileContent(dbStore.activeFileId, content);
  }
}

function closeActiveFile() {
  if (dbStore.activeFileId) dbStore.removeFile(dbStore.activeFileId);
}
</script>

<template>
  <div class="editor-wrapper">
    <EditorTabs />
    
    <EditorToolbar 
      @execute="execute"
      @format="formatQuery"
      @open="openFile"
      @save="saveToFile"
    />
    
    <MonacoEditor 
      ref="monacoEditorRef"
      :value="activeFile?.content || ''"
      @update:value="updateContent"
      @execute="execute"
      @format="formatQuery"
      @save="saveToFile"
      @new="dbStore.addFile()"
      @close="closeActiveFile"
    />
  </div>
</template>

<style scoped>
.editor-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1e1e1e;
}
</style>
