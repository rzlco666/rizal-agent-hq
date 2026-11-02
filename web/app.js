// Rizal Agent HQ — app: data live (WebSocket + cadangan REST), panel, detail 5 tab.
import { initOffice } from './office3d.js';

const $ = s => document.querySelector(s);
const state = { snap: null, selected: null, tab: 'ringkasan', raw: false, office: null, mode2d: false };

const fmtTok = n => n >= 1e6 ? (n / 1e6).toFixed(2).replace('.', ',') + ' jt' : n >= 1e3 ? (n / 1e3).toFixed(1).replace('.', ',') + ' rb' : String(n);
const fmtUsd = n => '$' + n.toFixed(2);
const fmtTime = ms => new Date(ms).toLocaleTimeString(LOC(), { hour: '2-digit', minute: '2-digit' });
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const STATUS_LABEL = new Proxy({}, { get: (_, k) => (window.HQ_STATUS || {})[k] });
const agentById = id => state.snap?.agents.find(a => a.id === id);

// ---------- render panel ----------
function renderKpi(s) {
  $('#kpi').innerHTML = `
    <div><b>${s.total_usage.requests.toLocaleString(LOC())}</b><span>${t('kpiReq')}</span></div>
    <div><b>${fmtTok(s.total_usage.input_tokens + s.total_usage.output_tokens)}</b><span>${t('kpiTok')}</span></div>
    <div><b>${fmtUsd(s.total_usage.cash_usd)}</b><span>${t('kpiCash')}</span></div>
    <div><b>${fmtUsd(s.total_usage.shadow_usd)}</b><span>${t('kpiShadow')}</span></div>
    <div><b>${s.agents.filter(a => a.status === 'bekerja').length} bekerja</b><span>dari ${s.agents.length} agent tampil</span></div>
    <div><b>${s.deepseek_balance_est != null ? fmtUsd(s.deepseek_balance_est) : '—'}</b><span>${t('kpiBal')}</span></div>`;
}

function renderAlerts(s) {
  const aktif = s.alerts.filter(a => a.state === 'aktif');
  const badge = $('#alertBadge');
  badge.textContent = `🚨 ${aktif.length}`;
  badge.classList.toggle('zero', aktif.length === 0);
  if (!aktif.length) { $('#alerts').innerHTML = `<p class="mut small">${t('allSafe')}</p>`; return; }
  const cls = l => l === 'kritis' ? '' : l === 'perlu-action' ? 'act' : 'warn';
  const lbl = l => l === 'kritis' ? '🔴 KRITIS' : l === 'perlu-action' ? '🟠 PERLU ACTION' : '🟡 PERINGATAN';
  $('#alerts').innerHTML = aktif.map(a => `
    <div class="alertCard ${cls(a.level)}">
      <b>${lbl(a.level)} — ${esc(a.title)}</b>
      <div class="needs">${esc(a.needs)}</div>
      <div class="small mut">${t('since')} ${fmtTime(a.since)} · ${t('agentW')}: ${esc(agentById(a.agent_id)?.name || a.agent_id)}</div>
      <div style="margin-top:6px;display:flex;gap:6px">
        <button data-ack="${esc(a.id)}">${t('ack')}</button>
        <button data-snooze="${esc(a.id)}" data-min="60">${t('snooze')} 1 jam</button>
        <button data-snooze="${esc(a.id)}" data-min="1440">${t('snooze')} 24 jam</button>
      </div>
    </div>`).join('');
  document.querySelectorAll('[data-ack]').forEach(b => b.onclick = () => alertAction(b.dataset.ack, 'ack'));
  document.querySelectorAll('[data-snooze]').forEach(b => b.onclick = () => alertAction(b.dataset.snooze, 'snooze', +b.dataset.min || 60));
}

function toast(msg, isErr) {
  const t = document.createElement('div');
  t.className = 'toast' + (isErr ? ' err' : '');
  t.textContent = msg;
  $('#toasts').appendChild(t);
  setTimeout(() => { t.style.opacity = '0'; t.style.transition = 'opacity .3s'; setTimeout(() => t.remove(), 320); }, 3400);
}

async function alertAction(id, kind, minutes) {
  const url = kind === 'ack' ? `/api/alerts/${encodeURIComponent(id)}/ack` : `/api/alerts/${encodeURIComponent(id)}/snooze`;
  const body = kind === 'snooze' ? { minutes: minutes || 60 } : undefined;
  try {
    const r = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
    if (!r.ok) throw new Error('HTTP ' + r.status);
    // Optimistis: kartu langsung hilang; server juga memperbarui snapshot seketika.
    if (state.snap) state.snap.alerts = state.snap.alerts.map(x => x.id === id ? { ...x, state: kind === 'ack' ? 'ditangani' : 'ditunda' } : x);
    renderAll(state.snap);
    toast(kind === 'ack' ? t('toastAck') : t('toastSnooze').replace('{h}', Math.round((minutes || 60) / 60)));
  } catch {
    toast(t('toastFail'), true);
  }
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
  state.office?.focusAgent?.(id);
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
    <div class="tabs">${tabBtn('ringkasan', t('tabSummary'))}${tabBtn('tugas', t('tabTasks'))}${tabBtn('log', t('tabLog'))}${tabBtn('usage', t('tabUsage'))}${tabBtn('sesi', t('tabSessions'))}</div>`;
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
      <div class="task"><b>${t.summary ? '✨ ' + esc(t.summary) : esc(t.title)}</b> <span class="pill ${esc(t.phase)}">${esc(t.phase)}</span>
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
      <dt>Request</dt><dd>${u.requests.toLocaleString(LOC())} (gagal ${u.failed})</dd>
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
  $('#modelsOverlay').classList.toggle('open', name === 'models');
  if (state.snap) { if (name === 'board') renderBoard(state.snap); if (name === 'fac') renderFac(state.snap); if (name === 'models') renderModels(state.snap); }
}
function renderBoard(s) {
  const cols = [
    ['berjalan', t('colRun')], ['menunggu-rizal', t('colWait')],
    ['selesai', t('colDone')], ['gagal', t('colFail')],
  ];
  const inCol = (t, k) => k === 'berjalan' ? (t.phase === 'berjalan' || t.phase === 'berpikir') : t.phase === k;
  $('#boardCols').innerHTML = cols.map(([k, label]) => {
    const items = s.tasks.filter(t => inCol(t, k));
    return `<div class="bcol"><h3>${label}<span>${items.length}</span></h3>` +
      (items.length ? items.map(t => `
        <div class="bcard"><b>${t.summary ? '✨ ' + esc(t.summary) : esc(t.title)}</b>${t.summary ? `<div class="mut small">judul asli: ${esc(t.title)}</div>` : ''}
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
      <div class="small mut" style="margin-top:6px">Layanan HQ: http://localhost:8900 · data diperbarui ${new Date(s.generated_at).toLocaleTimeString(LOC())} WIB</div></div>
    <div class="fcard"><h3>🚨 Kuota jebol</h3>
      ${jebol.length ? jebol.map(q => `<span class="fchip hot">${esc(q.provider)} · ${esc(q.label)} ${q.percent}%</span>`).join('') : '<span class="mut small">Semua kuota aman.</span>'}
      ${s.fallback?.label ? `<div class="small" style="margin-top:7px">⛑️ Fallback utama: <b>${esc(s.fallback.label)}</b> — ${s.fallback.active ? 'AKTIF' : 'siaga'}${s.fallback.reserve_balance_usd != null ? ` · cadangan DeepSeek langsung ±${fmtUsd(s.fallback.reserve_balance_usd)}` : ''}</div>` : ''}</div>
    <div class="fcard"><h3>📝 Commit terbaru di mesin</h3>
      ${e.recent_commits.length ? e.recent_commits.map(c => `<div class="evt" style="margin:5px 0">${esc(c)}</div>`).join('') : '<p class="mut small">Tidak terbaca.</p>'}</div>
    <div class="fcard"><h3>🏢 Tentang kantor ini</h3>
      <div class="small">Rizal Agent HQ v2.1 — Rust (Axum + WebSocket + SQLite) satu binary + Three.js. ${s.agents.length} agen tampil, ${s.agents.filter(x => x.status === 'bekerja').length} sedang bekerja. Klik interior kantor (sofa, meja tenis, rak server, layar meeting) untuk interaksi.</div></div>`;
}

// ---------- dashboard model & provider ----------
function renderModels(s) {
  const ms = s.models || [];
  const el = $('#modelsBody');
  if (!ms.length) { el.innerHTML = `<p class="mut">${t('modelsEmpty')}</p>`; return; }
  const byProv = new Map();
  for (const m of ms) {
    if (!byProv.has(m.provider)) byProv.set(m.provider, []);
    byProv.get(m.provider).push(m);
  }
  const cards = [...byProv.entries()].map(([p, rows]) => {
    const req = rows.reduce((x, m) => x + m.usage.requests, 0);
    const tok = rows.reduce((x, m) => x + m.usage.input_tokens + m.usage.output_tokens, 0);
    const cash = rows.reduce((x, m) => x + m.usage.cash_usd, 0);
    const sh = rows.reduce((x, m) => x + m.usage.shadow_usd, 0);
    const q = (s.quotas || []).filter(x => x.provider === p && x.percent != null);
    return `<div class="provCard"><b>${esc(p)}</b><div class="row"><span>${rows.length} model · ${req.toLocaleString(LOC())} req</span></div>
      <div class="row"><span>Token</span><b>${fmtTok(tok)}</b></div>
      <div class="row"><span>Biaya TUNAI</span><b>${fmtUsd(cash)}</b></div>
      <div class="row"><span>Biaya bayangan</span><b>${fmtUsd(sh)}</b></div>
      ${q.map(x => `<div class="row"><span>Kuota ${esc(x.label)}</span><b style="color:${x.percent >= 100 ? '#dc2626' : x.percent >= 80 ? '#d97706' : '#059669'}">${x.percent}%</b></div>`).join('')}</div>`;
  }).join('');
  const cls = c => c === 'tunai' ? `<span class="clsTunai">${t('clsTunai')}</span>` : c === 'bayangan' ? `<span class="clsBayangan">${t('clsBayangan')}</span>` : `<span class="clsGratis">${t('clsGratis')}</span>`;
  const rowsHtml = ms.map(m => `<tr>
    <td><b>${esc(m.friendly)}</b><div class="mut small">${esc(m.model)}</div></td>
    <td>${esc(m.provider)}</td>
    <td>${m.usage.requests.toLocaleString(LOC())}<div class="mut small">gagal ${m.usage.failed}</div></td>
    <td>${fmtTok(m.usage.input_tokens)}<div class="mut small">keluar ${fmtTok(m.usage.output_tokens)} · cache ${fmtTok(m.usage.cache_tokens)}</div></td>
    <td><b>${fmtUsd(m.usage.cash_usd + m.usage.shadow_usd)}</b><div class="mut small">tunai ${fmtUsd(m.usage.cash_usd)} · bayangan ${fmtUsd(m.usage.shadow_usd)}</div></td>
    <td>${cls(m.cost_class)}</td>
    <td><div class="shareBar"><i style="width:${Math.min(100, m.share_pct).toFixed(1)}%"></i></div><div class="mut small">${m.share_pct.toFixed(1)}% biaya</div></td></tr>`).join('');
  el.innerHTML = `<div class="provCards">${cards}</div>
    <div style="overflow:auto;max-height:52vh;border:1px solid var(--line);border-radius:14px"><table class="mtab"><thead><tr>
    <th>Model</th><th>Provider</th><th>Request</th><th>Token masuk</th><th>Biaya</th><th>Kelas</th><th>Pangsa</th></tr></thead>
    <tbody>${rowsHtml}</tbody></table></div>
    <p class="mut small" style="margin-top:9px">Kelas TUNAI = uang nyata keluar (DeepSeek API & model berbayar gateway). BAYANGAN = setara-API provider langganan, bukan tagihan baru (OpenCode Go termasuk bayangan — meteran limit dari langganan flat $10/bln). GRATIS = model $0. Diurutkan dari biaya terbesar.</p>`;
}


// ================= i18n: ID / EN / 中文 (kerangka UI; kalimat aktivitas agen dari server tetap apa adanya) =================
let LANG = localStorage.getItem('hq-lang') || 'id';
const DICT = {
  id: { navCtrl:'Ruang kendali', navBoard:'Papan tugas', navFac:'Fasilitas', navModels:'Model & Provider', navAlert:'Fokus alert', secToday:'Ringkasan hari ini', secNeeds:'Butuh Rizal', boardT:'Papan Tugas', boardSub:'Semua tugas agen, dikelompokkan per fase — diperbarui langsung', facT:'Fasilitas & Lingkungan', facSub:'Kondisi mesin, kontainer, dan layanan di sekitar kantor virtual', modT:'Dashboard Model & Provider', modSub:'Semua model dari semua provider — request, token, dan biaya per model (data omp stats)', close:'Tutup', searchPh:'Cari agen atau tim…',
    kpiReq:'request OMP tercatat', kpiTok:'token (masuk+keluar)', kpiCash:'biaya TUNAI', kpiShadow:'biaya bayangan (langganan)', kpiBal:'estimasi saldo DeepSeek', working:'bekerja', ofAgents:'dari {n} agent tampil', allSafe:'Tidak ada alert aktif. Semua aman.', ack:'✅ Sudah saya tangani', snooze:'⏰ Tunda', since:'Sejak', agentW:'agent', colRun:'🏃 Sedang berjalan', colWait:'✋ Menunggu Rizal', colDone:'✅ Selesai', colFail:'❌ Gagal',
    mModel:'Model', mProvider:'Provider', mReq:'Request', mTokIn:'Token masuk', mCost:'Biaya', mClass:'Kelas', mShare:'Pangsa', mFailed:'gagal', mOut:'keluar', mCache:'cache', mCash:'tunai', mShadow:'bayangan', modelsEmpty:'Belum ada data model — OMP stats belum terbaca.', modelsNote:'Kelas TUNAI = uang nyata keluar (DeepSeek API & model berbayar gateway). BAYANGAN = setara-API provider langganan, bukan tagihan baru (OpenCode Go termasuk bayangan — meteran limit dari langganan flat $10/bln). GRATIS = model $0. Diurutkan dari biaya terbesar.',
    clsTunai:'TUNAI', clsBayangan:'BAYANGAN', clsGratis:'GRATIS', tabSummary:'Ringkasan', tabTasks:'Tugas', tabLog:'Log', tabUsage:'Usage', tabSessions:'Sesi & File', toastAck:'✅ Alert ditandai sudah ditangani — tersimpan, tidak akan muncul lagi.', toastSnooze:'⏰ Alert ditunda {h} jam — tersimpan.', toastFail:'❌ Aksi alert gagal terkirim — cek koneksi ke server HQ.', updatedAgo:'diperbarui {n} dtk lalu', simPauseT:'Jeda / lanjutkan animasi', simOrbitT:'Kamera mengorbit otomatis', simFollowT:'Kamera mengikuti agen terpilih', pickAgentFirst:'Pilih agen dulu (klik figurnya), lalu aktifkan Ikuti 🎯', raw:'LIHAT MENTAH' },
  en: { navCtrl:'Control room', navBoard:'Task board', navFac:'Facilities', navModels:'Models & Providers', navAlert:'Focus alerts', secToday:'Today at a glance', secNeeds:'Needs Rizal', boardT:'Task Board', boardSub:'All agent tasks grouped by phase — updated live', facT:'Facilities & Environment', facSub:'Machine, container, and service conditions around the virtual office', modT:'Models & Providers Dashboard', modSub:'Every model from every provider — requests, tokens, and cost per model (omp stats data)', close:'Close', searchPh:'Search agent or team…',
    kpiReq:'OMP requests recorded', kpiTok:'tokens (in+out)', kpiCash:'CASH cost', kpiShadow:'shadow cost (subscriptions)', kpiBal:'est. DeepSeek balance', working:'working', ofAgents:'of {n} agents shown', allSafe:'No active alerts. All clear.', ack:'✅ Handled', snooze:'⏰ Snooze', since:'Since', agentW:'agent', colRun:'🏃 In progress', colWait:'✋ Waiting for Rizal', colDone:'✅ Done', colFail:'❌ Failed',
    mModel:'Model', mProvider:'Provider', mReq:'Requests', mTokIn:'Tokens in', mCost:'Cost', mClass:'Class', mShare:'Share', mFailed:'failed', mOut:'out', mCache:'cache', mCash:'cash', mShadow:'shadow', modelsEmpty:'No model data yet — OMP stats not read.', modelsNote:'CASH class = real money out (DeepSeek API & paid gateway models). SHADOW = API-equivalent cost of subscription providers, not a new bill (OpenCode Go counts as shadow — a metered limit on the flat $10/mo plan). FREE = $0 models. Sorted by cost, descending.',
    clsTunai:'CASH', clsBayangan:'SHADOW', clsGratis:'FREE', tabSummary:'Summary', tabTasks:'Tasks', tabLog:'Log', tabUsage:'Usage', tabSessions:'Sessions & Files', toastAck:'✅ Alert marked handled — saved, it will not reappear.', toastSnooze:'⏰ Alert snoozed for {h} h — saved.', toastFail:'❌ Alert action failed to send — check the HQ server connection.', updatedAgo:'updated {n}s ago', simPauseT:'Pause / resume animation', simOrbitT:'Auto-orbit camera', simFollowT:'Camera follows the selected agent', pickAgentFirst:'Select an agent first (click its figure), then enable Follow 🎯', raw:'VIEW RAW' },
  zh: { navCtrl:'控制室', navBoard:'任务看板', navFac:'设施', navModels:'模型与提供商', navAlert:'聚焦警报', secToday:'今日概览', secNeeds:'需要 Rizal 处理', boardT:'任务看板', boardSub:'按阶段分组的全部代理任务 — 实时更新', facT:'设施与环境', facSub:'虚拟办公室周边的机器、容器与服务状态', modT:'模型与提供商面板', modSub:'所有提供商的每个模型 — 请求、令牌与费用（omp stats 数据）', close:'关闭', searchPh:'搜索代理或团队…',
    kpiReq:'已记录的 OMP 请求', kpiTok:'令牌（入+出）', kpiCash:'现金成本', kpiShadow:'影子成本（订阅）', kpiBal:'DeepSeek 余额估算', working:'工作中', ofAgents:'共显示 {n} 个代理', allSafe:'没有活动警报，一切正常。', ack:'✅ 已处理', snooze:'⏰ 推迟', since:'自', agentW:'代理', colRun:'🏃 进行中', colWait:'✋ 等待 Rizal', colDone:'✅ 已完成', colFail:'❌ 失败',
    mModel:'模型', mProvider:'提供商', mReq:'请求', mTokIn:'输入令牌', mCost:'费用', mClass:'类别', mShare:'占比', mFailed:'失败', mOut:'输出', mCache:'缓存', mCash:'现金', mShadow:'影子', modelsEmpty:'暂无模型数据 — 尚未读取 OMP stats。', modelsNote:'现金类 = 真实支出（DeepSeek API 与付费网关模型）。影子类 = 订阅提供商的 API 等值费用，并非新账单（OpenCode Go 属于影子 — $10/月固定套餐的计量额度）。免费 = $0 模型。按费用从高到低排序。',
    clsTunai:'现金', clsBayangan:'影子', clsGratis:'免费', tabSummary:'概要', tabTasks:'任务', tabLog:'日志', tabUsage:'用量', tabSessions:'会话与文件', toastAck:'✅ 警报已标记为已处理 — 已保存，不会再次出现。', toastSnooze:'⏰ 警报已推迟 {h} 小时 — 已保存。', toastFail:'❌ 警报操作发送失败 — 请检查与 HQ 服务器的连接。', updatedAgo:'{n} 秒前更新', simPauseT:'暂停 / 继续动画', simOrbitT:'相机自动环绕', simFollowT:'相机跟随所选代理', pickAgentFirst:'请先选择一个代理（点击其形象），再开启跟随 🎯', raw:'查看原文' },
};
const STATUS_I18N = {
  id: { bekerja:'Bekerja', berpikir:'Berpikir', 'menunggu-rizal':'Menunggu Rizal', santai:'Santai', offline:'Offline', error:'Error' },
  en: { bekerja:'Working', berpikir:'Thinking', 'menunggu-rizal':'Waiting for Rizal', santai:'Idle', offline:'Offline', error:'Error' },
  zh: { bekerja:'工作中', berpikir:'思考中', 'menunggu-rizal':'等待 Rizal', santai:'空闲', offline:'离线', error:'错误' },
};
const t = k => (DICT[LANG] && DICT[LANG][k]) || DICT.id[k] || k;
const LOC = () => LANG === 'zh' ? 'zh-CN' : LANG === 'en' ? 'en-US' : 'id-ID';
const SL = st => (STATUS_I18N[LANG] || STATUS_I18N.id)[st] || st;
function applyLang() {
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
  document.querySelectorAll('[data-i18n-ph]').forEach(el => { el.placeholder = t(el.dataset.i18nPh); });
  document.querySelectorAll('[data-i18n-title]').forEach(el => { el.title = t(el.dataset.i18nTitle); });
  document.querySelectorAll('.langsw button').forEach(b => b.classList.toggle('on', b.dataset.lang === LANG));
  window.HQ_STATUS = STATUS_I18N[LANG];
  document.documentElement.lang = LANG === 'zh' ? 'zh-CN' : LANG;
  if (state.snap) renderAll(state.snap);
}

// ---------- siklus render ----------
function renderAll(s) {
  $('#clock').textContent = 'Diperbarui ' + new Date(s.generated_at).toLocaleString(LOC()) + ' WIB · ' + s.agents.filter(a => a.status === 'bekerja').length + ' agent bekerja';
  renderKpi(s); renderAlerts(s); renderQuotas(s); renderEnv(s); renderTasks(s); renderEvents(s); render2d(s);
  if (view.open === 'board') renderBoard(s);
  if (view.open === 'fac') renderFac(s);
  if (view.open === 'models') renderModels(s);
  if (state.office && !state.mode2d) state.office.update(s.agents);
  if (state.selected) renderDrawer();
  if (_wantMeet && state.office) { _wantMeet = false; state.office.callMeeting(); }
  state.office?.setPlumbob?.(state.selected || null);
  if (followOn) state.office?.setFollow?.(state.selected || null);
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
$('#navModels').onclick = () => openView(view.open === 'models' ? null : 'models');
document.querySelectorAll('[data-closeov]').forEach(b => b.onclick = () => openView(null));
let _wantMeet = false;
const _qp = new URLSearchParams(location.search);
if (_qp.get('view') === 'board') openView('board');
if (_qp.get('view') === 'fasilitas') openView('fac');
if (_qp.get('view') === 'models') openView('models');

// ---------- shortcut keyboard ----------
document.addEventListener('keydown', e => {
  const tag = (e.target.tagName || '').toLowerCase();
  if (tag === 'input' || tag === 'textarea' || e.metaKey || e.ctrlKey || e.altKey) return;
  const k = e.key.toLowerCase();
  if (k === 'escape') { openView(null); state.selected = null; renderDrawer(); }
  else if (k === 'm') state.office?.callMeeting();
  else if (k === 'b') openView(view.open === 'board' ? null : 'board');
  else if (k === 'f') openView(view.open === 'fac' ? null : 'fac');
  else if (k === 'p') openView(view.open === 'models' ? null : 'models');
  else if (['1','2','3','4','5','6','7'].includes(k)) {
    const zones = ['semua','Muse','omp','workers','cursor','meeting','mcp'];
    state.office?.preset(zones[+k - 1], state.snap?.agents);
  }
});
if (_qp.get('meet')) _wantMeet = true;
if (_qp.get('debug')) setInterval(() => { try { document.title = JSON.stringify(state.office?.debug?.() || {}); } catch {} }, 400);
$('#drawerClose').onclick = closeDrawer;
$('#rawToggle').onchange = e => { state.raw = e.target.checked; if (state.snap) { renderEvents(state.snap); if (state.selected) renderDrawer(); } };


// ---------- kendali ala The Sims ----------
let followOn = false;
let ambIdx = 0;
const AMB_CYCLE = [['day', '🌞'], ['sunset', '🌅'], ['night', '🌙']];
$('#simPause').onclick = () => {
  const nowPaused = $('#simPause').textContent !== '▶️';
  state.office?.setPaused(nowPaused);
  $('#simPause').textContent = nowPaused ? '▶️' : '⏸️';
  $('#simPause').classList.toggle('on', nowPaused);
};
document.querySelectorAll('#simSpeed button').forEach(b => b.onclick = () => {
  document.querySelectorAll('#simSpeed button').forEach(x => x.classList.toggle('on', x === b));
  state.office?.setSpeed(+b.dataset.v);
});
$('#simAmb').onclick = () => {
  ambIdx = (ambIdx + 1) % AMB_CYCLE.length;
  state.office?.setAmbience(AMB_CYCLE[ambIdx][0]);
  $('#simAmb').textContent = AMB_CYCLE[ambIdx][1];
};
$('#simOrbit').onclick = () => {
  const on = !$('#simOrbit').classList.contains('on');
  $('#simOrbit').classList.toggle('on', on);
  state.office?.setOrbit(on);
};
$('#simFollow').onclick = () => {
  if (!state.selected) { toast(t('pickAgentFirst')); return; }
  followOn = !followOn;
  $('#simFollow').classList.toggle('on', followOn);
  state.office?.setFollow(followOn ? state.selected : null);
};
document.querySelectorAll('.langsw button').forEach(b => b.onclick = () => {
  LANG = b.dataset.lang;
  localStorage.setItem('hq-lang', LANG);
  applyLang();
});
setInterval(() => {
  if (!state.snap) return;
  const age = Math.max(0, Math.round((Date.now() - state.snap.generated_at) / 1000));
  $('#ageEl').textContent = '· ' + t('updatedAgo').replace('{n}', age);
}, 1000);
applyLang();

window.HQ = state; // pegangan debug (konsol/puppeteer)
refresh();
connectWs();
setInterval(() => { if (!state.snap || Date.now() - state.snap.generated_at > 20000) refresh(); }, 10000);
