<script setup lang="ts">
import { ref, computed } from 'vue';
import { Check, Copy } from 'lucide-vue-next';
import { useDbStore } from '../../stores/db';

const dbStore = useDbStore();
const emit = defineEmits(['back']);
const copied = ref(false);

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
</script>

<template>
  <div class="details-view">
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
      <button @click="emit('back')" class="secondary-btn">Back to Explorer</button>
    </div>
  </div>
</template>

<style scoped>
.details-view {
  padding: 16px;
  display: flex;
  flex-direction: column;
  height: 100%;
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
.form-actions {
  padding: 16px;
  background: #2d2d2d;
  border-top: 1px solid #333;
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
</style>
