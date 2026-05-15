<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useDbStore, type ConnectionInfo } from '../stores/db';
import { Database, Table, Columns, Plus, Trash2, Server, Star, RefreshCcw, ArrowLeft, LogOut, Settings, Folder, Pencil, Info, Copy, Check, Eye, EyeOff } from 'lucide-vue-next';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

const dbStore = useDbStore();

type ViewMode = 'form' | 'list' | 'explorer' | 'settings' | 'exit' | 'details';
const viewMode = ref<ViewMode>('list');
const editingId = ref<string | null>(null);
const copied = ref(false);
const showPassword = ref(false);

const generatedConnectionString = computed(() => {
  if (dbStore.connectionString) return dbStore.connectionString;
  const info = dbStore.currentConnection;
  if (!info) return '';

  const userPass = info.user ? `${info.user}:${info.password || '••••••••'}@` : '';
  const port = info.port ? `:${info.port}` : '';
  const db = info.database ? `/${info.database}` : '';
  
  let params = [];
  if (info.trust_server_certificate) {
    if (info.db_type === 'postgres') params.push('sslmode=no-verify');
    if (info.db_type === 'mysql') params.push('ssl-mode=Disabled');
    if (info.db_type === 'mongodb') params.push('tlsInsecure=true');
  }

  const paramStr = params.length > 0 ? `?${params.join('&')}` : '';

  switch (info.db_type) {
    case 'postgres': return `postgres://${userPass}${info.host}${port}${db}${paramStr}`;
    case 'mysql': return `mysql://${userPass}${info.host}${port}${db}${paramStr}`;
    case 'mongodb': return `mongodb://${userPass}${info.host}${port}${db}${paramStr}`;
    case 'sqlserver': 
      if (info.use_entra_id) {
        return `Server=${info.host}${port};Initial Catalog=${info.database || ''};Authentication=ActiveDirectoryIntegrated;TrustServerCertificate=${info.trust_server_certificate ? 'true' : 'false'}`;
      }
      return `Server=${info.host}${port};Initial Catalog=${info.database || ''};User ID=${info.user || ''};Password=••••••••;TrustServerCertificate=${info.trust_server_certificate ? 'true' : 'false'}`;
    default: return 'Mock Connection';
  }
});

function copyToClipboard(text: string) {
  // Use connectionString if present for copying real password, 
  // or regenerate it without masking for copying if possible, 
  // but for UI display we masked it.
  // Actually let's just copy exactly what we see for now or provide a real copy.
  // Real copy logic:
  let realString = '';
  if (dbStore.connectionString) {
    realString = dbStore.connectionString;
  } else if (dbStore.currentConnection) {
    const info = dbStore.currentConnection;
    const userPass = info.user ? `${info.user}:${info.password || ''}@` : '';
    const port = info.port ? `:${info.port}` : '';
    const db = info.database ? `/${info.database}` : '';
    let params = [];
    if (info.trust_server_certificate) {
      if (info.db_type === 'postgres') params.push('sslmode=no-verify');
      if (info.db_type === 'mysql') params.push('ssl-mode=Disabled');
      if (info.db_type === 'mongodb') params.push('tlsInsecure=true');
    }
    const paramStr = params.length > 0 ? `?${params.join('&')}` : '';
    
    switch (info.db_type) {
      case 'postgres': realString = `postgres://${userPass}${info.host}${port}${db}${paramStr}`; break;
      case 'mysql': realString = `mysql://${userPass}${info.host}${port}${db}${paramStr}`; break;
      case 'mongodb': realString = `mongodb://${userPass}${info.host}${port}${db}${paramStr}`; break;
      case 'sqlserver': 
        if (info.use_entra_id) {
          realString = `Server=${info.host}${port};Initial Catalog=${info.database || ''};Authentication=ActiveDirectoryIntegrated;TrustServerCertificate=${info.trust_server_certificate ? 'true' : 'false'}`;
        } else {
          realString = `Server=${info.host}${port};Initial Catalog=${info.database || ''};User ID=${info.user || ''};Password=${info.password || ''};TrustServerCertificate=${info.trust_server_certificate ? 'true' : 'false'}`;
        }
        break;
      default: realString = 'Mock Connection';
    }
  }

  navigator.clipboard.writeText(realString || text);
  copied.value = true;
  setTimeout(() => copied.value = false, 2000);
}

const newConn = ref<ConnectionInfo>({
  db_type: 'postgres',
  host: 'localhost',
  port: 5432,
  user: '',
  password: '',
  database: ''
});

// Watch for db_type changes to set default ports
watch(() => newConn.value.db_type, (newType) => {
  // Only auto-port if we're not editing (or if it's a new form)
  if (editingId.value) return;

  switch (newType) {
    case 'postgres': newConn.value.port = 5432; break;
    case 'mysql': newConn.value.port = 3306; break;
    case 'sqlserver': newConn.value.port = 1433; break;
    case 'mongodb': newConn.value.port = 27017; break;
    case 'mock': newConn.value.port = 0; break;
  }
});

const useConnectionString = ref(false);
const connString = ref('');
const connectionAlias = ref('');
const shouldSave = ref(true);

function openNewConnectionForm() {
  editingId.value = null;
  connectionAlias.value = '';
  showPassword.value = false;
  newConn.value = {
    db_type: 'postgres',
    host: 'localhost',
    port: 5432,
    user: '',
    password: '',
    database: '',
    use_entra_id: false,
    trust_server_certificate: false
  };
  useConnectionString.value = false;
  connString.value = '';
  viewMode.value = 'form';
}

function editConnection(saved: any) {
  editingId.value = saved.id;
  connectionAlias.value = saved.alias;
  showPassword.value = false;
  
  if (typeof saved.info === 'string') {
    useConnectionString.value = true;
    connString.value = saved.info;
  } else {
    useConnectionString.value = false;
    newConn.value = { ...saved.info };
  }
  
  shouldSave.value = true;
  viewMode.value = 'form';
}

async function handleConnect() {
  try {
    const info = useConnectionString.value ? connString.value : { ...newConn.value };
    
    let alias = connectionAlias.value;
    if (!alias) {
      if (typeof info === 'string') {
        // Simple extraction of host from connection string
        alias = info.split('@').pop()?.split('/')[0] || 'New Connection';
      } else {
        alias = info.host + (info.database ? `/${info.database}` : '');
      }
    }

    await dbStore.connect(info, alias);
    
    if (shouldSave.value) {
      if (editingId.value) {
        dbStore.updateConnection(editingId.value, alias, info);
      } else {
        dbStore.saveConnection(alias, info);
      }
    }
    viewMode.value = 'explorer';
  } catch (e) {
    console.error(e);
  }
}

async function connectToSaved(saved: any) {
  try {
    // If already connected to this one, just switch to explorer
    const isCurrent = typeof saved.info === 'string' 
      ? dbStore.connectionString === saved.info 
      : JSON.stringify(dbStore.currentConnection) === JSON.stringify(saved.info);

    if (isCurrent && dbStore.connectionStatus === 'connected') {
      viewMode.value = 'explorer';
      return;
    }

    await dbStore.connect(saved.info, saved.alias);
    viewMode.value = 'explorer';
  } catch (e) {
    console.error(e);
  }
}

async function selectWorkspace() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === 'string') {
      dbStore.setWorkspace(selected);
    }
  } catch (e) {
    console.error('Failed to select workspace', e);
  }
}

function handleDisconnect() {
  dbStore.disconnect();
  viewMode.value = 'list';
}

function getStatusColor(status: string) {
  switch (status) {
    case 'connected': return '#4ec9b0';
    case 'connecting': return '#ce9178';
    case 'error': return '#f48771';
    default: return '#888';
  }
}
</script>

<template>
  <div class="sidebar">
    <div class="header">
      <div class="title"><Server :size="16" /> AeroCheetahDBM</div>
      <div class="status-indicator" :style="{ backgroundColor: getStatusColor(dbStore.connectionStatus) }" :title="dbStore.connectionStatus"></div>
      
      <div class="header-actions">
        <button v-if="viewMode === 'explorer'" @click="viewMode = 'list'" class="icon-btn" title="Back to list"><ArrowLeft :size="14" /></button>
        <button v-if="viewMode === 'explorer'" @click="viewMode = 'exit'" class="icon-btn danger" title="Exit"><LogOut :size="14" /></button>
        <button v-if="viewMode === 'exit'" @click="viewMode = 'explorer'" class="icon-btn" title="Back"><ArrowLeft :size="14" /></button>
        <button v-if="viewMode === 'form'" @click="viewMode = 'list'" class="icon-btn" title="Back to list"><Star :size="14" /></button>
        <button v-if="viewMode === 'list'" @click="openNewConnectionForm" class="icon-btn" title="New Connection"><Plus :size="14" /></button>
        <button v-if="viewMode !== 'settings'" @click="viewMode = 'settings'" class="icon-btn" title="Settings"><Settings :size="14" /></button>
      </div>
    </div>

    <!-- EXIT VIEW -->
    <div v-if="viewMode === 'exit'" class="exit-view">
      <div class="section-title">Manage Connection</div>
      <div class="exit-options">
        <button @click="viewMode = 'explorer'" class="secondary-btn">
          <Database :size="14" /> Return to Explorer
        </button>
        <button @click="viewMode = 'list'" class="secondary-btn">
          <Server :size="14" /> Browse Other Connections
        </button>
        <button @click="handleDisconnect" class="danger-btn mt-16">
          <LogOut :size="14" /> Disconnect
        </button>
      </div>
    </div>

    <!-- SETTINGS VIEW -->
    <div v-if="viewMode === 'settings'" class="settings-view">
      <div class="section-title">Settings</div>
      <div class="setting-item">
        <label>Default Workspace</label>
        <div class="workspace-picker">
          <div class="path-display" :title="dbStore.defaultWorkspace || 'Not set'">
            {{ dbStore.defaultWorkspace || 'Select a folder...' }}
          </div>
          <button @click="selectWorkspace" class="icon-btn"><Folder :size="14" /></button>
        </div>
        <p class="help-text">Queries will be stored in subfolders named after DB aliases.</p>
      </div>
      <div class="setting-item mt-16">
        <label>Idle Connection Timeout</label>
        <select v-model="dbStore.connectionTimeout" @change="dbStore.persistSettings">
          <option :value="0">Never</option>
          <option :value="300000">5 minutes</option>
          <option :value="600000">10 minutes</option>
          <option :value="900000">15 minutes</option>
          <option :value="1800000">30 minutes</option>
          <option :value="3600000">1 hour</option>
          <option :value="10800000">3 hours</option>
          <option :value="36000000">10 hours</option>
          <option :value="86400000">24 hours</option>
          <option :value="172800000">48 hours</option>
        </select>
        <p class="help-text">Auto-disconnect after period of inactivity.</p>
      </div>
      <div class="setting-item mt-16">
        <label>Connection Timeout (seconds)</label>
        <input type="number" v-model.number="dbStore.connectTimeout" @change="dbStore.persistSettings" min="1" max="300" />
        <p class="help-text">Timeout for initial connection (Default: 30).</p>
      </div>
      <button @click="viewMode = 'list'" class="secondary-btn mt-16">Close Settings</button>
    </div>

    <!-- CONNECTION FORM -->
    <div v-if="viewMode === 'form'" class="connect-form">
      <div class="section-header">
        <div class="section-title">{{ editingId ? 'Edit' : 'New' }} Connection</div>
        <div class="tabs">
          <button :class="{ active: !useConnectionString }" @click="useConnectionString = false">Fields</button>
          <button :class="{ active: useConnectionString }" @click="useConnectionString = true">String</button>
        </div>
      </div>

      <div class="form-scroll-area">
        <div class="field">
          <label><Star :size="10" /> Connection Alias</label>
          <input v-model="connectionAlias" placeholder="My Database" />
        </div>

        <div v-if="!useConnectionString" class="fields-container">
          <div class="field">
            <label><Database :size="10" /> Database Type</label>
            <select v-model="newConn.db_type">
              <option value="postgres">PostgreSQL</option>
              <option value="mysql">MySQL</option>
              <option value="sqlserver">SQL Server</option>
              <option value="mongodb">MongoDB</option>
              <option value="mock">Mock DB (Testing)</option>
            </select>
          </div>

          <div class="field">
            <label><Server :size="10" /> Host</label>
            <input v-model="newConn.host" placeholder="localhost" />
          </div>
          <div class="field">
            <label>Port</label>
            <input v-model.number="newConn.port" type="number" />
          </div>

          <div class="field" v-if="newConn.db_type === 'sqlserver'">
            <div class="checkbox-field">
              <input type="checkbox" v-model="newConn.use_entra_id" id="use-entra" />
              <label for="use-entra">Use Microsoft Entra ID (Integrated)</label>
            </div>
          </div>

          <div class="field">
            <label>User</label>
            <input v-model="newConn.user" placeholder="username" :disabled="newConn.use_entra_id" />
          </div>
          <div class="field">
            <label>Password</label>
            <div class="password-input-wrapper">
              <input v-model="newConn.password" :type="showPassword ? 'text' : 'password'" placeholder="••••••••" :disabled="newConn.use_entra_id" />
              <button type="button" @click="showPassword = !showPassword" class="password-toggle" :disabled="newConn.use_entra_id" :title="showPassword ? 'Hide password' : 'Show password'">
                <Eye v-if="!showPassword" :size="14" />
                <EyeOff v-else :size="14" />
              </button>
            </div>
          </div>
          <div class="field">
            <label>Database Name</label>
            <input v-model="newConn.database" placeholder="e.g. production_db" />
          </div>

          <div class="field">
            <div class="checkbox-field">
              <input type="checkbox" v-model="newConn.trust_server_certificate" id="trust-cert" />
              <label for="trust-cert">Trust Server Certificate</label>
            </div>
          </div>
        </div>
        
        <div v-else class="string-container">
          <div class="field">
            <label><Plus :size="10" /> Connection String</label>
            <textarea v-model="connString" placeholder="postgres://user:pass@host:port/db" rows="4"></textarea>
          </div>
        </div>

        <div class="checkbox-field">
          <input type="checkbox" v-model="shouldSave" id="save-conn" />
          <label for="save-conn">Remember this connection</label>
        </div>
      </div>

      <div class="form-actions">
        <div class="btn-group">
          <button @click="handleConnect" :disabled="dbStore.loading" class="primary-btn flex-2">
            {{ dbStore.loading ? 'Connecting...' : 'Connect' }}
          </button>
          <button @click="viewMode = 'list'" class="secondary-btn flex-1" :disabled="dbStore.loading">
            Cancel
          </button>
        </div>
        <div v-if="dbStore.error" class="error-box">
          {{ dbStore.error }}
        </div>
      </div>
    </div>

    <!-- SAVED CONNECTIONS LIST -->
    <div v-else-if="viewMode === 'list'" class="saved-connections">
      <div class="section-title">Saved Connections</div>
      <div v-if="dbStore.savedConnections.length === 0" class="empty-msg">
        No saved connections. Click + to add one.
      </div>
      <div v-for="saved in dbStore.savedConnections" :key="saved.id" class="saved-item" :class="{ active: dbStore.currentConnection === saved.info || dbStore.connectionString === saved.info }">
        <div class="saved-info" @click="connectToSaved(saved)">
          <Star :size="14" :fill="saved.id === 'demo-db' ? '#ce9178' : 'none'" :stroke="saved.id === 'demo-db' ? '#ce9178' : 'currentColor'" />
          <div class="alias">{{ saved.alias }}</div>
        </div>
        <div class="item-actions">
          <button @click="editConnection(saved)" class="action-btn" title="Edit connection"><Pencil :size="12" /></button>
          <button @click="dbStore.deleteConnection(saved.id)" class="action-btn delete" title="Remove saved connection"><Trash2 :size="12" /></button>
        </div>
      </div>
    </div>

    <!-- SCHEMA EXPLORER (CONNECTED) -->
    <div v-else-if="viewMode === 'explorer'" class="schema-explorer">
      <div class="explorer-header">
        <div class="conn-info" :title="dbStore.connectionString || 'Structured Info'">
          <Database :size="14" /> {{ dbStore.currentConnection?.database || 'Connected' }}
        </div>
        <div class="explorer-actions">
          <button @click="viewMode = 'details'" class="icon-btn" title="Connection Details"><Info :size="14" /></button>
          <button @click="dbStore.fetchTables()" class="icon-btn" title="Refresh Tables"><RefreshCcw :size="14" /></button>
        </div>
      </div>

      <div class="tables-list">
        <div v-for="table in dbStore.tables" :key="table.name" class="table-item">
          <div class="table-header" @click="dbStore.fetchColumns(table.name, table.schema)">
            <Table :size="14" /> {{ table.name }}
          </div>
          <div v-if="dbStore.selectedTable === table.name" class="columns-list">
            <div v-for="col in dbStore.columns[table.name]" :key="col.name" class="column-item">
              <Columns :size="12" /> {{ col.name }} <span class="type">{{ col.data_type }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- CONNECTION DETAILS VIEW -->
    <div v-else-if="viewMode === 'details'" class="details-view">
      <div class="section-header">
        <div class="section-title">Connection Details</div>
      </div>
      <div class="details-content">
        <div class="detail-item" v-if="dbStore.currentConnection">
          <label>Type</label>
          <div class="value">{{ dbStore.currentConnection.db_type }}</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection">
          <label>Host</label>
          <div class="value">{{ dbStore.currentConnection.host }}</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection">
          <label>Port</label>
          <div class="value">{{ dbStore.currentConnection.port }}</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection?.user">
          <label>User</label>
          <div class="value">{{ dbStore.currentConnection.user }}</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection?.database">
          <label>Database</label>
          <div class="value">{{ dbStore.currentConnection.database }}</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection?.use_entra_id">
          <label>Auth</label>
          <div class="value">Microsoft Entra ID</div>
        </div>
        <div class="detail-item" v-if="dbStore.currentConnection?.trust_server_certificate">
          <label>Security</label>
          <div class="value">Trust Server Certificate Enabled</div>
        </div>

        <div class="detail-item full-width">
          <label>Generated Connection String</label>
          <div class="string-box">
            <code>{{ generatedConnectionString }}</code>
            <button @click="copyToClipboard(generatedConnectionString)" class="copy-btn" :title="copied ? 'Copied!' : 'Copy to clipboard'">
              <Check v-if="copied" :size="14" />
              <Copy v-else :size="14" />
            </button>
          </div>
        </div>
      </div>
      <div class="form-actions">
        <button @click="viewMode = 'explorer'" class="secondary-btn">Back to Explorer</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 260px;
  background: #252526;
  border-right: 1px solid #333;
  display: flex;
  flex-direction: column;
  color: #ccc;
  overflow-y: auto;
}
.header {
  padding: 12px;
  background: #2d2d2d;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #333;
}
.header-actions {
  display: flex;
  gap: 8px;
}
.title {
  font-weight: bold;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.connect-form {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.section-header {
  padding: 12px 16px;
  background: #2d2d2d;
  border-bottom: 1px solid #333;
}
.form-scroll-area {
  flex: 1;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}
.form-actions {
  padding: 16px;
  background: #2d2d2d;
  border-top: 1px solid #333;
}
.field-row, .btn-group {
  display: flex;
  gap: 12px;
}
.flex-1 { flex: 1; }
.flex-2 { flex: 2; }
.flex-3 { flex: 3; }

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.password-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.password-input-wrapper input {
  width: 100%;
  padding-right: 36px;
}
.password-toggle {
  position: absolute;
  right: 8px;
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
}
.password-toggle:hover:not(:disabled) {
  color: #fff;
  background: #3c3c3c;
}
.password-toggle:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.field label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #aaa;
}
.checkbox-field {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: #ccc;
  margin-top: 4px;
}
input, select, textarea {
  background: #1e1e1e;
  border: 1px solid #3c3c3c;
  color: #fff;
  padding: 8px;
  border-radius: 4px;
  font-size: 13px;
  transition: border-color 0.2s;
}
select option {
  background: #252526;
  color: #fff;
}
input:focus, select:focus, textarea:focus {
  border-color: #007acc;
  outline: none;
}
.primary-btn {
  background: #007acc;
  color: white;
  border: none;
  padding: 10px;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  width: 100%;
}
.primary-btn:hover:not(:disabled) {
  background: #1c97ea;
}
.error-box {
  background: #4b1e1e;
  border: 1px solid #8c3333;
  color: #f48771;
  padding: 8px;
  border-radius: 4px;
  font-size: 12px;
  margin-top: 12px;
  word-break: break-all;
}
.secondary-btn {
  background: #3c3c3c;
  color: white;
  border: 1px solid #444;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s;
}
.secondary-btn:hover {
  background: #444;
}
.danger-btn {
  background: #8c3333;
  color: white;
  border: none;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s;
}
.danger-btn:hover {
  background: #a33c3c;
}
.primary-btn:disabled {
  background: #333;
  color: #666;
  cursor: not-allowed;
}
.schema-explorer, .saved-connections {
  padding: 8px;
}
.section-title {
  padding: 8px;
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
}
.empty-msg {
  padding: 16px;
  font-size: 12px;
  color: #555;
  text-align: center;
}
.saved-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  border-radius: 4px;
  cursor: pointer;
}
.saved-item:hover {
  background: #2a2d2e;
}
.saved-item.active {
  background: #37373d;
  border-left: 2px solid #007acc;
}
.saved-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}
.alias {
  font-size: 13px;
}
.item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}
.saved-item:hover .item-actions {
  opacity: 1;
}
.action-btn {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
}
.action-btn:hover {
  color: #fff;
  background: #3c3c3c;
}
.action-btn.delete:hover {
  color: #f48771;
}
.explorer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 8px;
  margin-bottom: 8px;
}
.explorer-actions {
  display: flex;
  gap: 4px;
}
.conn-info {
  flex: 1;
  padding: 6px 8px;
  font-size: 12px;
  background: #1e1e1e;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.current-conn-action {
  margin-top: 16px;
  padding: 8px;
  border-top: 1px solid #333;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.table-item {
  margin-bottom: 4px;
}
.table-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  cursor: pointer;
  font-size: 13px;
  border-radius: 4px;
}
.table-header:hover {
  background: #2a2d2e;
}
.columns-list {
  padding-left: 24px;
  margin-top: 4px;
  font-size: 12px;
  color: #888;
}
.column-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}
.type {
  font-size: 10px;
  color: #555;
}
.icon-btn {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
}
.icon-btn:hover {
  color: #fff;
  background: #3c3c3c;
}
.icon-btn.danger:hover {
  color: #f48771;
}
.status-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-left: 8px;
}
.settings-view {
  padding: 16px;
}
.exit-view, .details-view {
  padding: 16px;
  display: flex;
  flex-direction: column;
  height: 100%;
}
.details-content {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 20px;
}
.detail-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.detail-item label {
  font-size: 10px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
}
.detail-item .value {
  font-size: 13px;
  color: #fff;
  background: #1e1e1e;
  padding: 6px 8px;
  border-radius: 4px;
  border: 1px solid #333;
}
.detail-item.full-width {
  margin-top: 8px;
}
.string-box {
  background: #1e1e1e;
  border: 1px solid #333;
  padding: 10px;
  border-radius: 4px;
  position: relative;
  word-break: break-all;
  font-family: monospace;
  font-size: 11px;
  line-height: 1.4;
  color: #ce9178;
}
.copy-btn {
  position: absolute;
  top: 4px;
  right: 4px;
  background: #333;
  border: 1px solid #444;
  color: #ccc;
  padding: 4px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.copy-btn:hover {
  background: #444;
  color: #fff;
}
.exit-view {
  padding: 16px;
}
.exit-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 16px;
}
.exit-options button {
  justify-content: flex-start;
  padding: 10px;
}
.setting-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.workspace-picker {
  display: flex;
  gap: 8px;
  align-items: center;
}
.path-display {
  flex: 1;
  background: #1e1e1e;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border: 1px solid #333;
}
.help-text {
  font-size: 10px;
  color: #666;
  margin-top: 4px;
}
.mt-16 {
  margin-top: 16px;
}
.tabs {
  display: flex;
  background: #1e1e1e;
  border-radius: 4px;
  padding: 2px;
  margin-bottom: 12px;
}
.tabs button {
  flex: 1;
  background: transparent;
  border: none;
  color: #888;
  padding: 4px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 3px;
}
.tabs button.active {
  background: #3c3c3c;
  color: #fff;
}
textarea {
  background: #3c3c3c;
  border: 1px solid #555;
  color: #fff;
  padding: 6px;
  border-radius: 4px;
  resize: vertical;
  font-family: monospace;
}
input, select, textarea {
  color-scheme: dark;
}
</style>
