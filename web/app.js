'use strict';

/* ==========================================================================
   Nexulon Switcher — page controller
   Modules: ipc, store, appearance, rail, roster, account menu, settings,
   activity, command palette, modal, toasts.
   ========================================================================== */

const THEMES = [
  { id: 'circuit', name: 'Circuit' },
  { id: 'aurora', name: 'Aurora' },
  { id: 'ember', name: 'Ember' },
  { id: 'frost', name: 'Frost' },
  { id: 'mono', name: 'Mono' }
];

const ACCENTS = [
  { id: 'default', color: 'linear-gradient(135deg,#4d8dff,#38e8ff)', name: 'Theme default' },
  { id: 'cyan', color: '#16c6e8', name: 'Cyan' },
  { id: 'violet', color: '#9b6bff', name: 'Violet' },
  { id: 'emerald', color: '#1fce94', name: 'Emerald' },
  { id: 'amber', color: '#f0a020', name: 'Amber' },
  { id: 'rose', color: '#ff5d8f', name: 'Rose' }
];

const SORTS = [
  { id: 'recent', name: 'Recent first' },
  { id: 'az', name: 'Name A–Z' },
  { id: 'za', name: 'Name Z–A' },
  { id: 'most', name: 'Most used' }
];

const VIEWS = [
  { id: 'grid', name: 'Grid', icon: 'grid' },
  { id: 'tiles', name: 'Tiles', icon: 'tiles' },
  { id: 'details', name: 'Details', icon: 'list' }
];

const CATEGORIES = [
  { id: 'launcher', name: 'Game launchers', short: 'Launchers' },
  { id: 'game', name: 'Games', short: 'Games' },
  { id: 'browser', name: 'Browsers', short: 'Browsers' },
  { id: 'ai', name: 'AI agents', short: 'AI' },
  { id: 'chat', name: 'Communication', short: 'Chat' },
  { id: 'cloud', name: 'Cloud & productivity', short: 'Cloud' },
  { id: 'create', name: 'Creation', short: 'Creation' },
  { id: 'media', name: 'Media', short: 'Media' },
  { id: 'other', name: 'Other', short: 'Other' }
];

function groupPlatforms(platforms) {
  const groups = CATEGORIES.map((category) => ({ ...category, items: [] }));
  const byId = new Map(groups.map((group) => [group.id, group]));
  for (const platform of platforms || []) {
    const group = byId.get(platform.category) || byId.get('other');
    group.items.push(platform);
  }
  return groups.filter((group) => group.items.length);
}

// The rail normally shows the whole catalogue. "Installed only" trims it, but
// never hides the service that is currently open.
function railPlatforms() {
  if (!state.ui.installedOnly) return state.platforms;
  return state.platforms.filter((platform) => platform.installed || platform.name === state.platform);
}

const DEFAULT_UI = {
  theme: 'circuit',
  accent: 'default',
  density: 'comfortable',
  view: 'grid',
  motion: 'full',
  sort: 'recent',
  installedOnly: false,
  railFolders: true,
  lastPlatform: '',
  favorites: {},
  recents: [],
  stats: {}
};

const ICONS = {
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.2-3.2"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  check: '<path d="m5 13 4 4L19 7"/>',
  alert: '<path d="M12 9v4"/><path d="M12 17h.01"/><path d="M10.3 3.9 2.6 17a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 8h.01"/>',
  star: '<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1L3.2 9.5l6.1-.9L12 3Z"/>',
  refresh: '<path d="M21 12a9 9 0 1 1-2.64-6.36"/><path d="M21 3v6h-6"/>',
  log: '<path d="M4 6h16M4 12h16M4 18h10"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z"/>',
  copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/>',
  trash: '<path d="M4 7h16"/><path d="M9 7V4h6v3"/><path d="M6 7l1 13h10l1-13"/>',
  edit: '<path d="M4 20h4L20 8l-4-4L4 16v4Z"/>',
  note: '<path d="M5 4h14v16l-4-3H5V4Z"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  power: '<path d="M12 3v9"/><path d="M6.4 6.4a8 8 0 1 0 11.2 0"/>',
  login: '<path d="M10 17l5-5-5-5"/><path d="M15 12H3"/><path d="M13 4h6a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-6"/>',
  arrow: '<path d="M5 12h14"/><path d="m13 6 6 6-6 6"/>',
  chevron: '<path d="m9 6 6 6-6 6"/>',
  pin: '<path d="M9 4h6l-1 7 3 3v2H7v-2l3-3-1-7Z"/><path d="M12 16v4"/>',
  spark: '<path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8"/>',
  grid: '<rect x="4" y="4" width="7" height="7" rx="1.5"/><rect x="13" y="4" width="7" height="7" rx="1.5"/><rect x="4" y="13" width="7" height="7" rx="1.5"/><rect x="13" y="13" width="7" height="7" rx="1.5"/>',
  tiles: '<rect x="4" y="4" width="4.5" height="4.5" rx="1"/><rect x="9.75" y="4" width="4.5" height="4.5" rx="1"/><rect x="15.5" y="4" width="4.5" height="4.5" rx="1"/><rect x="4" y="9.75" width="4.5" height="4.5" rx="1"/><rect x="9.75" y="9.75" width="4.5" height="4.5" rx="1"/><rect x="15.5" y="9.75" width="4.5" height="4.5" rx="1"/><rect x="4" y="15.5" width="4.5" height="4.5" rx="1"/><rect x="9.75" y="15.5" width="4.5" height="4.5" rx="1"/><rect x="15.5" y="15.5" width="4.5" height="4.5" rx="1"/>',
  list: '<path d="M8 6h12M8 12h12M8 18h12"/><circle cx="4" cy="6" r="1"/><circle cx="4" cy="12" r="1"/><circle cx="4" cy="18" r="1"/>',
  image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="m4 18 5-5 4 4 3-3 4 4"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  settings: '<circle cx="12" cy="12" r="3.4"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6h.09A1.65 1.65 0 0 0 10 3.09V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9v.09a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"/>',
  trophy: '<path d="M8 4h8v5a4 4 0 0 1-8 0V4Z"/><path d="M8 5H5v2a3 3 0 0 0 3 3"/><path d="M16 5h3v2a3 3 0 0 1-3 3"/><path d="M12 13v3"/><path d="M8 20h8"/><path d="M10 20v-3h4v3"/>',
  chart: '<path d="M4 20V10M10 20V4M16 20v-7"/><path d="M2 20h20"/>'
};

/* -- helpers -------------------------------------------------------------- */
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));

function svgIcon(name) {
  if (name === 'dots') {
    return '<svg class="ico" viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="5" r="1.7"/><circle cx="12" cy="12" r="1.7"/><circle cx="12" cy="19" r="1.7"/></svg>';
  }
  return `<svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${ICONS[name] || ''}</svg>`;
}

function esc(value) {
  return String(value == null ? '' : value).replace(/[&<>"']/g, (character) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[character]));
}

function initials(name) {
  const parts = String(name || '').trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return '?';
  const first = parts[0][0] || '';
  const last = parts.length > 1 ? parts[parts.length - 1][0] : '';
  return (first + last).toUpperCase();
}

function hue(name) {
  let value = 0;
  for (const character of String(name || '')) value = (value * 31 + character.charCodeAt(0)) % 360;
  return value;
}

function shortId(id) {
  if (!id) return '';
  return id.length > 15 ? `${id.slice(0, 6)}…${id.slice(-4)}` : id;
}

function timeAgo(stamp) {
  if (!stamp) return '';
  const delta = Date.now() - stamp;
  if (delta < 60000) return 'just now';
  const minutes = Math.floor(delta / 60000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}

function keyFor(platform, account) {
  return `${platform}\u0000${account}`;
}

function fuzzy(text, query) {
  const haystack = String(text || '').toLowerCase();
  const needle = String(query || '').toLowerCase();
  if (!needle) return true;
  if (haystack.includes(needle)) return true;
  let index = 0;
  for (const character of needle) {
    index = haystack.indexOf(character, index);
    if (index < 0) return false;
    index += 1;
  }
  return true;
}

/* -- ipc ------------------------------------------------------------------ */
let ipcSequence = 0;
const ipcPending = new Map();

window.__nexulonReply = function (id, ok, payload) {
  const entry = ipcPending.get(id);
  if (!entry) return;
  ipcPending.delete(id);
  if (ok) entry.resolve(payload);
  else entry.reject(new Error(typeof payload === 'string' ? payload : 'The request failed.'));
};

function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++ipcSequence;
    const timer = setTimeout(() => {
      ipcPending.delete(id);
      reject(new Error('The switcher did not respond in time.'));
    }, 180000);
    ipcPending.set(id, {
      resolve: (value) => { clearTimeout(timer); resolve(value); },
      reject: (error) => { clearTimeout(timer); reject(error); }
    });
    try {
      window.ipc.postMessage(JSON.stringify({ id, method, params }));
    } catch (error) {
      ipcPending.delete(id);
      clearTimeout(timer);
      reject(error);
    }
  });
}

/* -- store ---------------------------------------------------------------- */
const state = {
  prefs: {},
  ui: { ...DEFAULT_UI },
  autostart: false,
  version: '',
  dataRoot: '',
  logPath: '',
  logo: '',
  mark: '',
  platforms: [],
  platform: '',
  platformData: null,
  accounts: [],
  current: null,
  visible: [],
  selected: -1,
  query: '',
  busy: false,
  settingsTab: 'general',
  paletteIndex: 0,
  paletteItems: []
};

let popover = null;
let cardClickTimer = 0;
let uiSaveTimer = null;
let pendingUI = {};
let windowThemeTimer = 0;

function hexColor(value, fallback) {
  const match = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(String(value || '').trim());
  if (!match) return fallback;
  let hex = match[1];
  if (hex.length === 3) hex = hex.split('').map((character) => character + character).join('');
  return `#${hex.toLowerCase()}`;
}

function mixHex(from, to, amount) {
  const channels = (hex) => [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16));
  const [r1, g1, b1] = channels(from);
  const [r2, g2, b2] = channels(to);
  const mix = (a, b) => Math.round(a + (b - a) * amount).toString(16).padStart(2, '0');
  return `#${mix(r1, r2)}${mix(g1, g2)}${mix(b1, b2)}`;
}

// The native caption is painted by DWM, so it needs the same colours the page
// just applied. Debounced because theme edits fire several saves in a row.
function syncWindowTheme() {
  clearTimeout(windowThemeTimer);
  windowThemeTimer = setTimeout(() => {
    const styles = getComputedStyle(document.documentElement);
    const bg = hexColor(styles.getPropertyValue('--bg'), '#060a14');
    const text = hexColor(styles.getPropertyValue('--text'), '#eef3ff');
    call('window_theme', { bg, text, border: mixHex(bg, '#ffffff', 0.1) }).catch(() => {});
  }, 40);
}

function applyAppearance() {
  const root = document.documentElement;
  const theme = THEMES.some((entry) => entry.id === state.ui.theme) ? state.ui.theme : 'circuit';
  const accent = ACCENTS.some((entry) => entry.id === state.ui.accent) ? state.ui.accent : 'default';
  const density = state.ui.density === 'compact' ? 'compact' : 'comfortable';
  root.dataset.theme = theme;
  root.dataset.accent = accent;
  root.dataset.density = density;
  root.dataset.motion = state.ui.motion === 'reduced' ? 'reduced' : 'full';
  syncWindowTheme();
}

function flushUI() {
  const patch = pendingUI;
  pendingUI = {};
  if (Object.keys(patch).length) call('prefs', { ui: patch }).catch(() => {});
}

function saveUI(patch) {
  Object.assign(state.ui, patch);
  applyAppearance();
  Object.assign(pendingUI, patch);
  clearTimeout(uiSaveTimer);
  uiSaveTimer = setTimeout(flushUI, 140);
}

function favoritesFor(platform) {
  return (state.ui.favorites && state.ui.favorites[platform]) || [];
}

function isFavorite(account) {
  return favoritesFor(state.platform).includes(account);
}

function toggleFavorite(account) {
  const favorites = { ...(state.ui.favorites || {}) };
  const list = new Set(favorites[state.platform] || []);
  if (list.has(account)) list.delete(account);
  else list.add(account);
  favorites[state.platform] = Array.from(list);
  saveUI({ favorites });
  renderRoster();
  toast('info', list.has(account) ? 'Pinned to the top' : 'Unpinned', account);
}

function statsFor(account) {
  return (state.ui.stats || {})[keyFor(state.platform, account)] || {};
}

function recordUsage(account) {
  const stats = { ...(state.ui.stats || {}) };
  const key = keyFor(state.platform, account);
  const previous = stats[key] || { count: 0, last: 0 };
  stats[key] = { count: (previous.count || 0) + 1, last: Date.now() };
  const recents = [
    { platform: state.platform, account, at: Date.now() },
    ...(state.ui.recents || []).filter((entry) => !(entry.platform === state.platform && entry.account === account))
  ].slice(0, 40);
  saveUI({ stats, recents });
  flushUI();
}

/* -- status --------------------------------------------------------------- */
function setStatus(text, kind = '') {
  $('#status-text').textContent = text;
  const dot = $('#status-dot');
  dot.className = `status-dot${kind === 'busy' ? ' busy' : kind === 'error' ? ' error' : ''}`;
}

function updateStatusMeta() {
  if (!state.accounts.length) {
    $('#status-meta').textContent = '';
    return;
  }
  const sort = SORTS.find((entry) => entry.id === state.ui.sort);
  $('#status-meta').textContent = `${state.accounts.length} account${state.accounts.length === 1 ? '' : 's'} · ${sort ? sort.name : 'Recent first'}`;
}

/* -- platform icon -------------------------------------------------------- */
function platformIconHTML(platform) {
  const source = platform && platform.icon;
  if (typeof source === 'string' && source.indexOf('<svg') === 0) {
    return source.replace('<svg', '<svg class="p-svg" fill="currentColor" aria-hidden="true"');
  }
  return `<span class="letter">${esc(initials(platform ? platform.name : '?'))}</span>`;
}

/* -- rail ----------------------------------------------------------------- */
function renderRail() {
  const rail = $('#rail');
  const groups = groupPlatforms(railPlatforms());
  const folders = state.ui.railFolders !== false;
  if (popover && popover.dataset.folder) closePopover();
  rail.classList.toggle('folders', folders);
  if (folders) {
    rail.innerHTML = groups.map((group) => {
      const active = group.items.find((platform) => platform.name === state.platform);
      const icon = active ? platformIconHTML(active) : svgIcon('folder');
      return `
        <button class="rail-folder ${active ? 'active' : ''}"
                data-folder="${group.id}"
                title="${esc(group.name)} · ${group.items.length} service${group.items.length === 1 ? '' : 's'}">
          <span class="folder-ico" style="--ph:${active ? hue(active.name) : hue(group.name)}">${icon}</span>
          <span class="folder-name">${esc(group.short)}</span>
          <span class="folder-count">${group.items.length}</span>
          <span class="folder-chev">${svgIcon('chevron')}</span>
        </button>
      `;
    }).join('');
  } else {
    rail.innerHTML = groups.map((group, groupIndex) => `
      <span class="rail-group ${groupIndex === 0 ? 'first' : ''}" role="presentation">${esc(group.short)}</span>
      ${group.items.map((platform) => `
        <button class="pill ${platform.name === state.platform ? 'active' : ''}"
                data-platform="${esc(platform.name)}"
                title="${esc(platform.name)} · ${platform.accounts} account${platform.accounts === 1 ? '' : 's'}">
          <span class="p-icon" style="--ph:${hue(platform.name)}">${platformIconHTML(platform)}</span>
          <span class="label">${esc(platform.name)}</span>
          <span class="count">${platform.accounts}</span>
        </button>
      `).join('')}
    `).join('');
  }
  updateRailFades();
  const active = $('#rail .pill.active') || $('#rail .rail-folder.active');
  if (active) requestAnimationFrame(() => active.scrollIntoView({ block: 'nearest', inline: 'center' }));
}

function updateRailFades() {
  const rail = $('#rail');
  const wrap = $('#rail-wrap');
  if (!rail || !wrap) return;
  wrap.classList.toggle('fade-left', rail.scrollLeft > 4);
  wrap.classList.toggle('fade-right', rail.scrollLeft + rail.clientWidth < rail.scrollWidth - 4);
}

/* -- head + roster -------------------------------------------------------- */
/// A launcher can be restarted logged out only when it is not profile-based and
/// its folder is known. Browsers and profile folders have no such step.
function canFreshSignIn() {
  const data = state.platformData || {};
  if ((data.mode || 'copy') === 'profile') return false;
  const settings = data.settings || {};
  return Boolean(settings.folder || settings.exe);
}

function renderHead(data) {
  const platform = state.platforms.find((entry) => entry.name === state.platform) || {
    name: state.platform, accounts: 0, installed: false, icon: null
  };
  const settings = (data && data.settings) || {};
  const mode = (data && data.mode) || 'copy';
  const isProfile = mode === 'profile';
  const installed = settings.installed !== undefined ? settings.installed : platform.installed;
  const location = settings.folder || settings.exe || '';
  const count = state.accounts.length;
  const view = VIEWS.some((entry) => entry.id === state.ui.view) ? state.ui.view : 'grid';
  $('#platform-head').innerHTML = `
    <div class="head-main">
      <div class="head-icon" style="--ph:${hue(state.platform)}">${platformIconHTML(platform)}</div>
      <div class="head-text">
        <h1>${esc(state.platform)}
          <span class="chip ${installed ? 'installed' : 'missing'}"><i></i>${installed ? 'Installed' : 'Not found'}</span>
          ${isProfile ? '<span class="chip profile"><i></i>Profiles</span>' : ''}
        </h1>
        <p>
          ${count} ${isProfile ? `profile${count === 1 ? '' : 's'}` : `account${count === 1 ? '' : 's'}`}
          <span class="dot"></span>
          ${location
            ? `<span title="${esc(location)}">${esc(location)}</span>`
            : '<span>Launcher folder not set</span>'}
        </p>
      </div>
    </div>
    <div class="head-actions">
      <div class="view-switch" role="group" aria-label="Roster view">
        ${VIEWS.map((entry) => `<button class="view-btn ${view === entry.id ? 'active' : ''}" data-view="${entry.id}" title="${entry.name} view" aria-pressed="${view === entry.id}">${svgIcon(entry.icon)}<span>${entry.name}</span></button>`).join('')}
      </div>
      <button class="btn ghost" data-action="detect" title="Find the launcher folder automatically">${svgIcon('search')}<span>Detect folder</span></button>
      ${isProfile
        ? `<button class="btn primary" data-action="reload" title="Read the profile list again">${svgIcon('refresh')}<span>Reload profiles</span></button>`
        : `${canFreshSignIn()
            ? `<button class="btn ghost" data-action="signin" title="Restart ${esc(state.platform)} logged out so you can sign in with a different account">${svgIcon('login')}<span>Sign in</span></button>`
            : ''}
           <button class="btn primary" data-action="add" title="Save the account that is already signed in">${svgIcon('plus')}<span>Add account</span></button>`}
    </div>
  `;
}

function sortedAccounts() {
  const favorites = favoritesFor(state.platform);
  const stats = state.ui.stats || {};
  const query = state.query.trim().toLowerCase();
  const base = state.accounts.filter((account) => {
    if (!query) return true;
    return account.name.toLowerCase().includes(query)
      || (account.notes || '').toLowerCase().includes(query)
      || (account.id || '').toLowerCase().includes(query);
  });
  const score = (account) => stats[keyFor(state.platform, account.name)] || {};
  const comparators = {
    recent: (a, b) => (score(b).last || 0) - (score(a).last || 0) || a.name.localeCompare(b.name),
    az: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
    za: (a, b) => b.name.localeCompare(a.name, undefined, { sensitivity: 'base' }),
    most: (a, b) => (score(b).count || 0) - (score(a).count || 0) || a.name.localeCompare(b.name)
  };
  const compare = comparators[state.ui.sort] || comparators.recent;
  const rank = (account) => account.current ? 0 : (favorites.includes(account.name) ? 1 : 2);
  return base.sort((a, b) => rank(a) - rank(b) || compare(a, b));
}

function cardHTML(account, index) {
  const pinned = isFavorite(account.name);
  const stats = statsFor(account.name);
  const notes = (account.notes || '').trim();
  const h1 = hue(account.name);
  const h2 = (h1 + 46) % 360;
  const cover = account.cover || '';
  const profile = account.profile || null;
  const tier = profile && profile.tier ? profile.tier : 0;
  const rank = tier ? (profile.rank || 'Ranked') : '';
  const rr = profile && profile.rr != null ? profile.rr : null;
  const level = profile && profile.level ? profile.level : 0;
  const unranked = Boolean(profile) && !tier;
  const alias = account.label && account.label !== account.name ? account.label : '';
  const detail = (account.detail || '').trim();
  const subtitle = alias || detail;
  // Keep a circle on every card. A fetched banner is used as the avatar until
  // the account gets its own picture, so a cover never leaves an empty slot.
  const avatarSrc = account.avatar || cover;
  return `
    <article class="card ${account.current ? 'current' : ''} ${cover ? 'has-cover' : ''}"
             data-name="${esc(account.name)}" data-index="${index}" tabindex="0"
             style="--h1:${h1};--h2:${h2};animation-delay:${Math.min(index * 24, 260)}ms"
             aria-label="${esc(account.name)}${account.current ? ', signed in' : ''}">
      ${cover ? `<div class="card-cover"><img src="${esc(cover)}" alt=""></div>` : ''}
      <div class="card-top">
        <div class="avatar">${avatarSrc ? `<img src="${esc(avatarSrc)}" alt="" class="${account.avatar ? '' : 'cover-avatar'}">` : esc(initials(account.name))}</div>
        <div class="card-id">
          <div class="card-name" title="${esc(account.name)}">${esc(account.name)}</div>
          ${subtitle ? `<div class="card-sub"><span class="${alias ? 'card-alias' : 'card-detail'}" title="${esc(subtitle)}">${esc(subtitle)}</span></div>` : ''}
        </div>
        <button class="kebab" data-action="menu" data-name="${esc(account.name)}" title="Account actions" aria-label="Actions for ${esc(account.name)}">${svgIcon('dots')}</button>
      </div>
      ${notes ? `<p class="card-notes">${esc(notes)}</p>` : ''}
      <div class="card-badges">
        ${account.current ? '<span class="badge live"><i></i>Signed in</span>' : ''}
        ${rank ? `<span class="badge rank" title="${esc(rank)}${rr != null ? ` · ${rr} RR` : ''}">${profile.icon ? `<img src="${esc(profile.icon)}" alt="">` : ''}${esc(rank)}${rr != null ? ` · ${rr} RR` : ''}</span>` : ''}
        ${unranked ? '<span class="badge unranked" title="This account has no competitive rank this act">Unranked</span>' : ''}
        ${level ? `<span class="badge level" title="Account level ${level}">Lv ${level}</span>` : ''}
        ${pinned ? '<span class="badge star">★ Pinned</span>' : ''}
        ${stats.count ? `<span class="badge">${stats.count}× used</span>` : ''}
        ${stats.last ? `<span class="badge subtle">${timeAgo(stats.last)}</span>` : ''}
      </div>
    </article>
  `;
}

function renderRoster() {
  state.visible = sortedAccounts();
  const roster = $('#roster');
  roster.dataset.view = VIEWS.some((entry) => entry.id === state.ui.view) ? state.ui.view : 'grid';
  if (!state.visible.length) {
    const searching = Boolean(state.query.trim());
    const isProfile = (state.platformData && state.platformData.mode) === 'profile';
    roster.innerHTML = `
      <div class="empty">
        <div class="orb">${svgIcon(searching ? 'search' : isProfile ? 'grid' : 'spark')}</div>
        <h3>${searching ? 'No accounts match that search' : isProfile ? 'No profiles found' : 'No accounts saved yet'}</h3>
        <p>${searching
          ? `Nothing on ${esc(state.platform)} matches “${esc(state.query.trim())}”. Try a different name, ID or note.`
          : isProfile
            ? `${esc(state.platform)} switches between profiles, and none were found in its profile folder. Create a profile there, then reload.`
            : `Sign in to ${esc(state.platform)} with the account you want to keep, then capture it here.`}</p>
        ${searching
          ? `<button class="btn" data-action="clear-search">${svgIcon('close')} Clear search</button>
             <button class="btn ghost" data-action="palette">${svgIcon('search')} Search all platforms</button>`
          : isProfile
            ? `<button class="btn primary" data-action="reload">${svgIcon('refresh')} Reload profiles</button>`
            : `<button class="btn primary" data-action="add">${svgIcon('plus')} Add account</button>
               ${canFreshSignIn() ? `<button class="btn ghost" data-action="signin">${svgIcon('login')} Sign in to another account</button>` : ''}`}
      </div>
    `;
  } else {
    roster.innerHTML = state.visible.map((account, index) => cardHTML(account, index)).join('');
  }
  if (state.selected >= state.visible.length) state.selected = state.visible.length - 1;
  paintSelection();
  updateStatusMeta();
}

function paintSelection() {
  $$('#roster .card').forEach((card, index) => card.classList.toggle('selected', index === state.selected));
  const active = $('#roster .card.selected');
  if (active) active.scrollIntoView({ block: 'nearest', inline: 'nearest' });
}

function gridColumns() {
  const columns = getComputedStyle($('#roster')).gridTemplateColumns;
  const count = columns.split(' ').filter(Boolean).length;
  return Math.max(1, count);
}

function moveSelection(delta) {
  if (!state.visible.length) return;
  if (state.selected < 0) state.selected = 0;
  else state.selected = Math.max(0, Math.min(state.visible.length - 1, state.selected + delta));
  paintSelection();
}

function renderSkeleton() {
  $('#platform-head').innerHTML = '';
  $('#roster').innerHTML = Array.from({ length: 6 }, () => '<div class="skeleton"></div>').join('');
}

/* -- platform loading ----------------------------------------------------- */
async function selectPlatform(name, options = {}) {
  if (!name) return;
  clearTimeout(cardClickTimer);
  cardClickTimer = 0;
  state.platform = name;
  state.query = '';
  state.selected = -1;
  $('#search').value = '';
  renderRail();
  renderSkeleton();
  try {
    const data = await call('accounts', { platform: name });
    if (state.platform !== name) return;
    state.platformData = data;
    state.accounts = data.accounts || [];
    state.current = data.current || null;
    const platform = state.platforms.find((entry) => entry.name === name);
    if (platform) {
      platform.accounts = state.accounts.length;
      platform.accountNames = state.accounts.map((account) => account.name);
      if (data.settings && data.settings.installed !== undefined) platform.installed = data.settings.installed;
    }
    renderRail();
    renderHead(data);
    renderRoster();
    if (!options.silent) saveUI({ lastPlatform: name });
    if (!options.keepStatus) setStatus(`${state.accounts.length} account${state.accounts.length === 1 ? '' : 's'} loaded`);
  } catch (error) {
    toast('error', 'Could not open platform', error.message);
    setStatus('Platform failed to load', 'error');
  }
}

async function refreshAccounts(options = {}) {
  if (!state.platform) return;
  try {
    const data = await call('accounts', { platform: state.platform });
    state.platformData = data;
    state.accounts = data.accounts || [];
    state.current = data.current || null;
    const platform = state.platforms.find((entry) => entry.name === state.platform);
    if (platform) {
      platform.accounts = state.accounts.length;
      platform.accountNames = state.accounts.map((account) => account.name);
      if (data.settings && data.settings.installed !== undefined) platform.installed = data.settings.installed;
    }
    renderRail();
    renderHead(data);
    renderRoster();
    if (!options.keepStatus) setStatus(`Refreshed ${state.accounts.length} account${state.accounts.length === 1 ? '' : 's'}`);
  } catch (error) {
    toast('error', 'Refresh failed', error.message);
  }
}

async function reloadPlatforms() {
  const list = await call('platforms');
  if (Array.isArray(list)) {
    state.platforms = list;
    renderRail();
  }
}

/* -- switching ------------------------------------------------------------ */
async function switchAccount(name, cardElement) {
  if (!name || state.busy) return;
  state.busy = true;
  setStatus(`Switching to ${name}…`, 'busy');
  const card = cardElement || $$('#roster .card').find((entry) => entry.dataset.name === name);
  if (card) {
    card.classList.add('busy');
    card.insertAdjacentHTML('beforeend', '<span class="spinner"></span>');
  }
  try {
    const result = await call('switch', {
      platform: state.platform,
      account: name,
      clear: state.prefs.clearOldLogin !== false,
      start: state.prefs.startLauncher !== false
    });
    recordUsage(name);
    const started = result && result.started;
    const stuck = (result && result.stillRunning) || [];
    if (stuck.length) {
      toast('warn', `Switched to ${name}, but ${stuck.join(', ')} is still running`, 'It may refuse to close. End it from Task Manager, or run Nexulon as administrator.');
    } else {
      toast('success', `Signed in as ${name}`, started ? 'The launcher is starting.' : 'The launcher was left closed.');
    }
    setStatus(`Signed in as ${name}`);
    if (started) scheduleAutoPull(state.platform, name, 8000);
    await refreshAccounts({ keepStatus: true });
  } catch (error) {
    toast('error', 'Switch failed', error.message);
    setStatus('Switch failed', 'error');
    if (card) {
      card.classList.remove('busy');
      const spinner = $('.spinner', card);
      if (spinner) spinner.remove();
    }
  } finally {
    state.busy = false;
  }
}

async function detectFolder() {
  setStatus('Looking for the launcher…', 'busy');
  try {
    const result = await call('detect', { platform: state.platform });
    if (result.found) {
      toast('success', 'Launcher found', result.folder);
      await refreshAccounts({ keepStatus: true });
      setStatus('Launcher found');
    } else {
      toast('info', 'Could not find the launcher', 'Set the folder manually in Settings → Platforms.');
      setStatus('Launcher not found');
    }
  } catch (error) {
    toast('error', 'Detection failed', error.message);
    setStatus('Detection failed', 'error');
  }
}

/* -- account actions ------------------------------------------------------ */
function openAccountMenu(name, anchor) {
  closePopover();
  const account = state.accounts.find((entry) => entry.name === name);
  if (!account) return;
  const pinned = isFavorite(name);
  const isProfile = (state.platformData && state.platformData.mode) === 'profile';
  const isRiot = /riot/i.test(state.platform || '');
  const menu = document.createElement('div');
  menu.className = 'popover';
  menu.innerHTML = `
    <div class="pop-head">
      <div class="pop-head-name" title="${esc(name)}">${esc(account.label && account.label !== name ? account.label : name)}</div>
      <div class="pop-head-id">
        ${account.id ? `<span class="mono" title="${esc(account.id)}">${esc(shortId(account.id))}</span>` : '<span class="faint">No ID captured</span>'}
      </div>
    </div>
    <div class="pop-sep"></div>
    <button class="pop-item" data-act="switch">${svgIcon('arrow')} ${isProfile ? 'Open this profile' : 'Switch to account'}</button>
    <button class="pop-item" data-act="fav">${svgIcon('star')} ${pinned ? 'Unpin from top' : 'Pin to top'}</button>
    <button class="pop-item" data-act="notes">${svgIcon('note')} Edit notes</button>
    ${isProfile ? '' : `<button class="pop-item" data-act="rename">${svgIcon('edit')} Rename</button>`}
    <div class="pop-sep"></div>
    <div class="pop-label">Artwork</div>
    <button class="pop-item" data-act="avatar">${svgIcon('image')} ${account.avatar ? 'Change picture…' : 'Add picture…'}</button>
    ${account.avatar ? `<button class="pop-item" data-act="avatar-clear">${svgIcon('close')} Remove picture</button>` : ''}
    <button class="pop-item" data-act="cover">${svgIcon('image')} ${account.cover ? 'Change cover…' : 'Add cover…'}</button>
    ${account.cover ? `<button class="pop-item" data-act="cover-clear">${svgIcon('close')} Remove cover</button>` : ''}
    ${isRiot ? `<button class="pop-item" data-act="valorant">${svgIcon('spark')} Fetch Valorant card…</button>
    <button class="pop-item" data-act="profile-refresh">${svgIcon('refresh')} Refresh Riot profile…</button>
    ${account.profile ? `<button class="pop-item" data-act="profile">${svgIcon('trophy')} View Riot profile</button>` : ''}` : ''}
    <div class="pop-sep"></div>
    <button class="pop-item" data-act="copy">${svgIcon('copy')} Copy ${isProfile ? 'profile folder' : 'account ID'}</button>
    <button class="pop-item" data-act="reveal">${svgIcon('folder')} Show folder</button>
    ${isProfile ? '' : `<div class="pop-sep"></div>
    <button class="pop-item danger" data-act="delete">${svgIcon('trash')} Delete</button>`}
  `;
  document.body.appendChild(menu);
  const rect = anchor.getBoundingClientRect();
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  let left = Math.min(rect.right - width, window.innerWidth - width - 10);
  let top = rect.bottom + 6;
  if (top + height > window.innerHeight - 10) top = Math.max(10, rect.top - height - 6);
  menu.style.left = `${Math.max(10, left)}px`;
  menu.style.top = `${top}px`;
  popover = menu;
  menu.addEventListener('click', (event) => {
    const button = event.target.closest('[data-act]');
    if (!button) return;
    const action = button.dataset.act;
    closePopover();
    if (action === 'switch') switchAccount(name);
    else if (action === 'fav') toggleFavorite(name);
    else if (action === 'notes') openNotesModal(name);
    else if (action === 'rename') openRenameModal(name);
    else if (action === 'copy') copyText(account.id || '');
    else if (action === 'reveal') revealAccount(name);
    else if (action === 'delete') deleteAccount(name);
    else if (action === 'avatar') chooseArtwork(name, 'avatar');
    else if (action === 'cover') chooseArtwork(name, 'cover');
    else if (action === 'avatar-clear') clearArtwork(name, 'avatar');
    else if (action === 'cover-clear') clearArtwork(name, 'cover');
    else if (action === 'valorant') openValorantModal(name);
    else if (action === 'profile') openProfile(name);
    else if (action === 'profile-refresh') openProfileRefresh(name);
  });
}

function openFolderMenu(groupId, anchor) {
  closePopover();
  const group = groupPlatforms(railPlatforms()).find((entry) => entry.id === groupId);
  if (!group) return;
  const menu = document.createElement('div');
  menu.className = 'popover folder-pop';
  menu.dataset.folder = groupId;
  menu.innerHTML = `
    <div class="pop-head">
      <div class="pop-head-name">${esc(group.name)}</div>
      <div class="pop-head-id">${group.items.length} service${group.items.length === 1 ? '' : 's'}</div>
    </div>
    <div class="pop-sep"></div>
    ${group.items.map((platform) => `
      <button class="folder-item ${platform.name === state.platform ? 'active' : ''}"
              data-platform="${esc(platform.name)}"
              title="${esc(platform.name)} · ${platform.accounts} account${platform.accounts === 1 ? '' : 's'}">
        <span class="p-icon" style="--ph:${hue(platform.name)}">${platformIconHTML(platform)}</span>
        <span class="folder-item-name">${esc(platform.name)}</span>
        <span class="folder-item-count">${platform.accounts}</span>
        ${platform.name === state.platform ? `<span class="folder-item-check">${svgIcon('check')}</span>` : ''}
      </button>
    `).join('')}
  `;
  document.body.appendChild(menu);
  const rect = anchor.getBoundingClientRect();
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  let left = Math.min(rect.left, window.innerWidth - width - 10);
  let top = rect.bottom + 6;
  if (top + height > window.innerHeight - 10) top = Math.max(10, rect.top - height - 6);
  menu.style.left = `${Math.max(10, left)}px`;
  menu.style.top = `${top}px`;
  popover = menu;
  menu.addEventListener('click', (event) => {
    const item = event.target.closest('[data-platform]');
    if (!item) return;
    event.preventDefault();
    event.stopPropagation();
    closePopover();
    selectPlatform(item.dataset.platform);
  });
}

async function chooseArtwork(name, kind) {
  const account = state.accounts.find((entry) => entry.name === name);
  if (!account) return;
  setStatus(kind === 'avatar' ? 'Choose a picture…' : 'Choose a cover image…', 'busy');
  try {
    const picked = await call('artwork_pick');
    if (!picked || !picked.path) {
      setStatus('Ready');
      return;
    }
    const result = await call('artwork_set', {
      platform: state.platform, account: name, kind, path: picked.path
    });
    if (kind === 'avatar') account.avatar = result.avatar || null;
    else account.cover = result.cover || null;
    renderRoster();
    toast('success', kind === 'avatar' ? 'Picture updated' : 'Cover updated', name);
    setStatus('Ready');
  } catch (error) {
    toast('error', 'Could not set picture', error.message);
    setStatus('Picture failed', 'error');
  }
}

async function clearArtwork(name, kind) {
  const account = state.accounts.find((entry) => entry.name === name);
  if (!account) return;
  try {
    await call('artwork_clear', { platform: state.platform, account: name, kind });
    if (kind === 'avatar') account.avatar = null;
    else account.cover = null;
    renderRoster();
    toast('info', kind === 'avatar' ? 'Picture removed' : 'Cover removed', name);
  } catch (error) {
    toast('error', 'Could not remove picture', error.message);
  }
}

async function openValorantModal(name) {
  let status;
  try {
    status = await call('valorant_status');
  } catch (error) {
    toast('error', 'Could not read the Riot sign-in', error.message);
    return;
  }
  if (!status || !status.signedIn) {
    toast('info', 'Riot is not signed in', 'Open the Riot Client, sign in to the account, then try again.');
    return;
  }
  const liveName = status.liveName || 'the signed-in account';
  const liveBase = String(liveName).split('#')[0].trim().toLowerCase();
  const mismatch = liveBase && liveBase !== name.trim().toLowerCase();
  openModal({
    icon: 'spark',
    title: 'Fetch Valorant card',
    subtitle: `${liveName} is signed in to Riot.`,
    body: `
      <p>Nexulon asks Riot which player card that account has equipped, downloads the art, and sets it as the cover for <b>${esc(name)}</b>.</p>
      ${status.running
        ? `<div class="notice info">${svgIcon('info')} <span>The Riot Client is open. Nexulon reads its local sign-in, so you can leave it running.</span></div>`
        : `<div class="notice quiet">${svgIcon('info')} <span>Riot rotates the sign-in token when it is used. Nexulon writes the new one back and keeps a backup, but an interrupted run may ask you to sign in to Riot again.</span></div>`}
      ${mismatch
        ? `<div class="notice info">${svgIcon('info')} <span>The client is signed in as <b>${esc(liveName)}</b>, not ${esc(name)}. The card will still be applied to ${esc(name)}.</span></div>`
        : ''}
    `,
    confirm: 'Fetch card',
    onConfirm: async () => {
      const result = await call('valorant_card', { platform: state.platform, account: name });
      const account = state.accounts.find((entry) => entry.name === name);
      if (account && result.cover) account.cover = result.cover;
      if (account && result.avatar) account.avatar = result.avatar;
      renderRoster();
      toast('success', 'Player card applied', `${result.cardName} is now the cover for ${name}.`);
      return true;
    }
  });
}

async function openProfileRefresh(name) {
  let status;
  try {
    status = await call('valorant_status');
  } catch (error) {
    toast('error', 'Could not read the Riot sign-in', error.message);
    return;
  }
  if (!status || !status.signedIn) {
    toast('info', 'Riot is not signed in', 'Open the Riot Client, sign in to the account, then try again.');
    return;
  }
  const liveName = status.liveName || 'the signed-in account';
  openModal({
    icon: 'trophy',
    title: 'Refresh Riot profile',
    subtitle: `${liveName} is signed in to Riot.`,
    body: `
      <p>Nexulon asks Riot for the current rank, RR, peak rank, account level, title, player card and sprays, then caches them on <b>${esc(name)}</b>.</p>
      ${status.running
        ? `<div class="notice info">${svgIcon('info')} <span>The Riot Client is open. Nexulon reads its local sign-in, so you can leave it running.</span></div>`
        : `<div class="notice quiet">${svgIcon('info')} <span>Riot rotates the sign-in token when it is used. Nexulon writes the new one back and keeps a backup, but an interrupted run may ask you to sign in to Riot again.</span></div>`}
    `,
    confirm: 'Refresh profile',
    onConfirm: async () => {
      const result = await call('riot_profile', { platform: state.platform, account: name });
      if (result && result.mismatch) {
        closeModal();
        confirmProfileMismatch(name, result.liveName);
        return false;
      }
      applyProfile(name, result);
      toast('success', 'Riot profile refreshed', profileLine(result.summary));
      return true;
    }
  });
}

function confirmProfileMismatch(name, liveName) {
  openModal({
    icon: 'alert',
    title: 'Different Riot account',
    subtitle: `${liveName} is signed in, not ${name}.`,
    body: `
      <div class="notice warn">${svgIcon('alert')} <span>This data belongs to <b>${esc(liveName)}</b>. Refreshing now saves it on <b>${esc(name)}</b>, replacing anything cached there.</span></div>
      <p>Sign in to ${esc(name)} in the Riot Client and refresh again if you meant to update that account.</p>
    `,
    confirm: 'Refresh anyway',
    onConfirm: async () => {
      const result = await call('riot_profile', {
        platform: state.platform, account: name, force: true
      });
      if (result && result.mismatch) return false;
      applyProfile(name, result);
      toast('success', 'Riot profile refreshed', profileLine(result.summary));
      return true;
    }
  });
}

function applyProfile(name, result) {
  const account = state.accounts.find((entry) => entry.name === name);
  if (account && result && result.summary) account.profile = result.summary;
  renderRoster();
}

/* -- automatic Riot details after a sign-in -------------------------------- */
const autoPullTimers = new Map();

/// Riot needs a few seconds after the client starts before its local API
/// answers, so the pull is deliberately delayed rather than fired immediately.
function scheduleAutoPull(platform, name, delay = 5000) {
  if (!/riot/i.test(platform)) return;
  if (state.prefs.autoProfile === false) return;
  const key = `${platform}/${name}`;
  const existing = autoPullTimers.get(key);
  if (existing) clearTimeout(existing);
  autoPullTimers.set(key, setTimeout(() => {
    autoPullTimers.delete(key);
    runAutoPull(platform, name);
  }, delay));
}

async function runAutoPull(platform, name, attempt = 0, options = {}) {
  const silent = options.silent === true;
  const force = options.force !== false;
  if (state.prefs.autoProfile === false) return;
  let status;
  try {
    status = await call('valorant_status');
  } catch {
    return;
  }
  if (!status || !status.signedIn) return;
  // Only read through the running client's local API. With the client closed
  // the fetch would rotate the refresh token, and the copy already saved in the
  // account folder would stop working.
  if (!status.running) {
    if (attempt >= 2) return;
    const key = `${platform}/${name}`;
    autoPullTimers.set(key, setTimeout(() => {
      autoPullTimers.delete(key);
      runAutoPull(platform, name, attempt + 1, options);
    }, 5000));
    return;
  }

  let result;
  try {
    // After a sign-in the session was just captured or restored, so the live
    // Riot account is this one and forcing skips the "different account"
    // check. The launch check passes force: false instead, because a switch
    // may have happened between the lookup and the fetch.
    result = await call('riot_profile', { platform, account: name, force });
  } catch {
    return; // the client may still be starting; the manual refresh remains
  }
  if (!result || result.mismatch) return;
  if (state.platform !== platform) return;

  applyProfile(name, result);
  const account = state.accounts.find((entry) => entry.name === name);
  if (account && (!account.cover || !account.avatar)) {
    try {
      const card = await call('valorant_card', { platform, account: name });
      if (account && !account.cover && card.cover) account.cover = card.cover;
      if (account && !account.avatar && card.avatar) account.avatar = card.avatar;
      renderRoster();
    } catch {
      // The rank and level above are still useful without the card art.
    }
  }
  if (!silent) {
    const line = profileLine(result.summary);
    toast('success', 'Riot details updated', line ? `${name} · ${line}` : name);
  }
}

/* -- automatic Riot details at launch -------------------------------------- */
/// Riot names its live session `Name#TAG`, while a saved account can be called
/// anything, so the bridge resolves which saved account is signed in. Only that
/// one is refreshed: reading another account would rotate its saved token.
function scheduleStartupCheck() {
  if (state.prefs.autoProfile === false) return;
  setTimeout(async () => {
    if (state.prefs.autoProfile === false) return;
    const platform = (state.platforms.find((entry) => /riot/i.test(entry.name)) || {}).name;
    if (!platform) return;
    let live;
    try {
      live = await call('riot_live_account', { platform });
    } catch {
      return;
    }
    if (!live || !live.account) return;
    runAutoPull(platform, live.account, 0, { silent: true, force: false });
  }, 5000);
}

function profileLine(summary) {
  if (!summary) return '';
  const parts = [];
  if (summary.rank) parts.push(summary.rr != null ? `${summary.rank} · ${summary.rr} RR` : summary.rank);
  if (summary.level) parts.push(`Level ${summary.level}`);
  return parts.join(' · ') || 'Profile updated';
}

async function openProfile(name) {
  let profile = null;
  try {
    profile = await call('profile', { platform: state.platform, account: name });
  } catch (error) {
    toast('error', 'Could not read the Riot profile', error.message);
    return;
  }
  if (!profile) {
    toast('info', 'No Riot profile cached', `Refresh the profile while ${name} is signed in to Riot.`);
    return;
  }
  const rank = profile.rank || {};
  const level = profile.level || 0;
  const icon = (state.accounts.find((entry) => entry.name === name) || {}).profile;
  const rankIcon = icon && icon.icon ? icon.icon : '';
  const history = Array.isArray(rank.history) ? rank.history : [];
  const extras = [];
  if (profile.title) extras.push(['Title', profile.title]);
  if (profile.banner) extras.push(['Player card', profile.banner]);
  if (Array.isArray(profile.sprays) && profile.sprays.length) {
    extras.push(['Sprays', profile.sprays.join(' · ')]);
  }
  $('#profile').innerHTML = `
    <div class="drawer-head">
      <div class="d-icon">${svgIcon('trophy')}</div>
      <div><h2>${esc(name)}</h2><p>${esc(profile.liveName || 'Riot profile')}${profile.region ? ` · ${esc(profile.region.toUpperCase())}` : ''}</p></div>
      <button class="icon-btn" data-close aria-label="Close profile">${svgIcon('close')}</button>
    </div>
    <div class="drawer-body">
      <div class="profile-hero">
        <div class="profile-rank-icon">${rankIcon ? `<img src="${esc(rankIcon)}" alt="">` : svgIcon('trophy')}</div>
        <div class="profile-hero-text">
          <div class="profile-rank-name">${esc(rank.name || 'Unranked')}</div>
          <div class="profile-rank-meta">${rank.rr != null ? `${rank.rr} RR` : 'No rating'}${rank.season ? ` · ${esc(rank.season)}` : ''}</div>
        </div>
        ${level ? `<div class="profile-level"><strong>${level}</strong><span>Level</span></div>` : ''}
      </div>

      <div class="profile-stats">
        <div><span>Peak rank</span><strong>${esc(rank.peak || '—')}</strong><small>${esc(rank.peakSeason || '')}</small></div>
        <div><span>Current</span><strong>${esc(rank.name || 'Unranked')}</strong><small>${rank.rr != null ? `${rank.rr} RR` : 'No rating'}</small></div>
      </div>

      ${extras.length ? `<div class="profile-extras">
        ${extras.map(([label, value]) => `<div><span>${esc(label)}</span><strong>${esc(value)}</strong></div>`).join('')}
      </div>` : ''}

      <div class="profile-history">
        <h3>${svgIcon('chart')} Season history</h3>
        ${history.length
          ? history.map((entry) => `
            <div class="history-row">
              <span class="history-season">${esc(entry.name)}</span>
              <span class="history-rank">${esc(entry.rank)}</span>
              <span class="history-rr">${entry.rr != null ? `${entry.rr} RR` : ''}</span>
            </div>`).join('')
          : '<div class="log-empty">No ranked seasons recorded for this account.</div>'}
      </div>

      <div class="profile-foot">
        <span>Fetched ${profile.fetchedAt ? timeAgo(profile.fetchedAt * 1000) : 'unknown'}</span>
        <button class="btn small" data-action="profile-refresh" data-name="${esc(name)}">${svgIcon('refresh')} Refresh</button>
      </div>
    </div>
  `;
  $('#profile').hidden = false;
  syncScrim();
}

function closePopover() {
  if (popover) {
    popover.remove();
    popover = null;
  }
}

async function copyText(text) {
  if (!text) {
    toast('info', 'Nothing to copy', 'This account has no saved ID yet.');
    return;
  }
  try {
    await navigator.clipboard.writeText(text);
    toast('success', 'Copied to clipboard', text);
  } catch (error) {
    const field = document.createElement('textarea');
    field.value = text;
    field.style.position = 'fixed';
    field.style.opacity = '0';
    document.body.appendChild(field);
    field.select();
    try { document.execCommand('copy'); } catch (ignored) { /* no-op */ }
    field.remove();
    toast('success', 'Copied to clipboard', text);
  }
}

async function revealAccount(name) {
  try {
    await call('reveal', { platform: state.platform, account: name });
  } catch (error) {
    toast('error', 'Could not open folder', error.message);
  }
}

async function openAddModal(options = {}) {
  const freshSignIn = options.freshSignIn === true;
  const isRiot = /riot/i.test(state.platform);
  let signedIn = false;
  let liveName = '';
  if (isRiot && !freshSignIn) {
    try {
      const status = await call('valorant_status');
      signedIn = Boolean(status && status.signedIn);
      liveName = (status && status.liveName) || '';
    } catch {
      signedIn = false;
    }
  }

  const hint = freshSignIn
    ? `Restarting ${esc(state.platform)} logged out…`
    : !isRiot
      ? `Sign in to ${esc(state.platform)} with the account you want to save, then give it a name here.`
      : signedIn
        ? `${esc(liveName || 'A Riot account')} is signed in. Press “Save current account” to store it.`
        : 'Restarting the Riot Client…';

  openModal({
    icon: freshSignIn ? 'login' : 'plus',
    title: freshSignIn ? 'Sign in to a new account' : 'Add account',
    subtitle: freshSignIn
      ? `${state.platform} restarts logged out, then you sign in with the account you want to save.`
      : isRiot
        ? signedIn
          ? 'Save the session that is already signed in.'
          : 'The Riot Client restarts logged out so you can sign in.'
        : `Capture the session currently signed in to ${state.platform}.`,
    body: `
      <div class="field">
        <label for="modal-input">Account name</label>
        <input class="input" id="modal-input" placeholder="e.g. Main, Smurf, Work" maxlength="48" autocomplete="off">
        <div class="hint" id="add-status">${hint}</div>
      </div>
    `,
    confirm: signedIn && !freshSignIn ? 'Save current account' : 'Add account',
    onMount: (root) => {
      if (!isRiot && !freshSignIn) return;
      const status = $('#add-status', root);
      const ok = $('[data-modal="ok"]', root);
      const restart = async () => {
        ok.disabled = true;
        if (status) {
          status.classList.remove('warn');
          status.textContent = `Restarting ${state.platform}…`;
        }
        try {
          await call('prepare_login', { platform: state.platform });
          if (status) {
            status.textContent = `${state.platform} has restarted. Sign in with the account you want to save, then press Add account.`;
          }
        } catch (error) {
          if (status) {
            status.classList.add('warn');
            status.textContent = error.message;
          }
          toast('error', 'Could not restart the launcher', error.message);
        } finally {
          ok.disabled = false;
        }
      };
      if (freshSignIn || !signedIn) restart();
    },
    onConfirm: async (root) => {
      const input = $('#modal-input', root);
      const name = input.value.trim();
      if (!name) {
        input.focus();
        return false;
      }
      await call('add', { platform: state.platform, name });
      await refreshAccounts();
      scheduleAutoPull(state.platform, name, 5000);
      toast('success', 'Account added', `${name} is ready to switch to.`);
      return true;
    }
  });
}

function openRenameModal(name) {
  openModal({
    icon: 'edit',
    title: 'Rename account',
    subtitle: 'The saved session folder is renamed; nothing is re-captured.',
    body: `
      <div class="field">
        <label for="modal-input">New name</label>
        <input class="input" id="modal-input" value="${esc(name)}" maxlength="48" autocomplete="off">
      </div>
    `,
    confirm: 'Rename',
    onConfirm: async (root) => {
      const newName = $('#modal-input', root).value.trim();
      if (!newName) return false;
      if (newName === name) return true;
      await call('rename', { platform: state.platform, account: name, new_name: newName });
      await refreshAccounts();
      toast('success', 'Account renamed', `${name} → ${newName}`);
      return true;
    }
  });
}

function openNotesModal(name) {
  const account = state.accounts.find((entry) => entry.name === name);
  openModal({
    icon: 'note',
    title: 'Account notes',
    subtitle: `Private notes for ${name}. Stored in your Nexulon data folder.`,
    body: `
      <div class="field">
        <label for="modal-input">Notes</label>
        <textarea class="input" id="modal-input" placeholder="e.g. Smurf — level 30, ranked only" maxlength="400">${esc(account ? account.notes : '')}</textarea>
        <div class="hint">Shown on the account card. Leave empty to remove.</div>
      </div>
    `,
    confirm: 'Save notes',
    onConfirm: async (root) => {
      const notes = $('#modal-input', root).value;
      await call('notes', { platform: state.platform, account: name, notes });
      await refreshAccounts({ keepStatus: true });
      toast('success', 'Notes saved', name);
      return true;
    }
  });
}

async function deleteAccount(name) {
  const platform = state.platform;
  try {
    await call('delete', { platform, account: name });
    if (state.platform === platform) await refreshAccounts();
    toast('success', 'Account deleted', name);
  } catch (error) {
    toast('error', 'Could not delete account', error.message);
  }
}

/* -- settings ------------------------------------------------------------- */
function openSettings(tab = 'general') {
  state.settingsTab = tab;
  $('#settings').hidden = false;
  renderSettings();
  syncScrim();
}

function closeDrawer(selector) {
  $(selector).hidden = true;
  syncScrim();
}

function toggleRow(title, subtitle, on, action) {
  return `
    <div class="setting-row">
      <div class="sr-text"><strong>${esc(title)}</strong><span>${esc(subtitle)}</span></div>
      <button class="switch ${on ? 'on' : ''}" role="switch" aria-checked="${on}" data-action="${action}" title="${on ? 'On' : 'Off'}"></button>
    </div>
  `;
}

function renderSettings() {
  const drawer = $('#settings');
  const body = $('#settings .drawer-body');
  const scroll = body ? body.scrollTop : 0;
  const tab = state.settingsTab;
  const tabs = [
    ['general', 'General'],
    ['appearance', 'Look'],
    ['platforms', 'Platforms'],
    ['about', 'About']
  ];
  let content = '';

  if (tab === 'general') {
    content = `
      <div>
        <h3 class="section-title">Switching</h3>
        <div class="section-card">
          ${toggleRow('Clear old login first', 'Remove stale session files before copying the new one', state.prefs.clearOldLogin !== false, 'pref-clear')}
          ${toggleRow('Start the launcher after switching', 'Launch the game client automatically', state.prefs.startLauncher !== false, 'pref-start')}
          ${toggleRow('Fetch Riot details automatically', 'Pull rank, level and the player card after a sign-in, and once about 5 seconds after launch', state.prefs.autoProfile !== false, 'pref-auto')}
          ${toggleRow('Launch at Windows sign-in', 'Start Nexulon Switcher with Windows', state.autostart, 'pref-autostart')}
        </div>
      </div>
      <div>
        <h3 class="section-title">Data</h3>
        <div class="section-card">
          <div class="setting-row">
            <div class="sr-text"><strong>Data folder</strong><span class="mono">${esc(state.dataRoot)}</span></div>
            <button class="btn small" data-action="open-data">${svgIcon('folder')} Open</button>
          </div>
          <div class="setting-row">
            <div class="sr-text"><strong>Quit Nexulon Switcher</strong><span>Close the app completely</span></div>
            <button class="btn small" data-action="quit">${svgIcon('power')} Quit</button>
          </div>
        </div>
      </div>
    `;
  } else if (tab === 'appearance') {
    content = `
      <div>
        <h3 class="section-title">Theme</h3>
        <div class="theme-grid">
          ${THEMES.map((theme) => `
            <button class="theme-card ${state.ui.theme === theme.id ? 'active' : ''}" data-theme-choice="${theme.id}">
              <span class="theme-preview ${theme.id}"></span><span>${theme.name}</span>
            </button>
          `).join('')}
        </div>
      </div>
      <div>
        <h3 class="section-title">Accent</h3>
        <div class="swatches">
          ${ACCENTS.map((accent) => `
            <button class="swatch ${state.ui.accent === accent.id ? 'active' : ''}"
                    style="--sw:${accent.color}" data-accent-choice="${accent.id}"
                    title="${accent.name}" aria-label="${accent.name}"></button>
          `).join('')}
        </div>
      </div>
      <div>
        <h3 class="section-title">Layout</h3>
        <div class="section-card">
          <div class="setting-row">
            <div class="sr-text"><strong>Card density</strong><span>How much room each account takes</span></div>
            <div class="seg">
              <button data-density="comfortable" class="${state.ui.density !== 'compact' ? 'active' : ''}">Comfort</button>
              <button data-density="compact" class="${state.ui.density === 'compact' ? 'active' : ''}">Compact</button>
            </div>
          </div>
          <div class="setting-row">
            <div class="sr-text"><strong>Default order</strong><span>Applied whenever a platform opens</span></div>
            <select class="select" id="sort-select">
              ${SORTS.map((sort) => `<option value="${sort.id}" ${state.ui.sort === sort.id ? 'selected' : ''}>${sort.name}</option>`).join('')}
            </select>
          </div>
          ${toggleRow('Reduce motion', 'Turn off ambient drift and entrance animation', state.ui.motion === 'reduced', 'toggle-motion')}
        </div>
      </div>
    `;
  } else if (tab === 'platforms') {
    content = `
      <div>
        <h3 class="section-title">Rail</h3>
        <div class="section-card">
          ${toggleRow('Show only installed services', 'Hide services that are not installed from the top rail', state.ui.installedOnly === true, 'toggle-installed')}
          ${toggleRow('Group services into folders', 'Show one folder per category instead of every service', state.ui.railFolders !== false, 'toggle-folders')}
        </div>
      </div>
      <div>
        <h3 class="section-title">Services & launchers</h3>
        <div class="pf-groups">
          ${groupPlatforms(state.platforms).map((group) => `
            <section class="pf-group">
              <h4 class="pf-group-title">${esc(group.name)}<span>${group.items.length}</span></h4>
              <div class="pf-group-list">
                ${group.items.map((platform) => `
                  <div class="pf-row">
                    <div class="pf-head" style="--ph:${hue(platform.name)}">
                      ${platformIconHTML(platform)}
                      <span class="pf-name">${esc(platform.name)}</span>
                      <span class="chip ${platform.installed ? 'installed' : 'missing'}"><i></i>${platform.installed ? 'Ready' : 'Missing'}</span>
                    </div>
                    <div class="pf-input-row">
                      <input class="input" value="${esc(platform.folder || '')}" placeholder="Launcher folder path"
                             data-folder-input="${esc(platform.name)}" spellcheck="false">
                      <button class="btn small" data-action="save-folder" data-platform="${esc(platform.name)}">Save</button>
                      <button class="btn small ghost" data-action="detect-one" data-platform="${esc(platform.name)}">Detect</button>
                    </div>
                  </div>
                `).join('')}
              </div>
            </section>
          `).join('')}
        </div>
        <div class="hint" style="margin-top:8px">Leave a path empty and press Save to fall back to the launcher's default location.</div>
      </div>
    `;
  } else {
    content = `
      <div class="about-hero">
        <img class="about-logo" src="${state.mark || state.logo}" alt="" width="64" height="64">
        <h3>NEXULON CORP</h3>
        <p>Session orchestration for persistent gaming.</p>
        <span class="about-version">Build 1.0.0</span>
      </div>
      <div>
        <h3 class="section-title">Storage</h3>
        <div class="path-box">${esc(state.dataRoot)}</div>
        <div style="display:flex;gap:8px;margin-top:8px">
          <button class="btn small" data-action="open-data">${svgIcon('folder')} Open data folder</button>
        </div>
      </div>
      <div class="hint">Data from an earlier switcher was imported once on first launch. That folder is no longer read or written.</div>
    `;
  }

  drawer.innerHTML = `
    <div class="drawer-head">
      <div class="d-icon">${svgIcon('settings')}</div>
      <div><h2>Settings</h2><p>Nexulon Corp</p></div>
      <button class="icon-btn" data-close aria-label="Close settings">${svgIcon('close')}</button>
    </div>
    <div class="drawer-body">
      <div class="tabs">
        ${tabs.map(([id, label]) => `<button class="tab ${tab === id ? 'active' : ''}" data-tab="${id}">${label}</button>`).join('')}
      </div>
      ${content}
    </div>
  `;
  const nextBody = $('#settings .drawer-body');
  if (nextBody) nextBody.scrollTop = scroll;
}

async function saveFolder(platformName) {
  const input = $$('[data-folder-input]').find((element) => element.dataset.folderInput === platformName);
  const path = input ? input.value.trim() : '';
  try {
    const result = await call('folder', { platform: platformName, path });
    const platform = state.platforms.find((entry) => entry.name === platformName);
    if (platform) {
      platform.folder = result.folder;
      platform.exe = result.exe;
      platform.installed = Boolean(result.installed);
    }
    toast('success', 'Folder saved', result.folder || 'Reset to the default location');
    if (state.platform === platformName) await refreshAccounts({ keepStatus: true });
    renderSettings();
  } catch (error) {
    toast('error', 'Could not save folder', error.message);
  }
}

async function detectOne(platformName) {
  try {
    const result = await call('detect', { platform: platformName });
    if (result.found) {
      const platform = state.platforms.find((entry) => entry.name === platformName);
      if (platform) platform.folder = result.folder;
      toast('success', 'Launcher found', result.folder);
      if (state.platform === platformName) await refreshAccounts({ keepStatus: true });
      renderSettings();
    } else {
      toast('info', 'Not found', 'Set the folder path manually.');
    }
  } catch (error) {
    toast('error', 'Detection failed', error.message);
  }
}

/* -- command palette ------------------------------------------------------ */
function paletteItems() {
  const items = [];
  const actions = [
    ['Open settings', 'General preferences', 'settings', () => openSettings('general')],
    ['Refresh accounts', state.platform || 'Current platform', 'refresh', () => refreshAccounts()],
    [`Add account to ${state.platform}`, 'Save the session that is signed in', 'plus', openAddModal],
    ...(canFreshSignIn()
      ? [[`Sign in to a new account in ${state.platform}`, 'Restart the launcher logged out', 'login', () => openAddModal({ freshSignIn: true })]]
      : []),
    ['Open data folder', state.dataRoot, 'folder', () => call('open_data').catch((error) => toast('error', 'Could not open folder', error.message))],
    ['Toggle compact cards', state.ui.density === 'compact' ? 'Currently compact' : 'Currently comfortable', 'grid', () => {
      saveUI({ density: state.ui.density === 'compact' ? 'comfortable' : 'compact' });
      toast('info', 'Layout updated', state.ui.density === 'compact' ? 'Compact cards' : 'Comfortable cards');
    }],
    ['Quit Nexulon Switcher', 'Close the app', 'power', () => call('quit').catch(() => {})]
  ];
  for (const [title, sub, iconName, run] of actions) {
    items.push({ group: 'Actions', title, sub, icon: iconName, run });
  }
  for (const platform of railPlatforms()) {
    items.push({
      group: 'Platforms',
      title: platform.name,
      sub: `${platform.accounts} account${platform.accounts === 1 ? '' : 's'}`,
      iconHtml: platformIconHTML(platform),
      run: () => selectPlatform(platform.name)
    });
  }
  for (const platform of state.platforms) {
    for (const name of platform.accountNames || []) {
      items.push({
        group: 'Accounts',
        title: name,
        sub: platform.name,
        icon: 'user',
        run: async () => {
          if (platform.name !== state.platform) await selectPlatform(platform.name, { silent: true });
          switchAccount(name);
        }
      });
    }
  }
  return items;
}

function buildPalette(query) {
  const all = paletteItems();
  const filtered = query.trim()
    ? all.filter((item) => fuzzy(`${item.title} ${item.sub} ${item.group}`, query.trim()))
    : all;
  state.paletteItems = filtered.slice(0, 80);
  if (state.paletteIndex >= state.paletteItems.length) state.paletteIndex = 0;
  let html = '';
  let group = '';
  state.paletteItems.forEach((item, index) => {
    if (item.group !== group) {
      group = item.group;
      html += `<div class="pal-group">${group}</div>`;
    }
    html += `
      <button class="pal-item ${index === state.paletteIndex ? 'active' : ''}" data-index="${index}">
        <span class="pal-ico">${item.iconHtml || svgIcon(item.icon)}</span>
        <span class="pal-main">
          <span class="pal-title">${esc(item.title)}</span>
          <span class="pal-sub">${esc(item.sub || '')}</span>
        </span>
        <span class="pal-hint">↵</span>
      </button>
    `;
  });
  if (!state.paletteItems.length) {
    html = `<div class="pal-empty">No matches for “${esc(query.trim())}”</div>`;
  }
  $('#palette-results').innerHTML = html;
  $('#palette-count').textContent = `${state.paletteItems.length} result${state.paletteItems.length === 1 ? '' : 's'}`;
}

function openPalette() {
  closePopover();
  $('#palette').hidden = false;
  $('#palette-input').value = '';
  state.paletteIndex = 0;
  buildPalette('');
  syncScrim();
  setTimeout(() => $('#palette-input').focus(), 20);
}

function closePalette() {
  $('#palette').hidden = true;
  syncScrim();
}

function movePalette(delta) {
  if (!state.paletteItems.length) return;
  state.paletteIndex = (state.paletteIndex + delta + state.paletteItems.length) % state.paletteItems.length;
  $$('#palette-results .pal-item').forEach((element, index) => element.classList.toggle('active', index === state.paletteIndex));
  const active = $('#palette-results .pal-item.active');
  if (active) active.scrollIntoView({ block: 'nearest' });
}

function runPaletteItem(index) {
  const item = state.paletteItems[index];
  if (!item) return;
  closePalette();
  item.run();
}

/* -- overlay plumbing ----------------------------------------------------- */
function syncScrim() {
  const open = !$('#palette').hidden || !$('#settings').hidden || !$('#profile').hidden || !$('#modal').hidden;
  $('#scrim').hidden = !open;
}

function closeTop() {
  if (popover) return closePopover();
  if (!$('#modal').hidden) return closeModal();
  if (!$('#palette').hidden) return closePalette();
  if (!$('#profile').hidden) return closeDrawer('#profile');
  if (!$('#settings').hidden) return closeDrawer('#settings');
}

let modalConfirm = null;

function openModal(options) {
  const root = $('#modal');
  root.hidden = false;
  root.innerHTML = `
    <div class="modal-card" role="dialog" aria-modal="true">
      <div class="modal-head">
        <div class="m-icon">${svgIcon(options.icon || 'spark')}</div>
        <div>
          <h3>${esc(options.title)}</h3>
          ${options.subtitle ? `<p>${esc(options.subtitle)}</p>` : ''}
        </div>
      </div>
      <div class="modal-body">${options.body || ''}</div>
      <div class="modal-foot">
        <button class="btn ghost" data-modal="cancel">Cancel</button>
        <button class="btn ${options.confirmClass || 'primary'}" data-modal="ok">${esc(options.confirm || 'Continue')}</button>
      </div>
    </div>
  `;
  modalConfirm = options.onConfirm || null;
  if (options.onMount) options.onMount(root);
  const ok = $('[data-modal="ok"]', root);
  ok.addEventListener('click', async () => {
    if (!modalConfirm) return closeModal();
    ok.disabled = true;
    try {
      const result = await modalConfirm(root);
      if (result !== false) closeModal();
    } catch (error) {
      toast('error', 'Something went wrong', error.message);
    } finally {
      ok.disabled = false;
    }
  });
  $('[data-modal="cancel"]', root).addEventListener('click', closeModal);
  root.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && event.target.tagName === 'INPUT') {
      event.preventDefault();
      ok.click();
    }
  });
  syncScrim();
  setTimeout(() => {
    const input = $('#modal-input', root);
    if (input) {
      input.focus();
      if (input.tagName === 'INPUT') input.select();
    }
  }, 30);
}

function closeModal() {
  const root = $('#modal');
  root.hidden = true;
  root.innerHTML = '';
  modalConfirm = null;
  syncScrim();
}

/* -- toasts --------------------------------------------------------------- */
function toast(kind, title, detail, duration = 4200, action = null) {
  const node = document.createElement('div');
  node.className = `toast ${kind}`;
  node.innerHTML = `
    <div class="toast-icon">${svgIcon(kind === 'success' ? 'check' : kind === 'error' || kind === 'warn' ? 'alert' : 'info')}</div>
    <div class="toast-body"><strong>${esc(title)}</strong>${detail ? `<span>${esc(detail)}</span>` : ''}</div>
    ${action ? `<button class="toast-action">${esc(action.label)}</button>` : ''}
    <button class="toast-x" aria-label="Dismiss">×</button>
    <i class="toast-bar" style="animation-duration:${duration}ms"></i>
  `;
  $('#toasts').appendChild(node);
  const remove = () => {
    if (node.classList.contains('leaving')) return;
    node.classList.add('leaving');
    setTimeout(() => node.remove(), 220);
  };
  node.querySelector('.toast-x').addEventListener('click', remove);
  const actionButton = node.querySelector('.toast-action');
  if (actionButton && action) {
    actionButton.addEventListener('click', async () => {
      actionButton.disabled = true;
      try {
        await action.run();
      } catch (error) {
        toast('error', 'Could not undo', error.message);
      }
      remove();
    });
  }
  setTimeout(remove, duration);
}

/* -- event wiring --------------------------------------------------------- */
function wire() {
  document.addEventListener('contextmenu', (event) => {
    if (!/^(INPUT|TEXTAREA)$/.test(event.target.tagName)) event.preventDefault();
  });
  document.addEventListener('dragover', (event) => event.preventDefault());
  document.addEventListener('drop', (event) => event.preventDefault());

  $('#rail').addEventListener('click', (event) => {
    const folder = event.target.closest('[data-folder]');
    if (folder) {
      event.stopPropagation();
      if (popover && popover.dataset.folder === folder.dataset.folder) {
        closePopover();
        return;
      }
      folder.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'smooth' });
      openFolderMenu(folder.dataset.folder, folder);
      return;
    }
    const pill = event.target.closest('[data-platform]');
    if (pill) selectPlatform(pill.dataset.platform);
  });

  // Let the mouse wheel move the service rail sideways, so folders that sit
  // past the right edge are always reachable without a trackpad gesture.
  $('#rail').addEventListener('wheel', (event) => {
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
    event.preventDefault();
    closePopover();
    $('#rail').scrollLeft += event.deltaY;
  }, { passive: false });
  $('#rail').addEventListener('scroll', updateRailFades, { passive: true });
  window.addEventListener('resize', updateRailFades);

  $('#roster').addEventListener('click', (event) => {
    const action = event.target.closest('[data-action]');
    if (action) {
      const name = action.dataset.action;
      if (name === 'menu') {
        event.stopPropagation();
        openAccountMenu(action.dataset.name, action);
        return;
      }
      if (name === 'add') return openAddModal();
      if (name === 'signin') return openAddModal({ freshSignIn: true });
      if (name === 'reload') return refreshAccounts();
      if (name === 'clear-search') {
        $('#search').value = '';
        state.query = '';
        renderRoster();
        return;
      }
      if (name === 'palette') return openPalette();
    }
    const card = event.target.closest('.card');
    if (!card) return;
    const index = Number(card.dataset.index);
    if (!Number.isNaN(index)) {
      state.selected = index;
      paintSelection();
    }
    clearTimeout(cardClickTimer);
    cardClickTimer = 0;
    const account = state.visible[index];
    // A single click opens the cached Riot profile. The delay gives a double
    // click time to cancel it, so signing in still works exactly as before.
    if (account && account.profile) {
      cardClickTimer = setTimeout(() => {
        cardClickTimer = 0;
        openProfile(account.name);
      }, 280);
    }
  });

  $('#roster').addEventListener('dblclick', (event) => {
    if (event.target.closest('[data-action="menu"]')) return;
    clearTimeout(cardClickTimer);
    cardClickTimer = 0;
    const card = event.target.closest('.card');
    if (card) switchAccount(card.dataset.name, card);
  });

  $('#platform-head').addEventListener('click', (event) => {
    const viewBtn = event.target.closest('[data-view]');
    if (viewBtn) {
      saveUI({ view: viewBtn.dataset.view });
      renderHead(state.platformData);
      renderRoster();
      return;
    }
    const action = event.target.closest('[data-action]');
    if (!action) return;
    if (action.dataset.action === 'add') openAddModal();
    if (action.dataset.action === 'signin') openAddModal({ freshSignIn: true });
    if (action.dataset.action === 'detect') detectFolder();
    if (action.dataset.action === 'reload') refreshAccounts();
  });

  $('#search').addEventListener('input', (event) => {
    state.query = event.target.value;
    state.selected = -1;
    renderRoster();
  });
  $('#search').addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      event.target.value = '';
      state.query = '';
      renderRoster();
      event.target.blur();
    } else if (event.key === 'Enter' && state.visible.length) {
      const account = state.visible[Math.max(0, state.selected)];
      if (account) switchAccount(account.name);
    }
  });

  $('#refresh-btn').addEventListener('click', async () => {
    setStatus('Refreshing…', 'busy');
    try {
      await reloadPlatforms();
      await refreshAccounts({ keepStatus: true });
      toast('info', 'Accounts refreshed', `${state.accounts.length} loaded from ${state.platform}`);
      setStatus('Ready');
    } catch (error) {
      toast('error', 'Refresh failed', error.message);
      setStatus('Refresh failed', 'error');
    }
  });
  $('#settings-btn').addEventListener('click', () => openSettings('general'));

  $('#scrim').addEventListener('click', closeTop);

  document.addEventListener('click', (event) => {
    if (popover && !popover.contains(event.target)) closePopover();
  });
  $('#content').addEventListener('scroll', closePopover, { passive: true });
  window.addEventListener('resize', closePopover);

  $('#settings').addEventListener('click', async (event) => {
    if (event.target.closest('[data-close]')) return closeDrawer('#settings');
    const tab = event.target.closest('[data-tab]');
    if (tab) {
      state.settingsTab = tab.dataset.tab;
      renderSettings();
      return;
    }
    const theme = event.target.closest('[data-theme-choice]');
    if (theme) {
      saveUI({ theme: theme.dataset.themeChoice });
      renderSettings();
      return;
    }
    const accent = event.target.closest('[data-accent-choice]');
    if (accent) {
      saveUI({ accent: accent.dataset.accentChoice });
      renderSettings();
      return;
    }
    const density = event.target.closest('button[data-density]');
    if (density) {
      saveUI({ density: density.dataset.density });
      renderSettings();
      return;
    }
    const action = event.target.closest('[data-action]');
    if (!action) return;
    const name = action.dataset.action;
    if (name === 'pref-clear') {
      state.prefs.clearOldLogin = !(state.prefs.clearOldLogin !== false);
      call('prefs', { clearOldLogin: state.prefs.clearOldLogin }).catch(() => {});
      renderSettings();
    } else if (name === 'pref-start') {
      state.prefs.startLauncher = !(state.prefs.startLauncher !== false);
      call('prefs', { startLauncher: state.prefs.startLauncher }).catch(() => {});
      renderSettings();
    } else if (name === 'pref-auto') {
      state.prefs.autoProfile = !(state.prefs.autoProfile !== false);
      call('prefs', { autoProfile: state.prefs.autoProfile }).catch(() => {});
      renderSettings();
    } else if (name === 'pref-autostart') {
      try {
        const result = await call('autostart', { enabled: !state.autostart });
        state.autostart = Boolean(result.enabled);
        toast('info', state.autostart ? 'Launch at sign-in enabled' : 'Launch at sign-in disabled', 'Nexulon Switcher');
        renderSettings();
      } catch (error) {
        toast('error', 'Could not update startup', error.message);
      }
    } else if (name === 'toggle-motion') {
      saveUI({ motion: state.ui.motion === 'reduced' ? 'full' : 'reduced' });
      renderSettings();
    } else if (name === 'toggle-installed') {
      saveUI({ installedOnly: !state.ui.installedOnly });
      renderRail();
      renderSettings();
    } else if (name === 'toggle-folders') {
      saveUI({ railFolders: state.ui.railFolders === false });
      renderRail();
      renderSettings();
    } else if (name === 'open-data') {
      call('open_data').catch((error) => toast('error', 'Could not open folder', error.message));
    } else if (name === 'open-log') {
      call('open_log').catch((error) => toast('error', 'Could not open log', error.message));
    } else if (name === 'quit') {
      call('quit').catch(() => {});
    } else if (name === 'save-folder') {
      await saveFolder(action.dataset.platform);
    } else if (name === 'detect-one') {
      await detectOne(action.dataset.platform);
    }
  });

  $('#settings').addEventListener('change', (event) => {
    if (event.target.id === 'sort-select') {
      saveUI({ sort: event.target.value });
      renderRoster();
    }
  });

  $('#profile').addEventListener('click', (event) => {
    if (event.target.closest('[data-close]')) return closeDrawer('#profile');
    const action = event.target.closest('[data-action="profile-refresh"]');
    if (!action) return;
    const name = action.dataset.name;
    closeDrawer('#profile');
    openProfileRefresh(name);
  });

  $('#palette-input').addEventListener('input', (event) => {
    state.paletteIndex = 0;
    buildPalette(event.target.value);
  });
  $('#palette-results').addEventListener('click', (event) => {
    const item = event.target.closest('[data-index]');
    if (item) runPaletteItem(Number(item.dataset.index));
  });

  document.addEventListener('keydown', (event) => {
    const field = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName);
    const onButton = /^(BUTTON|A)$/.test(document.activeElement.tagName);
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      openPalette();
      return;
    }
    if (event.key === 'Escape') {
      closeTop();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      $('#search').focus();
      $('#search').select();
      return;
    }
    if (!$('#palette').hidden) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        movePalette(event.key === 'ArrowDown' ? 1 : -1);
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        runPaletteItem(state.paletteIndex);
        return;
      }
    }
    if (!$('#modal').hidden) return;
    if (field) return;
    if ((event.ctrlKey || event.metaKey) && /^[1-9]$/.test(event.key)) {
      const platform = railPlatforms()[Number(event.key) - 1];
      if (platform) {
        event.preventDefault();
        selectPlatform(platform.name);
      }
      return;
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      moveSelection(1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      moveSelection(-1);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      moveSelection(gridColumns());
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      moveSelection(-gridColumns());
    } else if (event.key === 'Enter' && !onButton && state.selected >= 0 && state.visible[state.selected]) {
      event.preventDefault();
      switchAccount(state.visible[state.selected].name);
    } else if (event.key === '/') {
      event.preventDefault();
      $('#search').focus();
    }
  });
}

/* -- boot ----------------------------------------------------------------- */
async function boot() {
  try {
    const data = await call('bootstrap');
    state.prefs = data.prefs || {};
    state.ui = { ...DEFAULT_UI, ...(state.prefs.ui || {}) };
    // Older builds stored the theme as "nebula"; the Circuit palette replaced
    // it, so migrate the saved value instead of showing no active theme card.
    if (!THEMES.some((entry) => entry.id === state.ui.theme)) state.ui.theme = 'circuit';
    state.autostart = Boolean(data.autostart);
    state.version = data.version || '';
    state.dataRoot = data.dataRoot || '';
    state.logPath = data.logPath || '';
    state.platforms = data.platforms || [];
    state.logo = data.logo || '';
    state.mark = data.mark || '';
    applyAppearance();
    renderRail();
    const first = state.platforms.find((platform) => platform.name === state.ui.lastPlatform) || state.platforms[0];
    if (first) {
      await selectPlatform(first.name, { keepStatus: true });
      setStatus('Ready');
    } else {
      $('#platform-head').innerHTML = '';
      $('#roster').innerHTML = `
        <div class="empty">
          <div class="orb">${svgIcon('spark')}</div>
          <h3>No platforms enabled</h3>
          <p>Enable a launcher in the original switcher, or add platform definitions to your Nexulon data folder.</p>
        </div>
      `;
      setStatus('No platforms');
    }
    updateStatusMeta();
    scheduleStartupCheck();
  } catch (error) {
    setStatus('Could not start', 'error');
    $('#roster').innerHTML = `
      <div class="empty">
        <div class="orb">${svgIcon('alert')}</div>
        <h3>The bridge did not answer</h3>
        <p>${esc(error.message)}</p>
      </div>
    `;
  }
}

wire();
boot();
