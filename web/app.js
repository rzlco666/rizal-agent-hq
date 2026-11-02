// Rizal Agent HQ — app: data live (WebSocket + cadangan REST), panel, detail 5 tab.
import { initOffice } from './office3d.js';

const $ = s => document.querySelector(s);
const state = { snap: null, selected: null, tab: 'ringkasan', raw: false, office: null, mode2d: false };

const fmtTok = n => n >= 1e6 ? (n / 1e6).toFixed(2).replace('.', ',') + ' jt' : n >= 1e3 ? (n / 1e3).toFixed(1).replace('.', ',') + ' rb' : String(n);
const fmtUsd = n => '$' + n.toFixed(2);
const fmtTime = ms => new Date(ms).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' });
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const STATUS_LABEL = { 'bekerja': 'Bekerja', 'berpikir': 'Berpikir', 'menunggu-rizal': 'Menunggu Rizal', 'santai': 'Santai', 'offline': 'Offline', 'error': 'Error' };
const agentById = id => state.snap?.agents.find(a => a.id === id);

// ---------- render panel ----------
function renderKpi(s) {
  $('#kpi').innerHTML = `
    <div><b>${s.total_usage.requests.toLocaleString('id-ID')}</b><span>request OMP tercatat</span></div>
    <div><b>${fmtTok(s.total_usage.input_tokens + s.total_usage.output_tokens)}</b><span>token (masuk+keluar)</span></div>
    <div><b>${fmtUsd(s.total_usage.cash_usd)}</b><span>biaya TUNAI</span></div>
    <div><b>${fmtUsd(s.total_usage.shadow_usd)}</b><span>biaya bayangan (langganan)</span></div>
    <div><b>${s.agents.filter(a => a.status === 'bekerja').length} bekerja</b><span>dari ${s.agents.length} agent tampil</span></div>
    <div><b>${s.deepseek_balance_est != null ? fmtUsd(s.deepseek_balance_est) : '—'}</b><span>estimasi saldo DeepSeek</span></div>`;
}

function renderAlerts(s) {
  const aktif = s.alerts.filter(a => a.state === 'aktif');
  const badge = $('#alertBadge');
  badge.textContent = `🚨 ${aktif.length}`;
  badge.classList.toggle('zero', aktif.length === 0);
  if (!aktif.length) { $('#alerts').innerHTML = '<p class="mut small">Tidak ada alert aktif. Semua aman.</p>'; return; }
  const cls = l => l === 'kritis' ? '' : l === 'perlu-action' ? 'act' : 'warn';
  const lbl = l => l === 'kritis' ? '🔴 KRITIS' : l === 'perlu-action' ? '🟠 PERLU ACTION' : '🟡 PERINGATAN';
  $('#alerts').innerHTML = aktif.map(a => `
    <div class="alertCard ${cls(a.level)}">
      <b>${lbl(a.level)} — ${esc(a.title)}</b>
      <div class="needs">${esc(a.needs)}</div>
      <div class="small mut">Sejak ${fmtTime(a.since)} WIB · agent: ${esc(agentById(a.agent_id)?.name || a.agent_id)}</div>
      <div style="margin-top:6px;display:flex;gap:6px">
        <button data-ack="${esc(a.id)}">✅ Sudah saya tangani</button>
        <button data-snooze="${esc(a.id)}">⏰ Tunda 1 jam</button>
      </div>
    </div>`).join('');
  document.querySelectorAll('[data-ack]').forEach(b => b.onclick = () => alertAction(b.dataset.ack, 'ack'));
  document.querySelectorAll('[data-snooze]').forEach(b => b.onclick = () => alertAction(b.dataset.snooze, 'snooze'));
}

async function alertAction(id, kind) {
  const url = kind === 'ack' ? `/api/alerts/${encodeURIComponent(id)}/ack` : `/api/alerts/${encodeURIComponent(id)}/snooze`;
  const body = kind === 'snooze' ? { minutes: 60 } : undefined;
  await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined }).catch(() => {});
  refresh();
}

function renderQuotas(s) {
  const rows = s.quotas.filter(q => q.percent != null);
  if (!rows.length) { $('#quotas').innerHTML = '<p class="mut small">Belum terbaca.</p>'; return; }
  $('#quotas').innerHTML = rows.map(q => `
    <div class="qrow">
      <div class="top"><span>${esc(q.provider)} · ${esc(q.label)}</span><b>${q.percent}%</b></div>
      <div class="bar"><i class="${q.percent >= 100 ? 'full' : q.percent >= 80 ? 'hot' : ''}" style="width:${Math.min(q.percent, 100)}%"></i></div>
      <div class="mut small">${esc(q.extra || '')}</div>
    </div>`).join('');
}

function renderEnv(s) {
  const e = s.env;
  $('#env').innerHTML = `
    🖥️ ${esc(e.hostname)} · CPU ${e.cpu_percent}% · RAM ${e.mem_percent}% · Disk ${e.disk_percent}%<br>
    🐳 Docker jalan: ${e.docker_up.length ? esc(e.docker_up.join(', ')) : '—'}${e.docker_down.length ? `<br><span class="mut">berhenti: ${esc(e.docker_down.join(', '))}</span>` : ''}<br>
    📦 ${e.process_total} proses · ⏱️ uptime ${Math.floor(e.uptime_sec / 3600)} jam<br>
    ${e.recent_commits.length ? '<br><b>Commit terbaru:</b><br>' + e.recent_commits.slice(0, 4).map(c => '• ' + esc(c)).join('<br>') : ''}`;
}

function renderTasks(s) {
  if (!s.tasks.length) { $('#tasks').innerHTML = '<p class="mut small">Belum ada tugas terbaca.</p>'; return; }
  $('#tasks').innerHTML = s.tasks.map(t => `
    <div class="task">
      <div style="display:flex;gap:8px;align-items:baseline">
        <b style="flex:1">${esc(t.title)}</b>
        <span class="pill ${esc(t.phase)}">${esc(t.phase.replace('-', ' '))}</span>
      </div>
      <div class="mut small">${esc(agentById(t.agent_id)?.name || t.agent_id)} · ${esc(t.detail)}${t.started_at ? ' · mulai ' + fmtTime(t.started_at) + ' WIB' : ''}</div>
      ${t.steps?.length ? `<ul class="steps">${t.steps.map(st => `<li class="${st.done ? 'done' : ''}">${st.done ? '✅' : '⏳'} ${esc(st.label)}</li>`).join('')}</ul>` : ''}
    </div>`).join('');
}

function renderEvents(s) {
  if (!s.events.length) { $('#events').innerHTML = '<p class="mut small">Belum ada log.</p>'; return; }
  $('#events').innerHTML = s.events.map(e => `
    <div class="evt">
      <span class="mut small">${fmtTime(e.at)}</span> ${esc(state.raw && e.raw ? e.raw : e.human)}
      <span class="mut small">· ${esc(agentById(e.agent_id)?.name || e.agent_id)}</span>
    </div>`).join('');
}

function render2d(s) {
  $('#grid2d').innerHTML = s.agents.map(a => `
    <div class="card2d" data-sel="${esc(a.id)}">
      <b>${esc(a.emoji)} ${esc(a.name)}</b> <span class="pill ${esc(a.status)}">${STATUS_LABEL[a.status] || esc(a.status)}</span>
      <div class="small" style="margin-top:5px">${esc(a.activity)}</div>
      <div class="mut small">${esc(a.role)} · token ${fmtTok(a.usage.input_tokens + a.usage.output_tokens)} · ${fmtUsd(a.usage.cash_usd + a.usage.shadow_usd)}</div>
    </div>`).join('');
  document.querySelectorAll('[data-sel]').forEach(c => c.onclick = () => selectAgent(c.dataset.sel));
}

// ---------- drawer detail 5 tab ----------
function selectAgent(id) {
  state.selected = id;
  state.tab = 'ringkasan';
  renderDrawer();
  $('#drawer').classList.add('open');
  if (state.office && !state.mode2d) state.office.focusAgent(id);
}
function closeDrawer() { $('#drawer').classList.remove('open'); state.selected = null; }

function tabBtn(id, label) { return `<button data-tab="${id}" class="${state.tab === id ? 'on' : ''}">${label}</button>`; }

function renderDrawer() {
  const a = agentById(state.selected);
  if (!a) { closeDrawer(); return; }
  const s = state.snap;
  const u = a.usage;
  const head = `
    <h3>${esc(a.emoji)} ${esc(a.name)}</h3>
    <div><span class="pill ${esc(a.status)}">${STATUS_LABEL[a.status] || esc(a.status)}</span>
    <span class="mut small"> ${esc(a.role)}</span></div>
    <p>${esc(a.activity)}</p>
    <div class="tabs">${tabBtn('ringkasan', 'Ringkasan')}${tabBtn('tugas', 'Tugas')}${tabBtn('log', 'Log')}${tabBtn('usage', 'Usage')}${tabBtn('sesi', 'Sesi & File')}</div>`;
  let body = '';
  if (state.tab === 'ringkasan') {
    const myAlerts = s.alerts.filter(x => x.agent_id === a.id && x.state === 'aktif');
    body = `<dl class="kv">
      <dt>Status</dt><dd>${STATUS_LABEL[a.status] || esc(a.status)}</dd>
      <dt>Jenis</dt><dd>${a.kind === 'utama' ? 'Agent utama (manusia)' : a.kind === 'worker' ? 'Worker (robot)' : 'MCP (robot)'}</dd>
      <dt>Induk</dt><dd>${a.parent ? esc(agentById(a.parent)?.name || a.parent) : '—'}</dd>
      <dt>Provider</dt><dd>${esc(a.provider || '—')}</dd>
      <dt>Model</dt><dd>${esc(a.model || '—')}</dd>
      <dt>Proses</dt><dd>${a.processes} · CPU ${a.cpu_percent}%</dd>
      <dt>Alert aktif</dt><dd>${myAlerts.length ? myAlerts.map(x => esc(x.title)).join('; ') : 'tidak ada'}</dd></dl>`;
  } else if (state.tab === 'tugas') {
    const mine = s.tasks.filter(t => t.agent_id === a.id);
    body = mine.length ? mine.map(t => `
      <div class="task"><b>${esc(t.title)}</b> <span class="pill ${esc(t.phase)}">${esc(t.phase)}</span>
      <div class="mut small">${esc(t.detail)}${t.started_at ? ' · ' + fmtTime(t.started_at) + ' WIB' : ''}</div>
      ${t.steps?.length ? `<ul class="steps">${t.steps.map(st => `<li class="${st.done ? 'done' : ''}">${st.done ? '✅' : '⏳'} ${esc(st.label)}</li>`).join('')}</ul>` : ''}</div>`).join('')
      : '<p class="mut">Belum ada tugas tercatat untuk agent ini.</p>';
  } else if (state.tab === 'log') {
    const mine = s.events.filter(e => e.agent_id === a.id);
    body = (mine.length ? mine : s.events.slice(0, 8)).map(e => `
      <div class="evt"><span class="mut small">${fmtTime(e.at)}</span> ${esc(state.raw && e.raw ? e.raw : e.human)}</div>`).join('')
      + '<p class="mut small">Log ditampilkan manusiawi; aktifkan "lihat mentah" di panel Log untuk teks asal.</p>';
  } else if (state.tab === 'usage') {
    body = `<dl class="kv">
      <dt>Request</dt><dd>${u.requests.toLocaleString('id-ID')} (gagal ${u.failed})</dd>
      <dt>Token masuk</dt><dd>${fmtTok(u.input_tokens)}</dd>
      <dt>Token keluar</dt><dd>${fmtTok(u.output_tokens)}</dd>
      <dt>Token cache</dt><dd>${fmtTok(u.cache_tokens)}</dd>
      <dt>Biaya TUNAI</dt><dd>${fmtUsd(u.cash_usd)}</dd>
      <dt>Biaya bayangan</dt><dd>${fmtUsd(u.shadow_usd)}</dd></dl>
      <p class="mut small">Tunai = uang nyata (API berbayar). Bayangan = nilai setara API dari langganan, bukan tagihan baru.</p>`;
  } else if (state.tab === 'sesi') {
    const mine = s.tasks.filter(t => t.agent_id === a.id);
    body = `<b>Sesi/tugas terakhir agent ini</b>` + (mine.length ? mine.map(t => `<div class="task">${esc(t.title)}<div class="mut small">${esc(t.detail)}</div></div>`).join('') : '<p class="mut">—</p>')
      + `<b>Commit terbaru di mesin</b>` + (s.env.recent_commits.length ? s.env.recent_commits.map(c => `<div class="evt">${esc(c)}</div>`).join('') : '<p class="mut">—</p>');
  }
  $('#drawerBody').innerHTML = head + body;
  document.querySelectorAll('#drawerBody [data-tab]').forEach(b => b.onclick = () => { state.tab = b.dataset.tab; renderDrawer(); });
}

// ---------- siklus render ----------
function renderAll(s) {
  $('#clock').textContent = 'Diperbarui ' + new Date(s.generated_at).toLocaleString('id-ID') + ' WIB · ' + s.agents.filter(a => a.status === 'bekerja').length + ' agent bekerja';
  renderKpi(s); renderAlerts(s); renderQuotas(s); renderEnv(s); renderTasks(s); renderEvents(s); render2d(s);
  if (state.office && !state.mode2d) state.office.update(s.agents);
  if (state.selected) renderDrawer();
}

async function refresh() {
  try {
    const r = await fetch('/api/state');
    if (r.ok) { state.snap = await r.json(); renderAll(state.snap); }
  } catch { /* abaikan; WS/poll berikutnya mencoba lagi */ }
}

// WebSocket live; jatuh ke polling 10 dtk bila gagal.
function connectWs() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const ws = new WebSocket(`${proto}//${location.host}/ws`);
  ws.onopen = () => { $('#wsState').textContent = '🟢 live (WebSocket)'; };
  ws.onmessage = ev => {
    try { state.snap = JSON.parse(ev.data); renderAll(state.snap); } catch { /* abaikan */ }
  };
  ws.onclose = () => { $('#wsState').textContent = '🟡 polling (WS putus)'; setTimeout(connectWs, 5000); };
  ws.onerror = () => ws.close();
}

// ---------- mulai ----------
try {
  state.office = initOffice($('#stage'), {
    onSelect: id => selectAgent(id),
    onHover: (id, ev) => {
      const tip = $('#bubbleTip');
      const a = agentById(id);
      if (!a) { tip.style.display = 'none'; return; }
      tip.innerHTML = `<b>${esc(a.emoji)} ${esc(a.name)}</b> · ${STATUS_LABEL[a.status] || esc(a.status)}<br>${esc(a.activity)}`;
      tip.style.display = 'block';
      const st = $('#stage').getBoundingClientRect();
      tip.style.left = Math.min(ev.clientX - st.left + 14, st.width - 315) + 'px';
      tip.style.top = (ev.clientY - st.top + 12) + 'px';
    },
  });
} catch (e) {
  // WebGL/CDN gagal -> mode 2D otomatis.
  state.mode2d = true;
  $('#grid2d').style.display = 'grid';
  $('#btn2d').textContent = '🗂️ Mode 2D (aktif)';
}
document.querySelectorAll('[data-cam]').forEach(b => b.onclick = () => state.office?.preset(b.dataset.cam, state.snap?.agents));
$('#btn2d').onclick = () => {
  state.mode2d = !state.mode2d;
  $('#grid2d').style.display = state.mode2d ? 'grid' : 'none';
  const cv = $('#stage canvas');
  if (cv) cv.style.display = state.mode2d ? 'none' : 'block';
  $('#btn2d').textContent = state.mode2d ? '🏢 Mode 3D' : '🗂️ Mode 2D';
  if (state.snap) renderAll(state.snap);
};
$('#alertBadge').onclick = () => document.querySelector('aside').scrollTo({ top: 0, behavior: 'smooth' });
$('#drawerClose').onclick = closeDrawer;
$('#rawToggle').onchange = e => { state.raw = e.target.checked; if (state.snap) { renderEvents(state.snap); if (state.selected) renderDrawer(); } };

refresh();
connectWs();
setInterval(() => { if (!state.snap || Date.now() - state.snap.generated_at > 20000) refresh(); }, 10000);
