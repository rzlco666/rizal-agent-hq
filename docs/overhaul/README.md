# Rizal Agent HQ v3 — Implementation Plan

**Design (approved):** [`00-design.md`](00-design.md)
**Status plan:** siap dieksekusi manual, sesi demi sesi.
**Tanggal:** 2026-11-16

Dokumen ini adalah pintu masuk. Setiap sesi punya file sendiri di
[`sessions/`](sessions/) dan tidak boleh dieksekusi sebelum prasyaratnya lulus.

---

## 1. Cara mengeksekusi

1. Buat worktree terpisah supaya layanan v2 tetap hidup:
   `git worktree add ../rizal-agent-hq-v3 -b overhaul/v3`
2. Kerjakan sesi **berurutan** sesuai indeks. Satu sesi = satu rangkaian commit
   di `overhaul/v3`. Setelah gate lulus, tag `sNN-done`.
3. Setiap tugas dimulai dengan tes yang gagal, lalu implementasi minimal, lalu
   tes hijau, lalu commit. Jangan menulis implementasi sebelum tesnya ada.
4. Setelah gate lulus: isi `## Hasil` di file sesi (tanggal, commit, output gate),
   lalu lanjut. Jangan menumpuk dua sesi yang belum lulus.
5. Layanan v2 tetap di `127.0.0.1:8900`. Selama pengembangan, kandidat berjalan
   di `127.0.0.1:8901` dengan data dir terpisah.
6. Jika sesi melewati 3 jam atau gate gagal dua kali: berhenti, tulis blocker di
   `sessions/NOTES.md`, jangan lanjut ke sesi berikutnya.
7. Jangan pernah commit secret. Gunakan `hq secret set` (S17) atau env untuk uji.

**Definisi selesai satu sesi:** semua task di file sesi selesai, gate perintah
lulus tanpa warning, tidak ada `TODO`/stub/placeholder, dan `## Hasil` terisi.

## 2. Kontrak global

Berlaku untuk semua sesi; setiap task mewarisi bagian ini.

| Area | Aturan |
|---|---|
| Rust | 1.96+, edition 2024, resolver 3, `rustfmt`, `clippy -D warnings` |
| Frontend | SvelteKit 2, Svelte 5, Vite 7, TypeScript 5.9 strict, Tailwind 4, Threlte 8 |
| Paket | Cargo.lock + pnpm lockfile dikomit; `cargo deny check`; pnpm `ignore-scripts` |
| Build | Satu binary; UI/font/ikon di-embed; tidak ada CDN saat runtime |
| Data | SQLite bundled + WAL; satu writer; migrasi forward-only |
| Waktu | Simpan UTC ms; tampilkan `Asia/Jakarta`; `tabular-nums` |
| Teks | Semua teks bebas lewat redaksi sebelum persist; tidak ada secret di event/log/error |
| API | `/api/v1`; OpenAPI dihasilkan; klien TS digenerate; tidak ada endpoint tanpa skema |
| Auth | Loopback default; remote wajib token; aksi kontrol wajib capability + audit |
| i18n | ID default, EN, 中文; tidak ada string UI hardcoded |
| Desain | Token §13 design; grid 4px; ikon lucide 16/18 stroke 1.5; kontras AA; tanpa emoji ikon |
| Gerak | Satu kurva; 150–300 ms; reduced-motion wajib |
| Data palsu | Dilarang. Kosong = empty state jujur. Connector tanpa kredensial = `unconfigured` |
| Kode | `unwrap`/`expect` hanya di tes; error typed; tidak ada `panic!` pada input |
| Commit | Conventional commit; badan boleh Indonesia; satu commit = satu langkah logis |

**Dependency floor** (pin exact di S00): axum 0.8, tokio 1.47, rusqlite 0.41,
reqwest 0.12 rustls, utoipa 5, schemars 1, jsonschema 0.30, jsonpath-rust 1,
rmcp 0.8, opentelemetry 0.31, fastembed 5, age 0.11, clap 4, ulid 1, thiserror 2,
tracing 0.1, rust-embed 8, hmac 0.12, sha2 0.10, wiremock 0.6, proptest 1,
insta 1, criterion 0.5. Frontend: svelte 5, @sveltejs/kit 2, vite 7,
typescript 5.9, tailwindcss 4, @threlte/core 8, three 0.180, uplot 1.6,
lucide-svelte, @playwright/test 1.5x, vitest 3.

**Review Focus** (lima kelas input yang paling mungkin menggigit; tesnya wajib
ada di task yang memiliki kode terkait, lihat [`08-testing-gates.md`](08-testing-gates.md)):

1. Sumber live hilang di tengah poll (proses mati, file terpotong) — snapshot
   tidak boleh menggantung; agent jadi `Offline`/`Error` dengan bukti terakhir.
2. Webhook duplikat/terlambat — tepat satu perubahan state, tanpa alert ganda.
3. String mirip secret di output agent — teredaksi sebelum persist, tidak muncul
   di API/WS/pencarian.
4. SQLite gagal tulis/disk penuh/crash antara append dan projection — restart
   pulih, tidak ada state separuh.
5. Request remote lewat proxy salah konfigurasi — tidak dianggap owner; aksi
   kontrol ditolak.

## 3. Template file sesi

Setiap `sessions/Sxx.md` memakai bentuk ini.

```markdown
# Sxx — Judul

> **Prasyarat:** Sxx-1 lulus · **Spec:** 00-design.md §…
> **Estimasi:** 1–3 jam · **Milestone:** …

**Goal:** satu kalimat.

**Files:**
- Create: `path`
- Modify: `path:line`
- Test: `path`

**Interfaces:**
- Consumes: signature exact dari sesi sebelumnya.
- Produces: signature exact yang dipakai sesi berikutnya.

## Tasks

### Task 1 — Nama

- [ ] **Step 1: Tulis tes gagal** (kode tes + assertion exact)
- [ ] **Step 2: Jalankan, pastikan gagal** — perintah + output yang diharapkan
- [ ] **Step 3: Implementasi** — signature + file; body hanya bila algoritma tidak ditentukan
- [ ] **Step 4: Jalankan, pastikan lulus**
- [ ] **Step 5: Commit** — `feat(scope): …`

## Acceptance

- [ ] perilaku yang bisa diamati, bukan "kode ada"

## Gate

```bash
perintah
```
Expected: …

## Rollback

`git revert <range>` atau `git tag -d sNN-done`; catat batas data yang aman.

## Hasil

- Tanggal:
- Commit:
- Gate:
- Catatan:
```

Beberapa task memakai `Step 2–5` sebagai singkatan baku: jalankan tes (gagal)
→ implementasi → jalankan tes (lulus) → commit. Isinya tetap satu deliverable.

## 4. Indeks sesi

27 sesi utama. S23 dan S24 dipecah menjadi A/B/C saat eksekusi karena melewati
batas 3 jam; total 30 file, dependensi tetap berurutan.

| Sesi | Judul | Prasyarat | Deliverable | Gate |
|---|---|---|---|---|
| [S00](sessions/S00.md) | Fondasi, CI, oracle paritas | — | workspace + CI + oracle beku | CI lokal hijau, checksum oracle |
| [S01](sessions/S01.md) | Domain, event, redaksi | S00 | tipe + katalog + redaksi v2 | unit + corpus redaksi |
| [S02](sessions/S02.md) | Store, projection, replay | S01 | SQLite + rebuild | rebuild identik |
| [S03](sessions/S03.md) | Ingest, idempotency, bus, WS | S02 | pipeline + live cursor | duplikat no-op, reconnect gap-free |
| [S04](sessions/S04.md) | Collector sistem | S03 | proses/tmux/git/docker/sistem | agent nyata, tanpa hang |
| [S05](sessions/S05.md) | Humanizer + task | S04 | kalimat + lifecycle 3 locale | golden corpus |
| [S06](sessions/S06.md) | API v1 + OpenAPI + auth | S05 | REST + klien TS | schema + authz |
| [S07](sessions/S07.md) | Design system + shell | S06 | token + komponen + shell | visual + axe |
| [S08](sessions/S08.md) | Today | S07 | home operasional | E2E + LCP |
| [S09](sessions/S09.md) | Collector runtime A | S04, S05 | OMP/Cursor/Muse | paritas subset |
| [S10](sessions/S10.md) | Collector runtime B | S09 | Claude/Codex/Gemini/OpenCode/Copilot/Devin/MCP | paritas + unconfigured jujur |
| [S11](sessions/S11.md) | Alert engine | S08 | rule + dedup + eskalasi | badai alert |
| [S12](sessions/S12.md) | Agent detail | S11 | 5 tab + kontrol | E2E |
| [S13](sessions/S13.md) | Office core | S07 | scene + kamera + render-on-demand | screenshot + budget |
| [S14](sessions/S14.md) | Office karakter | S13 | nav + pathfinding + animasi | 40 agent, 60 fps |
| [S15](sessions/S15.md) | Office interaktif + 2D | S14 | objek data + fallback | binding + fallback |
| [S16](sessions/S16.md) | Timeline, replay, knowledge | S06 | browser + as_of + hybrid search | as_of + eval |
| [S17](sessions/S17.md) | Connector framework + secret | S03, S06 | trait + YAML + secret | preset e2e + scan |
| [S18](sessions/S18.md) | GitHub | S17 | App/PAT + webhook | signature + live smoke |
| [S19](sessions/S19.md) | Telegram | S17 | alert + approval + command | <5 s + roundtrip |
| [S20](sessions/S20.md) | MCP client + server | S10, S17 | tools/resources + policy | roundtrip |
| [S21](sessions/S21.md) | OTLP + Prometheus | S03, S06 | ingest + metrics + health | scrape + trace |
| [S22](sessions/S22.md) | Preset batch 1 | S17, S19 | Slack/Discord/WhatsApp/email/ntfy/webhook | fake server per connector |
| [S23A](sessions/S23A.md) | Preset batch 2A | S17, S19 | Linear/Jira/Sentry/HA/Calendar/PagerDuty/Teams | manifest + test |
| [S23B](sessions/S23B.md) | Preset batch 2B | S17, S19 | Lark/Mattermost/Gotify/Pushover/Twilio/Resend/Vercel/Cloudflare/RSS/Docker | manifest + test |
| [S23C](sessions/S23C.md) | Notion + markets | S16, S17 | sync Notion, watchlist read-only | quarantine + tanpa nasihat |
| [S24A](sessions/S24A.md) | Control: policy + approval + audit | S11, S12, S17, S19 | policy engine, approval, audit API | deny default + replay |
| [S24B](sessions/S24B.md) | Control: executor + UI | S24A | shell/OMP/MCP/HTTP/queue, approvals UI | threat matrix + E2E |
| [S25](sessions/S25.md) | Migrasi + paritas + cutover | semua inti | data v2 pindah, switch | paritas + rollback |
| [S26](sessions/S26.md) | Hardening + rilis | S25 | perf/security/a11y/backup | suite penuh + v3.0.0 |

## 5. Gate global

Dijalankan di akhir setiap sesi yang menyentuh area terkait; wajib untuk S25.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
pnpm -C web check
pnpm -C web test
pnpm -C web build
pnpm -C web exec playwright test
```

`pnpm` dipasang lewat corepack di WSL pada S00; jangan memakai pnpm Windows.

## 6. Peta dokumen

| Dokumen | Isi |
|---|---|
| [00-design.md](00-design.md) | design lengkap yang disetujui |
| [01-architecture.md](01-architecture.md) | kontrak tipe, crate, skema DB, API |
| [02-runtime-adapters.md](02-runtime-adapters.md) | bukti per runtime, paritas, fixture |
| [03-connectors.md](03-connectors.md) | trait, engine deklaratif, katalog, secret |
| [04-control-plane.md](04-control-plane.md) | capability, policy, approval, executor, audit |
| [05-design-system.md](05-design-system.md) | token, tipe, komponen, motion, a11y, audit v2 |
| [06-frontend.md](06-frontend.md) | route, state, 3D, i18n, budget |
| [07-operations.md](07-operations.md) | deploy, backup, migrasi, observability, doctor |
| [08-testing-gates.md](08-testing-gates.md) | parity, golden, contract, e2e, perf, security |
| [09-roadmap.md](09-roadmap.md) | fase, milestone, cutover, risiko |
| [10-alerts.md](10-alerts.md) | rule, dedup, eskalasi, notification router |
