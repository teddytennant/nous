// Nous Desktop — Application Logic

const { invoke } = window.__TAURI__.core;

// ─── Navigation ───

document.querySelectorAll('.nav-item').forEach(item => {
  item.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach(i => i.classList.remove('active'));
    document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));

    item.classList.add('active');
    const view = document.getElementById(`view-${item.dataset.view}`);
    if (view) view.classList.add('active');
  });
});

// ─── Dashboard ───

async function loadDashboard() {
  try {
    const status = await invoke('get_node_status');

    document.getElementById('stat-did').textContent =
      status.did.substring(0, 20) + '...';
    document.getElementById('stat-peers').textContent = status.peers;
    document.getElementById('stat-uptime').textContent =
      formatUptime(status.uptime_secs);
    document.getElementById('stat-version').textContent = status.version;

    const grid = document.getElementById('modules-grid');
    grid.innerHTML = status.modules
      .map(m => `
        <div class="module-card">
          <span class="module-name">${m.name}</span>
          <span class="module-status ${m.status}">${m.status}</span>
        </div>
      `).join('');
  } catch (e) {
    console.error('Failed to load dashboard:', e);
  }
}

function formatUptime(secs) {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86400)}d`;
}

// ─── Wallet ───

async function loadWallet() {
  try {
    const balances = await invoke('get_wallet_balances');
    const grid = document.getElementById('balances-grid');
    grid.innerHTML = balances
      .map(b => `
        <div class="balance-card">
          <div class="balance-token">${b.token}</div>
          <div class="balance-amount">${b.balance}</div>
          ${b.usd_value ? `<div class="balance-usd">${b.usd_value}</div>` : ''}
        </div>
      `).join('');
  } catch (e) {
    console.error('Failed to load wallet:', e);
  }
}

// ─── Identity ───

async function loadIdentity() {
  try {
    const identity = await invoke('get_identity');
    document.getElementById('identity-did').textContent = identity.did;

    const keysSection = document.getElementById('keys-section');
    keysSection.innerHTML = identity.keys
      .map(k => `
        <div class="key-row">
          <span class="key-type">${k.type}</span>
          <span class="key-purpose">${k.purpose}</span>
        </div>
      `).join('');
  } catch (e) {
    console.error('Failed to load identity:', e);
  }
}

// ─── Version ───

async function loadVersion() {
  try {
    const version = await invoke('app_version');
    document.getElementById('version').textContent = `v${version}`;
  } catch (e) {
    console.error('Failed to load version:', e);
  }
}

// ─── Initialize ───

document.addEventListener('DOMContentLoaded', async () => {
  await Promise.all([
    loadDashboard(),
    loadWallet(),
    loadIdentity(),
    loadVersion(),
  ]);
});
