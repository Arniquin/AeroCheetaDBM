import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';

export interface ConnectionInfo {
  db_type: 'postgres' | 'mysql' | 'sqlserver' | 'mongodb' | 'mock';
  host: string;
  port: number;
  user?: string;
  password?: string;
  database?: string;
}

export interface TableInfo {
  name: string;
  schema?: string;
}

export interface ColumnInfo {
  name: string;
  data_type: string;
}

export interface QueryResult {
  columns: string[];
  rows: any[];
  execution_time_ms: number;
}

export interface SavedConnection {
  id: string;
  alias: string;
  info: ConnectionInfo | string;
}

export interface QueryFile {
  id: string;
  name: string;
  content: string;
  isDirty?: boolean;
}

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

const DEFAULT_FILE: QueryFile = {
  id: 'default-query',
  name: 'New Query',
  content: 'SELECT * FROM users LIMIT 10;',
  isDirty: false
};

const DEMO_CONN: SavedConnection = {
  id: 'demo-db',
  alias: 'Demo Database (Mock)',
  info: {
    db_type: 'mock',
    host: 'localhost',
    port: 0,
    database: 'demo_db'
  }
};

export const useDbStore = defineStore('db', {
  state: () => ({
    currentConnection: null as ConnectionInfo | null,
    currentAlias: null as string | null,
    connectionString: '' as string,
    connectionStatus: 'disconnected' as ConnectionStatus,
    defaultWorkspace: null as string | null,
    connectionTimeout: 3600000 as number, // Default to 1 hour
    lastActionTimestamp: 0 as number,
    timeoutCheckInterval: null as any | null,
    savedConnections: [] as SavedConnection[],
    files: [] as QueryFile[],
    activeFileId: null as string | null,
    tables: [] as TableInfo[],
    selectedTable: null as string | null,
    columns: {} as Record<string, ColumnInfo[]>,
    queryResult: null as QueryResult | null,
    loading: false,
    error: null as string | null,
  }),
  actions: {
    init() {
      this.loadSavedConnections();
      this.loadFiles();
      this.loadSettings();
      this.startTimeoutMonitor();
    },
    updateLastAction() {
      this.lastActionTimestamp = Date.now();
    },
    startTimeoutMonitor() {
      if (this.timeoutCheckInterval) return;
      
      this.timeoutCheckInterval = setInterval(() => {
        if (this.connectionStatus === 'connected' && this.connectionTimeout > 0) {
          const idleTime = Date.now() - this.lastActionTimestamp;
          if (idleTime >= this.connectionTimeout) {
            this.disconnect();
            this.error = 'Connection closed due to inactivity timeout.';
          }
        }
      }, 30000); // Check every 30 seconds
    },
    async connect(info: ConnectionInfo | string, alias: string | null = null) {
      this.connectionStatus = 'connecting';
      this.loading = true;
      this.error = null;
      try {
        if (typeof info === 'string') {
          await invoke('execute_query', { info, query: 'SELECT 1' });
          this.connectionString = info;
          this.currentConnection = null;
        } else {
          await invoke('test_connection', { info });
          this.currentConnection = info;
          this.connectionString = '';
        }
        this.currentAlias = alias || (typeof info === 'object' ? info.database : null) || 'default';
        this.connectionStatus = 'connected';
        this.updateLastAction();
        await this.fetchTables();
      } catch (e: any) {
        this.connectionStatus = 'error';
        this.error = e.toString();
        throw e;
      } finally {
        this.loading = false;
      }
    },
    async disconnect() {
      this.currentConnection = null;
      this.currentAlias = null;
      this.connectionString = '';
      this.connectionStatus = 'disconnected';
      this.tables = [];
      this.columns = {};
      this.queryResult = null;
    },
    saveConnection(alias: string, info: ConnectionInfo | string) {
      const id = crypto.randomUUID();
      this.savedConnections.push({ id, alias, info });
      this.persistConnections();
    },
    updateConnection(id: string, alias: string, info: ConnectionInfo | string) {
      const index = this.savedConnections.findIndex(c => c.id === id);
      if (index !== -1) {
        this.savedConnections[index] = { id, alias, info };
        this.persistConnections();
      }
    },
    deleteConnection(id: string) {
      this.savedConnections = this.savedConnections.filter(c => c.id !== id);
      this.persistConnections();
      if (id === 'demo-db') {
        localStorage.setItem('dbm_has_deleted_demo', 'true');
      }
    },
    persistConnections() {
      localStorage.setItem('dbm_saved_connections', JSON.stringify(this.savedConnections));
    },
    loadSavedConnections() {
      const saved = localStorage.getItem('dbm_saved_connections');
      const hasDeletedDemo = localStorage.getItem('dbm_has_deleted_demo');
      if (saved) {
        this.savedConnections = JSON.parse(saved);
      } else if (!hasDeletedDemo) {
        this.savedConnections = [DEMO_CONN];
        this.persistConnections();
      }
    },
    addFile(name = 'New Query', content = '', isDirty = false) {
      const id = crypto.randomUUID();
      const newFile = { id, name, content, isDirty };
      this.files.push(newFile);
      this.activeFileId = id;
      this.persistFiles();
    },
    removeFile(id: string) {
      const index = this.files.findIndex(f => f.id === id);
      if (index === -1) return;
      this.files.splice(index, 1);
      if (this.activeFileId === id) {
        this.activeFileId = this.files.length > 0 ? this.files[this.files.length - 1].id : null;
      }
      if (this.files.length === 0) {
        this.addFile();
      }
      this.persistFiles();
    },
    updateFileContent(id: string, content: string) {
      const file = this.files.find(f => f.id === id);
      if (file && file.content !== content) {
        file.content = content;
        file.isDirty = true;
        this.persistFiles();
      }
    },
    setFileDirty(id: string, isDirty: boolean) {
      const file = this.files.find(f => f.id === id);
      if (file) {
        file.isDirty = isDirty;
        this.persistFiles();
      }
    },
    renameFile(id: string, name: string) {
      const file = this.files.find(f => f.id === id);
      if (file) {
        file.name = name;
        this.persistFiles();
      }
    },
    persistFiles() {
      localStorage.setItem('dbm_query_files', JSON.stringify(this.files));
      localStorage.setItem('dbm_active_file_id', this.activeFileId || '');
    },
    loadFiles() {
      const savedFiles = localStorage.getItem('dbm_query_files');
      const savedActiveId = localStorage.getItem('dbm_active_file_id');
      if (savedFiles) {
        this.files = JSON.parse(savedFiles);
        this.activeFileId = savedActiveId || (this.files.length > 0 ? this.files[0].id : null);
      }
      if (this.files.length === 0) {
        this.files = [{ ...DEFAULT_FILE }];
        this.activeFileId = DEFAULT_FILE.id;
        this.persistFiles();
      }
    },
    setWorkspace(path: string | null) {
      this.defaultWorkspace = path;
      this.persistSettings();
    },
    persistSettings() {
      localStorage.setItem('dbm_settings', JSON.stringify({
        defaultWorkspace: this.defaultWorkspace,
        connectionTimeout: this.connectionTimeout
      }));
    },
    loadSettings() {
      const settings = localStorage.getItem('dbm_settings');
      if (settings) {
        const parsed = JSON.parse(settings);
        this.defaultWorkspace = parsed.defaultWorkspace;
        this.connectionTimeout = parsed.connectionTimeout !== undefined ? parsed.connectionTimeout : 3600000;
      }
    },
    async fetchTables() {
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      try {
        this.tables = await invoke('get_tables', { info });
        // Automatically fetch columns for all tables to populate autocomplete
        for (const table of this.tables) {
          this.fetchColumns(table.name, table.schema);
        }
      } catch (e: any) {
        this.error = e.toString();
      }
    },
    async fetchColumns(table: string, schema?: string) {
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      try {
        const columns = await invoke<ColumnInfo[]>('get_columns', { info, table, schema });
        this.columns[table] = columns;
        this.selectedTable = table;
      } catch (e: any) {
        this.error = e.toString();
      }
    },
    async executeQuery(query: string) {
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      this.loading = true;
      this.queryResult = null;
      this.error = null;
      try {
        this.queryResult = await invoke('execute_query', { info, query });
      } catch (e: any) {
        this.error = e.toString();
      } finally {
        this.loading = false;
      }
    }
  }
});
