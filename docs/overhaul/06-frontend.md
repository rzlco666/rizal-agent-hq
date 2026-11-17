# 06 — Frontend

**Status:** siap dieksekusi. Dokumen ini mengikat S07–S16 dan semua sesi UI
sesudahnya.
**Spec induk:** [`00-design.md`](00-design.md) §14, [`01-architecture.md`](01-architecture.md) §4–5,
[`05-design-system.md`](05-design-system.md).
**Gate:** [`08-testing-gates.md`](08-testing-gates.md) §6–7.

Frontend adalah satu SPA statis yang di-embed ke binary. Tidak ada SSR, tidak
ada Node saat runtime, tidak ada aset dari jaringan. Semua data berasal dari
`/api/v1`; semua kalimat hasil humanizer dirender server; UI tidak menyusun
kalimat dari potongan teks.

---

## 1. SvelteKit

### 1.1 Versi dan konfigurasi

SvelteKit 2, Svelte 5 (runes), Vite 7, TypeScript 5.9 strict, Tailwind 4,
Threlte 8, uPlot 1.6, lucide-svelte. Semua pin masuk `package.json` +
`pnpm-lock.yaml`; tidak ada dependency baru tanpa memperbarui
`01-architecture.md` §1.

`web/svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: false
    }),
    paths: { base: '', relative: false },
    alias: { $lib: 'src/lib' }
  }
};
```

`web/src/routes/+layout.ts`:

```ts
export const ssr = false;
export const prerender = false;
export const csr = true;
export const trailingSlash = 'never';
```

Alasan: state aplikasi adalah snapshot + patch realtime; tidak ada keuntungan
SSR, dan `fallback: index.html` membuat semua route dinamis (`/agents/[id]`,
`/timeline`, `/office`) bekerja tanpa prerender per halaman. `relative: false`
memastikan aset `/_app/...` tetap benar di route bersarang.

`web/vite.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8900',
      '/ws': { target: 'ws://127.0.0.1:8900', ws: true }
    }
  },
  build: { sourcemap: false }
});
```

Tailwind hanya untuk layout utility. Warna, radius, bayangan, dan tipografi
selalu memakai token `05-design-system.md` §3; tidak ada arbitrary value warna
di komponen. `web/src/app.css` memuat `@import "tailwindcss";`, token, reset,
dan `@font-face`.

### 1.2 Struktur file

```
web/
  package.json
  pnpm-lock.yaml
  svelte.config.js
  vite.config.ts
  tsconfig.json
  playwright.config.ts
  vitest.config.ts
  openapi.json                      # artefak generate, dikomit
  scripts/
    gen-api.mjs
    check-i18n.mjs
    check-bundle.mjs
    subset-fonts.mjs
  static/
    logo.svg
    fonts/                          # WOFF2 subset, di-embed
  src/
    app.html
    app.css
    app.d.ts
    error.html
    hooks.client.ts
    lib/
      api/
        schema.d.ts                 # generated, jangan diedit
        client.ts
        errors.ts
        stream.ts
        types.ts
      stores/
        entities.svelte.ts
        stream.svelte.ts
        session.svelte.ts
        ui.svelte.ts
        toast.svelte.ts
      i18n/
        index.ts
        id.json
        en.json
        zh.json
      format/
        number.ts
        datetime.ts
        duration.ts
      keyboard/
        map.ts
        actions.ts
      components/
        ui/                         # 05 §6
        domain/
        shell/                      # AppShell, Sidebar, TopBar, OfflineBanner, TokenGate
      office/
        OfficeCanvas.svelte
        Scene.svelte
        Lighting.svelte
        Floor.svelte
        camera.ts
        stats.ts
        nav.ts
        characters.ts
        rig.ts
        objects.ts
        DataBoardDrawer.svelte
        Fallback2D.svelte
    routes/
      +layout.ts
      +layout.svelte
      +error.svelte
      +page.ts
      +page.svelte                  # /
      agents/[id]/+page.ts
      agents/[id]/+page.svelte
      office/+page.ts
      office/+page.svelte
      office/+error.svelte
      timeline/+page.ts
      timeline/+page.svelte
      alerts/+page.ts
      alerts/+page.svelte
      automations/+page.ts
      automations/+page.svelte
      integrations/+page.ts
      integrations/+page.svelte
      knowledge/+page.ts
      knowledge/+page.svelte
      costs/+page.ts
      costs/+page.svelte
      settings/+page.ts
      settings/+page.svelte
  tests/
    fixtures/e2e.json
    unit/
    e2e/
    visual/
    a11y/
    perf/
```

### 1.3 Shell, layout, dan error boundary

`+layout.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import AppShell from '$lib/components/shell/AppShell.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  let { children } = $props();
  onMount(() => ui.applyDocumentTheme());
</script>

<AppShell>
  {@render children()}
</AppShell>
```

`AppShell` memegang:

- `<header>` top bar 52px: wordmark, pencarian, status koneksi, tombol
  attention (hitungan), pemilih bahasa, tombol palette.
- `<nav>` sidebar 248/64px (desktop); bottom nav 56px (ponsel).
- `<main id="main">` dengan skip link target.
- `<aside>` attention rail 320px (≥1280) atau tombol sheet.
- `<OfflineBanner/>`, `<Toaster/>`, `<CommandPalette/>`, `<TokenGate/>`.
- `data-connection` pada top bar untuk tes.

`+error.svelte` menampilkan status HTTP, pesan aman, `request_id` bila ada, dan
dua aksi: "Muat ulang" dan "Kembali". Tidak ada stack trace. Route `/office`
punya `+error.svelte` sendiri karena kegagalan WebGL tidak boleh menjatuhkan
seluruh shell; kegagalan WebGL ditangani sebagai state komponen, bukan throw.

`web/src/error.html` adalah fallback statis bila bundle gagal dimuat sebelum
SvelteKit hidup. Isinya dua baris: "Aplikasi gagal dimuat" / "Muat ulang
halaman ini." Tanpa aset eksternal.

`hooks.client.ts`:

```ts
export const handleError = ({ error, status }) => {
  const requestId = (error as { requestId?: string })?.requestId ?? null;
  console.error('ui_error', { status, requestId });
  return { message: 'Terjadi kesalahan saat memuat tampilan.', requestId };
};
```

`unhandledrejection` dan `window.onerror` memakai jalur yang sama; satu toast
critical per 5 detik, tidak ada toast storm.

### 1.4 Build dan base path

- Output: `web/build/`; entry `web/build/index.html`; aset `web/build/_app/`.
- Base path kosong; aplikasi dilayani di `/`. API di `/api/v1`, WS di
  `/api/v1/ws`, SSE di `/api/v1/events/stream`.
- `paths.relative = false`; semua URL aset absolut dari root.
- `web/build/` tidak dikomit. CI membangunnya sebelum `cargo build`; build.rs
  gagal bila tidak ada (lihat §11).
- `pnpm dev` memakai proxy Vite ke binary di 8900; tidak ada Node di runtime
  produksi.

---

## 2. Klien API

### 2.1 Generate dari OpenAPI

Sumber kontrak adalah `GET /api/v1/openapi.json`. Untuk build offline, binary
menyediakan `hq openapi --out <path>` dan `hq openapi --check`
(`08-testing-gates.md` §7). `web/scripts/gen-api.mjs`:

1. Mode default dan `--check` membaca `web/openapi.json` yang dikomit; tidak
   membutuhkan server atau binary.
2. Mode `--refresh` dengan `HQ_BIN`: jalankan `$HQ_BIN openapi --out web/openapi.json`.
3. Mode `--refresh` tanpa `HQ_BIN`: ambil
   `${HQ_OPENAPI_URL ?? 'http://127.0.0.1:8900/api/v1/openapi.json'}`.
4. Jalankan `openapi-typescript web/openapi.json -o web/src/lib/api/schema.d.ts`.
5. Tulis banner "generated, jangan diedit" dan urutkan nama tipe.
6. Mode `--check`: generate ke file sementara, bandingkan byte-per-byte,
   keluar 1 bila berbeda.

Alur kontrak: job Rust menjalankan `hq openapi --check` (schema kode = file
OpenAPI); job web menjalankan `pnpm api:check` (file OpenAPI = `schema.d.ts`).
Keduanya tidak boleh drift.

`package.json`:

```json
{
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "api:gen": "node scripts/gen-api.mjs",
    "api:check": "node scripts/gen-api.mjs --check",
    "check:i18n": "node scripts/check-i18n.mjs",
    "check:bundle": "node scripts/check-bundle.mjs",
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
```

File generated: `web/src/lib/api/schema.d.ts`. File tulisan tangan:
`web/src/lib/api/client.ts`, `errors.ts`, `stream.ts`, `types.ts`. Tidak ada
tipe response yang ditulis ulang manual; `types.ts` hanya alias dari
`components['schemas']`.

### 2.2 Pembungkus typed

```ts
// web/src/lib/api/types.ts
import type { components } from './schema';

export type Overview = components['schemas']['Overview'];
export type AgentCard = components['schemas']['AgentCard'];
export type AgentDetail = components['schemas']['AgentDetail'];
export type TaskView = components['schemas']['TaskView'];
export type LogView = components['schemas']['LogView'];
export type SessionView = components['schemas']['SessionView'];
export type ChangedFile = components['schemas']['ChangedFile'];
export type AlertView = components['schemas']['AlertView'];
export type AlertDetail = components['schemas']['AlertDetail'];
export type ApprovalView = components['schemas']['ApprovalView'];
export type ConnectorView = components['schemas']['ConnectorView'];
export type EventView = components['schemas']['EventView'];
export type UsageReport = components['schemas']['UsageReport'];
export type CostReport = components['schemas']['CostReport'];
export type KnowledgeHit = components['schemas']['KnowledgeHit'];
export type Patch = components['schemas']['Patch'];
export type Status = components['schemas']['Status'];
export type AgentKind = components['schemas']['AgentKind'];

export interface Page<T> {
  items: T[];
  next_cursor: string | null;
  seq: number;
}
```

`client.ts` menyediakan satu fungsi transport dan pembungkus per endpoint.
Semua pembungkus menerima `signal?: AbortSignal` dan `fetch?: typeof globalThis.fetch`
agar bisa dipakai di `load` SvelteKit dan tes.

```ts
interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE';
  query?: Record<string, string | number | boolean | null | undefined>;
  body?: unknown;
  signal?: AbortSignal;
  fetch?: typeof globalThis.fetch;
  mutation?: boolean;
}

export async function request<T>(path: string, options?: RequestOptions): Promise<T>;
```

Pembungkus yang dipakai UI:

| Fungsi | Endpoint |
|---|---|
| `getHealth()` | `GET /health` |
| `getOverview()` | `GET /overview` |
| `getAgents(q)` | `GET /agents?status=&kind=&q=&cursor=&limit=` |
| `getAgent(id, { asOf }?)` | `GET /agents/{id}?as_of=` |
| `getAgentTasks(id, q)` | `GET /agents/{id}/tasks?phase=&limit=&cursor=` |
| `getAgentLogs(id, q)` | `GET /agents/{id}/logs?level=&q=&limit=&before=` |
| `getAgentUsage(id, window)` | `GET /agents/{id}/usage?window=` |
| `getAgentSessions(id)` | `GET /agents/{id}/sessions` |
| `getAgentFiles(id)` | `GET /agents/{id}/files` |
| `getTasks(q)` | `GET /tasks?agent=&phase=&q=` |
| `getAlerts(q)` | `GET /alerts?state=&level=` |
| `getAlert(id)` | `GET /alerts/{id}` |
| `ackAlert(id, body)` | `POST /alerts/{id}/ack` |
| `snoozeAlert(id, body)` | `POST /alerts/{id}/snooze` |
| `resolveAlert(id, body)` | `POST /alerts/{id}/resolve` |
| `getTimeline(q)` | `GET /timeline?since=&until=&kind=&subject=&limit=&before=` |
| `getUsage(q)` | `GET /usage?group_by=&window=` |
| `getCosts(window)` | `GET /costs?window=` |
| `getConnectors()` | `GET /connectors` |
| `testConnector(id)` | `POST /connectors/{id}/test` |
| `pollConnector(id)` | `POST /connectors/{id}/poll` |
| `setConnectorEnabled(id, enabled)` | `POST /connectors/{id}/enable` atau `/disable` |
| `runConnectorAction(id, action, input)` | `POST /connectors/{id}/actions/{action}` |
| `getApprovals(q)` | `GET /approvals?state=` |
| `approveApproval(id, { nonce, note? })` | `POST /approvals/{id}/approve` |
| `denyApproval(id, { nonce, note? })` | `POST /approvals/{id}/deny` |
| `getAudits(q)` | `GET /audits?actor=&decision=&limit=&before=` |
| `postAction(input)` | `POST /actions` |
| `getKnowledgeSearch(q, limit)` | `GET /knowledge/search?q=&limit=` |
| `getKnowledgePage(id)` | `GET /knowledge/pages/{id}` |
| `syncKnowledge(connector?)` | `POST /knowledge/sync` |
| `promoteKnowledge(pageId, title)` | `POST /knowledge/promote` |
| `getAuthWsTicket()` | `POST /auth/ws-ticket` |
| `getSettings()` | `GET /settings` |
| `updateSettings(patch)` | `PATCH /settings` |
| `getDevices()` | `GET /devices` |
| `revokeDevice(id)` | `DELETE /devices/{id}` |
| `getBackup()` | `GET /backup` |
| `startBackup()` | `POST /backup` |
| `getPolicyConfig()` | `GET /config/policy` |
| `putPolicyConfig({ content, confirm_phrase })` | `PUT /config/policy` |

Transport:

- `Accept: application/json`.
- `Accept-Language` dari locale aktif (§6).
- `Content-Type: application/json` untuk body.
- `credentials: 'same-origin'`, `cache: 'no-store'`.
- `X-CSRF-Token` pada mutasi bila autentikasi memakai cookie.
- `Authorization: Bearer <token>` bila device token ada (§2.4).
- Timeout GET 10 detik, mutasi 20 detik; `AbortSignal.any` dengan signal
  pemanggil.
- GET boleh diulang sekali pada kegagalan jaringan setelah 300ms; mutasi tidak
  pernah diulang otomatis.

### 2.3 Kontrak data yang dikonsumsi

`01-architecture.md` §5 mendefinisikan endpoint dan nama DTO, tetapi belum
menetapkan isi beberapa DTO. Bentuk berikut adalah kontrak minimum yang dipakai
UI. Field tambahan diperbolehkan. Sesi pemilik wajib mendaftarkan bentuk final
di `01-architecture.md` dan OpenAPI sebelum route-nya dimulai; UI tidak
menyediakan fallback karangan.

```ts
interface Overview {
  seq: number;
  generated_ms: number;
  agents: {
    total: number;
    by_status: Record<Status, number>;
  };
  attention: {
    approvals: number;
    alerts_critical: number;
    alerts_warning: number;
    agents_error: number;
  };
  usage_today: {
    requests: number;
    input_tokens: number;
    output_tokens: number;
    cache_tokens: number;
    cash_usd: number;
    shadow_usd: number;
  };
  active_sessions: number;
  health: {
    db: 'ok' | 'degraded';
    projection_lag_seq: number;
    connectors_down: number;
    last_event_ts_ms: number;
  };
}

interface UsageSeriesPoint {
  ts_ms: number;
  cash_usd: number;
  shadow_usd: number;
  requests: number;
  input_tokens: number;
  output_tokens: number;
}

interface UsageTotals {
  requests: number;
  input_tokens: number;
  output_tokens: number;
  cache_tokens: number;
  cash_usd: number;
  shadow_usd: number;
}

interface UsageBucket extends UsageTotals {
  key: string;                    // agent id, model, atau provider
  label: string;
  attribution: 'exact' | 'global' | 'estimated';
}

interface UsageReport {
  seq: number;
  window: '24h' | '7d' | '30d';
  from_ms: number;
  to_ms: number;
  bucket_ms: number;
  series: UsageSeriesPoint[];
  totals: UsageTotals;
  buckets: UsageBucket[];         // mengikuti group_by
}

interface CostReport {
  seq: number;
  window: '24h' | '7d' | '30d';
  from_ms: number;
  to_ms: number;
  bucket_ms: number;
  series: UsageSeriesPoint[];
  totals: UsageTotals;
  by_agent: UsageBucket[];
  by_model: UsageBucket[];
  by_provider: UsageBucket[];
}

interface Settings {
  seq: number;
  locale: 'id' | 'en' | 'zh';
  timezone: string;               // IANA; default Asia/Jakarta
  theme: 'day' | 'night' | 'system';
  retention_days: number;
  backup: {
    last_backup_ms: number | null;
    path: string | null;
    size_bytes: number | null;
  };
}

interface DeviceView {
  id: string;
  name: string;
  scopes: string[];
  created_ms: number;
  last_seen_ms: number | null;
  revoked_ms: number | null;
}

interface BackupStatus {
  last_backup_ms: number | null;
  path: string | null;
  size_bytes: number | null;
  running: boolean;
}

interface PolicyConfig {
  content: string;
  sha256: string;
  rules: number;
}

interface AuditView {
  id: string;
  seq: number;
  actor: string;
  capability: string;
  target: string;                 // contoh: agent:omp, config:policy
  decision: 'allow' | 'require_approval' | 'deny';
  at_ms: number;
  correlation_id: string | null;
}

interface AuditPage {
  items: AuditView[];
  next: string | null;
}

interface ApprovalDecision {
  id: string;
  nonce: string;                  // hanya dari ActionOutcome pembuat request
  note?: string;
}

interface KnowledgePage {
  id: string;
  source: string;
  title: string;
  url: string | null;
  content_hash: string;
  word_count: number;
  status: 'active' | 'quarantined';
  updated_ms: number;
  chunks: { idx: number; text: string }[];
}
```

Endpoint tambahan yang belum ada di `01-architecture.md` §5 dan menjadi
kewajiban sesi pemilik:

| Endpoint | Bentuk | Pemilik | Dipakai |
|---|---|---|---|
| `GET /settings` / `PATCH /settings` | `Settings`; PATCH menerima subset field | S06 | Settings, shell |
| `GET /devices` / `DELETE /devices/{id}` | `Page<DeviceView>`; device baru dibuat lewat CLI `hq device add`, bukan UI | S06 | Settings |
| `GET /backup` / `POST /backup` | `BackupStatus`; POST 202 | S02/S26 | Settings |
| `GET /audits` | `AuditPage`; filter aktor/keputusan | S24 (`04-control-plane.md` §9.4) | Automations |
| `GET /config/policy` | `PolicyConfig`; pasangan baca untuk editor | S24 | Automations |
| `PUT /config/policy` | body `{content, confirm_phrase:"UBAH-POLICY"}` | S24 (`04-control-plane.md` §3.7) | Automations |
| `GET /knowledge/pages/{id}` | `KnowledgePage` | S16 | Knowledge |
| `GET /alerts/{id}` | `AlertDetail` (keputusan + delivery) | S11 (`10-alerts.md` §7) | Alerts |

`UsageReport` dan `CostReport` wajib mengisi `series`, `totals`, `bucket_ms`,
`from_ms`, `to_ms`. Bucket: 24h = 5 menit, 7d = 1 jam, 30d = 6 jam; maksimal
720 titik. Agregasi dilakukan server dari `usage_minute`/`usage_daily`; klien
tidak menjumlahkan event.

### 2.4 Error, CSRF, dan device token

`errors.ts`:

```ts
export class ApiError extends Error {
  status: number;
  code: string;
  detail: unknown;
  requestId: string | null;
}
export function isApiError(value: unknown): value is ApiError;
```

Body error mengikuti `01-architecture.md` §5: `{code, message, detail, request_id}`.
UI tidak pernah menampilkan `message` mentah dari server sebagai satu-satunya
teks; pesan diterjemahkan lewat katalog dan `request_id` ditampilkan di detail.

| Status / kode | Perilaku UI |
|---|---|
| 401 `unauthenticated` / `untrusted_forwarded_header` | Tampilkan `TokenGate` bila mode remote; bila cookie loopback, muat ulang sekali lalu error. |
| 403 `insufficient_scope` / `forbidden` | Pesan inline "Butuh izin: `<capability>`" + `request id`; aksi tetap terlihat tetapi tidak bisa diulang tanpa perubahan state. |
| 403 `csrf_invalid` | Muat ulang halaman sekali; bila berulang, error banner. |
| 404 `not_found` | `EmptyState` "Tidak ditemukan" / "Objek mungkin sudah dihapus." |
| 409 `approval_already_decided` / `approval_expired` | Refetch snapshot route, toast "Data sudah berubah. Memuat ulang." |
| 422 `validation_failed` / `policy_invalid` | Map `detail.fields` ke error field; fokus ke field pertama. |
| 429 | Banner "Terlalu banyak permintaan. Coba lagi dalam N detik." dengan hitung mundur. |
| 5xx | Error state route + "Coba lagi"; data lama tetap tampil dengan penanda stale. |
| Jaringan putus | `OfflineBanner`, state `offline`, mutasi dinonaktifkan. |
| `AbortError` | Diabaikan; bukan error UI. |

CSRF:

- Server menetapkan cookie sesi (HttpOnly) dan cookie double-submit `csrf`
  (SameSite=Strict, Secure saat TLS, tidak HttpOnly) pada pairing/GET pertama.
- Klien membaca cookie `csrf` dan mengirim header `X-CSRF-Token` untuk
  POST/PATCH/PUT/DELETE saat autentikasi memakai cookie. Nama header mengikuti
  `04-control-plane.md` §4.6 dan §9.5.
- Tidak ada token CSRF di URL, localStorage, atau log.
- S06 wajib memakai cookie `csrf` + header `X-CSRF-Token`; perubahan nama
  adalah perubahan kontrak.

Device token (remote):

- Disimpan di `sessionStorage` dengan key `hq.deviceToken`; tidak di
  localStorage, tidak di URL.
- Dikirim sebagai `Authorization: Bearer <token>` pada REST dan pada
  `POST /auth/ws-ticket`.
- `TokenGate` muncul saat 401 `unauthenticated`; field token `type="password"`,
  `autocomplete="off"`, `spellcheck="false"`; tombol "Simpan" menguji
  `getOverview()` sebelum menutup gate.
- 401 berikutnya menghapus token dan membuka gate lagi. 403 capability tidak
  menghapus token.
- Token tidak pernah ditampilkan kembali setelah disimpan.

---

## 3. Realtime

### 3.1 Urutan koneksi

1. Route load mengambil snapshot dan menyimpan `seq` minimum dari semua
   response yang dipakai route.
2. `POST /api/v1/auth/ws-ticket` (cookie atau bearer) → `{ticket, expires_ms}`.
   Ticket sekali pakai, masa berlaku pendek; tidak pernah dicatat.
3. Buka `ws(s)://<host>/api/v1/ws?ticket=<ticket>&since=<seq>`.
4. Server mengirim `Patch` berurutan. Klien menerapkan patch dan memperbarui
   `seq`.
5. Bila `patch.seq !== lastSeq + 1`: gap. Klien menutup koneksi, memanggil
   refetch snapshot route, lalu membuka koneksi baru dari `seq` snapshot.
6. Bila `patch.seq <= lastSeq`: duplikat, diabaikan.
7. Bila patch tidak valid (JSON rusak, tag entity tidak dikenal): tutup,
   refetch, reconnect; satu toast "Menyinkronkan ulang".

`stream.ts`:

```ts
export type ConnectionState =
  | 'connecting' | 'live' | 'reconnecting'
  | 'syncing' | 'degraded' | 'offline';

export interface StreamOptions {
  since: number;
  onpatch: (patch: Patch) => ApplyResult;
  onstate: (state: ConnectionState) => void;
  onseq: (seq: number) => void;
}
export function startStream(options: StreamOptions): () => void;
```

Backoff: 500ms, 1s, 2s, 4s, 8s, 8s + jitter 0–250ms. Setelah 6 kegagalan WS,
pindah ke SSE. Setelah 3 kegagalan SSE, pindah ke polling 5 detik dan state
`degraded`.

### 3.2 Patch

`01-architecture.md` §4: `{seq, upserts, deletes}`; entity bertag `agent`,
`task`, `alert`, `usage`, `connector`, `approval`. Penerapan:

- `agent|task|alert|connector|approval`: upsert ke map entity berdasarkan id;
  delete menghapus dari map.
- `usage`: jangan dijumlahkan di klien (rawan dobel); tandai aggregate
  `usage`/`costs` stale dan refetch debounce 5 detik bila route terlihat.
- Upsert alert yang sudah `resolved` tetap disimpan untuk halaman History;
  daftar Open memfilter di selector.
- Patch diterapkan dalam satu microtask; render Svelte dibatch. Tidak ada
  animasi pada perubahan angka.

Batas: antrean patch maksimal 200. Bila terlampaui, buang antrean, refetch
snapshot, reconnect. Payload tipikal <4KB (`00-design.md` §14.2); payload besar
tidak memblokir UI karena diterapkan sebagai data, bukan HTML.

### 3.3 SSE dan polling

- SSE: `GET /api/v1/events/stream?since=<seq>`, event `patch` dengan `id` =
  seq. Klien menutup dan membuka ulang dengan `since` terbaru pada error,
  bukan membiarkan browser memakai URL lama.
- Polling: `GET /overview` + endpoint route setiap 5 detik; tidak ada patch;
  state `degraded`; tombol "Muat ulang" tetap tersedia.
- Saat `document.visibilityState === 'hidden'` lebih dari 30 detik, tutup
  koneksi. Saat terlihat, reconnect dari `seq` terakhir; gap recovery berlaku.
- `navigator.onLine === false` → state `offline`, banner, mutasi disabled.
  `online` → reconnect segera tanpa menunggu backoff.

### 3.4 UI state koneksi

| State | Dot | Label | Perilaku |
|---|---|---|---|
| `connecting` | `ink-10` | Menghubungkan | skeleton halus, tanpa spinner global |
| `live` | `success` | Live | normal |
| `reconnecting` | `attention` | Menyambung ulang | data tetap, mutasi tetap boleh |
| `syncing` | `attention` | Menyinkronkan | tombol mutasi disabled sampai snapshot |
| `degraded` | `attention` | Polling 5 dtk | data bisa tertinggal; penanda stale |
| `offline` | `critical` | Offline | banner + mutasi disabled |

Dot statis; tidak ada pulse. Teks berubah di `aria-live="polite"`; toast hanya
sekali bila state buruk berlangsung >30 detik. Top bar membawa
`data-connection={state}` untuk tes.

---

## 4. State

### 4.1 Bentuk

Svelte 5 runes, modul `.svelte.ts`, singleton per domain. Tidak ada Redux,
Zustand, TanStack Query, atau library state global lain.

```ts
// web/src/lib/stores/entities.svelte.ts
import { SvelteMap } from 'svelte/reactivity';

export type ApplyResult = 'applied' | 'duplicate' | 'gap' | 'invalid';

export class EntityStore {
  seq = $state(0);
  agents = $state(new SvelteMap<string, AgentCard>());
  tasks = $state(new SvelteMap<string, TaskView>());
  alerts = $state(new SvelteMap<string, AlertView>());
  connectors = $state(new SvelteMap<string, ConnectorView>());
  approvals = $state(new SvelteMap<string, ApprovalView>());

  hydrate(snapshot: {
    seq: number;
    agents?: AgentCard[];
    tasks?: TaskView[];
    alerts?: AlertView[];
    connectors?: ConnectorView[];
    approvals?: ApprovalView[];
  }): void;

  apply(patch: Patch): ApplyResult;
}

export const entities = new EntityStore();
```

`SvelteMap` dipakai agar penambahan/penghapusan entity memicu reaktivitas.
Selector berupa `$derived` di komponen, bukan getter global yang di-cache
manual.

Store lain:

| File | Isi |
|---|---|
| `session.svelte.ts` | `deviceToken`, `authState: 'unknown'|'owner'|'required'|'denied'`, `capabilityDenied` |
| `stream.svelte.ts` | `state: ConnectionState`, `lastSeq`, `lastPatchAt`, `start(since)`, `stop()` |
| `ui.svelte.ts` | `theme`, `locale`, `railOpen`, `sidebarOpen`, `paletteOpen`, `reducedMotion`, `applyDocumentTheme()` |
| `toast.svelte.ts` | antrean toast, `showToast`, `dismissToast` |

Data route yang bukan entity (aggregate usage/costs, settings, automation
rules, knowledge page) disimpan per route atau per store kecil; tidak masuk
`EntityStore`.

### 4.2 Cache dan invalidasi

Entity tidak punya TTL; kebenarannya datang dari snapshot + patch. Aggregate
punya TTL 60 detik dan ditandai stale oleh event.

| Event kind | Dampak |
|---|---|
| `agent.observed`, `agent.seen`, `agent.status_changed`, `agent.disappeared` | `entities.agents`; KPI overview refetch debounce 500ms |
| `session.started`, `session.updated`, `session.ended` | sessions agent; overview `active_sessions` refetch |
| `log.recorded` | append log ke tampilan agent aktif; simpan maksimal 500 di memori |
| `usage.recorded`, `cost.estimated`, `quota.observed` | tandai `usage` + `costs` stale; refetch 5 detik bila route terlihat |
| `alert.*` | `entities.alerts`; hitungan attention + overview refetch 500ms |
| `connector.*` | `entities.connectors`; halaman Integrations |
| `action.*`, `approval.*` | `entities.approvals`; riwayat keputusan pada detail |
| `knowledge.*` | hasil pencarian ditandai stale; refetch hanya saat route Knowledge terlihat |
| `config.changed`, `device.*` | settings/devices refetch |
| `system.health_observed` | indikator health top bar; tidak memicu toast |

Route snapshot disimpan di memori selama sesi tab. Kembali ke route memakai
snapshot bila `seq` masih sama; bila tidak, refetch. Tidak ada localStorage
untuk data operasional; hanya `hq.deviceToken` di sessionStorage dan
`hq.locale`/`hq.theme` sebagai petunjuk awal sebelum `GET /settings`.

### 4.3 Optimistic

Hanya dua mutasi yang optimistis:

```ts
async function ackOptimistic(alert: AlertView) {
  const previous = { ...alert };
  entities.alerts.set(alert.id, { ...alert, state: 'acknowledged', ack_ms: Date.now() });
  try {
    const updated = await ackAlert(alert.id, {});
    entities.alerts.set(updated.id, updated);
  } catch (error) {
    entities.alerts.set(alert.id, previous);
    showToast({ tone: 'critical', message: t('toast.ack_failed') });
  }
}
```

Snooze sama: set `snoozed` + `snooze_until_ms`; rollback pada gagal. Resolve,
approve, deny, connector enable/disable/test, settings, dan semua aksi agent
menunggu server; tombol busy dan tidak dapat diklik ganda. Tidak ada optimistic
untuk `action.*` karena efek sampingnya nyata.

---

## 5. Route

Semua route berada di bawah shell kecuali `/office` yang menyembunyikan
attention rail dan memakai layout full-bleed. Data route diambil di `+page.ts`
dengan `fetch` SvelteKit dan `AbortSignal`; komponen membaca store. String
humanizer (`title`, `detail`, `sentence`) sudah dirender server sesuai
`Accept-Language`; UI tidak merender template.

### 5.1 `/` — Today

Data:

- `getOverview()`: KPI, attention count, health, `usage_today`.
- `getAgents({ limit: 25, status, kind, q })`: tabel agent.
- `getAlerts({ state: 'open' })`: rail (critical/warning; Info tidak).
- `getCosts('24h')`: chart ledger + ringkasan biaya.

WS: `agent.*`, `alert.*`; `usage.*` menandai chart stale. URL:
`?status=&kind=&q=`. Aksi rail: ack/snooze optimistis; item approval membuka
drawer.

```
AppShell
├─ PageHeader (tanggal, meta "diperbarui 19.53 WIB", status koneksi)
├─ KpiStrip
│  └─ KpiFigure ×4 (agent aktif, menunggu, request, biaya hari ini)
├─ AgentTable (Table + AgentMonogram + StatusDot)
├─ Split
│  ├─ Section "Ledger 24 jam" → LedgerChart
│  └─ Section "Biaya per agent" → CostBreakdown (bar tipis, bukan chart)
└─ AttentionRail (shell, ≥1280)
```

State: loading KPI + 8 baris skeleton; empty agent "Belum ada agent
terdeteksi" / "Jalankan collector atau tambahkan heartbeat."; error per panel
dengan "Coba lagi" tanpa mengosongkan panel lain. Ponsel: KPI 2×2, tombol
attention membuka sheet, agent berupa baris 56px, chart 160px.

### 5.2 `/agents/[id]` — Agent detail

Data:

- `getAgent(id, { asOf })`: identitas, status, aktivitas, counter, evidence.
- Tab Summary: `getAgentTasks(id, { limit: 5 })`, `getAgentLogs(id,
  { limit: 20 })`.
- Tab Tasks: `getAgentTasks(id, { phase, cursor })`.
- Tab Log: `getAgentLogs(id, { level, q, before, limit: 50 })`; toggle raw
  menampilkan `raw` yang sudah teredaksi.
- Tab Usage: `getAgentUsage(id, window)`.
- Tab Sessions: `getAgentSessions(id)`, `getAgentFiles(id)`.
- Kontrol: `postAction({ capability: 'agent.pause'|'agent.resume'|'agent.stop',
  target: id, input: {} })`.

Tab dimuat malas saat pertama dibuka, lalu di-cache 30 detik. `?as_of=<ms>`
menampilkan banner "Mode waktu 16 Nov 19.00" + "Keluar"; selama as-of, patch
live untuk agent itu diabaikan. URL: `?tab=summary|tasks|log|usage|sessions`,
`?as_of=`, `?level=`, `?q=`, `?window=`.

```
AppShell
└─ AgentDetailPage
   ├─ DetailHeader (AgentMonogram, nama, StatusDot, meta, ActionBar)
   ├─ Tabs
   ├─ SummaryPanel (CurrentTask, ActivityFeed, MetaPanel)
   ├─ TasksPanel (Table, filter phase)
   ├─ LogPanel (LogList, raw Switch, filter level/q)
   ├─ UsagePanel (LedgerChart unit usd, window Tabs)
   ├─ SessionsPanel (Table sessions + FilesList)
   └─ ApprovalDrawer (bila action butuh approval)
```

State: header skeleton; tab error inline; empty per tab dua baris (log:
"Belum ada log untuk agent ini" / "Collector akan mengisi saat ada
aktivitas."). Ponsel: header bertumpuk, aksi di overflow, tabel menjadi baris,
log 44px, raw toggle di overflow.

### 5.3 `/office` — Kantor

Data:

- `getOverview()`, `getAgents({ limit: 200 })` (maksimal 40 terlihat),
  `getAlerts({ state: 'open' })`, `getConnectors()`,
  `getApprovals({ state: 'pending' })`.
- WS: agent/alert/connector/approval.
- Skeleton lantai dirender lebih dulu; chunk 3D dimuat dinamis setelah
  snapshot tersedia.

URL: `?mode=3d|2d`, `?camera=iso|top|omp|muse|cursor|workers|meeting|mcp`,
`?zone=`, `?agent=`.

```
OfficePage
├─ OfficeToolbar (camera presets, mode, daftar agent, follow)
├─ OfficeCanvas (lazy import; Threlte)
│  ├─ Lighting, Floor, Zones, Furniture, Agents, DataObjects, CameraRig
├─ ZoneBar (6 zona + hitungan)
├─ Inspector (agent terpilih: status, task, tautan detail)
├─ DataBoardDrawer (task/alert/connector/approval)
└─ Fallback2D (WebGL gagal atau mode 2d)
```

State: floor skeleton statis (tanpa karakter palsu); empty "Kantor kosong" /
"Belum ada agent terdeteksi."; WebGL gagal → otomatis 2D + satu baris
"Mode 2D aktif karena WebGL tidak tersedia." Ponsel default 2D; 3D tersedia
lewat overflow.

### 5.4 `/timeline` — Event stream

Data: `getTimeline({ since, until, kind, subject, limit: 50, before })`.
Event membawa `seq`, `id`, `event_ts_ms`, `ingest_ts_ms`, `kind`, `subject`,
`source`, `payload` (sudah teredaksi). Detail event di drawer: payload JSON
pretty, copy, dan tautan `as_of` bila subject agent.

URL: `?kind=&subject=&since=&until=`. Live tail default aktif; tombol
"Jeda" menghentikan penambahan. Filter "Audit" adalah preset
`kind=action.*` (requested/approved/denied/executed/failed), bukan route
terpisah.

```
TimelinePage
├─ PageHeader
├─ FilterBar (kind Select, subject Field, rentang waktu, Live Switch)
├─ EventList (virtualized, baris 44px, mono waktu + kind)
└─ EventDetailDrawer (payload, copy, tautan as_of)
```

State: loading 10 skeleton; empty "Tidak ada event pada filter ini" / "Longgarkan
filter atau pilih rentang waktu lain."; error inline. Ponsel: kartu event.

### 5.5 `/alerts` — Alert

Data: `getAlerts({ state, level })`; detail `getAlert(id)` (keputusan +
delivery, `10-alerts.md` §7); aksi `ackAlert`, `snoozeAlert`, `resolveAlert`.
Info hanya muncul di halaman ini, bukan di rail.

URL: `?state=open|history&level=&id=`. Aksi ack/snooze optimistis; resolve
menunggu server. Detail drawer menampilkan subject, umur, occurrences,
keputusan, dan status pengiriman kanal.

```
AlertsPage
├─ PageHeader + hitungan per level
├─ Tabs (Open, History)
├─ AlertFilters (level, subject)
├─ AlertList (dot level, satu kalimat, subjek, umur, aksi utama)
└─ AlertDetailDrawer (detail, riwayat, delivery, ack/snooze/resolve)
```

State: empty "Tidak ada alert terbuka" / "Alert baru muncul di sini saat rule
terpicu."; error + retry. Ponsel: daftar + bottom sheet.

### 5.6 `/automations` — Approval, audit, policy

Halaman ini adalah permukaan control plane, bukan editor rule alert. Rule dan
quiet hours dideklarasikan di `$HQ_CONFIG/automations.yaml` dan hanya diedit
owner di file; tidak ada endpoint tulis untuk automation (`04-control-plane.md`
§4.2). UI menampilkan antrean approval, audit, dan editor policy.

Data:

- `getApprovals({ state: 'pending' })` untuk antrean.
- `getAudits({ actor, decision, limit })` untuk tabel audit.
- `getTimeline({ kind: 'action.*', limit })` untuk rantai event saat baris audit
  dibuka.
- `getPolicyConfig()` dan `putPolicyConfig({ content, confirm_phrase:
  'UBAH-POLICY' })` untuk policy; server memvalidasi YAML dan menolak tanpa
  frasa.
- Nonce keputusan hanya ada di `ActionOutcome` saat aksi dibuat dari sesi ini.
  Approval tanpa nonce di memori bersifat baca-saja; UI menautkan ke Telegram
  atau meminta aksi diulang.

URL: `?approval=&audit=`. Tidak ada `?rule=`; file rule tidak diedit dari UI.

```
AutomationsPage
├─ PageHeader
├─ Section "Menunggu keputusan" → ApprovalQueue (Table)
├─ ApprovalDrawer (capability, target, input, expiry, nonce, approve/deny)
├─ Section "Audit" → AuditTable (waktu WIB, aktor, capability, target, keputusan)
├─ AuditDetailDrawer (rantai requested → approved/denied → executed/failed)
└─ Section "Policy" → PolicyEditor (YAML, frasa UBAH-POLICY, simpan)
```

State: approval kosong "Tidak ada approval" / "Aksi yang butuh keputusan muncul
di sini."; audit kosong dua baris; policy loading/error dengan "Coba lagi";
`policy_invalid` menampilkan error server tanpa menimpa isi editor. Ponsel:
antrean kartu, drawer menjadi sheet, editor satu kolom mono 12/16.

### 5.7 `/integrations` — Connector

Data: `getConnectors()`; aksi `testConnector`, `pollConnector`,
`setConnectorEnabled`, `runConnectorAction`; manifest dan daftar aksi datang
dari `ConnectorView`. Secret tidak pernah dikirim balik; UI hanya menampilkan
"Belum dikonfigurasi" atau "Terkonfigurasi, terakhir dirotasi <tanggal>".

URL: `?tier=&health=&id=`. Detail drawer: Status, Auth, Test, Actions,
Manifest. Test dan poll menampilkan hasil terakhir + durasi; kegagalan auth
menampilkan pesan aman dan tautan dokumentasi auth.

```
IntegrationsPage
├─ PageHeader
├─ TierSection ×3 (Native, Preset, Generic)
│  └─ ConnectorTable (StatusDot, tier, last run, actions)
└─ ConnectorDetailDrawer
   ├─ StatusPanel
   ├─ AuthPanel (write-only secret field)
   ├─ TestPanel (test/poll, hasil)
   ├─ ActionsPanel (dari manifest)
   └─ ManifestPanel (read-only)
```

State: empty per tier "Belum ada connector di tier ini" / "Tambahkan lewat
`hq connector add`."; `unconfigured` jujur, bukan sehat. Ponsel: kartu 88px,
aksi lebar penuh di sheet.

### 5.8 `/knowledge` — Pencarian dan halaman

Data: `getKnowledgeSearch(q, limit)`; `getKnowledgePage(id)`;
`syncKnowledge(connector?)`; `promoteKnowledge(pageId, title)`. Hasil membawa
`page_id`, `title`, `source`, `score`, `snippet`; halaman membawa chunk teks.

URL: `?q=&page=`. Pencarian debounce 250ms; Enter langsung; Escape
mengosongkan. Promote meminta judul (default judul halaman) dan menampilkan
hasil.

```
KnowledgePage
├─ PageHeader + SyncButton
├─ SearchBar
├─ Layout
│  ├─ ResultList (judul, sumber, skor mono, snippet 2 baris)
│  └─ ArticleView (judul 20/28, meta, chunk, PromoteButton)
└─ PromoteDialog (judul)
```

State: sebelum query "Cari di knowledge" / "Hasil muncul setelah kamu mengetik.";
tidak ada hasil "Tidak ada hasil" / "Coba kata kunci lain atau sinkronkan
sumber."; halaman quarantined menampilkan alasan dan tombol promote bila
diizinkan. Ponsel: satu kolom.

### 5.9 `/costs` — Biaya

Data: `getCosts(window)` untuk chart + total; `getUsage({ group_by, window })`
untuk tabel breakdown. `attribution` (`exact|global|estimated`) tampil sebagai
teks tenang, bukan badge warna. Catatan legacy: riwayat per agent dimulai saat
cutover; total global legacy tidak diatribusikan (`00-design.md` §16).

URL: `?window=24h|7d|30d&group_by=agent|model|provider`.

```
CostsPage
├─ PageHeader + WindowTabs
├─ CostSummary (cash, shadow, requests, tokens)
├─ Section "Cash vs shadow" → LedgerChart height 320
├─ GroupByTabs
└─ BreakdownTable (UsageBucket, bar tipis di kolom share)
```

State: loading; empty "Belum ada usage pada jendela ini" / "Collector usage
belum mencatat data."; error + retry. Ponsel: chart 180, ringkasan 2×2, tabel
kartu.

### 5.10 `/markets` — Watchlist dan portofolio

Data: `getMarkets()` (`GET /api/v1/markets`) untuk watchlist + portofolio,
`refreshMarkets()` (`POST /api/v1/markets/refresh`) yang menghormati gate
harian. Setiap baris menampilkan sumber (`eodhd|fmp`) dan umur data.

```
MarketsPage
├─ PageHeader + FreshnessText + RefreshButton (owner)
├─ WatchlistTable (symbol, name, price, currency, source, age)
├─ PortfolioTable (symbol, shares, avg_buy_price, value, P&L)
└─ Disclaimer (muted, satu baris: HQ tidak memberi nasihat investasi)
```

P&L hanya muncul bila `avg_buy_price` diisi; selain itu sel berisi "tidak
tersedia", bukan angka karangan. Tidak ada tombol beli/jual, tidak ada chart
kedua. Kosong: "Belum ada watchlist" / "Tambahkan simbol di
`config/connectors/markets.yaml`". Connector tanpa kunci = `unconfigured`.
Ponsel: tabel menjadi kartu; harga tetap mono dan rata kanan.

### 5.11 `/settings` — Pengaturan

Data: `getSettings()`, `updateSettings(patch)`, `getDevices()`,
`revokeDevice`, `getBackup`, `startBackup`, `getHealth()` untuk versi dan
status DB. Simpan per seksi; form kotor memblokir navigasi dengan dialog "Buang
perubahan?".

Seksi: Tampilan (tema day/night/system), Bahasa & waktu (locale, timezone),
Retensi (hari, konsekuensi arsip), Backup (status terakhir, tombol "Backup
sekarang", jalur file), Perangkat (tabel device: nama, scope, terakhir terlihat;
device baru dibuat lewat CLI `hq device add`, bukan UI), Tentang (versi, commit,
uptime, ukuran DB).

URL: `?section=`. Aksi ireversibel (cabut device) memakai modal konfirmasi
dengan nama device yang harus diketik.

```
SettingsPage
├─ PageHeader
├─ SettingsNav (200px)
└─ SettingsContent (720px)
   ├─ AppearanceSection (theme)
   ├─ LocaleSection (locale, timezone)
   ├─ RetentionSection (retention_days, konsekuensi)
   ├─ BackupSection (status, startBackup)
   ├─ DevicesSection (DeviceTable, revoke, perintah hq device add)
   └─ AboutSection (versi, health)
```

State: loading skeleton per seksi; error inline; device kosong "Belum ada
device" / "Jalankan `hq device add` untuk membuat device baru."; ponsel: daftar
seksi → detail, field 44px.

---

## 6. i18n

### 6.1 Katalog dan tipe

Tiga katalog flat: `web/src/lib/i18n/{id,en,zh}.json`. Kunci memakai titik dan
huruf kecil, contoh:

```json
{
  "nav.today": "Hari ini",
  "status.menunggu-rizal": "Menunggu Rizal",
  "action.ack": "Tandai ditangani",
  "empty.alerts.title": "Tidak ada alert terbuka",
  "empty.alerts.body": "Alert baru muncul di sini saat rule terpicu.",
  "toast.ack_failed": "Gagal menandai alert. Coba lagi.",
  "time.minutes_ago": "{count} menit lalu"
}
```

```ts
import id from './id.json';
export type MessageKey = keyof typeof id;
export function t(key: MessageKey, params?: Record<string, string | number>): string;
export function localeTag(): string; // id-ID, en-US, zh-Hans
```

`en.json` dan `zh.json` memakai `satisfies Record<MessageKey, string>` sehingga
key yang hilang gagal saat `pnpm check`. Tidak ada nested object, tidak ada
concatenation kalimat di komponen; satu kalimat utuh satu kunci. Placeholder
hanya `{name}` dan diganti literal, bukan HTML.

`locale` diselesaikan: `settings.locale` > cookie `hq.locale` > `navigator.language`
yang dipetakan (`id*` → id, `zh*` → zh, selain itu en) > `id`. Saat berubah:
simpan cookie, `PATCH /settings`, set `document.documentElement.lang`
(`id`, `en`, `zh-Hans`), lalu `invalidateAll()` agar snapshot dengan kalimat
server dirender ulang.

API selalu menerima `Accept-Language` lengkap: `id-ID,id;q=0.9,en;q=0.8`,
`en-US,en;q=0.9`, atau `zh-Hans,zh;q=0.9,en;q=0.8`.

### 6.2 Format

| Fungsi | Aturan |
|---|---|
| `formatNumber(n)` | `Intl.NumberFormat(locale)`; tanpa desimal untuk integer; `tabular-nums` di CSS |
| `formatTokens(n)` | <1.000.000 penuh; ≥1.000.000 compact 1 desimal; nilai persis di `title` |
| `formatUsd(n)` | `style:'currency', currency:'USD'`; 2 desimal; <1 memakai 3 desimal |
| `formatPercent(n)` | bilangan bulat; satu desimal hanya untuk kuota |
| `formatDate(ms)` | `16 Nov 2026` (locale-aware) |
| `formatTime(ms)` | `19.53` di `Asia/Jakarta` |
| `formatDateTime(ms)` | tanggal + waktu; tidak ada "T" ISO di UI |
| `formatRelative(ms, now)` | <1 menit "baru saja"; <60 menit "{n} menit lalu"; <24 jam "{n} jam lalu"; selebihnya tanggal absolut |
| `formatDuration(ms)` | dua unit terbesar, label dari katalog |

Satu interval 30 detik di layout memperbarui waktu relatif; tidak ada timer per
baris. Semua angka memakai `font-variant-numeric: tabular-nums`; angka di tabel
dan KPI memakai Plex Mono. Tanggal/waktu absolut memakai timezone dari
`Settings.timezone`; default `Asia/Jakarta`.

### 6.3 Larangan hardcoded

`web/scripts/check-i18n.mjs`:

1. Bandingkan set kunci ketiga katalog; gagal bila beda.
2. Laporkan kunci tidak terpakai dan kunci yang dipakai tetapi tidak ada.
3. Parse setiap `.svelte`: text node yang mengandung huruf dan atribut
   `aria-label`, `title`, `placeholder`, `alt` harus berasal dari `t(...)`
   atau berada di allowlist teknis (angka, simbol, `kbd`, `code`, nama merek).
4. Gagal bila menemukan literal yang menyerupai kalimat.

Tes `ui source has no hardcoded copy` menjalankan pemeriksaan yang sama di
Vitest. Pengecualian hanya `error.html` (fallback pra-bundle) dan komentar.

CJK: string 中文 memakai `--font-cjk` (font sistem); tidak ada unduhan font
Han saat runtime. Angka dan ID tetap Plex Mono. Tidak ada pemenggalan kata
manual; biarkan `line-break: strict` untuk teks zh.

---

## 7. Arsitektur 3D

### 7.1 Scene graph

`/office` memuat `OfficeCanvas.svelte` secara dinamis setelah snapshot tersedia.
Threlte 8, Three 0.180, orthographic camera, `frameloop="demand"`.

```svelte
<Canvas orthographic frameloop="demand" shadows dpr={[1, 1.5]}>
  <Scene>
    <Lighting mode={dayNight()} />
    <Floor />
    <Zones />                 <!-- 6 zona, label DOM saat hover -->
    <Furniture />             <!-- InstancedMesh per kategori -->
    <Agents max={40} />
    <DataObjects bindings={bindingsFrom(stores)} />
    <CameraRig preset={camera} follow={followId} />
  </Scene>
</Canvas>
```

Lantai 24×16 m, grid nav 0,5 m (48×32), enam zona 8×8 m: OMP, Muse, Cursor di
baris belakang; Workers, Meeting, MCP di baris depan. Kamera ortho isometrik
45°/54,7°, fit 26 m. Visual, material, cahaya, dan batas “bukan game” ada di
`05-design-system.md` §11.

### 7.2 Nav grid, A*, dan steering

- `buildNavGrid(zones, obstacles)` menandai sel terhalang dari furniture
  (meja, rak, dinding potong).
- `findPath` A* 8 arah, biaya diagonal √2, heuristik oktil; path dihaluskan
  dengan line-of-sight agar karakter tidak zig-zag.
- Steering: kecepatan maksimal 1,2 m/s; separation radius 0,6 m; arrive radius
  0,15 m; replan bila sel berikutnya terhalang. Tidak ada pathfinding di main
  thread per frame; path dihitung saat target berubah.
- Target agent: meja zona masing-masing; meeting table untuk `meet`. Jika tidak
  ada jalur, agent tetap di posisi terakhir dan status tetap; tidak ada
  teleport tanpa alasan.

### 7.3 Rig dan state machine

Rig prosedural, tanpa GLTF: manusia low-poly untuk `AgentKind::Utama`, robot
untuk `Worker`/`Mcp`; tinggi 1,7 m / 1,6 m. Semua mesh dibagi; animasi
mengubah rotasi part, bukan skala global. Tidak ada skeletal animation.

| State | Pemicu | Animasi |
|---|---|---|
| `error` | status `Error` | LED critical, tubuh diam, kepala menunduk pendek |
| `wait` | status `MenungguRizal` | berat badan bergeser 0,2 Hz, tangan di meja |
| `meet` | meeting aktif di meja rapat | duduk, angguk sesekali |
| `walk` | target posisi berbeda | kaki ±25° pada 1,8 Hz, sepadan kecepatan |
| `type` | status `Bekerja`, sudah di meja | lengan 6 Hz bergantian, kecil |
| `think` | status `Berpikir` | kepala miring, satu tangan |
| `idle` | status `Santai` | napas 0,3 Hz, amplitudo 0,02 |
| `sleep` | status `Offline` | duduk menunduk, statis |

Prioritas saat sinyal bertabrakan: `error` > `wait` > `meet` > `walk` > `type` >
`think` > `idle` > `sleep`. `Bekerja` memakai `walk` saat berpindah, lalu
`type` saat tiba. Perubahan status memakai crossfade pose 150ms; reduced motion
memakai crossfade tanpa interpolasi langkah.

### 7.4 Binding objek

| Objek | Sumber store | Aksi |
|---|---|---|
| Papan putih | `entities.tasks`, hari ini, maks 5 + N | drawer task; tautan agent |
| Papan alert | `entities.alerts` open warning/critical | drawer alert; ack/snooze |
| Rak server | `entities.connectors` tier MCP/native server | drawer connector; tautan Integrations |
| Meja rapat | `entities.approvals` pending | drawer approval; approve/deny |

`bindingsFrom(state)` mengembalikan hitungan dan item; objek tanpa data tidak
interaktif. Hover memakai outline 1px `attention`; seleksi 2px. Label objek
hanya saat hover/seleksi, satu pada satu waktu, 12/16, `surface` + hairline.
Klik objek membuka drawer DOM, bukan tooltip yang tidak dapat difokus.

### 7.5 Kamera, hari/malam, dan render

- Preset dari `05-design-system.md` §11.3; transisi 300ms `--ease`; reduced
  motion instan. Tidak ada auto-orbit.
- Follow agent hanya setelah pengguna menekan "Fokus"; memilih agent tidak
  memindahkan kamera.
- Day/night dari jam `Asia/Jakarta`, dihitung ulang saat melewati 06.00/18.00;
  tidak ada kontrol manual.
- `frameloop="demand"`: `invalidate()` dipanggil saat patch entity yang
  memengaruhi scene, kamera, seleksi, atau status agent. Loop berjalan saat
  ada `walk`/`meet`; animasi ambient (`type`/`think`/`wait`) dibatasi 10 fps;
  saat semua `idle`/`sleep`, frame berhenti setelah pose settle.
- `OfficeStats` (05 §13.2) diperbarui 1 detik sekali dan diekspos sebagai
  `window.__hqOfficeStats`; tidak ada UI statistik kecuali `?debug=1`.

Budget: initial bundle <350KB gzip; chunk `/office` <300KB gzip; draw call
≤120; triangle ≤120k; satu shadow map 1024; target 60 fps, gate ≥55 fps pada
40 agent. 3D tidak pernah masuk chunk awal; komponen `Fallback2D` tersedia
sebelum chunk 3D selesai.

### 7.6 2D fallback dan reduced motion

- Fallback otomatis bila WebGL tidak tersedia atau `?mode=2d`; default di
  ponsel.
- SVG floor plan 24×16 dengan enam zona berlabel, dot agent, hitungan; klik
  agent/zona sama dengan 3D. Daftar agent DOM di samping/bawah adalah jalur
  keyboard penuh.
- Reduced motion: walk cycle mati (posisi berpindah dengan crossfade 150ms),
  transisi kamera instan, animasi ambient dimatikan, tidak ada parallax.
- Tidak ada build mode, needs, kustomisasi karakter, atau multiplayer.

---

## 8. Keyboard dan command palette

### 8.1 Peta keyboard

| Tombol | Aksi | Cakupan |
|---|---|---|
| `Ctrl/Cmd+K` | Buka/tutup command palette | global |
| `/` | Fokus pencarian halaman | global kecuali input |
| `?` | Dialog shortcut | global kecuali input |
| `g` lalu `t` | Today | global |
| `g` lalu `a` | Alerts | global |
| `g` lalu `o` | Office | global |
| `g` lalu `l` | Timeline | global |
| `g` lalu `i` | Integrations | global |
| `g` lalu `k` | Knowledge | global |
| `g` lalu `c` | Costs | global |
| `g` lalu `s` | Settings | global |
| `j` / `k` | Baris berikutnya/sebelumnya | daftar/tabel dengan `data-list` |
| `Enter` | Buka baris terpilih | daftar/tabel |
| `a` | Ack alert terpilih | daftar alert, rail |
| `s` | Snooze 1 jam alert terpilih | daftar alert, rail |
| `[` | Buka/tutup attention rail | global desktop |
| `\` | Ciutkan/bentangkan sidebar | global desktop |
| `Esc` | Tutup overlay; kosongkan search | overlay/input |
| `r` | Muat ulang data route | global, bukan aksi utama |

Urutan `g` memiliki timeout 800ms dan menampilkan hint keycap di bawah top bar.
Shortcut diabaikan bila fokus berada di `input`, `textarea`, `select`, atau
`[contenteditable]`, kecuali `Escape` dan `Ctrl/Cmd+K`.

### 8.2 Command palette

Aksi statis + agent dari store. Grup: Navigasi, Agent, Aksi, Tampilan.

| id | Label kunci | Kapabilitas | Muncul saat |
|---|---|---|---|
| `navigate.today` | `nav.today` | — | selalu |
| `navigate.alerts` | `nav.alerts` | — | selalu |
| `navigate.office` | `nav.office` | — | selalu |
| `navigate.timeline` | `nav.timeline` | — | selalu |
| `navigate.integrations` | `nav.integrations` | — | selalu |
| `navigate.knowledge` | `nav.knowledge` | — | selalu |
| `navigate.costs` | `nav.costs` | — | selalu |
| `navigate.settings` | `nav.settings` | — | selalu |
| `agent.open.<id>` | nama agent | — | agent ada |
| `alert.ack.<id>` | `action.ack` | owner action | alert open |
| `alert.snooze.<id>` | `action.snooze` | owner action | alert open |
| `approval.approve.<id>` | `action.approve` | keputusan owner | approval pending |
| `approval.deny.<id>` | `action.deny` | keputusan owner | approval pending |
| `view.theme.day` | `theme.day` | — | selalu |
| `view.theme.night` | `theme.night` | — | selalu |
| `view.locale.id/en/zh` | `locale.*` | — | selalu |
| `office.camera.<preset>` | `office.camera` | — | route office |
| `office.follow` | `office.follow` | — | agent terpilih |
| `office.2d` | `office.mode2d` | — | route office |
| `copy.request_id` | `action.copy_request_id` | — | error terakhir ada |

Aksi kontrol tidak pernah dijalankan tanpa dialog konfirmasi bila destruktif
(stop agent, cabut device, deny approval). Palette menampilkan alasan disabled
("Butuh approval", "Agent offline") dan tidak menampilkan aksi yang tidak
relevan dengan state.

---

## 9. Form, approval, dan state UI

### 9.1 Form

- Satu form per seksi; simpan per seksi, bukan satu tombol global.
- Validasi blur + submit; error inline 12/16 `critical`, `aria-invalid`,
  `aria-describedby`; fokus ke error pertama.
- Tombol simpan disabled saat form bersih; loading mempertahankan lebar;
  sukses → toast `success`; gagal → error inline + toast critical bila bukan
  error field.
- Navigasi keluar dengan form kotor memunculkan modal "Buang perubahan?".
- Field rahasia (token connector/device) hanya write-only; tidak pernah
  ditampilkan kembali. `autocomplete="off"`, `spellcheck="false"`.
- Angka (retensi) memakai input number dengan satuan "hari" di luar field;
  tidak ada slider.

### 9.2 Approval

- Approval drawer menampilkan capability, target, pemohon, waktu kedaluwarsa,
  dan `input_json` read-only (maks tinggi 240, scroll, mono 12/16).
- Nonce berasal dari `ActionOutcome.approval.nonce` saat aksi dibuat dari sesi
  ini; simpan di `session.svelte.ts` dengan map `approvalId → nonce`, hanya di
  memori. `ApprovalView` tidak membawa nonce (`04-control-plane.md` §5.3).
- Tanpa nonce, "Setujui" dan "Tolak" disabled dengan alasan "Setujui lewat
  Telegram atau ulangi aksi"; drawer tetap menampilkan detail.
- Dengan nonce, "Setujui" adalah primary; "Tolak" memakai gaya destructive
  trigger dan membuka modal alasan (opsional) karena keputusan tidak dapat
  dibatalkan. Keduanya mengirim `{nonce, note?}`.
- Kedaluwarsa: hitung mundur 1 detik; saat habis, tombol disabled dan drawer
  refetch. Tidak ada auto-approve dan tidak ada endpoint extend.
- Server menolak (`403 forbidden`, `409 approval_expired`/`approval_already_decided`)
  → pesan + `request id`; tidak ada optimistic update.
- Setelah sukses, nonce dihapus dari map; tautan "Lihat di timeline" dengan
  preset `/timeline?kind=action.*`; keputusan juga terlihat di detail alert/agent.

### 9.3 Konfirmasi destruktif

Modal 460px, judul berupa verba + objek, body menjelaskan akibat. Untuk aksi
ireversibel seperti cabut device, pengguna mengetik nama device; tombol
konfirmasi baru aktif. Tombol batal mendapat fokus awal.
Konfirmasi tidak pernah menjadi primary halaman; hanya primary di dalam modal,
dengan fill `critical` + `on-critical`. Tidak ada undo palsu.

### 9.4 Error, empty, loading

| Keadaan | Komponen | Aturan |
|---|---|---|
| Loading awal | `Skeleton` | tinggi sama dengan konten; maks satu per region; tanpa shimmer |
| Loading aksi | Button loading | label tetap, `aria-busy`, tidak bisa diklik ganda |
| Empty | `EmptyState` | dua baris, satu aksi sekunder; tanpa data contoh |
| Error route | `+error.svelte` | status, pesan aman, request id, "Muat ulang"/"Kembali" |
| Error panel | inline | data panel lain tetap; "Coba lagi" |
| Stale | penanda teks | "Data terakhir 19.53" di header/panel; bukan toast |
| Offline | `OfflineBanner` | 36px di bawah top bar; teks + "Coba lagi"; `role="status"` |
| Reconnect | banner 2 detik | "Tersambung kembali"; fade 150ms; bukan toast |

Offline banner: `ground` + rule kiri 2px `attention` + teks `ink-12`; tidak
menutupi konten (mendorong layout 36px) dan tidak muncul berulang. Mutasi
dinonaktifkan dengan alasan "Offline"; navigasi dan baca tetap jalan.

---

## 10. Testing

### 10.1 Unit (Vitest, jsdom)

`web/tests/unit/entities.test.ts`

- `apply patch in sequence updates entities`
- `duplicate patch is ignored`
- `seq gap requests a route snapshot`
- `malformed patch requests a route snapshot`
- `usage patch marks aggregates stale without summing`
- `ack rolls back when the request fails`
- `snooze rolls back when the request fails`
- `resolve waits for the server`

`web/tests/unit/errors.test.ts`

- `maps 403 to a capability message with request id`
- `maps 409 to a refetch`
- `maps network failure to offline state`
- `never renders a raw server message`

`web/tests/unit/i18n.test.ts`

- `all locales have the same keys`
- `no unused keys`
- `ui source has no hardcoded copy`
- `params replace without html injection`

`web/tests/unit/format.test.ts`

- `numbers use locale grouping and tabular figures`
- `usd uses three decimals below one`
- `tokens use compact above one million with exact title`
- `time renders in asia jakarta`
- `relative time switches to absolute after a day`

`web/tests/unit/keyboard.test.ts`

- `palette ranks prefix before substring`
- `palette ignores shortcuts inside text fields`
- `g sequence expires after 800ms`

`web/tests/unit/office-nav.test.ts` dan `office-state.test.ts` (S14) ada di
`05-design-system.md` §13.3.

### 10.2 E2E (Playwright)

Seed: `web/tests/fixtures/e2e.json`, dijalankan dengan
`hq serve --data <tmp> --seed <fixture> --allow-seed --port 8901`. Fixture
berisi: 6 agent (satu `error`, satu `offline`, satu `menunggu-rizal`), 3 alert,
1 approval pending, 2 task per agent, 40 event usage, 4 connector (satu
`unconfigured`), 1 halaman knowledge. Fixture deterministik; tidak ada data
palsu di produksi.

`web/tests/e2e/today.spec.ts`

- `today shows agents and attention from events`
- `today search filters agent rows`
- `today ledger toggle shows a data table`
- `today empty state appears without agents`

`web/tests/e2e/agent-detail.spec.ts`

- `agent detail tabs switch without reload`
- `raw log toggle reveals redacted raw text`
- `as of banner exits time travel`
- `pause action requests approval`

`web/tests/e2e/alerts.spec.ts`

- `ack alert updates the rail count without reload`
- `snooze alert moves it out of open`
- `alert history lists resolved alerts`

`web/tests/e2e/office.spec.ts`

- `office loads the scene and clicking an agent opens detail`
- `office 2d fallback works without webgl`
- `office object click opens real data`

`web/tests/e2e/timeline.spec.ts`

- `timeline filters by kind and subject`
- `timeline event links to agent as of`
- `timeline live tail pauses and resumes`

`web/tests/e2e/approvals.spec.ts`

- `approved action appears in the audit timeline`
- `denied approval requires a reason step`
- `expired approval disables both actions`

`web/tests/e2e/automations.spec.ts`

- `approval queue lists pending approvals`
- `approving without a nonce is disabled`
- `policy save requires the UBAH-POLICY phrase`
- `audit row opens its event chain`

`web/tests/e2e/integrations.spec.ts`

- `unconfigured connector is shown honestly`
- `connector test reports an error without leaking a secret`
- `connector poll result is visible after completion`

`web/tests/e2e/knowledge.spec.ts`

- `search returns ranked pages and opens one`
- `promote page updates the list`

`web/tests/e2e/costs.spec.ts`

- `costs switches window and groups by model`
- `legacy global attribution is labelled`

`web/tests/e2e/settings.spec.ts`

- `settings save updates theme and language`
- `device list shows scope and last seen`
- `revoking a device requires typing its name`

`web/tests/e2e/auth.spec.ts`

- `remote request without token returns 401`
- `remote request with token returns 200`
- `token gate accepts a device token`

`web/tests/e2e/offline.spec.ts`

- `offline banner appears and disables mutations`
- `reconnect resumes from the last seq`

### 10.3 Visual

Ukuran dan tema: 1440×900 light, 1440×900 dark, 390×844 light. Toleransi
0,1% pixel (`08-testing-gates.md` §7). Animasi dimatikan dengan
`emulateMedia({ reducedMotion: 'reduce' })`; tunggu `document.fonts.ready` dan
`data-connection="live"`.

- `web/tests/visual/shell.spec.ts`: `shell matches day snapshot at 1440x900`,
  `shell matches night snapshot at 1440x900`,
  `shell matches phone snapshot at 390x844`
- `web/tests/visual/today.spec.ts`: `today matches day snapshot at 1440x900`,
  `today matches night snapshot at 1440x900`,
  `today matches phone snapshot at 390x844`
- `web/tests/visual/agent-detail.spec.ts`: tiga nama yang sama untuk detail.
- `web/tests/visual/alerts.spec.ts`: tiga nama yang sama untuk alerts.
- `web/tests/visual/office.spec.ts`: `office matches day snapshot at 1440x900`,
  `office matches night snapshot at 1440x900`,
  `office 2d matches phone snapshot at 390x844`.
- `web/tests/visual/costs.spec.ts`: tiga nama yang sama untuk costs.

### 10.4 A11y

`web/tests/a11y/pages.spec.ts`

- `today has no serious or critical axe violations`
- `agent detail has no serious or critical axe violations`
- `alerts has no serious or critical axe violations`
- `office has no serious or critical axe violations`
- `settings has no serious or critical axe violations`
- `keyboard only can acknowledge an alert`
- `focus returns to the trigger after a dialog closes`
- `status is not conveyed by color alone`
- `chart exposes a data table`

### 10.5 Perf

Fixture terpisah: `web/tests/fixtures/office-40.json` (40 agent, status
campuran, 6 zona) dan `web/tests/fixtures/office-idle.json` (semua agent
`santai`/`offline`). Keduanya deterministik dan hanya aktif dengan
`--allow-seed`.

`web/tests/perf/lcp.spec.ts`

- `today lcp is under 1500ms locally`

`web/tests/perf/bundle.spec.ts`

- `initial bundle is under 350kb gzip`
- `office chunk is under 300kb gzip`
- `no asset is loaded from a remote origin`

`web/tests/perf/office.spec.ts` (server `office-40` di 8903)

- `office averages 55fps with 40 agents`
- `office draw calls stay under 120`

`web/tests/perf/office-idle.spec.ts` (server `office-idle` di 8904)

- `office stops rendering when idle`

### 10.6 Konfigurasi

`web/playwright.config.ts`:

- Project `chromium` 1440×900 dan `phone` 390×844, keduanya ke server 8901.
- Project `perf` ke 8903 (`office-40`), `perf-idle` ke 8904 (`office-idle`),
  dan `remote` ke 8902 untuk tes auth.
- `webServer` array: 8901 (E2E seed), 8902 (`HQ_TRUST_LOOPBACK=0` + device
  token), 8903 (`office-40`), 8904 (`office-idle`); semuanya
  `HQ_ALLOW_SEED=1`, data dir di `.tmp/`.
- `baseURL` `http://127.0.0.1:8901`; remote API memakai `request` fixture ke
  `http://127.0.0.1:8902`.
- `reuseExistingServer: !process.env.CI`; artifact trace + screenshot saat
  gagal.
- Binary path `HQ_BIN ?? '../../target/debug/rizal-agent-hq'`.

`web/vitest.config.ts`: environment `jsdom`, include
`tests/unit/**/*.test.ts`, setup `tests/unit/setup.ts` (mock `matchMedia`,
`ResizeObserver`, `document.fonts`).

---

## 11. Build dan embed

### 11.1 rust-embed

`crates/hq-api/src/ui.rs`:

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/build/"]
#[include = ["**/*"]
#[exclude = ["**/*.map"]]
struct UiAssets;

pub fn ui_router() -> axum::Router {
    axum::Router::new().fallback(serve_asset)
}
```

Aturan layanan:

- `GET /_app/immutable/*`: `Cache-Control: public, max-age=31536000, immutable`;
  ETag dari hash nama file.
- `GET /`: `Cache-Control: no-cache`; `index.html` selalu direvalidasi.
- Route non-API yang tidak punya file dan bukan aset (tanpa titik pada segmen
  terakhir) → `index.html` 200.
- Aset hilang (path berisi titik) → 404, bukan SPA fallback.
- `/api/*` tidak pernah jatuh ke `index.html`.
- `Content-Type` dari `mime_guess`; gzip/brotli via `tower-http` compression
  untuk HTML/CSS/JS/JSON, bukan WOFF2.
- CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:; font-src 'self'; connect-src 'self' ws: wss:;
  object-src 'none'; base-uri 'none'; frame-ancestors 'none'`.
- Header tambahan: `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: no-referrer`, `Cross-Origin-Opener-Policy: same-origin`.

### 11.2 build.rs assertion

`crates/hq-api/build.rs` gagal dengan pesan yang dapat ditindaklanjuti bila:

1. `web/build/index.html` tidak ada.
2. `web/build/_app/` tidak ada atau kosong.
3. `index.html` tidak memuat `_app/`.
4. Font wajib tidak ada: `web/build/fonts/plex-sans-400.woff2`,
   `plex-sans-500.woff2`, `plex-sans-600.woff2`, `plex-mono-400.woff2`,
   `plex-mono-500.woff2`.
5. Pemindaian `.html`, `.css`, `.js` menemukan URL eksternal yang tidak masuk
   allowlist: `unpkg.com`, `jsdelivr.net`, `cdnjs.cloudflare.com`,
   `fonts.googleapis.com`, `fonts.gstatic.com`, `cdn.` pada host apa pun.
   Allowlist hanya namespace SVG (`http://www.w3.org/`) dan komentar lisensi.
6. `web/build` memuat file `.map`.

Pesan gagal menyebut file dan baris. Perintah perbaikan ada di pesan:
`pnpm -C web build`.

### 11.3 Font

`web/scripts/subset-fonts.mjs` memakai `subset-font` (devDependency) dan
menulis:

```
web/static/fonts/plex-sans-400.woff2
web/static/fonts/plex-sans-500.woff2
web/static/fonts/plex-sans-600.woff2
web/static/fonts/plex-mono-400.woff2
web/static/fonts/plex-mono-500.woff2
```

Codepoint minimum: Basic Latin, Latin-1 Supplement, General Punctuation
(`– — … ·`), dan simbol mata uang `$`. Tidak ada italic. `@font-face` memakai
`font-display: swap`, `unicode-range` sesuai subset, dan preload hanya
`plex-sans-400.woff2`. Lisensi OFL disalin ke `web/static/fonts/OFL.txt`.

CJK tidak di-embed; `--font-cjk` memakai font sistem (lihat 05 §4). Ini
keputusan budget: satu subset Noto Sans SC penuh melampaui 350KB gzip dan
memperlambat LCP. Tidak ada permintaan font ke jaringan.

### 11.4 CI dan bundle

`.github/workflows/ci.yml` job `web`:

1. Node 22 + corepack; `pnpm install --frozen-lockfile --ignore-scripts`.
2. `pnpm -C web api:check` (schema tidak drift).
3. `pnpm -C web check:i18n`.
4. `pnpm -C web check`.
5. `pnpm -C web test`.
6. `pnpm -C web build`.
7. `pnpm -C web check:bundle`.
8. `cargo build` (meng-embed `web/build`).

`check-bundle.mjs` menghitung gzip semua chunk entry (semua yang dimuat
`index.html`, kecuali chunk `/office`) dan gagal >350×1024 byte; chunk office
gagal >300×1024 byte. Angka dicetak ke artefak.

E2E job: build binary, jalankan seed 8901 + remote 8902, `playwright install
--with-deps chromium`, `pnpm -C web exec playwright test`.

Tidak ada CDN, tidak ada Node saat runtime, tidak ada service worker yang
mengunduh aset dari jaringan. PWA installability bukan scope v1.

---

## 12. Interface per sesi (S07–S16)

| Sesi | File utama | Menghasilkan | Tes |
|---|---|---|---|
| S06 | `web/src/lib/api/{schema.d.ts,client.ts,errors.ts,types.ts}`, `web/scripts/gen-api.mjs` | tipe generated, `request<T>`, `ApiError`, pembungkus endpoint | `api:check`, `maps 403 ...` |
| S07 | `web/src/app.css`, `lib/components/{ui,domain,shell}`, `lib/stores/{ui,toast}.svelte.ts`, `lib/i18n/*`, `lib/format/*`, `routes/+layout.*`, `routes/+error.svelte` | `AppShell`, seluruh komponen 05 §6, `t()`, `format*`, `showToast`, `ui` store, `OfflineBanner`, `TokenGate` | `shell matches day/night/phone`, `shell has no serious or critical axe violations` |
| S08 | `routes/+page.{ts,svelte}`, `lib/components/domain/{AgentTable,CostBreakdown,KpiStrip}.svelte`, `lib/stores/{entities,stream}.svelte.ts`, `lib/api/stream.ts` | `EntityStore.hydrate/apply`, `startStream`, `ApplyResult`, Today | `today shows agents and attention from events`, `today lcp is under 1500ms locally` |
| S09 | — (backend) | tidak membuat file web; wajib mempertahankan field `AgentCard` untuk Today | paritas backend, bukan UI |
| S10 | — (backend) | tidak membuat file web; status `unconfigured` terlihat di Integrations | paritas backend, bukan UI |
| S11 | `routes/alerts/*`, `lib/components/domain/{AlertList,AlertDetail,AlertActions}.svelte` | `ackOptimistic`, `snoozeOptimistic`, detail alert | `ack alert updates the rail count without reload`, `snooze alert moves it out of open` |
| S12 | `routes/agents/[id]/*`, `lib/components/domain/{DetailHeader,TasksPanel,LogPanel,UsagePanel,SessionsPanel,ApprovalDrawer}.svelte` | detail 5 tab, as-of, kontrol agent | `agent detail tabs switch without reload`, `raw log toggle reveals redacted raw text` |
| S13 | `routes/office/*`, `lib/office/{OfficeCanvas,Scene,Lighting,Floor,camera,stats}.{svelte,ts}` | `OfficeHandle`, `CameraPreset`, `OfficeStats` | `office uses orthographic isometric camera by default`, `office draw calls stay under 120` |
| S14 | `lib/office/{nav,characters,rig}.ts` | `buildNavGrid`, `findPath`, `animationFor`, `rigFor` | `a star routes around furniture`, `forty agents keep walking without overlap` |
| S15 | `lib/office/{objects,DataBoardDrawer,Fallback2D}.{ts,svelte}` | `bindingsFrom`, binding objek, fallback 2D | `whiteboard lists today tasks and opens the task drawer`, `office 2d fallback works without webgl` |
| S16 | `routes/timeline/*`, `routes/knowledge/*`, `lib/components/domain/{EventList,EventDetail,SearchResults,ArticleView,PromoteDialog}.svelte` | timeline filter + as-of, knowledge search/page/promote | `timeline filters by kind and subject`, `search returns ranked pages and opens one` |

Kontrak antar-sesi:

- S07 tidak boleh mengubah bentuk `EntityStore`; S08 yang menetapkannya.
- S11/S12 memakai `EntityStore.apply` dan `showToast`, bukan store baru.
- S13–S15 tidak boleh memanggil API langsung; semua data lewat store yang
  sudah diisi route `/office`.
- S16 tidak menambah library chart; timeline adalah daftar, knowledge adalah
  teks.
- Route `/automations`, `/integrations`, `/settings` sudah ditetapkan di §5,
  tetapi implementasinya jatuh pada S24/S17/S26 setelah kontrak §2.3 terdaftar
  di OpenAPI.

---

## Diserahkan ke builder

- Implementasi SQL/agregasi `series` untuk `UsageReport`/`CostReport` (bucket
  5 menit/1 jam/6 jam) beserta indeks yang dibutuhkan; bentuk respons sudah
  diikat di §2.3, rencana query bukan.
- Pemilihan tekstur papan kantor (CanvasTexture runtime versus tekstur
  dipanggang saat build) dan mekanisme invalidasi saat data berubah.
- Komposisi visual 2D fallback dan tata letak daftar agent DOM di sampingnya;
  data dan aksi sudah diikat, gambar dan susunan akhir belum.
- Daftar glyph final font setelah katalog i18n beku, termasuk keputusan CJK
  sistem versus subset Noto yang di-embed; batasnya budget §11.4.
- Peninjauan terjemahan 中文 oleh penutur; struktur kunci, `lang`, dan format
  angka sudah diikat.
