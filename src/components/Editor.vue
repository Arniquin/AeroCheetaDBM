<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue';
import * as monaco from 'monaco-editor';
import { format } from 'sql-formatter';
import { useDbStore } from '../stores/db';
import { Plus, X, Save, FileCode, Play, Database } from 'lucide-vue-next';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeTextFile, readTextFile, mkdir } from '@tauri-apps/plugin-fs';

const dbStore = useDbStore();
const editorContainer = ref<HTMLElement | null>(null);
let editor: monaco.editor.IStandaloneCodeEditor | null = null;
let completionProvider: monaco.IDisposable | null = null;

const SQL_KEYWORDS = [
  'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'LIMIT', 'INSERT', 'UPDATE', 'DELETE',
  'CREATE', 'DROP', 'ALTER', 'TABLE', 'INDEX', 'VIEW', 'JOIN', 'LEFT', 'RIGHT',
  'INNER', 'OUTER', 'ON', 'GROUP', 'BY', 'ORDER', 'HAVING', 'DISTINCT', 'COUNT',
  'SUM', 'AVG', 'MIN', 'MAX', 'AS', 'IN', 'IS', 'NOT', 'NULL', 'LIKE', 'BETWEEN',
  'EXISTS', 'ALL', 'ANY', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END', 'UNION'
];

const activeFile = computed(() => 
  dbStore.files.find(f => f.id === dbStore.activeFileId) || dbStore.files[0]
);

const currentConnectionId = computed(() => {
  if (dbStore.connectionStatus !== 'connected') return '';
  
  // 1. Try matching by alias first (most reliable if using saved connections)
  if (dbStore.currentAlias) {
    const byAlias = dbStore.savedConnections.find(c => c.alias === dbStore.currentAlias);
    if (byAlias) return byAlias.id;
  }

  // 2. Try matching by core properties (fallback for desynced aliases)
  const active = dbStore.savedConnections.find(c => {
    if (typeof c.info === 'string') {
      return c.info === dbStore.connectionString;
    }
    
    const curr = dbStore.currentConnection;
    if (!curr || typeof c.info !== 'object') return false;
    
    // Compare core fields, ignoring connect_timeout which is injected at runtime
    return c.info.db_type === curr.db_type &&
           c.info.host === curr.host &&
           c.info.port === curr.port &&
           c.info.user === curr.user &&
           c.info.database === curr.database;
  });

  return active ? active.id : 'temp';
});

async function handleConnectionChange(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  const conn = dbStore.savedConnections.find(c => c.id === id);
  if (conn) {
    try {
      await dbStore.connect(conn.info, conn.alias);
    } catch (e) {
      console.error('Connection failed', e);
    }
  }
}

onMounted(() => {
  // Register SQL completion provider
  completionProvider = monaco.languages.registerCompletionItemProvider('sql', {
    provideCompletionItems: (model, position) => {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn
      };

      const suggestions: monaco.languages.CompletionItem[] = [
        ...SQL_KEYWORDS.map(kw => ({
          label: kw,
          kind: monaco.languages.CompletionItemKind.Keyword,
          insertText: kw,
          range
        })),
        ...dbStore.tables.map(table => ({
          label: table.name,
          kind: monaco.languages.CompletionItemKind.Class,
          insertText: table.name,
          detail: 'Table',
          range
        }))
      ];

      // Add columns for all tables
      Object.entries(dbStore.columns).forEach(([tableName, columns]) => {
        if (Array.isArray(columns)) {
          columns.forEach(col => {
            suggestions.push({
              label: col.name,
              kind: monaco.languages.CompletionItemKind.Field,
              insertText: col.name,
              detail: `Column (${tableName}) - ${col.data_type}`,
              range
            });
          });
        }
      });

      return { suggestions };
    }
  });

  if (editorContainer.value) {
    editor = monaco.editor.create(editorContainer.value, {
      value: activeFile.value?.content || '',
      language: 'sql',
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      scrollBeyondLastLine: false,
      wordBasedSuggestions: 'off' // We want our custom provider to take precedence
    });

    editor.onDidChangeModelContent(() => {
      if (dbStore.activeFileId) {
        dbStore.updateFileContent(dbStore.activeFileId, editor?.getValue() || '');
      }
    });

    // Shortcuts
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => execute());
    editor.addCommand(monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF, () => formatQuery());
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => saveToFile());
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyN, () => dbStore.addFile());
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyW, () => {
      if (dbStore.activeFileId) dbStore.removeFile(dbStore.activeFileId);
    });
  }
});

onBeforeUnmount(() => {
  editor?.dispose();
  completionProvider?.dispose();
});

watch(() => dbStore.activeFileId, (newId) => {
  const file = dbStore.files.find(f => f.id === newId);
  if (editor && file && file.content !== editor.getValue()) {
    editor.setValue(file.content);
  }
});

function execute() {
  const selection = editor?.getSelection();
  const selectedText = selection ? editor?.getModel()?.getValueInRange(selection) : '';
  const queryToExecute = selectedText || editor?.getValue() || '';
  if (queryToExecute.trim()) {
    dbStore.executeQuery(queryToExecute);
  }
}

function formatQuery() {
  if (!editor) return;
  try {
    const formatted = format(editor.getValue(), { language: 'sql', keywordCase: 'upper' });
    editor.setValue(formatted);
  } catch (e) {
    console.error('Formatting failed', e);
  }
}

async function saveToFile() {
  if (!editor) return;
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
      await writeTextFile(path, editor.getValue());
      if (dbStore.activeFileId) {
        // Update tab name to file name and mark as clean
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
</script>

<template>
  <div class="editor-wrapper">
    <div class="tabs-bar">
      <div 
        v-for="file in dbStore.files" 
        :key="file.id" 
        class="tab"
        :class="{ active: dbStore.activeFileId === file.id, dirty: file.isDirty }"
        @click="dbStore.activeFileId = file.id"
      >
        <FileCode :size="14" />
        <span class="tab-name">{{ file.name }}</span>
        <span v-if="file.isDirty" class="dirty-dot"></span>
        <button class="close-tab" @click.stop="dbStore.removeFile(file.id)"><X :size="12" /></button>
      </div>
      <button class="add-tab" @click="dbStore.addFile()" title="New Tab (Ctrl+N)"><Plus :size="16" /></button>
    </div>
    
    <div class="toolbar">
      <div class="left-tools">
        <div class="connection-selector">
          <Database :size="14" />
          <select :value="currentConnectionId" @change="handleConnectionChange" class="conn-select">
            <option value="" disabled>No Connection</option>
            <option v-if="currentConnectionId === 'temp'" value="temp">Active (Unsaved)</option>
            <option v-for="conn in dbStore.savedConnections" :key="conn.id" :value="conn.id">
              {{ conn.alias }}
            </option>
          </select>
        </div>
        <button @click="execute" class="run-btn" title="Run (Ctrl+Enter)" :disabled="!dbStore.connectionStatus || dbStore.connectionStatus !== 'connected'">
          <Play :size="14" fill="currentColor" /> Run
        </button>
        <button @click="formatQuery" title="Format (Shift+Alt+F)">Format</button>
      </div>
      <div class="right-tools">
        <button @click="openFile" title="Open File">Open</button>
        <button @click="saveToFile" title="Save File (Ctrl+S)">
          <Save :size="14" /> Save
        </button>
      </div>
    </div>
    
    <div ref="editorContainer" class="monaco-editor-container"></div>
  </div>
</template>

<style scoped>
.editor-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1e1e1e;
}

/* TABS BAR */
.tabs-bar {
  display: flex;
  background: #252526;
  height: 35px;
  overflow-x: auto;
  overflow-y: hidden;
}
.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  background: #2d2d2d;
  color: #969696;
  border-right: 1px solid #252526;
  cursor: pointer;
  font-size: 12px;
  min-width: 100px;
  max-width: 200px;
  position: relative;
}
.tab.active {
  background: #1e1e1e;
  color: #fff;
}
.tab.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: #007acc;
}
.tab-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dirty-dot {
  width: 8px;
  height: 8px;
  background-color: #fff;
  border-radius: 50%;
  margin: 0 4px;
  flex-shrink: 0;
}
.close-tab {
  background: transparent;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  opacity: 0;
}
.tab:hover .close-tab {
  opacity: 1;
}
.close-tab:hover {
  background: #3c3c3c;
}
.add-tab {
  background: transparent;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 0 10px;
}
.add-tab:hover {
  background: #3c3c3c;
}

/* TOOLBAR */
.toolbar {
  padding: 6px 12px;
  background: #1e1e1e;
  border-bottom: 1px solid #333;
  display: flex;
  justify-content: space-between;
  gap: 8px;
}
.left-tools, .right-tools {
  display: flex;
  gap: 8px;
  align-items: center;
}
.connection-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #2d2d2d;
  padding: 0 8px;
  border-radius: 4px;
  border: 1px solid #444;
  color: #888;
}
.conn-select {
  background: transparent;
  border: none;
  color: #ccc;
  font-size: 12px;
  padding: 4px 0;
  cursor: pointer;
  outline: none;
  max-width: 150px;
}
.conn-select option {
  background: #252526;
  color: #fff;
}
.monaco-editor-container {
  flex: 1;
  min-height: 0;
}
button {
  background: #333;
  color: #ccc;
  border: 1px solid #444;
  padding: 3px 10px;
  cursor: pointer;
  border-radius: 4px;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}
button:hover {
  background: #444;
  color: #fff;
}
.run-btn {
  background: #2d4a2e;
  color: #4ec9b0;
  border-color: #3e5a3e;
}
.run-btn:hover {
  background: #3e5a3e;
}
</style>
