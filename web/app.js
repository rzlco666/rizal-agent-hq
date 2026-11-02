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
  const fb = s.fallback || {};
  const fbHtml = fb.label ? `<div class="qrow" style="border:1px solid ${fb.active ? '#e5484d' : '#c9d2dd'};border-radius:8px;padding:6px 8px;margin-top:8px">
      <div class="top"><span>⛑️ Fallback utama · ${esc(fb.label)}</span><b>${fb.active ? 'AKTIF' : 'Siaga'}</b></div>
      <div class="mut small">${esc(fb.model || '')}${fb.reserve_label ? `<br>Cadangan terakhir: ${esc(fb.reserve_label)}${fb.reserve_balance_usd != null ? ' · saldo ±' + fmtUsd(fb.reserve_balance_usd) : ''}` : ''}<br>${esc(fb.note || '')}</div>
    </div>` : '';
  $('#quotas').innerHTML = rows.map(q => `
    <div class="qrow">
      <div class="top"><span>${esc(q.provider)} · ${esc(q.label)}</span><b>${q.percent}%</b></div>
      <div class="bar"><i class="${q.percent >= 100 ? 'full' : q.percent >= 80 ? 'hot' : ''}" style="width:${Math.min(q.percent, 100)}%"></i></div>
      <div class="mut small">${esc(q.extra || '')}</div>
    </div>`).join('') + fbHtml;
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

// ---------- overlay: papan tugas & fasilitas ----------
const view = { open: null };
function openView(name) {
  view.open = name;
  $('#boardOverlay').classList.toggle('open', name === 'board');
  $('#facOverlay').classList.toggle('open', name === 'fac');
  if (state.snap) { if (name === 'board') renderBoard(state.snap); if (name === 'fac') renderFac(state.snap); }
}
function renderBoard(s) {
  const cols = [
    ['berjalan', '🏃 Sedang berjalan'], ['menunggu-rizal', '✋ Menunggu Rizal'],
    ['selesai', '✅ Selesai'], ['gagal', '❌ Gagal'],
  ];
  const inCol = (t, k) => k === 'berjalan' ? (t.phase === 'berjalan' || t.phase === 'berpikir') : t.phase === k;
  $('#boardCols').innerHTML = cols.map(([k, label]) => {
    const items = s.tasks.filter(t => inCol(t, k));
    return `<div class="bcol"><h3>${label}<span>${items.length}</span></h3>` +
      (items.length ? items.map(t => `
        <div class="bcard"><b>${esc(t.title)}</b>
          <div class="mut small">${esc(agentById(t.agent_id)?.name || t.agent_id)}${t.started_at ? ' · ' + fmtTime(t.started_at) + ' WIB' : ''}</div>
          <div class="small">${esc(t.detail)}</div>
          ${t.steps?.length ? `<ul class="steps">${t.steps.map(st => `<li class="${st.done ? 'done' : ''}">${st.done ? '✅' : '⏳'} ${esc(st.label)}</li>`).join('')}</ul>` : ''}
        </div>`).join('') : '<p class="mut small">Tidak ada tugas di fase ini.</p>') + '</div>';
  }).join('');
}
function renderFac(s) {
  const e = s.env;
  const bar = (label, pct) => `<div class="small">${label} — ${pct}%</div><div class="fbar"><i style="width:${Math.min(pct, 100)}%"></i></div>`;
  const jebol = s.quotas.filter(q => q.percent != null && q.percent >= 100);
  $('#facBody').innerHTML = `
    <div class="fcard"><h3>🖥️ Mesin Host — ${esc(e.hostname)}</h3>
      ${bar('CPU', e.cpu_percent)}${bar('RAM', e.mem_percent)}${bar('Disk', e.disk_percent)}
      <div class="small mut">⏱️ Uptime ${Math.floor(e.uptime_sec / 3600)} jam · 📦 ${e.process_total} proses berjalan</div></div>
    <div class="fcard"><h3>🐳 Docker</h3>
      ${e.docker_up.length ? e.docker_up.map(d => `<span class="fchip">${esc(d)}</span>`).join('') : '<span class="mut small">Tidak ada kontainer jalan.</span>'}
      ${e.docker_down.length ? '<div style="margin-top:6px">' + e.docker_down.map(d => `<span class="fchip down">${esc(d)} (berhenti)</span>`).join('') + '</div>' : ''}</div>
    <div class="fcard"><h3>🌐 Koneksi Muse (Tailscale)</h3>
      <div class="small">${e.tailscale_muse_online == null ? 'Status tidak terbaca.' : e.tailscale_muse_online ? '🟢 Muse terhubung ke tailnet — jalur SSH WSL aktif.' : '🔴 Muse tidak terlihat di tailnet.'}</div>
      <div class="small mut" style="margin-top:6px">Layanan HQ: http://localhost:8900 · data diperbarui ${new Date(s.generated_at).toLocaleTimeString('id-ID')} WIB</div></div>
    <div class="fcard"><h3>🚨 Kuota jebol</h3>
      ${jebol.length ? jebol.map(q => `<span class="fchip hot">${esc(q.provider)} · ${esc(q.label)} ${q.percent}%</span>`).join('') : '<span class="mut small">Semua kuota aman.</span>'}
      ${s.fallback?.label ? `<div class="small" style="margin-top:7px">⛑️ Fallback utama: <b>${esc(s.fallback.label)}</b> — ${s.fallback.active ? 'AKTIF' : 'siaga'}${s.fallback.reserve_balance_usd != null ? ` · cadangan DeepSeek langsung ±${fmtUsd(s.fallback.reserve_balance_usd)}` : ''}</div>` : ''}</div>
    <div class="fcard"><h3>📝 Commit terbaru di mesin</h3>
      ${e.recent_commits.length ? e.recent_commits.map(c => `<div class="evt" style="margin:5px 0">${esc(c)}</div>`).join('') : '<p class="mut small">Tidak terbaca.</p>'}</div>
    <div class="fcard"><h3>🏢 Tentang kantor ini</h3>
      <div class="small">Rizal Agent HQ v2.1 — Rust (Axum + WebSocket + SQLite) satu binary + Three.js. ${s.agents.length} agen tampil, ${s.agents.filter(x => x.status === 'bekerja').length} sedang bekerja. Klik interior kantor (sofa, meja tenis, rak server, layar meeting) untuk interaksi.</div></div>`;
}

// ---------- siklus render ----------
function renderAll(s) {
  $('#clock').textContent = 'Diperbarui ' + new Date(s.generated_at).toLocaleString('id-ID') + ' WIB · ' + s.agents.filter(a => a.status === 'bekerja').length + ' agent bekerja';
  renderKpi(s); renderAlerts(s); renderQuotas(s); renderEnv(s); renderTasks(s); renderEvents(s); render2d(s);
  if (view.open === 'board') renderBoard(s);
  if (view.open === 'fac') renderFac(s);
  if (state.office && !state.mode2d) state.office.update(s.agents);
  if (state.selected) renderDrawer();
  if (_wantMeet && state.office) { _wantMeet = false; state.office.callMeeting(); }
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
    onInteract: (hid) => {
      const tip = $('#bubbleTip');
      const mcp = agentById('mcp');
      const texts = {
        pingpong: '🏓 Meja tenis meja di ruang santai — agen terdekat sampai melompat kegirangan.',
        sofa: '🛋️ Ruang santai — sofa, beanbag, dan meja kopi untuk agen melepas lelah.',
        chart: '📊 Layar meeting — grafik aktivitas kantor. Kamera kufokuskan ke ruang meeting.',
        rack: `🗄️ Rak server MCP — ${mcp ? esc(mcp.activity) : 'server MCP berjalan.'}`,
        entrance: '🏢 RIZAL AGENT HQ — markas semua agen Rizal. Kamera kembali ke seluruh kantor.',
      };
      if (hid === 'chart') state.office?.preset('meeting', state.snap?.agents);
      if (hid === 'rack') state.office?.preset('mcp', state.snap?.agents);
      if (hid === 'entrance') state.office?.preset('semua', state.snap?.agents);
      tip.innerHTML = texts[hid] || '';
      const st = $('#stage').getBoundingClientRect();
      tip.style.left = Math.max(10, st.width / 2 - 160) + 'px';
      tip.style.top = '54px';
      tip.style.display = 'block';
      clearTimeout(state._tipT);
      state._tipT = setTimeout(() => { tip.style.display = 'none'; }, 3000);
    },
    onMeetingChange: on => { $('#navMeet').classList.toggle('on', on); $('#navMeet').textContent = on ? '📣 Meeting berjalan… (klik untuk bubarkan)' : '📣 Panggil meeting'; },
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
document.querySelectorAll('[data-zone]').forEach(b => b.onclick = () => {
  document.querySelectorAll('[data-zone]').forEach(x => x.classList.toggle('on', x.dataset.zone === b.dataset.zone));
  state.office?.preset(b.dataset.zone, state.snap?.agents);
});
$('#btn2d').onclick = () => {
  state.mode2d = !state.mode2d;
  $('#grid2d').style.display = state.mode2d ? 'grid' : 'none';
  const cv = $('#stage canvas');
  if (cv) cv.style.display = state.mode2d ? 'none' : 'block';
  const lb = $('#labels'); if (lb) lb.style.display = state.mode2d ? 'none' : 'block';
  $('#btn2d').textContent = state.mode2d ? '🏢 Mode 3D' : '🗂️ Mode 2D';
  if (state.snap) renderAll(state.snap);
};
$('#btnPanel').onclick = () => $('#panel').classList.toggle('show');
$('#searchBox').addEventListener('input', e => {
  const q = e.target.value.trim().toLowerCase();
  if (q.length < 2 || !state.snap) return;
  const hit = state.snap.agents.find(a => a.name.toLowerCase().includes(q));
  if (hit && state.office && !state.mode2d) state.office.focusAgent(hit.id);
});
$('#searchBox').addEventListener('keydown', e => {
  if (e.key !== 'Enter' || !state.snap) return;
  const q = e.target.value.trim().toLowerCase();
  const hit = state.snap.agents.find(a => a.name.toLowerCase().includes(q));
  if (hit) selectAgent(hit.id);
});
$('#alertBadge').onclick = () => { $('#panel').classList.add('show'); $('#panel').scrollTo({ top: 0, behavior: 'smooth' }); };
$('#navMeet').onclick = () => { state.office?.callMeeting(); };
$('#navBoard').onclick = () => openView(view.open === 'board' ? null : 'board');
$('#navFac').onclick = () => openView(view.open === 'fac' ? null : 'fac');
document.querySelectorAll('[data-closeov]').forEach(b => b.onclick = () => openView(null));
let _wantMeet = false;
const _qp = new URLSearchParams(location.search);
if (_qp.get('view') === 'board') openView('board');
if (_qp.get('view') === 'fasilitas') openView('fac');
if (_qp.get('meet')) _wantMeet = true;
if (_qp.get('debug')) setInterval(() => { try { document.title = JSON.stringify(state.office?.debug?.() || {}); } catch {} }, 400);
$('#drawerClose').onclick = closeDrawer;
$('#rawToggle').onchange = e => { state.raw = e.target.checked; if (state.snap) { renderEvents(state.snap); if (state.selected) renderDrawer(); } };

window.HQ = state; // pegangan debug (konsol/puppeteer)
refresh();
connectWs();
setInterval(() => { if (!state.snap || Date.now() - state.snap.generated_at > 20000) refresh(); }, 10000);
