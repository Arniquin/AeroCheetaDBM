<script setup lang="ts">
import { ref, watch } from 'vue';
import { Star, Database, Server, Eye, EyeOff, Plus } from 'lucide-vue-next';
import { useDbStore, type ConnectionInfo } from '../../stores/db';

const props = defineProps<{
  editingId: string | null;
  initialAlias: string;
  initialInfo: ConnectionInfo | string;
}>();

const emit = defineEmits(['connected', 'cancel']);

const dbStore = useDbStore();
const showPassword = ref(false);
const useConnectionString = ref(typeof props.initialInfo === 'string');
const connString = ref(typeof props.initialInfo === 'string' ? props.initialInfo as string : '');
const connectionAlias = ref(props.initialAlias);
const shouldSave = ref(true);

const newConn = ref<ConnectionInfo>(
  typeof props.initialInfo === 'object' 
    ? { ...props.initialInfo } 
    : {
        db_type: 'postgres',
        host: 'localhost',
        port: 5432,
        user: '',
        password: '',
        database: '',
        use_entra_id: false,
        trust_server_certificate: false
      }
);

watch(() => newConn.value.db_type, (newType) => {
  if (props.editingId) return;

  switch (newType) {
    case 'postgres': newConn.value.port = 5432; break;
    case 'mysql': newConn.value.port = 3306; break;
    case 'sqlserver': newConn.value.port = 1433; break;
    case 'mongodb': newConn.value.port = 27017; break;
    case 'mock': newConn.value.port = 0; break;
  }
});

async function handleConnect() {
  try {
    const info = useConnectionString.value ? connString.value : { ...newConn.value };
    
    let alias = connectionAlias.value;
    if (!alias) {
      if (typeof info === 'string') {
        alias = info.split('@').pop()?.split('/')[0] || 'New Connection';
      } else {
        alias = info.host + (info.database ? `/${info.database}` : '');
      }
    }

    await dbStore.connect(info, alias);
    
    if (shouldSave.value) {
      if (props.editingId) {
        dbStore.updateConnection(props.editingId, alias, info);
      } else {
        dbStore.saveConnection(alias, info);
      }
    }
    emit('connected');
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <div class="connect-form">
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
        <button @click="emit('cancel')" class="secondary-btn flex-1" :disabled="dbStore.loading">
          Cancel
        </button>
      </div>
      <div v-if="dbStore.error" class="error-box">
        {{ dbStore.error }}
      </div>
    </div>
  </div>
</template>

<style scoped>
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
.section-title {
  padding: 8px;
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  font-weight: bold;
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
.form-scroll-area {
  flex: 1;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #aaa;
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
input:focus, select:focus, textarea:focus {
  border-color: #007acc;
  outline: none;
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
.checkbox-field {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: #ccc;
  margin-top: 4px;
}
.form-actions {
  padding: 16px;
  background: #2d2d2d;
  border-top: 1px solid #333;
}
.btn-group {
  display: flex;
  gap: 12px;
}
.flex-1 { flex: 1; }
.flex-2 { flex: 2; }
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
.primary-btn:disabled {
  background: #333;
  color: #666;
  cursor: not-allowed;
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
textarea {
  background: #3c3c3c;
  border: 1px solid #555;
  color: #fff;
  padding: 6px;
  border-radius: 4px;
  resize: vertical;
  font-family: monospace;
}
</style>
