'use strict';

/* RYNXCP panel client. No external dependencies, CSP friendly. */

const S = {
  user: null,
  csrf: null,
  current: 'overview',
  navToken: 0,
  system: null,
  mode: null,
  impersonatedBy: null,
  data: { accounts: [], websites: [], apps: [], pools: [], certs: [], users: [] },
};

const RUNTIMES = {
  static: { label: 'Sitio estático', short: 'Static', versions: [], commands: [] },
  php: { label: 'PHP (FPM)', short: 'PHP', versions: ['8.5', '8.4', '8.3', '8.2', '8.1'], commands: [] },
  node: { label: 'Node.js (systemd)', short: 'Node', versions: ['24', '22', '20', '18'], commands: ['node'], entry: 'server.js', exampleArgs: 'server.js' },
  python: { label: 'Python (WSGI/ASGI)', short: 'Python', versions: ['3.13', '3.12', '3.11', '3.10'], commands: ['gunicorn', 'uvicorn', 'python3'], entry: 'app.py', exampleArgs: 'app:app --bind=127.0.0.1:4100' },
  ruby: { label: 'Ruby', short: 'Ruby', versions: ['3.4', '3.3', '3.2'], commands: ['puma', 'bundle', 'rackup', 'ruby'], entry: 'config.ru', exampleArgs: 'config.ru' },
  java: { label: 'Java (JAR)', short: 'Java', versions: ['21', '17', '11'], commands: ['java'], entry: 'app.jar', exampleArgs: '-jar app.jar' },
  dotnet: { label: '.NET', short: '.NET', versions: ['9', '8'], commands: ['dotnet'], entry: 'App.dll', exampleArgs: 'App.dll' },
  go: { label: 'Go (binario compilado)', short: 'Go', versions: [], commands: [], entry: 'server', exampleArgs: '' },
};

const MANAGED = ['node', 'python', 'ruby', 'java', 'dotnet', 'go'];

const NAV_ITEMS = [
  { id: 'overview', label: 'Resumen', group: 'Operación' },
  { id: 'accounts', label: 'Cuentas', group: 'Operación' },
  { id: 'websites', label: 'Sitios web', group: 'Operación' },
  { id: 'apps', label: 'Aplicaciones', group: 'Operación' },
  { id: 'php', label: 'PHP-FPM', group: 'Operación' },
  { id: 'databases', label: 'Bases de datos', group: 'Operación' },
  { id: 'ftp', label: 'FTP', group: 'Operación' },
  { id: 'dns', label: 'DNS', group: 'Operación' },
  { id: 'ssl', label: 'Certificados', group: 'Operación' },
  { id: 'backups', label: 'Backups', group: 'Operación' },
  { id: 'monitor', label: 'Monitor', group: 'Operación' },
  { id: 'users', label: 'Usuarios', group: 'Administración', admin: true },
  { id: 'security', label: 'Seguridad', group: 'Administración', admin: true },
  { id: 'firewall', label: 'Cortafuegos', group: 'Administración', admin: true },
  { id: 'programs', label: 'Programas', group: 'Administración', admin: true },
  { id: 'console', label: 'Consola', group: 'Administración', admin: true },
  { id: 'audit', label: 'Auditoría', group: 'Administración', admin: true },
  { id: 'system', label: 'Sistema', group: 'Administración' },
];

const TITLES = {
  databases: ['Bases de datos', 'MariaDB aislada por dominio, con usuario y contraseña propios'],
  ftp: ['FTP', 'Usuarios virtuales Pure-FTPd con chroot al document root'],
  dns: ['DNS', 'Zonas autoritativas con SPF, DKIM, DMARC, MX y CAA'],
  backups: ['Backups', 'Copias de home, vhosts, pools y unidades con retención'],
  monitor: ['Monitor', 'Series históricas y estado de cada servicio'],
  security: ['Seguridad', 'Intentos de acceso, bloqueos y cadena de auditoría'],
  firewall: ['Cortafuegos', 'Reglas nftables, IPs baneadas y listas blancas'],
  programs: ['Programas', 'Software instalado y actualizaciones del sistema'],
  console: ['Consola admin', 'Acciones privilegiadas auditadas y sin shell libre'],
  overview: ['Resumen', 'Estado del nodo y de los recursos gestionados'],
  accounts: ['Cuentas', 'Usuarios Linux aislados con cuota y home dedicado'],
  websites: ['Sitios web', 'Virtual hosts Nginx con TLS y reverse proxy'],
  apps: ['Aplicaciones', 'Procesos gestionados por systemd (Node, Python, Ruby, Java, .NET, Go)'],
  php: ['PHP-FPM', 'Pools por cuenta con open_basedir y funciones restringidas'],
  ssl: ['Certificados', 'Emisión y estado de certificados Let\'s Encrypt'],
  users: ['Usuarios del panel', 'Acceso al panel con roles y sesiones revocables'],
  audit: ['Auditoría', 'Registro append-only encadenado por SHA-256'],
  system: ['Sistema', 'Información del nodo y reconciliación de estado'],
};

/* ---------------- utilities ---------------- */

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

function esc(value) {
  return String(value === undefined || value === null ? '' : value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function td(html, cls = '') {
  return `<td class="${cls}">${html}</td>`;
}

function badge(text, kind = '') {
  return `<span class="badge ${kind}">${esc(text)}</span>`;
}

function btn(label, action, id = '', cls = 'tiny ghost', title = '') {
  return `<button class="btn ${cls}" data-action="${action}" data-id="${esc(id)}" title="${esc(title)}">${label}</button>`;
}

function table(headers, rows, emptyText = 'Sin registros todavía.') {
  if (!rows.length) {
    return `<div class="card empty"><strong>Sin registros</strong><span>${esc(emptyText)}</span></div>`;
  }
  const head = headers.map((h) => `<th>${esc(h)}</th>`).join('');
  const body = rows.map((r) => `<tr>${r.join('')}</tr>`).join('');
  return `<div class="table-wrap"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

function sectionHead(title, actionHtml = '') {
  return `<div class="section-head"><h2>${esc(title)}</h2><div>${actionHtml}</div></div>`;
}

function fmtDate(raw) {
  if (!raw) return '—';
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) return esc(raw);
  return date.toLocaleString();
}

function fmtUptime(secs) {
  const value = Number(secs) || 0;
  const d = Math.floor(value / 86400);
  const h = Math.floor((value % 86400) / 3600);
  const m = Math.floor((value % 3600) / 60);
  return `${d}d ${h}h ${m}m`;
}

function fmtKb(kb) {
  const value = Number(kb) || 0;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} GiB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} MiB`;
  return `${value} KiB`;
}

function pct(used, total) {
  if (!total) return 0;
  return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
}

function runtimeLabel(runtime) {
  if (!runtime) return '—';
  const meta = RUNTIMES[runtime.engine] || { short: runtime.engine };
  return runtime.version ? `${meta.short} ${runtime.version}` : meta.short;
}

function val(form, name) {
  const el = form.elements[name];
  return el ? String(el.value || '').trim() : '';
}

function num(form, name, fallback = 0) {
  const parsed = Number(val(form, name));
  return Number.isFinite(parsed) ? parsed : fallback;
}

function bool(form, name) {
  const el = form.elements[name];
  return !!(el && el.checked);
}

function T(text) {
  return window.RYNX_I18N ? window.RYNX_I18N.tr(text) : text;
}

function toAsciiDomain(value) {
  const raw = String(value || '')
    .trim()
    .replace(/^https?:\/\//i, '')
    .replace(/\/.*$/, '')
    .replace(/\.$/, '');
  if (!raw) return '';
  try {
    return new URL(`http://${raw}`).hostname.replace(/\.$/, '');
  } catch {
    return raw.toLowerCase();
  }
}

function toAsciiEmail(value) {
  const raw = String(value || '').trim();
  const at = raw.lastIndexOf('@');
  if (at <= 0) return raw;
  return `${raw.slice(0, at + 1)}${toAsciiDomain(raw.slice(at + 1))}`;
}

function translateUi(root) {
  if (window.RYNX_I18N) window.RYNX_I18N.translateDom(root || document.body);
}

function toast(message, kind = 'ok') {
  const root = $('#toasts');
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  const title = kind === 'error' ? T('Error') : T('Hecho');
  el.innerHTML = `<div class="t-title">${esc(title)}</div><div>${esc(T(message))}</div>`;
  root.appendChild(el);
  setTimeout(() => el.remove(), kind === 'error' ? 7000 : 3500);
}

/* ---------------- API ---------------- */

async function api(path, options = {}) {
  const method = options.method || 'GET';
  const headers = {};
  if (options.body !== undefined) headers['content-type'] = 'application/json';
  if (method !== 'GET' && S.csrf) headers['x-csrf-token'] = S.csrf;

  const response = await fetch(path, {
    method,
    headers,
    credentials: 'same-origin',
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  });

  const text = await response.text();
  let data = null;
  if (text) {
    try { data = JSON.parse(text); } catch { data = null; }
  }

  if (response.status === 401) {
    showLogin();
    throw new Error('La sesión ha expirado');
  }
  if (!response.ok) {
    const message = data && data.error && data.error.message
      ? data.error.message
      : `Error HTTP ${response.status}`;
    throw new Error(message);
  }
  return data;
}

/* ---------------- session ---------------- */

function showLogin() {
  S.user = null;
  S.csrf = null;
  S.impersonatedBy = null;
  S.data = { accounts: [], websites: [], apps: [], pools: [], certs: [], users: [] };
  $('#app').classList.add('hidden');
  $('#login').classList.remove('hidden');
  translateUi(document.body);
  const input = $('#login-form').elements.username;
  if (input) setTimeout(() => input.focus(), 30);
}

function enterApp() {
  S.data = { accounts: [], websites: [], apps: [], pools: [], certs: [], users: [] };
  $('#login').classList.add('hidden');
  $('#app').classList.remove('hidden');
  $('#user-name').textContent = S.user.username;
  $('#user-role').textContent = S.user.role;
  $('#avatar').textContent = S.user.username.slice(0, 1);
  const banner = $('#impersonation');
  if (banner) {
    banner.classList.toggle('hidden', !S.impersonatedBy);
    const target = $('#impersonated-name');
    if (target) target.textContent = S.user.username;
  }
  buildNav();
  if (isScoped()) {
    const mode = $('#mode-badge');
    if (mode) {
      mode.textContent = 'cliente';
      mode.className = 'badge';
    }
  }
  navigate(isScoped() ? 'accounts' : 'overview');
}

function isAdmin() {
  return S.user && (S.user.role === 'owner' || S.user.role === 'admin');
}

function isScoped() {
  return Boolean(S.user && S.user.account);
}

function canWrite() {
  return Boolean(S.user && ['owner', 'admin', 'operator'].includes(S.user.role));
}

const SCOPED_NAV = new Set([
  'accounts',
  'websites',
  'apps',
  'php',
  'databases',
  'ftp',
  'dns',
  'ssl',
  'backups',
]);

function buildNav() {
  const nav = $('#nav');
  let html = '';
  let group = '';
  for (const item of NAV_ITEMS) {
    if (isScoped() && !SCOPED_NAV.has(item.id)) continue;
    if (item.admin && !isAdmin()) continue;
    if (item.group !== group) {
      group = item.group;
      html += `<div class="nav-group">${esc(group)}</div>`;
    }
    const label = item.id === 'accounts' && isScoped() ? 'Mi cuenta' : item.label;
    html += `<button data-view="${item.id}"><span class="dot"></span>${esc(label)}</button>`;
  }
  nav.innerHTML = html;
  $$('#nav button').forEach((button) => {
    button.addEventListener('click', () => navigate(button.dataset.view));
  });
}

async function boot() {
  try {
    const response = await fetch('/api/v1/auth/me', { credentials: 'same-origin' });
    if (!response.ok) throw new Error('unauthenticated');
    const data = await response.json();
    S.user = data.user;
    S.csrf = data.csrf_token;
    S.impersonatedBy = data.impersonated_by || null;
    enterApp();
  } catch {
    showLogin();
  }
}

/* ---------------- navigation ---------------- */

const renderers = {
  databases: renderDatabases,
  ftp: renderFtp,
  dns: renderDns,
  backups: renderBackups,
  monitor: renderMonitor,
  security: renderSecurity,
  firewall: renderFirewall,
  programs: renderPrograms,
  console: renderConsole,
  overview: renderOverview,
  accounts: renderAccounts,
  websites: renderWebsites,
  apps: renderApps,
  php: renderPhp,
  ssl: renderSsl,
  users: renderUsers,
  audit: renderAudit,
  system: renderSystem,
};

async function navigate(id) {
  if (!renderers[id]) id = 'overview';
  S.current = id;
  const token = ++S.navToken;
  $$('#nav button').forEach((b) => b.classList.toggle('active', b.dataset.view === id));
  const [title, subtitle] = TITLES[id];
  $('#view-title').textContent = title;
  $('#view-subtitle').textContent = subtitle;
  $('#view').innerHTML = `<div class="loading">${T('Cargando…')}</div>`;
  try {
    await renderers[id]();
    translateUi(document.body);
  } catch (err) {
    if (token !== S.navToken) return;
    $('#view').innerHTML = `<div class="card empty"><strong>No se pudo cargar la vista</strong><span>${esc(err.message)}</span></div>`;
    translateUi(document.body);
  }
}

async function refresh() {
  await navigate(S.current);
}

/* ---------------- data loaders ---------------- */

const LIST_ENDPOINTS = {
  accounts: ['/api/v1/accounts', 'accounts'],
  websites: ['/api/v1/websites', 'websites'],
  apps: ['/api/v1/apps', 'apps'],
  pools: ['/api/v1/php-pools', 'pools'],
  certs: ['/api/v1/certs', 'certs'],
  databases: ['/api/v1/databases', 'databases'],
  ftp: ['/api/v1/ftp', 'accounts'],
  backups: ['/api/v1/backups', 'backups'],
  dns: ['/api/v1/dns', 'zones'],
  programs: ['/api/v1/system/programs', 'programs'],
  updates: ['/api/v1/system/updates', 'updates'],
  users: ['/api/v1/users', null],
};

async function loadList(key) {
  const [url, field] = LIST_ENDPOINTS[key];
  const data = await api(url);
  S.data[key] = field ? (data[field] || []) : (Array.isArray(data) ? data : []);
  return S.data[key];
}

async function loadSystem() {
  const data = await api('/api/v1/system');
  S.system = data.info;
  S.mode = data.info.mode;
  updateModeBadge();
  return S.system;
}

function updateModeBadge() {
  const el = $('#mode-badge');
  if (!el) return;
  if (S.mode === 'simulation') {
    el.textContent = 'simulación';
    el.className = 'badge warn';
    el.title = 'No se toca el sistema operativo real';
  } else if (S.mode === 'linux') {
    el.textContent = 'linux';
    el.className = 'badge ok';
    el.title = 'Daemon privilegiado sobre systemd/nginx';
  } else {
    el.textContent = S.mode || 'conectando';
    el.className = 'badge';
  }
}

/* ---------------- modal ---------------- */

function openModal(options) {
  const root = $('#modal-root');
  const submitLabel = options.submitLabel || 'Guardar';
  const danger = options.danger ? 'danger' : 'primary';
  root.innerHTML = `
    <div class="modal-backdrop" data-backdrop>
      <div class="modal" role="dialog" aria-modal="true">
        <h2>${esc(options.title)}</h2>
        <form id="modal-form">
          ${options.html}
          <div class="modal-actions">
            <button type="button" class="btn ghost" data-close>Cancelar</button>
            <button type="submit" class="btn ${danger}">${esc(submitLabel)}</button>
          </div>
        </form>
      </div>
    </div>`;

  translateUi(root);
  const form = $('#modal-form');
  const close = () => { root.innerHTML = ''; };

  root.querySelector('[data-close]').addEventListener('click', close);
  root.querySelector('[data-backdrop]').addEventListener('click', (event) => {
    if (event.target === event.currentTarget) close();
  });
  const onKey = (event) => {
    if (event.key === 'Escape') { close(); document.removeEventListener('keydown', onKey); }
  };
  document.addEventListener('keydown', onKey);

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const submit = form.querySelector('button[type="submit"]');
    submit.disabled = true;
    try {
      await options.onSubmit(form);
      document.removeEventListener('keydown', onKey);
      // A submit handler may have replaced the modal (e.g. showOutput).
      if (root.contains(form)) close();
    } catch (err) {
      toast(err.message, 'error');
    } finally {
      submit.disabled = false;
    }
  });

  if (options.onMount) options.onMount(form);
  const first = form.querySelector('input, select, textarea');
  if (first) setTimeout(() => first.focus(), 30);
}

function field(name, label, html, help = '', full = false) {
  return `<label class="field ${full ? 'full' : ''}"><span>${esc(label)}</span>${html}${help ? `<small class="hint">${esc(help)}</small>` : ''}</label>`;
}

function fInput(name, label, opts = {}) {
  const type = opts.type || 'text';
  const attrs = [
    `name="${esc(name)}"`,
    `type="${esc(type)}"`,
    opts.value !== undefined ? `value="${esc(opts.value)}"` : '',
    opts.placeholder ? `placeholder="${esc(opts.placeholder)}"` : '',
    opts.required ? 'required' : '',
    opts.min !== undefined ? `min="${esc(opts.min)}"` : '',
    opts.max !== undefined ? `max="${esc(opts.max)}"` : '',
    opts.step !== undefined ? `step="${esc(opts.step)}"` : '',
    opts.autocomplete ? `autocomplete="${esc(opts.autocomplete)}"` : '',
    opts.list ? `list="${esc(opts.list)}"` : '',
  ].filter(Boolean).join(' ');
  return field(name, label, `<input ${attrs}>`, opts.help, opts.full);
}

function fSelect(name, label, options, opts = {}) {
  const items = options.map((option) => {
    const value = typeof option === 'string' ? option : option.value;
    const text = typeof option === 'string' ? option : option.label;
    const selected = opts.value !== undefined && String(opts.value) === String(value) ? ' selected' : '';
    return `<option value="${esc(value)}"${selected}>${esc(text)}</option>`;
  }).join('');
  return field(name, label, `<select name="${esc(name)}">${items}</select>`, opts.help, opts.full);
}

function fCheck(name, label, checked = true) {
  return `<label class="check"><input type="checkbox" name="${esc(name)}" ${checked ? 'checked' : ''}><span>${esc(label)}</span></label>`;
}

function fTextarea(name, label, opts = {}) {
  return field(name, label, `<textarea name="${esc(name)}" placeholder="${esc(opts.placeholder || '')}">${esc(opts.value || '')}</textarea>`, opts.help, opts.full);
}

/* ---------------- shared cell builders ---------------- */

function accountOptions(includeEmpty = false) {
  let items = S.data.accounts.map((a) => ({
    value: a.spec.username,
    label: `${a.spec.username}${a.spec.enabled ? '' : ' (suspendida)'}`,
  }));
  if (S.user && S.user.account) {
    items = items.filter((item) => item.value === S.user.account);
  }
  if (includeEmpty) items.unshift({ value: '', label: '- selecciona -' });
  return items;
}

function statusBadge(enabled) {
  return enabled ? badge('activa', 'ok') : badge('suspendida', 'warn');
}

function certBadge(status) {
  const map = { active: 'ok', expiring: 'warn', expired: 'bad', missing: 'warn', error: 'bad' };
  return badge(status, map[status] || '');
}

/* ---------------- overview ---------------- */

async function renderOverview() {
  const results = await Promise.allSettled([
    loadList('accounts'),
    loadList('websites'),
    loadList('apps'),
    loadList('certs'),
    loadSystem(),
  ]);
  const failed = results.filter((r) => r.status === 'rejected');
  const system = S.system;

  const stats = [
    ['Cuentas', S.data.accounts.length, 'usuarios Linux gestionados'],
    ['Sitios web', S.data.websites.length, 'virtual hosts Nginx'],
    ['Aplicaciones', S.data.apps.length, 'procesos systemd'],
    ['Certificados', S.data.certs.filter((c) => c.status === 'active').length, 'activos'],
  ];

  let html = `<div class="grid cols-4">${stats.map(([label, value, hint]) => `
    <div class="card stat">
      <div class="label">${esc(label)}</div>
      <div class="value">${esc(value)}</div>
      <div class="hint">${esc(hint)}</div>
    </div>`).join('')}</div>`;

  if (system) {
    const memUsed = system.mem_total_kb - system.mem_available_kb;
    const diskUsed = system.disk_total_kb - system.disk_available_kb;
    html += `
      <div class="grid cols-2">
        <div class="card">
          <div class="section-head"><h2>Nodo</h2>${badge(system.mode, system.mode === 'simulation' ? 'warn' : 'ok')}</div>
          <dl class="kv" style="margin-top:14px">
            <dt>Hostname</dt><dd>${esc(system.hostname)}</dd>
            <dt>Sistema</dt><dd>${esc(system.os)}</dd>
            <dt>Kernel</dt><dd>${esc(system.kernel)}</dd>
            <dt>Uptime</dt><dd>${esc(fmtUptime(system.uptime_secs))}</dd>
            <dt>CPU</dt><dd>${esc(system.cpu_count)} vCPU</dd>
            <dt>Load (1/5/15)</dt><dd>${(system.load || [0, 0, 0]).map((v) => (v / 100).toFixed(2)).join(' / ')}</dd>
            <dt>Daemon</dt><dd>v${esc(system.daemon_version)}</dd>
          </dl>
        </div>
        <div class="card">
          <div class="section-head"><h2>Recursos</h2></div>
          <div style="margin-top:16px" class="stack">
            <div>
              <div class="muted small">Memoria · ${esc(fmtKb(memUsed))} de ${esc(fmtKb(system.mem_total_kb))}</div>
              <div class="bar" style="margin-top:6px"><i style="width:${pct(memUsed, system.mem_total_kb)}%"></i></div>
            </div>
            <div>
              <div class="muted small">Disco · ${esc(fmtKb(diskUsed))} de ${esc(fmtKb(system.disk_total_kb))}</div>
              <div class="bar" style="margin-top:6px"><i style="width:${pct(diskUsed, system.disk_total_kb)}%"></i></div>
            </div>
          </div>
        </div>
      </div>`;
  } else {
    html += `<div class="card empty"><strong>Daemon no disponible</strong><span>No se pudo leer el estado del nodo. Revisa rynxd.</span></div>`;
  }

  if (isAdmin()) {
    try {
      const data = await api('/api/v1/audit?limit=8');
      const rows = (data.entries || []).slice().reverse().map((entry) => [
        td(`<span class="chip">#${esc(entry.seq)}</span>`),
        td(esc(fmtDate(entry.at))),
        td(`<strong>${esc(entry.actor)}</strong>`),
        td(`<span class="mono small">${esc(entry.action)}</span>`),
        td(esc(entry.target)),
      ]);
      html += sectionHead('Actividad reciente');
      html += table(['#', 'Fecha', 'Actor', 'Acción', 'Objetivo'], rows, 'Sin actividad registrada.');
    } catch {
      /* daemon may be down; overview already reported it */
    }
  }

  if (failed.length) {
    html += `<p class="muted small">${failed.length} consulta(s) fallaron al cargar el resumen.</p>`;
  }
  $('#view').innerHTML = html;
}

/* ---------------- accounts ---------------- */

async function renderAccounts() {
  await loadList('accounts');
  if (isAdmin() && !isScoped()) await loadList('users');
  const rows = S.data.accounts.map((account) => {
    const spec = account.spec;
    const scoped = isScoped();
    const panelUser = isAdmin() && !scoped
      ? (S.data.users || []).find((user) => user.account === spec.username)
      : null;
    const actions = [];
    if (isAdmin() && !scoped) {
      actions.push(panelUser
        ? btn('Entrar', 'impersonate', panelUser.username, 'tiny ghost')
        : btn('Crear acceso', 'account-panel-access', spec.username, 'tiny ghost'));
    }
    if (!scoped) {
      actions.push(btn(spec.enabled ? 'Suspender' : 'Activar', 'account-toggle', spec.username));
      actions.push(btn('Eliminar', 'account-delete', spec.username, 'tiny danger'));
    }
    return [
      td(`<div class="cell-title">${esc(spec.username)}</div><div class="cell-sub">${esc(spec.email)}</div>`),
      td(`<span class="chip">UID ${esc(account.uid)}</span>`),
      td(`${esc(spec.disk_limit_mb)} MB`),
      td(spec.php_version ? `PHP ${esc(spec.php_version)}` : '<span class="muted">—</span>'),
      td(statusBadge(spec.enabled)),
      td(`<span class="mono small">${esc(account.home)}</span>`),
      td(actions.join(''), 'actions'),
    ];
  });

  const scoped = isScoped();
  $('#view').innerHTML =
    sectionHead(scoped ? 'Mi cuenta' : 'Cuentas de hosting',
      scoped ? '' : `<button class="btn primary" data-action="account-new">Nueva cuenta</button>`) +
    table(['Cuenta', 'UID', 'Disco', 'PHP', 'Estado', 'Home', ''], rows,
      scoped ? 'No tienes una cuenta vinculada.' : 'Crea la primera cuenta para empezar.');
}

function openAccountModal() {
  openModal({
    title: 'Nueva cuenta de hosting',
    submitLabel: 'Crear cuenta',
    html: `<div class="form-grid">
      ${fInput('username', 'Usuario del sistema', { required: true, placeholder: 'acme', help: 'minúsculas, 3-24 caracteres; se crea un usuario Linux' })}
      ${fInput('email', 'Email', { type: 'email', required: true, placeholder: 'admin@example.com' })}
      ${fInput('disk_limit_mb', 'Cuota de disco (MB)', { type: 'number', value: 1024, min: 64, required: true })}
      ${fSelect('php_version', 'PHP por defecto', [{ value: '', label: '— sin pool —' }, ...RUNTIMES.php.versions.map((v) => ({ value: v, label: `PHP ${v}` }))], { value: '8.3' })}
      <div class="full">${fCheck('enabled', 'Cuenta activa', true)}</div>
      ${fSelect('panel_access', 'Acceso al panel del cliente', [
        { value: 'viewer', label: 'Solo lectura (viewer)' },
        { value: 'operator', label: 'Operador (gestiona su cuenta)' },
        { value: 'none', label: 'Sin acceso al panel' },
      ], { value: 'viewer', help: 'El operador puede gestionar los recursos de su propia cuenta; el viewer es solo lectura.' })}
    </div>`,
    onSubmit: async (form) => {
      const access = val(form, 'panel_access');
      const body = {
        username: val(form, 'username').toLowerCase(),
        email: toAsciiEmail(val(form, 'email')),
        disk_limit_mb: num(form, 'disk_limit_mb', 1024),
        php_version: val(form, 'php_version') || null,
        enabled: bool(form, 'enabled'),
        panel_access: access !== 'none',
      };
      if (access !== 'none') body.panel_role = access;
      const result = await api('/api/v1/accounts', { method: 'POST', body });
      if (result.panel_password) {
        showOutput(
          T('Cuenta y acceso creados'),
          `${T('usuario del panel:')} ${result.panel_user.username}\n` +
            `${T('contraseña:')} ${result.panel_password}\n\n` +
            `${T('Entra en')} ${location.origin} ${T('con esas credenciales.')}\n` +
            T('Guarda la contraseña ahora; no se volverá a mostrar.'),
        );
      } else {
        toast('Cuenta creada');
      }
      await refresh();
    },
  });
}

/* ---------------- websites ---------------- */

async function renderWebsites() {
  await Promise.all([loadList('websites'), loadList('accounts'), loadList('apps')]);
  const writable = canWrite();
  const rows = S.data.websites.map((website) => {
    const spec = website.spec;
    const aliases = spec.aliases && spec.aliases.length
      ? `<div class="cell-sub">+ ${esc(spec.aliases.join(', '))}</div>` : '';
    return [
      td(`<div class="cell-title">${esc(spec.domain)}</div>${aliases}`),
      td(esc(spec.username)),
      td(`<span class="chip">${esc(runtimeLabel(spec.runtime))}</span>${spec.app ? `<div class="cell-sub">app: ${esc(spec.app)}</div>` : ''}`),
      td(spec.tls === 'disabled' ? badge('sin tls', 'warn') : badge(spec.tls === 'lets_encrypt' ? 'let\'s encrypt' : 'manual', 'info')),
      td(spec.enabled ? badge('activo', 'ok') : badge('deshabilitado', 'warn')),
      td(`<span class="mono small">${esc(spec.docroot)}</span>`),
      td(writable ? btn('Eliminar', 'website-delete', spec.domain, 'tiny danger') : '', 'actions'),
    ];
  });

  $('#view').innerHTML =
    sectionHead('Sitios web', writable ? `<button class="btn primary" data-action="website-new">Nuevo sitio</button>` : '') +
    table(['Dominio', 'Cuenta', 'Runtime', 'TLS', 'Estado', 'Docroot', ''], rows, 'Aún no hay virtual hosts.');
}

function openWebsiteModal() {
  if (!S.data.accounts.length) {
    toast('Primero crea una cuenta de hosting', 'error');
    return;
  }
  const engineOptions = ['static', 'php', ...MANAGED].map((engine) => ({
    value: engine,
    label: RUNTIMES[engine].label,
  }));
  const accountItems = accountOptions(false);

  openModal({
    title: 'Nuevo sitio web',
    submitLabel: 'Publicar sitio',
    html: `<div class="form-grid">
      ${fInput('domain', 'Dominio', { required: true, placeholder: 'example.com', help: 'se valida y normaliza en el daemon' })}
      ${fInput('aliases', 'Alias', { placeholder: 'www.example.com, cdn.example.com', help: 'separados por coma' })}
      ${fSelect('username', 'Cuenta', accountItems, { required: true })}
      ${fSelect('engine', 'Runtime', engineOptions, { value: 'php' })}
      ${fSelect('version', 'Versión', RUNTIMES.php.versions.map((v) => ({ value: v, label: v })), { value: '8.3' })}
      ${fSelect('app', 'Aplicación', [{ value: '', label: '— selecciona —' }], { help: 'obligatoria para runtimes gestionados' })}
      ${fInput('docroot', 'Document root', { value: 'public_html', required: true, help: 'relativo al home de la cuenta' })}
      ${fSelect('tls', 'TLS', [
        { value: 'disabled', label: 'Sin TLS' },
        { value: 'lets_encrypt', label: "Let's Encrypt" },
        { value: 'manual', label: 'Certificado manual' },
      ], { value: 'lets_encrypt' })}
      <div class="full">${fCheck('enabled', 'Sitio activo', true)}</div>
    </div>`,
    onMount: (form) => {
      const engine = form.elements.engine;
      const version = form.elements.version;
      const app = form.elements.app;
      const username = form.elements.username;

      const syncVersions = () => {
        const versions = (RUNTIMES[engine.value] && RUNTIMES[engine.value].versions) || [];
        version.innerHTML = versions.length
          ? versions.map((v) => `<option value="${esc(v)}">${esc(v)}</option>`).join('')
          : '<option value="">—</option>';
        version.disabled = !versions.length;
      };
      const syncApps = () => {
        const managed = MANAGED.includes(engine.value);
        const items = S.data.apps.filter((item) => item.spec.username === username.value
          && item.spec.runtime.engine === engine.value);
        app.innerHTML = managed
          ? (items.length
            ? items.map((item) => `<option value="${esc(item.spec.name)}">${esc(item.spec.name)} · ${esc(item.spec.port)}</option>`).join('')
            : '<option value="">— sin aplicaciones compatibles —</option>')
          : '<option value="">— no aplica —</option>';
        app.disabled = !managed || !items.length;
      };

      engine.addEventListener('change', () => { syncVersions(); syncApps(); });
      username.addEventListener('change', syncApps);
      syncVersions();
      syncApps();
    },
    onSubmit: async (form) => {
      const engine = val(form, 'engine');
      const version = val(form, 'version');
      const runtime = (engine === 'static' || engine === 'go')
        ? { engine }
        : { engine, version };
      const managed = MANAGED.includes(engine);
      const app = managed ? val(form, 'app') : null;
      if (managed && !app) throw new Error('Selecciona una aplicación compatible o créala antes');

      await api('/api/v1/websites', {
        method: 'POST',
        body: {
          domain: toAsciiDomain(val(form, 'domain')),
          username: val(form, 'username'),
          aliases: val(form, 'aliases').split(',').map((a) => toAsciiDomain(a)).filter(Boolean),
          runtime,
          app,
          docroot: val(form, 'docroot'),
          tls: val(form, 'tls'),
          enabled: bool(form, 'enabled'),
          locale: window.RYNX_I18N ? window.RYNX_I18N.getLanguage() : 'en',
        },
      });
      toast('Sitio publicado');
      await refresh();
    },
  });
}

/* ---------------- applications ---------------- */

async function renderApps() {
  await Promise.all([loadList('apps'), loadList('accounts')]);
  const writable = canWrite();
  const rows = S.data.apps.map((app) => {
    const spec = app.spec;
    return [
      td(`<div class="cell-title">${esc(spec.name)}</div><div class="cell-sub">${esc(spec.username)}</div>`),
      td(`<span class="chip">${esc(runtimeLabel(spec.runtime))}</span>`),
      td(`<span class="mono">127.0.0.1:${esc(spec.port)}</span>`),
      td(`<span class="mono small">${esc(spec.command)} ${esc((spec.args || []).join(' '))}</span>`),
      td(spec.enabled ? badge('activa', 'ok') : badge('detenida', 'warn')),
      td(writable ? btn('Eliminar', 'app-delete', `${spec.username}/${spec.name}`, 'tiny danger') : '', 'actions'),
    ];
  });

  $('#view').innerHTML =
    sectionHead('Aplicaciones gestionadas', writable ? `<button class="btn primary" data-action="app-new">Nueva aplicación</button>` : '') +
    table(['Aplicación', 'Runtime', 'Escucha', 'Comando', 'Estado', ''], rows, 'Node, Python, Ruby, Java, .NET y Go se ejecutan bajo systemd.');
}

function openAppModal() {
  if (!S.data.accounts.length) {
    toast('Primero crea una cuenta de hosting', 'error');
    return;
  }
  const engineOptions = MANAGED.map((engine) => ({ value: engine, label: RUNTIMES[engine].label }));

  openModal({
    title: 'Nueva aplicación',
    submitLabel: 'Crear aplicación',
    html: `<div class="form-grid">
      ${fSelect('username', 'Cuenta', accountOptions(false), { required: true })}
      ${fInput('name', 'Nombre', { required: true, placeholder: 'api', help: 'minúsculas, 2-32 caracteres' })}
      ${fSelect('engine', 'Runtime', engineOptions, { value: 'node' })}
      ${fSelect('version', 'Versión', RUNTIMES.node.versions.map((v) => ({ value: v, label: v })), { value: '22' })}
      ${fInput('port', 'Puerto interno', { type: 'number', value: 4100, min: 1024, max: 65535, required: true, help: 'solo loopback; Nginx hará proxy' })}
      ${fInput('entrypoint', 'Entrypoint', { value: 'server.js', required: true, help: 'archivo o artefacto relativo a la app' })}
      ${fInput('command', 'Comando', { value: 'node', required: true, list: 'app-commands', help: 'ejecutable permitido para el runtime' })}
      <datalist id="app-commands">
        <option value="node"></option>
        <option value="gunicorn"></option>
        <option value="uvicorn"></option>
        <option value="python3"></option>
        <option value="puma"></option>
        <option value="bundle"></option>
        <option value="ruby"></option>
        <option value="java"></option>
        <option value="dotnet"></option>
      </datalist>
      ${fInput('args', 'Argumentos', { placeholder: 'server.js', help: 'separados por coma' })}
      ${fTextarea('env', 'Variables de entorno', { placeholder: 'NODE_ENV=production\nAPP_KEY=...', help: 'una KEY=VALUE por línea', full: true })}
      <div class="full">${fCheck('enabled', 'Arrancar aplicación', true)}</div>
    </div>`,
    onMount: (form) => {
      const engine = form.elements.engine;
      const version = form.elements.version;
      const entrypoint = form.elements.entrypoint;
      const command = form.elements.command;
      const args = form.elements.args;
      const port = form.elements.port;

      const sync = () => {
        const meta = RUNTIMES[engine.value] || { versions: [], commands: [] };
        version.innerHTML = meta.versions.length
          ? meta.versions.map((v) => `<option value="${esc(v)}">${esc(v)}</option>`).join('')
          : '<option value="">—</option>';
        version.disabled = !meta.versions.length;
        if (meta.commands.length) command.value = meta.commands[0];
        if (meta.entry) entrypoint.value = meta.entry;
        if (meta.exampleArgs !== undefined) {
          args.value = meta.exampleArgs.replaceAll('4100', String(port.value || 4100));
        }
      };
      engine.addEventListener('change', sync);
      port.addEventListener('input', () => {
        const meta = RUNTIMES[engine.value];
        if (meta && meta.exampleArgs) {
          args.value = meta.exampleArgs.replaceAll('4100', String(port.value || 4100));
        }
      });
      sync();
    },
    onSubmit: async (form) => {
      const engine = val(form, 'engine');
      const version = val(form, 'version');
      const runtime = (engine === 'go') ? { engine } : { engine, version };
      const env = {};
      for (const line of val(form, 'env').split('\n')) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const index = trimmed.indexOf('=');
        if (index <= 0) throw new Error(`Variable de entorno inválida: ${trimmed}`);
        env[trimmed.slice(0, index).trim()] = trimmed.slice(index + 1);
      }
      await api('/api/v1/apps', {
        method: 'POST',
        body: {
          username: val(form, 'username'),
          name: val(form, 'name').toLowerCase(),
          runtime,
          port: num(form, 'port', 4100),
          entrypoint: val(form, 'entrypoint'),
          command: val(form, 'command'),
          args: val(form, 'args').split(',').map((a) => a.trim()).filter(Boolean),
          env,
          enabled: bool(form, 'enabled'),
        },
      });
      toast('Aplicación creada');
      await refresh();
    },
  });
}

/* ---------------- php ---------------- */

async function renderPhp() {
  await Promise.all([loadList('pools'), loadList('accounts')]);
  const writable = canWrite();
  const rows = S.data.pools.map((pool) => {
    const spec = pool.spec;
    return [
      td(`<div class="cell-title">${esc(spec.username)}</div>`),
      td(`<span class="chip">PHP ${esc(spec.version)}</span>`),
      td(esc(spec.process_manager)),
      td(esc(spec.max_children)),
      td(`${esc(spec.memory_limit_mb)} MB`),
      td(`<span class="mono small">${esc(pool.socket)}</span>`),
      td(statusBadge(spec.enabled)),
      td(writable ? btn('Eliminar', 'php-delete', spec.username, 'tiny danger') : '', 'actions'),
    ];
  });

  $('#view').innerHTML =
    sectionHead('Pools PHP-FPM', writable ? `<button class="btn primary" data-action="php-new">Nuevo pool</button>` : '') +
    table(['Cuenta', 'Versión', 'PM', 'Workers', 'Memoria', 'Socket', 'Estado', ''], rows, 'No hay pools; se crean automáticamente al publicar un sitio PHP.');
}

function openPhpModal() {
  if (!S.data.accounts.length) {
    toast('Primero crea una cuenta de hosting', 'error');
    return;
  }
  openModal({
    title: 'Nuevo pool PHP-FPM',
    submitLabel: 'Crear pool',
    html: `<div class="form-grid">
      ${fSelect('username', 'Cuenta', accountOptions(false), { required: true })}
      ${fSelect('version', 'Versión PHP', RUNTIMES.php.versions.map((v) => ({ value: v, label: v })), { value: '8.3' })}
      ${fSelect('process_manager', 'Gestor de procesos', [
        { value: 'dynamic', label: 'dynamic' },
        { value: 'ondemand', label: 'ondemand' },
        { value: 'static', label: 'static' },
      ], { value: 'dynamic' })}
      ${fInput('max_children', 'Max children', { type: 'number', value: 8, min: 1, max: 64, required: true })}
      ${fInput('memory_limit_mb', 'Memoria por worker (MB)', { type: 'number', value: 256, min: 16, max: 2048, required: true })}
      ${fInput('upload_max_mb', 'Subida máxima (MB)', { type: 'number', value: 64, min: 1, max: 4096, required: true })}
      <div class="full">${fCheck('enabled', 'Pool activo', true)}</div>
    </div>`,
    onSubmit: async (form) => {
      await api('/api/v1/php-pools', {
        method: 'POST',
        body: {
          username: val(form, 'username'),
          version: val(form, 'version'),
          process_manager: val(form, 'process_manager'),
          max_children: num(form, 'max_children', 8),
          memory_limit_mb: num(form, 'memory_limit_mb', 256),
          upload_max_mb: num(form, 'upload_max_mb', 64),
          enabled: bool(form, 'enabled'),
        },
      });
      toast('Pool PHP actualizado');
      await refresh();
    },
  });
}

/* ---------------- ssl ---------------- */

async function renderSsl() {
  await Promise.all([loadList('certs'), loadList('websites')]);
  const writable = canWrite();
  const scoped = isScoped();
  const rows = S.data.certs.map((cert) => [
    td(`<div class="cell-title">${esc(cert.domain)}</div><div class="cell-sub mono">${esc(cert.path)}</div>`),
    td(esc(cert.issuer || '—')),
    td(esc(fmtDate(cert.not_after))),
    td(certBadge(cert.status)),
  ]);

  const renew = scoped ? '' : '<button class="btn ghost" data-action="cert-renew">Renovar todos</button>';
  const issue = writable ? '<button class="btn primary" data-action="cert-new">Emitir certificado</button>' : '';
  $('#view').innerHTML =
    sectionHead('Certificados TLS', renew + issue) +
    table(['Dominio', 'Emisor', 'Vence', 'Estado'], rows, 'No hay certificados; publica un sitio con TLS Let\'s Encrypt.');
}

function openCertModal() {
  const candidates = S.data.websites.filter((w) => w.spec.tls === 'lets_encrypt');
  if (!candidates.length) {
    toast('No hay sitios configurados con Let\'s Encrypt', 'error');
    return;
  }
  openModal({
    title: 'Emitir certificado',
    submitLabel: 'Solicitar',
    html: `<div class="form-grid">
      ${fSelect('domain', 'Sitio', candidates.map((w) => ({ value: w.spec.domain, label: w.spec.domain })), { required: true })}
      ${fInput('email', 'Email de contacto', { type: 'email', value: S.user.email, required: true })}
      <div class="full">${fCheck('staging', 'Usar entorno staging (pruebas)', false)}</div>
      <p class="hint full">El reto HTTP-01 se sirve desde el webroot ACME. Al emitirse, el vhost se re-renderiza con TLS.</p>
    </div>`,
    onSubmit: async (form) => {
      await api('/api/v1/certs', {
        method: 'POST',
        body: {
          domain: val(form, 'domain'),
          email: toAsciiEmail(val(form, 'email')),
          staging: bool(form, 'staging'),
        },
      });
      toast('Certificado emitido');
      await refresh();
    },
  });
}

/* ---------------- users ---------------- */

async function renderUsers() {
  await loadList('users');
  const rows = S.data.users.map((user) => {
    const self = user.username === S.user.username;
    return [
      td(`<div class="cell-title">${esc(user.username)}</div>${user.account ? `<div class="cell-sub">cuenta: ${esc(user.account)}</div>` : ''}${self ? '<div class="cell-sub">tú</div>' : ''}`),
      td(esc(user.email)),
      td(badge(user.role, user.role === 'owner' ? 'info' : '')),
      td(user.active ? badge('activo', 'ok') : badge('deshabilitado', 'warn')),
      td([
        self ? '' : btn('Entrar', 'impersonate', user.username),
        btn('Contraseña', 'user-passwd', user.username),
        self ? '' : btn(user.active ? 'Desactivar' : 'Activar', 'user-toggle', user.username),
        self ? '' : btn('Eliminar', 'user-delete', user.username, 'tiny danger'),
      ].join(''), 'actions'),
    ];
  });

  $('#view').innerHTML =
    sectionHead('Usuarios del panel', `<button class="btn primary" data-action="user-new">Nuevo usuario</button>`) +
    table(['Usuario', 'Email', 'Rol', 'Estado', ''], rows, 'Solo existe el propietario inicial.');
}

function openUserModal() {
  const roles = isOwner() ? ['owner', 'admin', 'operator', 'viewer'] : ['admin', 'operator', 'viewer'];
  openModal({
    title: 'Nuevo usuario del panel',
    submitLabel: 'Crear usuario',
    html: `<div class="form-grid">
      ${fInput('username', 'Usuario', { required: true, placeholder: 'operador' })}
      ${fInput('email', 'Email', { type: 'email', required: true })}
      ${fInput('password', 'Contraseña', { type: 'password', required: true, autocomplete: 'new-password', help: 'mínimo 12 caracteres, 3 clases' })}
      ${fSelect('role', 'Rol', roles.map((r) => ({ value: r, label: r })), { value: 'operator' })}
    </div>`,
    onSubmit: async (form) => {
      await api('/api/v1/users', {
        method: 'POST',
        body: {
          username: val(form, 'username').toLowerCase(),
          email: toAsciiEmail(val(form, 'email')),
          password: val(form, 'password'),
          role: val(form, 'role'),
        },
      });
      toast('Usuario creado');
      await refresh();
    },
  });
}

function isOwner() {
  return S.user && S.user.role === 'owner';
}

/* ---------------- audit ---------------- */

async function renderAudit() {
  const data = await api('/api/v1/audit?limit=250');
  const entries = (data.entries || []).slice().reverse();
  const rows = entries.map((entry) => [
    td(`<span class="chip">#${esc(entry.seq)}</span>`),
    td(esc(fmtDate(entry.at))),
    td(`<strong>${esc(entry.actor)}</strong>`),
    td(`<span class="mono small">${esc(entry.action)}</span>`),
    td(esc(entry.target)),
    td(`<span class="muted small">${esc(entry.detail)}</span>`),
  ]);

  $('#view').innerHTML =
    sectionHead('Registro encadenado', `<button class="btn ghost" data-action="reload">Actualizar</button>`) +
    table(['#', 'Fecha', 'Actor', 'Acción', 'Objetivo', 'Detalle'], rows, 'Sin eventos.');
}

/* ---------------- system ---------------- */

async function renderSystem() {
  const info = await loadSystem();
  const memUsed = info.mem_total_kb - info.mem_available_kb;
  const diskUsed = info.disk_total_kb - info.disk_available_kb;

  let html = `<div class="grid cols-2">
    <div class="card">
      <div class="section-head"><h2>Identidad</h2>${badge(info.mode, info.mode === 'simulation' ? 'warn' : 'ok')}</div>
      <dl class="kv" style="margin-top:14px">
        <dt>Hostname</dt><dd>${esc(info.hostname)}</dd>
        <dt>Sistema</dt><dd>${esc(info.os)}</dd>
        <dt>Kernel</dt><dd>${esc(info.kernel)}</dd>
        <dt>Uptime</dt><dd>${esc(fmtUptime(info.uptime_secs))}</dd>
        <dt>vCPU</dt><dd>${esc(info.cpu_count)}</dd>
        <dt>Load</dt><dd>${(info.load || [0, 0, 0]).map((v) => (v / 100).toFixed(2)).join(' / ')}</dd>
        <dt>Daemon</dt><dd>v${esc(info.daemon_version)}</dd>
      </dl>
    </div>
    <div class="card">
      <div class="section-head"><h2>Capacidad</h2></div>
      <div class="stack" style="margin-top:16px">
        <div>
          <div class="muted small">Memoria · ${esc(fmtKb(memUsed))} / ${esc(fmtKb(info.mem_total_kb))}</div>
          <div class="bar" style="margin-top:6px"><i style="width:${pct(memUsed, info.mem_total_kb)}%"></i></div>
        </div>
        <div>
          <div class="muted small">Disco · ${esc(fmtKb(diskUsed))} / ${esc(fmtKb(info.disk_total_kb))}</div>
          <div class="bar" style="margin-top:6px"><i style="width:${pct(diskUsed, info.disk_total_kb)}%"></i></div>
        </div>
      </div>
    </div>
  </div>`;

  html += `<div class="card">
    <div class="section-head"><h2>Reconciliación</h2>
      ${isAdmin() ? '<button class="btn primary" data-action="reconcile">Re-aplicar estado</button>' : badge('solo lectura', '')}
    </div>
    <p class="muted small" style="margin:12px 0 0">
      El motor declarativo vuelve a renderizar vhosts, pools, unidades systemd y permisos
      a partir del estado persistido. Úsalo tras cambios manuales o recuperaciones.
    </p>
  </div>`;

  $('#view').innerHTML = html;
}

/* ---------------- actions ---------------- */

async function handleAction(action, id) {
  switch (action) {
    case 'reload':
      return refresh();
    case 'account-new':
      return openAccountModal();
    case 'account-toggle': {
      const account = S.data.accounts.find((a) => a.spec.username === id);
      if (!account) throw new Error('Cuenta no encontrada');
      await api('/api/v1/accounts', {
        method: 'POST',
        body: { ...account.spec, enabled: !account.spec.enabled },
      });
      toast(account.spec.enabled ? 'Cuenta suspendida' : 'Cuenta activada');
      return refresh();
    }
    case 'account-panel-access': {
      const account = S.data.accounts.find((a) => a.spec.username === id);
      if (!account) throw new Error('Cuenta no encontrada');
      const result = await api('/api/v1/accounts', {
        method: 'POST',
        body: { ...account.spec, panel_access: true, panel_role: 'viewer' },
      });
      if (result.panel_password) {
        showOutput(
          T('Acceso al panel creado'),
          `${T('usuario:')} ${result.panel_user.username}\n` +
            `${T('contraseña:')} ${result.panel_password}\n\n` +
            `${T('Entra en')} ${location.origin}.\n` +
            T('El acceso es de solo lectura (viewer).'),
        );
      } else {
        toast('El acceso ya existía');
      }
      return refresh();
    }
    case 'account-delete': {
      if (!confirm(`¿Eliminar la cuenta ${id}? Se borrarán el home, sitios, apps, bases de datos, DNS, FTP, certificados y backups.`)) return;
      await api(`/api/v1/accounts/${encodeURIComponent(id)}?purge=true`, { method: 'DELETE' });
      toast('Cuenta eliminada');
      return refresh();
    }
    case 'website-new':
      return openWebsiteModal();
    case 'website-delete': {
      if (!confirm(`¿Eliminar el sitio ${id}?`)) return;
      await api(`/api/v1/websites/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Sitio eliminado');
      return refresh();
    }
    case 'app-new':
      return openAppModal();
    case 'app-delete': {
      if (!confirm(`¿Eliminar la aplicación ${id}?`)) return;
      const [user, name] = id.split('/');
      await api(`/api/v1/apps/${encodeURIComponent(user)}/${encodeURIComponent(name)}`, { method: 'DELETE' });
      toast('Aplicación eliminada');
      return refresh();
    }
    case 'php-new':
      return openPhpModal();
    case 'php-delete': {
      if (!confirm(`¿Eliminar el pool PHP de ${id}?`)) return;
      await api(`/api/v1/php-pools/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Pool eliminado');
      return refresh();
    }
    case 'cert-new':
      return openCertModal();
    case 'user-new':
      return openUserModal();
    case 'user-toggle': {
      const user = S.data.users.find((u) => u.username === id);
      if (!user) throw new Error('Usuario no encontrado');
      if (user.username === S.user.username) throw new Error('No puedes desactivar tu propia cuenta');
      await api(`/api/v1/users/${encodeURIComponent(id)}`, {
        method: 'PATCH',
        body: { active: !user.active },
      });
      toast(user.active ? 'Usuario desactivado' : 'Usuario activado');
      return refresh();
    }
    case 'user-passwd': {
      openModal({
        title: `Nueva contraseña · ${id}`,
        submitLabel: 'Actualizar',
        html: `<div class="stack">${fInput('password', 'Contraseña', { type: 'password', required: true, autocomplete: 'new-password', help: 'mínimo 12 caracteres, 3 clases; revoca sesiones' })}</div>`,
        onSubmit: async (form) => {
          await api(`/api/v1/users/${encodeURIComponent(id)}`, {
            method: 'PATCH',
            body: { password: val(form, 'password') },
          });
          toast('Contraseña actualizada');
        },
      });
      return;
    }
    case 'user-delete': {
      if (!confirm(`¿Eliminar al usuario ${id}? Si tiene una cuenta de hosting, también se borrarán su cuenta y todos sus datos.`)) return;
      await api(`/api/v1/users/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Usuario eliminado');
      return refresh();
    }
    case 'reconcile': {
      const result = await api('/api/v1/reconcile', { method: 'POST' });
      toast(result.message || 'Estado re-aplicado');
      return refresh();
    }
    case 'impersonate': {
      const data = await api(`/api/v1/users/${encodeURIComponent(id)}/impersonate`, { method: 'POST' });
      S.user = data.user;
      S.csrf = data.csrf_token;
      S.impersonatedBy = data.impersonated_by || null;
      enterApp();
      toast(`${T('Ahora viendo el panel como')} ${data.user.username}`);
      return;
    }
    case 'cert-renew': {
      const result = await api('/api/v1/certs/renew', { method: 'POST' });
      toast(result.message || 'Certificados renovados');
      return refresh();
    }
    case 'reveal': {
      const el = document.querySelector(`[data-secret="${id}"]`);
      if (el) {
        el.textContent = el.dataset.value || '';
        el.classList.remove('secret');
      }
      return;
    }
    case 'database-new':
      return openDatabaseModal();
    case 'database-delete': {
      if (!confirm('¿Eliminar esta base de datos y su usuario? Los datos se perderán.')) return;
      await api(`/api/v1/databases/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Base de datos eliminada');
      return refresh();
    }
    case 'ftp-new':
      return openFtpModal();
    case 'ftp-delete': {
      if (!confirm('¿Eliminar este usuario FTP?')) return;
      await api(`/api/v1/ftp/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Usuario FTP eliminado');
      return refresh();
    }
    case 'dns-record-new':
      return openDnsRecordModal();
    case 'dns-record-delete': {
      const split = id.indexOf('/');
      const domain = id.slice(0, split);
      const recordId = id.slice(split + 1);
      if (!confirm('¿Eliminar este registro DNS?')) return;
      await api(`/api/v1/dns/${encodeURIComponent(domain)}/records/${encodeURIComponent(recordId)}`, { method: 'DELETE' });
      toast('Registro eliminado');
      return refresh();
    }
    case 'dns-mail': {
      if (!S.dnsDomain) return;
      await api(`/api/v1/dns/${encodeURIComponent(S.dnsDomain)}/mail`, { method: 'POST' });
      toast('SPF, DKIM, DMARC y MX provisionados');
      return refresh();
    }
    case 'backup-new':
      return openBackupModal();
    case 'backup-delete': {
      if (!confirm('¿Eliminar esta copia de seguridad?')) return;
      await api(`/api/v1/backups/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Backup eliminado');
      return refresh();
    }
    case 'backup-restore': {
      if (!confirm('¿Restaurar esta copia? Se sobrescribirán archivos del home y la configuración gestionada.')) return;
      const result = await api(`/api/v1/backups/${encodeURIComponent(id)}/restore`, { method: 'POST' });
      toast(result.message || 'Backup restaurado');
      return refresh();
    }
    case 'firewall-rule-new':
      return openFirewallRuleModal();
    case 'firewall-rule-delete': {
      if (!confirm('¿Eliminar esta regla del cortafuegos?')) return;
      await api(`/api/v1/firewall/rules/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('Regla eliminada');
      return refresh();
    }
    case 'firewall-block-new':
      return openFirewallBlockModal();
    case 'firewall-unblock': {
      await api(`/api/v1/firewall/block/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('IP desbloqueada');
      return refresh();
    }
    case 'firewall-whitelist-new':
      return openWhitelistModal();
    case 'firewall-whitelist-remove': {
      await api(`/api/v1/firewall/whitelist/${encodeURIComponent(id)}`, { method: 'DELETE' });
      toast('IP retirada de la lista blanca');
      return refresh();
    }
    case 'updates-apply': {
      if (!confirm('¿Actualizar todos los paquetes del sistema ahora? Puede reiniciar servicios.')) return;
      const result = await api('/api/v1/system/updates/apply', { method: 'POST' });
      showOutput('Actualización completada', result.output || 'Sin salida.');
      return refresh();
    }
    default:
      throw new Error(`Acción desconocida: ${action}`);
  }
}

/* ---------------- databases ---------------- */

async function renderDatabases() {
  await Promise.all([loadList('databases'), loadList('websites')]);
  const writable = canWrite();
  const rows = S.data.databases.map((db) => [
    td(`<div class="cell-title">${esc(db.domain)}</div><div class="cell-sub">${esc(db.host)}</div>`),
    td(`<span class="mono small">${esc(db.username)}</span>`),
    td(`<span class="secret mono small" data-secret="${esc(db.id)}" data-value="${esc(db.password)}">${esc(db.password)}</span> ${btn('Ver', 'reveal', db.id)}`),
    td(esc(db.engine)),
    td(esc(fmtDate(db.created_at))),
    td(writable ? btn('Eliminar', 'database-delete', db.id, 'tiny danger') : '', 'actions'),
  ]);
  $('#view').innerHTML =
    sectionHead('Bases de datos por dominio', writable ? '<button class="btn primary" data-action="database-new">Nueva base</button>' : '') +
    table(['Dominio', 'Usuario', 'Contraseña', 'Motor', 'Creada', ''], rows, 'Publica un sitio y crea su base de datos aislada.');
}

function openDatabaseModal() {
  const candidates = S.data.websites.filter((w) => !S.data.databases.some((db) => db.domain === w.spec.domain));
  if (!candidates.length) {
    toast('Todos los sitios ya tienen base de datos', 'error');
    return;
  }
  openModal({
    title: 'Nueva base de datos',
    submitLabel: 'Crear base',
    html: `<div class="form-grid">
      ${fSelect('domain', 'Dominio', candidates.map((w) => ({ value: w.spec.domain, label: w.spec.domain })), { required: true })}
      <p class="hint full">Se crea una base utf8mb4 y un usuario MariaDB con contraseña aleatoria de 32 caracteres. Solo puede acceder por socket local.</p>
    </div>`,
    onSubmit: async (form) => {
      const result = await api('/api/v1/databases', { method: 'POST', body: { domain: val(form, 'domain') } });
      const db = result.database;
      showOutput('Base de datos creada', `dominio: ${db.domain}\nusuario: ${db.username}\ncontraseña: ${db.password}\nhost: ${db.host}\n\nGuarda la contraseña ahora.`);
      await refresh();
    },
  });
}

/* ---------------- ftp ---------------- */

async function renderFtp() {
  await Promise.all([loadList('ftp'), loadList('websites')]);
  const writable = canWrite();
  const rows = S.data.ftp.map((account) => [
    td(`<div class="cell-title">${esc(account.login)}</div><div class="cell-sub">${esc(account.domain)}</div>`),
    td(`<span class="mono small">${esc(account.home)}</span>`),
    td(account.readonly ? badge('solo lectura', 'warn') : badge('lectura/escritura', 'ok')),
    td(`<span class="secret mono small" data-secret="${esc(account.id)}" data-value="${esc(account.password)}">${esc(account.password)}</span> ${btn('Ver', 'reveal', account.id)}`),
    td(esc(fmtDate(account.created_at))),
    td(writable ? btn('Eliminar', 'ftp-delete', account.id, 'tiny danger') : '', 'actions'),
  ]);
  $('#view').innerHTML =
    sectionHead('Usuarios FTP', writable ? '<button class="btn primary" data-action="ftp-new">Nuevo usuario FTP</button>' : '') +
    table(['Login', 'Home (chroot)', 'Permisos', 'Contraseña', 'Creado', ''], rows, 'Crea usuarios FTPS con chroot al document root del sitio.');
}

function openFtpModal() {
  if (!S.data.websites.length) {
    toast('Primero publica un sitio', 'error');
    return;
  }
  openModal({
    title: 'Nuevo usuario FTP',
    submitLabel: 'Crear usuario',
    html: `<div class="form-grid">
      ${fSelect('domain', 'Sitio', S.data.websites.map((w) => ({ value: w.spec.domain, label: w.spec.domain })), { required: true })}
      ${fInput('username', 'Nombre corto', { required: true, placeholder: 'deploy', help: 'se usará como ftp_<nombre>' })}
      ${fInput('password', 'Contraseña', { type: 'password', placeholder: 'vacío = generar', help: 'mínimo 12 caracteres, 3 clases', autocomplete: 'new-password' })}
      <div class="full">${fCheck('readonly', 'Solo lectura (sin escritura en el docroot)', false)}</div>
    </div>`,
    onSubmit: async (form) => {
      const body = { domain: val(form, 'domain'), username: val(form, 'username').toLowerCase(), readonly: bool(form, 'readonly') };
      const password = val(form, 'password');
      if (password) body.password = password;
      const result = await api('/api/v1/ftp', { method: 'POST', body });
      showOutput('Usuario FTP creado', `login: ${result.account.login}\ncontraseña: ${result.account.password}\nhome: ${result.account.home}\n\nUsa FTPS explícito (puerto 21).`);
      await refresh();
    },
  });
}

/* ---------------- dns ---------------- */

async function renderDns() {
  const data = await api('/api/v1/dns');
  const writable = canWrite();
  const zones = data.zones || [];
  if (!zones.length) {
    $('#view').innerHTML = `<div class="card empty"><strong>Sin zonas DNS</strong><span>Publica un sitio con DNS habilitado para generar automáticamente SOA, NS, A, SPF, DKIM, DMARC y MX.</span></div>`;
    return;
  }
  const selected = zones.find((zone) => zone.domain === S.dnsDomain) || zones[0];
  S.dnsDomain = selected.domain;
  const rows = selected.records.map((record) => [
    td(`<span class="mono">${esc(record.spec.name)}</span>`),
    td(`<span class="chip">${esc(record.spec.rtype)}</span>`),
    td(`<span class="mono small">${esc(record.spec.value)}</span>`),
    td(esc(record.spec.ttl)),
    td(record.managed ? badge('auto', 'info') : badge('manual', '')),
    td(writable ? btn('Eliminar', 'dns-record-delete', `${selected.domain}/${record.id}`, 'tiny danger') : '', 'actions'),
  ]);
  const options = zones.map((zone) => `<option value="${esc(zone.domain)}" ${zone.domain === selected.domain ? 'selected' : ''}>${esc(zone.domain)}</option>`).join('');
  $('#view').innerHTML = `
    <div class="section-head">
      <div class="row-inline">
        <h2>Zona</h2>
        <select id="dns-zone-select" class="compact">${options}</select>
      </div>
      <div class="row-inline">
        ${writable ? '<button class="btn ghost" data-action="dns-mail">Provisionar correo</button><button class="btn primary" data-action="dns-record-new">Añadir registro</button>' : ''}
      </div>
    </div>
    <div class="grid cols-4">
      <div class="card stat"><div class="label">Servidores NS</div><div class="value small-value mono">${esc(selected.nameservers.join(', ') || 'sin configurar')}</div></div>
      <div class="card stat"><div class="label">Serial</div><div class="value">${esc(selected.serial)}</div></div>
      <div class="card stat"><div class="label">Correo</div><div class="value small-value">${selected.mail_ready ? badge('preparado', 'ok') : badge('pendiente', 'warn')}</div><div class="hint">SPF · DKIM · DMARC · MX</div></div>
      <div class="card stat"><div class="label">Fichero</div><div class="value small-value mono">${esc(selected.zone_file)}</div></div>
    </div>
    ${table(['Nombre', 'Tipo', 'Valor', 'TTL', 'Origen', ''], rows, 'Sin registros.')}
    <div class="card">
      <strong>Entregabilidad de correo</strong>
      <p class="muted small" style="margin:8px 0 0">
        La zona incluye SPF, DKIM (<span class="mono">${esc(selected.dkim_selector)}._domainkey</span>), DMARC y MX.
        El PTR inverso se configura en el proveedor de la IP: debe resolver a <span class="mono">${esc(S.mailHost || 'mail.' + selected.domain)}</span>.
        Verifica con <span class="mono">dig -x TU_IP</span> antes de enviar correo masivo.
      </p>
    </div>`;

  $('#dns-zone-select').addEventListener('change', (event) => {
    S.dnsDomain = event.target.value;
    renderDns();
  });
}

function openDnsRecordModal() {
  const domain = S.dnsDomain;
  if (!domain) return;
  openModal({
    title: `Nuevo registro · ${domain}`,
    submitLabel: 'Guardar',
    html: `<div class="form-grid">
      ${fInput('name', 'Nombre', { value: '@', required: true, help: '@, www, _dmarc, mail._domainkey…' })}
      ${fSelect('rtype', 'Tipo', ['A', 'AAAA', 'CNAME', 'MX', 'TXT', 'NS', 'SRV', 'CAA'], { value: 'A' })}
      ${fInput('value', 'Valor', { required: true, placeholder: '203.0.113.10' })}
      ${fInput('ttl', 'TTL', { type: 'number', value: 300, min: 60, max: 86400, required: true })}
      ${fInput('priority', 'Prioridad (MX)', { type: 'number', value: 10, min: 0, max: 65535 })}
      <p class="hint full">Nota: se admite un registro por combinación nombre + tipo.</p>
    </div>`,
    onSubmit: async (form) => {
      const rtype = val(form, 'rtype');
      await api(`/api/v1/dns/${encodeURIComponent(domain)}/records`, {
        method: 'POST',
        body: {
          domain,
          name: val(form, 'name'),
          rtype,
          value: val(form, 'value'),
          ttl: num(form, 'ttl', 300),
          priority: rtype === 'MX' ? num(form, 'priority', 10) : null,
        },
      });
      toast('Registro DNS guardado');
      await refresh();
    },
  });
}

/* ---------------- backups ---------------- */

async function renderBackups() {
  await Promise.all([loadList('backups'), loadList('accounts')]);
  const writable = canWrite();
  const scoped = isScoped();
  const rows = S.data.backups.map((backup) => [
    td(`<div class="cell-title">${esc(backup.username)}</div><div class="cell-sub mono">${esc(backup.id)}</div>`),
    td(esc(fmtDate(backup.created_at))),
    td(esc(fmtBytes(backup.size_bytes))),
    td(badge(backup.status, backup.status === 'simulated' ? 'warn' : 'ok')),
    td(`<span class="mono small">${esc(backup.path)}</span>`),
    td([
      isAdmin() && !scoped ? btn('Restaurar', 'backup-restore', backup.id, 'tiny ghost') : '',
      writable ? btn('Eliminar', 'backup-delete', backup.id, 'tiny danger') : '',
    ].join(''), 'actions'),
  ]);
  $('#view').innerHTML =
    sectionHead('Copias de seguridad', writable ? '<button class="btn primary" data-action="backup-new">Crear backup</button>' : '') +
    table(['Cuenta', 'Fecha', 'Tamaño', 'Estado', 'Ruta', ''], rows, 'Aún no hay copias. Se conservan las últimas 7 por cuenta.');
}

function openBackupModal() {
  if (!S.data.accounts.length) {
    toast('Primero crea una cuenta', 'error');
    return;
  }
  openModal({
    title: 'Crear backup',
    submitLabel: 'Crear',
    html: `<div class="form-grid">${fSelect('username', 'Cuenta', accountOptions(false), { required: true })}</div>`,
    onSubmit: async (form) => {
      await api('/api/v1/backups', { method: 'POST', body: { username: val(form, 'username') } });
      toast('Backup creado');
      await refresh();
    },
  });
}

/* ---------------- monitor ---------------- */

function sparkline(samples, pick, color, suffix = '') {
  if (!samples || samples.length < 2) return `<div class="empty small">Recopilando datos…</div>`;
  const values = samples.map(pick);
  const max = Math.max(...values, 1);
  const min = Math.min(...values, 0);
  const span = Math.max(max - min, 1);
  const points = values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * 600;
      const y = 112 - ((value - min) / span) * 100;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
  return `<svg class="spark" viewBox="0 0 600 120" preserveAspectRatio="none" aria-hidden="true"><polyline fill="none" stroke="${color}" stroke-width="2" vector-effect="non-scaling-stroke" points="${points}"/></svg><div class="muted small">${esc(suffix)}</div>`;
}

async function renderMonitor() {
  const [servicesData, historyData] = await Promise.all([
    api('/api/v1/monitor/services'),
    api('/api/v1/monitor/history?minutes=1440'),
  ]);
  const services = servicesData.services || [];
  const samples = historyData.samples || [];
  const active = services.filter((s) => s.status === 'active').length;
  const last = samples[samples.length - 1];

  const rows = services.map((service) => [
    td(`<span class="chip">${esc(service.kind)}</span>`),
    td(`<div class="cell-title">${esc(service.name)}</div><div class="cell-sub mono">${esc(service.unit)}</div>`),
    td(service.status === 'active' ? badge('activo', 'ok') : service.status === 'failed' ? badge('fallido', 'bad') : badge(service.status, 'warn')),
    td(`${service.cpu_percent.toFixed(2)} %`),
    td(esc(fmtKb(service.memory_kb))),
    td(esc(fmtUptime(service.uptime_secs))),
  ]);

  $('#view').innerHTML = `
    <div class="grid cols-4">
      <div class="card stat"><div class="label">Servicios</div><div class="value">${active}/${services.length}</div><div class="hint">activos ahora</div></div>
      <div class="card stat"><div class="label">Load 1m</div><div class="value">${last ? (last.load1 / 100).toFixed(2) : '—'}</div><div class="hint">media 1 minuto</div></div>
      <div class="card stat"><div class="label">Memoria usada</div><div class="value small-value">${last ? esc(fmtKb(last.mem_used_kb)) : '—'}</div><div class="hint">última muestra</div></div>
      <div class="card stat"><div class="label">Disco usado</div><div class="value small-value">${last ? esc(fmtKb(last.disk_used_kb)) : '—'}</div><div class="hint">última muestra</div></div>
    </div>
    <div class="grid cols-2">
      <div class="card"><div class="section-head"><h2>Load</h2><span class="badge info">24 h</span></div><div class="chart">${sparkline(samples, (s) => s.load1, '#57e6a1')}</div></div>
      <div class="card"><div class="section-head"><h2>Memoria</h2><span class="badge info">24 h</span></div><div class="chart">${sparkline(samples, (s) => s.mem_used_kb, '#4cc9f0')}</div></div>
      <div class="card"><div class="section-head"><h2>Disco</h2><span class="badge info">24 h</span></div><div class="chart">${sparkline(samples, (s) => s.disk_used_kb, '#ffc857')}</div></div>
      <div class="card"><div class="section-head"><h2>Servicios activos</h2><span class="badge info">24 h</span></div><div class="chart">${sparkline(samples, (s) => s.services_active, '#a78bfa')}</div></div>
    </div>
    ${sectionHead('Servicios', '<span class="muted small">actualizado cada 60 s</span>')}
    ${table(['Tipo', 'Servicio', 'Estado', 'CPU media', 'RAM', 'Uptime'], rows, 'No hay servicios registrados.')}`;
}

/* ---------------- security ---------------- */

async function renderSecurity() {
  const [stats, eventsData] = await Promise.all([
    api('/api/v1/security/stats?hours=24'),
    api('/api/v1/security/auth-events?limit=200'),
  ]);
  const events = eventsData.events || [];
  const rows = events.map((event) => [
    td(esc(fmtDate(event.at))),
    td(`<strong>${esc(event.username)}</strong>`),
    td(`<span class="mono small">${esc(event.ip)}</span>`),
    td(event.outcome === 'success' ? badge('ok', 'ok') : event.outcome === 'blocked' ? badge('bloqueado', 'bad') : event.outcome === 'impersonation' ? badge('suplantación', 'info') : badge('fallo', 'warn')),
    td(esc(event.reason)),
    td(`<span class="muted small">${esc(event.user_agent)}</span>`),
  ]);
  $('#view').innerHTML = `
    <div class="grid cols-4">
      <div class="card stat"><div class="label">Fallos 24 h</div><div class="value">${stats.failures}</div><div class="hint">credenciales inválidas</div></div>
      <div class="card stat"><div class="label">Bloqueos</div><div class="value">${stats.blocked}</div><div class="hint">rate limit / lockout</div></div>
      <div class="card stat"><div class="label">Accesos OK</div><div class="value">${stats.successes}</div><div class="hint">sesiones creadas</div></div>
      <div class="card stat"><div class="label">IPs únicas</div><div class="value">${stats.unique_ips}</div><div class="hint">con intentos fallidos</div></div>
    </div>
    ${sectionHead('Eventos de autenticación', '<span class="muted small">también en /var/log/rynxcp/auth.log (fail2ban)</span>')}
    ${table(['Fecha', 'Usuario', 'IP', 'Resultado', 'Motivo', 'User-Agent'], rows, 'Sin eventos de autenticación.')}`;
}

/* ---------------- firewall ---------------- */

async function renderFirewall() {
  const data = await api('/api/v1/firewall');
  const fw = data.state;
  const ruleRows = fw.rules.map((rule) => [
    td(rule.spec.action === 'allow' ? badge('permitir', 'ok') : badge('denegar', 'bad')),
    td(`<span class="chip">${esc(rule.spec.proto)}</span>`),
    td(esc(rule.spec.port)),
    td(`<span class="mono small">${esc(rule.spec.source || '*')}</span>`),
    td(esc(rule.spec.comment || '—')),
    td(btn('Eliminar', 'firewall-rule-delete', rule.id, 'tiny danger'), 'actions'),
  ]);
  const blockedRows = fw.blocked.map((entry) => [
    td(`<span class="mono">${esc(entry.ip)}</span>`),
    td(esc(entry.reason)),
    td(badge(entry.source, entry.source === 'brute-force' ? 'warn' : '')),
    td(esc(entry.expires_at ? fmtDate(entry.expires_at) : 'permanente')),
    td(btn('Quitar', 'firewall-unblock', entry.ip, 'tiny ghost'), 'actions'),
  ]);
  const whiteRows = fw.whitelist.map((ip) => [
    td(`<span class="mono">${esc(ip)}</span>`),
    td(btn('Quitar', 'firewall-whitelist-remove', ip, 'tiny ghost'), 'actions'),
  ]);
  $('#view').innerHTML = `
    <div class="grid cols-4">
      <div class="card stat"><div class="label">Cortafuegos</div><div class="value small-value">${fw.enabled ? badge('activo', 'ok') : badge('desactivado', 'warn')}</div><div class="hint">${esc(fw.backend)}</div></div>
      <div class="card stat"><div class="label">Baneadas</div><div class="value">${fw.blocked.length}</div><div class="hint">manual + fuerza bruta</div></div>
      <div class="card stat"><div class="label">Lista blanca</div><div class="value">${fw.whitelist.length}</div><div class="hint">prioridad máxima</div></div>
      <div class="card stat"><div class="label">SSH</div><div class="value">${esc(fw.ssh_port)}</div><div class="hint">siempre permitido</div></div>
    </div>
    <div class="card"><p class="muted small" style="margin:0">
      Las reglas se aplican en la tabla nftables aislada <span class="mono">inet rynxcp</span>.
      La lista blanca tiene prioridad sobre los baneos. El puerto SSH se permite siempre para evitar bloqueos.
      Los baneos automáticos duran 1 hora y se retiran solos.
    </p></div>
    ${sectionHead('Reglas', '<button class="btn primary" data-action="firewall-rule-new">Añadir regla</button>')}
    ${table(['Acción', 'Protocolo', 'Puerto', 'Origen', 'Comentario', ''], ruleRows, 'Sin reglas personalizadas.')}
    ${sectionHead('IPs baneadas', '<button class="btn primary" data-action="firewall-block-new">Banear IP</button>')}
    ${table(['IP', 'Motivo', 'Origen', 'Expira', ''], blockedRows, 'No hay IPs baneadas.')}
    ${sectionHead('Lista blanca', '<button class="btn primary" data-action="firewall-whitelist-new">Añadir IP</button>')}
    ${table(['IP / CIDR', ''], whiteRows, 'Sin direcciones privilegiadas.')}`;
}

function openFirewallRuleModal() {
  openModal({
    title: 'Nueva regla de cortafuegos',
    submitLabel: 'Aplicar',
    html: `<div class="form-grid">
      ${fSelect('action', 'Acción', [{ value: 'allow', label: 'Permitir' }, { value: 'deny', label: 'Denegar' }], { value: 'allow' })}
      ${fSelect('proto', 'Protocolo', ['tcp', 'udp', 'any'], { value: 'tcp' })}
      ${fInput('port', 'Puerto', { type: 'number', value: 8080, min: 1, max: 65535, required: true })}
      ${fInput('source', 'Origen (IP/CIDR)', { placeholder: 'opcional, ej. 10.0.0.0/8' })}
      ${fInput('comment', 'Comentario', { placeholder: 'motivo de la regla', full: true })}
    </div>`,
    onSubmit: async (form) => {
      await api('/api/v1/firewall/rules', {
        method: 'POST',
        body: {
          action: val(form, 'action'),
          proto: val(form, 'proto'),
          port: num(form, 'port', 0),
          source: val(form, 'source') || null,
          comment: val(form, 'comment'),
        },
      });
      toast('Regla aplicada');
      await refresh();
    },
  });
}

function openFirewallBlockModal() {
  openModal({
    title: 'Banear dirección IP',
    submitLabel: 'Banear',
    html: `<div class="form-grid">
      ${fInput('ip', 'IP o CIDR', { required: true, placeholder: '198.51.100.7' })}
      ${fSelect('duration', 'Duración', [{ value: '3600', label: '1 hora' }, { value: '86400', label: '24 horas' }, { value: '0', label: 'Permanente' }], { value: '3600' })}
      ${fInput('reason', 'Motivo', { placeholder: 'manual', full: true })}
    </div>`,
    onSubmit: async (form) => {
      await api('/api/v1/firewall/block', {
        method: 'POST',
        body: { ip: val(form, 'ip'), seconds: num(form, 'duration', 3600), reason: val(form, 'reason') || 'manual' },
      });
      toast('IP baneada');
      await refresh();
    },
  });
}

function openWhitelistModal() {
  openModal({
    title: 'Añadir a lista blanca',
    submitLabel: 'Añadir',
    html: `<div class="form-grid">${fInput('ip', 'IP o CIDR', { required: true, placeholder: '203.0.113.4', full: true })}</div>`,
    onSubmit: async (form) => {
      await api('/api/v1/firewall/whitelist', { method: 'POST', body: { ip: val(form, 'ip') } });
      toast('IP añadida a la lista blanca');
      await refresh();
    },
  });
}

/* ---------------- programs ---------------- */

async function renderPrograms() {
  await Promise.all([loadList('programs'), loadList('updates')]);
  const updates = S.data.updates;
  const security = updates.filter((u) => u.security).length;
  const updateRows = updates.map((update) => [
    td(`<span class="mono">${esc(update.package)}</span>`),
    td(esc(update.current_version || '—')),
    td(esc(update.candidate_version)),
    td(update.security ? badge('seguridad', 'bad') : badge('normal', '')),
  ]);
  const programRows = S.data.programs.map((program) => [
    td(`<div class="cell-title">${esc(program.name)}</div><div class="cell-sub">${esc(program.detail)}</div>`),
    td(`<span class="mono small">${esc(program.version)}</span>`),
    td(`<span class="chip">${esc(program.kind)}</span>`),
    td(program.status === 'installed' ? badge('instalado', 'ok') : badge('ausente', 'warn')),
  ]);
  $('#view').innerHTML = `
    <div class="grid cols-3">
      <div class="card stat"><div class="label">Actualizaciones</div><div class="value">${updates.length}</div><div class="hint">${security} de seguridad</div></div>
      <div class="card stat"><div class="label">Programas detectados</div><div class="value">${S.data.programs.length}</div><div class="hint">nginx, runtimes, DB, DNS, FTP, mail</div></div>
      <div class="card stat"><div class="label">Repositorio</div><div class="value small-value">apt</div><div class="hint">Debian / Ubuntu</div></div>
    </div>
    ${sectionHead('Actualizaciones disponibles', `<button class="btn primary" data-action="updates-apply" ${updates.length ? '' : 'disabled'}>Actualizar todo</button>`)}
    ${table(['Paquete', 'Actual', 'Disponible', 'Tipo'], updateRows, 'El sistema está actualizado.')}
    ${sectionHead('Software instalado', '<button class="btn ghost" data-action="reload">Refrescar</button>')}
    ${table(['Programa', 'Versión', 'Tipo', 'Estado'], programRows, 'No se detectó software.')}`;
}

/* ---------------- console ---------------- */

const CONSOLE_ACTIONS = [
  ['nginx_test', 'Validar configuración de Nginx'],
  ['nginx_reload', 'Recargar Nginx'],
  ['services', 'Listar servicios'],
  ['disk_usage', 'Uso de disco'],
  ['reconcile', 'Reconciliar todo el estado'],
  ['cert_renew', 'Renovar certificados'],
  ['backup_all', 'Crear backups de todas las cuentas'],
  ['php_reload', 'Recargar PHP-FPM'],
  ['app_restart', 'Reiniciar aplicación'],
  ['app_logs', 'Ver logs de aplicación'],
];

async function renderConsole() {
  await Promise.all([loadList('accounts'), loadList('apps')]);
  const appOptions = S.data.apps.map((app) => ({
    value: `${app.spec.username}/${app.spec.name}`,
    label: `${app.spec.username}/${app.spec.name}`,
  }));
  $('#view').innerHTML = `
    <div class="card">
      <div class="section-head"><h2>Acción</h2>${badge('sin shell libre', 'info')}</div>
      <form id="console-form" class="form-grid" style="margin-top:16px">
        ${fSelect('action', 'Comando', CONSOLE_ACTIONS.map(([value, label]) => ({ value, label })), { value: 'services', full: true })}
        ${fSelect('php_version', 'Versión PHP (php_reload)', RUNTIMES.php.versions.map((v) => ({ value: v, label: v })), { value: '8.3' })}
        ${fSelect('app', 'Aplicación (restart / logs)', appOptions.length ? appOptions : [{ value: '', label: '— sin aplicaciones —' }])}
        ${fInput('lines', 'Líneas de log', { type: 'number', value: 100, min: 10, max: 500 })}
        <div class="full row-inline"><button class="btn primary" type="submit">Ejecutar</button><span class="muted small">Toda acción queda registrada en la auditoría encadenada.</span></div>
      </form>
    </div>
    <div class="card">
      <div class="section-head"><h2>Salida</h2><span class="muted small">máx. 64 KiB</span></div>
      <pre id="console-output" class="console-out">${esc(S.consoleOutput || 'Listo. Elige una acción y pulsa Ejecutar.')}</pre>
    </div>`;

  $('#console-form').addEventListener('submit', async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const actionName = val(form, 'action');
    let action = { action: actionName };
    if (actionName === 'php_reload') action.version = val(form, 'php_version');
    if (actionName === 'app_restart' || actionName === 'app_logs') {
      const target = val(form, 'app');
      if (!target) throw new Error('Selecciona una aplicación');
      const [username, name] = target.split('/');
      action.username = username;
      action.name = name;
      if (actionName === 'app_logs') action.lines = num(form, 'lines', 100);
    }
    try {
      const result = await api('/api/v1/console', { method: 'POST', body: { action } });
      S.consoleOutput = result.output || '(sin salida)';
      const pre = $('#console-output');
      if (pre) pre.textContent = S.consoleOutput;
      toast(`${result.action} ejecutado`);
    } catch (err) {
      toast(err.message, 'error');
    }
  });
}

/* ---------------- helpers for new views ---------------- */

function fmtBytes(bytes) {
  const value = Number(bytes) || 0;
  if (value >= 1024 * 1024 * 1024) return `${(value / 1024 / 1024 / 1024).toFixed(2)} GiB`;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${value} B`;
}

function showOutput(title, text) {
  openModal({
    title,
    submitLabel: 'Cerrar',
    html: `<pre class="console-out">${esc(text)}</pre>`,
    onSubmit: async () => {},
  });
}

/* ---------------- wiring ---------------- */

window.RYNX_ON_LANG_CHANGE = () => {
  if (S.user) {
    refresh();
  } else {
    showLogin();
  }
};

document.addEventListener('DOMContentLoaded', () => {
  $('#login-form').addEventListener('submit', async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const button = form.querySelector('button[type="submit"]');
    button.disabled = true;
    try {
      const response = await fetch('/api/v1/auth/login', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify({
          username: val(form, 'username'),
          password: form.elements.password.value,
        }),
      });
      const data = await response.json().catch(() => null);
      if (!response.ok) {
        throw new Error(data && data.error ? data.error.message : `Error HTTP ${response.status}`);
      }
      S.user = data.user;
      S.csrf = data.csrf_token;
      S.impersonatedBy = data.impersonated_by || null;
      form.reset();
      enterApp();
    } catch (err) {
      toast(err.message, 'error');
    } finally {
      button.disabled = false;
    }
  });

  const stopImpersonation = $('#stop-impersonation');
  if (stopImpersonation) {
    stopImpersonation.addEventListener('click', async () => {
      try {
        const data = await api('/api/v1/auth/stop-impersonation', { method: 'POST' });
        S.user = data.user;
        S.csrf = data.csrf_token;
        S.impersonatedBy = data.impersonated_by || null;
        enterApp();
        toast('Sesión de administrador restaurada');
      } catch (err) {
        toast(err.message, 'error');
      }
    });
  }

  setInterval(() => {
    if (S.user && S.current === 'monitor' && document.visibilityState === 'visible') {
      renderMonitor().catch(() => {});
    }
  }, 20000);

  $('#logout').addEventListener('click', async () => {
    try { await api('/api/v1/auth/logout', { method: 'POST' }); } catch { /* ignore */ }
    showLogin();
  });

  $('#view').addEventListener('click', async (event) => {
    const target = event.target.closest('[data-action]');
    if (!target) return;
    try {
      await handleAction(target.dataset.action, target.dataset.id || '');
    } catch (err) {
      toast(err.message, 'error');
    }
  });

  boot();
});




