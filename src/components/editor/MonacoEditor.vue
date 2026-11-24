<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import * as monaco from 'monaco-editor';
import { useDbStore } from '../../stores/db';

const props = defineProps<{
  value: string;
}>();

const emit = defineEmits(['update:value', 'execute', 'save', 'format', 'new', 'close']);

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

onMounted(() => {
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
      value: props.value,
      language: 'sql',
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      scrollBeyondLastLine: false,
      wordBasedSuggestions: 'off'
    });

    editor.onDidChangeModelContent(() => {
      emit('update:value', editor?.getValue() || '');
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => emit('execute'));
    editor.addCommand(monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF, () => emit('format'));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => emit('save'));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyN, () => emit('new'));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyW, () => emit('close'));
  }
});

onBeforeUnmount(() => {
  editor?.dispose();
  completionProvider?.dispose();
});

watch(() => props.value, (newVal) => {
  if (editor && newVal !== editor.getValue()) {
    editor.setValue(newVal);
  }
});

defineExpose({
  getValue: () => editor?.getValue() || '',
  setValue: (val: string) => editor?.setValue(val),
  getSelection: () => {
    const selection = editor?.getSelection();
    return selection ? editor?.getModel()?.getValueInRange(selection) : '';
  }
});
</script>

<template>
  <div ref="editorContainer" class="monaco-editor-container"></div>
</template>

<style scoped>
.monaco-editor-container {
  flex: 1;
  min-height: 0;
}
</style>
