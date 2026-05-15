import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';

export interface ConnectionInfo {
  db_type: 'postgres' | 'mysql' | 'sqlserver' | 'mongodb' | 'mock';
  host: string;
  port: number;
  user?: string;
  password?: string;
  database?: string;
  use_entra_id?: boolean;
  trust_server_certificate?: boolean;
  connect_timeout?: number;
}

export interface TableInfo {
  name: string;
  schema?: string;
  database?: string;
}

export interface ColumnInfo {
  name: string;
  data_type: string;
  is_nullable?: boolean;
  default_value?: string;
  max_length?: number;
}

export interface QueryResult {
  columns: string[];
  rows: any[];
  execution_time_ms: number;
}

export interface ResultTab {
  id: string;
  name: string;
  type: 'results' | 'structure';
  data: QueryResult | ColumnInfo[];
  pinned: boolean;
  table?: string;
  schema?: string;
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
    connectTimeout: 30 as number, // Default connection timeout in seconds
    resultLimit: 400 as number, // Default row limit
    lastActionTimestamp: 0 as number,
    timeoutCheckInterval: null as any | null,
    savedConnections: [] as SavedConnection[],
    files: [] as QueryFile[],
    activeFileId: null as string | null,
    tables: [] as TableInfo[],
    databases: [] as string[],
    selectedTable: null as string | null,
    selectedTableSchema: null as string | null,
    columns: {} as Record<string, ColumnInfo[]>,
    
    // Updated Results State
    resultTabs: [] as ResultTab[],
    activeResultTabId: null as string | null,
    
    loading: false,
    error: null as string | null,
  }),
  getters: {
    activeResultTab: (state) => state.resultTabs.find(t => t.id === state.activeResultTabId),
    queryResult: (state) => {
      const active = state.resultTabs.find(t => t.id === state.activeResultTabId);
      return active && active.type === 'results' ? active.data as QueryResult : null;
    },
    tableStructure: (state) => {
      const active = state.resultTabs.find(t => t.id === state.activeResultTabId);
      return active && active.type === 'structure' ? active.data as ColumnInfo[] : [];
    }
  },
  actions: {
    async viewTableStructure(table: string, schema?: string) {
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      this.loading = true;
      this.error = null;
      try {
        const structure = await invoke<ColumnInfo[]>('get_columns', { info, table, schema });
        this.selectedTable = table;
        this.selectedTableSchema = schema || null;
        
        // Find existing structure tab for this table or create new
        const existingId = `structure-${schema || 'default'}-${table}`;
        const existing = this.resultTabs.find(t => t.id === existingId);
        
        if (existing) {
          existing.data = structure;
          this.activeResultTabId = existingId;
        } else {
          const newTab: ResultTab = {
            id: existingId,
            name: `Structure: ${table}`,
            type: 'structure',
            data: structure,
            pinned: false,
            table,
            schema
          };
          this.resultTabs.push(newTab);
          this.activeResultTabId = newTab.id;
        }
      } catch (e: any) {
        this.error = e.toString();
      } finally {
        this.loading = false;
      }
    },

    closeResultTab(id: string) {
      const index = this.resultTabs.findIndex(t => t.id === id);
      if (index === -1) return;
      
      this.resultTabs.splice(index, 1);
      if (this.activeResultTabId === id) {
        this.activeResultTabId = this.resultTabs.length > 0 ? this.resultTabs[this.resultTabs.length - 1].id : null;
      }
    },

    togglePinResultTab(id: string) {
      const tab = this.resultTabs.find(t => t.id === id);
      if (tab) {
        tab.pinned = !tab.pinned;
      }
    },

    async generateTableScript(includeRows: boolean) {
      if (!this.selectedTable) return;
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      this.loading = true;
      this.error = null;
      try {
        const columns = await invoke<ColumnInfo[]>('get_columns', { 
          info, 
          table: this.selectedTable, 
          schema: this.selectedTableSchema 
        });

        let script = '';
        const schema = this.selectedTableSchema || (typeof info === 'object' && info.db_type === 'sqlserver' ? 'dbo' : 'public');
        const dbType = typeof info === 'object' ? info.db_type : (info.includes('sqlserver') ? 'sqlserver' : 'postgres');

        // Generate CREATE TABLE (DDL)
        if (dbType === 'sqlserver') {
          script += `CREATE TABLE [${schema}].[${this.selectedTable}] (\n`;
          const colDefs = columns.map(col => {
            let def = `    [${col.name}] ${col.data_type}`;
            if (col.max_length) {
              def += col.max_length === -1 ? '(MAX)' : `(${col.max_length})`;
            }
            if (!col.is_nullable) def += ' NOT NULL';
            if (col.default_value) def += ` DEFAULT ${col.default_value}`;
            return def;
          });
          script += colDefs.join(',\n') + '\n);\n\n';
        } else {
          script += `CREATE TABLE ${schema}.${this.selectedTable} (\n`;
          const colDefs = columns.map(col => {
            let def = `    ${col.name} ${col.data_type}`;
            if (col.max_length) def += `(${col.max_length})`;
            if (!col.is_nullable) def += ' NOT NULL';
            if (col.default_value) def += ` DEFAULT ${col.default_value}`;
            return def;
          });
          script += colDefs.join(',\n') + '\n);\n\n';
        }

        // Generate INSERTs (Data)
        if (includeRows) {
          const tableRef = dbType === 'sqlserver' ? `[${schema}].[${this.selectedTable}]` : `${schema}.${this.selectedTable}`;
          const data = await invoke<QueryResult>('execute_query', { info, query: `SELECT * FROM ${tableRef}` });
          
          for (const row of data.rows) {
            const names = data.columns.map(n => dbType === 'sqlserver' ? `[${n}]` : n).join(', ');
            const values = data.columns.map(col => {
              const val = row[col];
              if (val === null) return 'NULL';
              if (typeof val === 'string') return `'${val.replace(/'/g, "''")}'`;
              return val;
            }).join(', ');
            script += `INSERT INTO ${tableRef} (${names}) VALUES (${values});\n`;
          }
        }

        this.addFile(`${this.selectedTable}_script.sql`, script);
      } catch (e: any) {
        this.error = e.toString();
      } finally {
        this.loading = false;
      }
    },
    
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
          const connectionInfo = { ...info, connect_timeout: this.connectTimeout };
          await invoke('test_connection', { info: connectionInfo });
          this.currentConnection = connectionInfo;
          this.connectionString = '';
        }
        this.currentAlias = alias || (typeof info === 'object' ? info.database : null) || 'default';
        this.connectionStatus = 'connected';
        this.updateLastAction();
        await Promise.all([
          this.fetchTables(),
          this.fetchDatabases()
        ]);
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
      this.databases = [];
      this.columns = {};
      this.resultTabs = [];
      this.activeResultTabId = null;
    },
    async fetchDatabases() {
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      try {
        this.databases = await invoke('get_databases', { info });
      } catch (e: any) {
        console.error('Failed to fetch databases', e);
      }
    },
    async switchDatabase(dbName: string) {
      if (this.connectionString) {
        if (this.connectionString.toLowerCase().includes("initial catalog=")) {
            this.connectionString = this.connectionString.replace(/initial catalog=[^;]*/i, `Initial Catalog=${dbName}`);
        } else if (this.connectionString.toLowerCase().includes("database=")) {
            this.connectionString = this.connectionString.replace(/database=[^;]*/i, `Database=${dbName}`);
        } else if (this.connectionString.startsWith("postgres://") || this.connectionString.startsWith("mysql://") || this.connectionString.startsWith("mongodb://")) {
            const url = new URL(this.connectionString);
            url.pathname = `/${dbName}`;
            this.connectionString = url.toString();
        }
      } else if (this.currentConnection) {
        this.currentConnection.database = dbName;
      }
      
      this.loading = true;
      try {
        await this.fetchTables();
      } catch (e: any) {
        this.error = e.toString();
      } finally {
        this.loading = false;
      }
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
        connectionTimeout: this.connectionTimeout,
        connectTimeout: this.connectTimeout,
        resultLimit: this.resultLimit
      }));
    },
    loadSettings() {
      const settings = localStorage.getItem('dbm_settings');
      if (settings) {
        const parsed = JSON.parse(settings);
        this.defaultWorkspace = parsed.defaultWorkspace;
        this.connectionTimeout = parsed.connectionTimeout !== undefined ? parsed.connectionTimeout : 3600000;
        this.connectTimeout = parsed.connectTimeout !== undefined ? parsed.connectTimeout : 30;
        this.resultLimit = parsed.resultLimit !== undefined ? parsed.resultLimit : 400;
      }
    },
    async fetchTables() {
      this.updateLastAction();
      const info = this.currentConnection || this.connectionString;
      if (!info) return;
      try {
        this.tables = await invoke('get_tables', { info });
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
      this.error = null;
      try {
        const result = await invoke<QueryResult>('execute_query', { info, query });
        
        // Find existing unpinned result tab or create new
        let tab = this.resultTabs.find(t => t.type === 'results' && !t.pinned);
        
        if (tab) {
          tab.data = result;
          this.activeResultTabId = tab.id;
        } else {
          const newTab: ResultTab = {
            id: crypto.randomUUID(),
            name: `Results ${this.resultTabs.filter(t => t.type === 'results').length + 1}`,
            type: 'results',
            data: result,
            pinned: false
          };
          this.resultTabs.push(newTab);
          this.activeResultTabId = newTab.id;
        }
      } catch (e: any) {
        this.error = e.toString();
      } finally {
        this.loading = false;
      }
    }
  }
});
