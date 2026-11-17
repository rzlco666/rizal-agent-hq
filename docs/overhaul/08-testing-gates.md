# 08 — Testing and Gates

Setiap sesi punya gate sendiri. Dokumen ini mendefinisikan harness yang dipakai
berulang, supaya sesi tidak mengarang alat baru.

## 1. Lapisan tes

| Lapis | Lokasi | Isi | Jaringan |
|---|---|---|---|
| Unit | `src` + `tests/` | invarian, parser, policy, redaksi, humanizer | tidak |
| Property | `tests/proptest` | event roundtrip, redaksi idempoten, policy deterministik | tidak |
| Store | `crates/hq-store/tests` | append+project atomic, replay, migrasi, backup | tidak |
| Parity | `tools/parity` | oracle v2 vs kandidat | ya (localhost) |
| Contract | `crates/hq-api/tests` | OpenAPI valid, klien TS compile, authz | tidak |
| Integration | `crates/hq-*/tests` | fake HTTP server (wiremock), webhook, OTLP | localhost |
| E2E | `web/tests/e2e` | Playwright terhadap binary nyata | localhost |
| Visual | `web/tests/visual` | screenshot light/dark/phone | tidak |
| A11y | `web/tests/a11y` | axe-core | tidak |
| Perf | `web/tests/perf`, `benches/` | budget API, bundle, 3D | tidak |
| Security | `cargo deny`, `hq doctor --scan-secrets` | advisories, secret, authz | tidak |

Aturan anti-flaky:

- Waktu di-inject (`Clock` trait) di unit test; tidak ada `sleep` > 50 ms.
- Tokio time dipause untuk tes retry/backoff.
- ULID dan ID di-inject lewat `IdGen` agar fixture deterministik.
- Setiap tes memakai temp dir sendiri; tidak ada state global.
- Tes integrasi tidak pernah memakai DB produksi atau port 8900.

## 2. Review Focus

Lima kelas input yang paling mungkin menggigit. Tesnya wajib ada di sesi yang
memiliki kode terkait.

| # | Kondisi | Perilaku yang diharapkan | Sesi | Nama tes |
|---|---|---|---|---|
| 1 | Sumber live hilang saat poll (proses mati, file terpotong) | snapshot selesai; agent `Offline`/`Error` dengan bukti terakhir; tidak menggantung | S04, S09, S10 | `collector_survives_disappearing_source` |
| 2 | Webhook duplikat atau event terlambat | tepat satu perubahan state; tanpa alert ganda; agregat jendela diperbarui | S03, S17, S18, S23 | `duplicate_webhook_is_noop`, `late_event_updates_bucket` |
| 3 | String mirip secret di output agent/log | teredaksi sebelum persist; tidak muncul di API/WS/FTS | S01, S05, S06 | `redacts_secret_corpus`, `api_never_returns_secret` |
| 4 | Crash/disk penuh antara append dan projection | restart pulih; tidak ada state separuh; event tetap ada | S02 | `append_and_project_are_atomic`, `restart_after_failed_commit` |
| 5 | Request remote lewat proxy salah konfigurasi | bukan owner; aksi kontrol 401/403; audit mencatat penolakan | S06, S24 | `forwarded_header_is_not_owner`, `control_requires_capability` |

## 3. Parity harness

Tujuan: membuktikan rewrite collector tidak menurunkan akurasi v2.

### 3.1 Freeze di S00

`tools/parity/freeze.sh` menjalankan:

1. Salin binary v2 yang sedang dipakai + unit systemd + versi.
2. Ambil 200 sampel `GET /api/state` setiap 6 detik (20 menit) ke
   `fixtures/state-<ts>.json`.
3. Pada 20 sampel pertama, simpan bukti mentah: `ps`, `omp stats`, listing file
   sesi, `tmux ls`, `docker ps`, `git log -1`.
4. Tulis `MANIFEST.sha256` untuk binary + semua fixture.
5. Tulis `fixtures/aliases.json` berisi alias v2 → canonical v3.

Gate S00: `tools/parity/verify.sh` mengembalikan 0 dan checksum cocok.

### 3.2 Uji yang mengikat

Oracle v2 tetap hidup di `127.0.0.1:8900`; kandidat di `127.0.0.1:8901`
dengan `HQ_DATA` terpisah. Sampling simultan 10 menit:

```bash
hq parity \
  --oracle http://127.0.0.1:8900 \
  --candidate http://127.0.0.1:8901 \
  --duration 600 \
  --out tools/parity/report-$(date +%s).md
```

Normalisasi sebelum membandingkan:

- agent dipetakan lewat `aliases.json`; id asing di kandidat dipertahankan.
- urutkan agent by id; buang field waktu tampilan.
- usage dibulatkan 3 significant figures.
- status dinormalisasi ke 6 nilai.
- alert dibandingkan berdasarkan `(rule_id, subject, level)`, bukan id acak.

Kriteria lulus:

| Kriteria | Ambang |
|---|---|
| Cakupan | setiap agent v2 punya padanan v3 (v2 ⊆ v3) |
| Status | cocok ≥ 90% sampel untuk agent berpasangan |
| Usage global | selisih ≤ 15% pada jendela waktu sama |
| Alert | fingerprint set sama untuk alert yang masih open |
| Tambahan | agent baru hanya jika ada bukti runtime (proses/file) |

Laporan memuat: tabel per agent (status match %, usage diff), daftar agent
tanpa padanan, alert beda, dan verdict `PASS`/`FAIL`. Fixture beku dipakai
untuk menguji normalizer dan humanizer; bukan pengganti uji live.

## 4. Golden corpus

| Corpus | Lokasi | Bentuk |
|---|---|---|
| Humanizer | `crates/hq-humanize/tests/golden/*.jsonl` | `{fact, sentence, rendered:{id,en,zh}}` |
| Redaksi | `crates/hq-core/tests/corpus/redaction/*.txt` | `input → expected`, termasuk false-positive git hash/UUID |
| Policy | `crates/hq-control/tests/corpus/policy/*.yaml` | policy + request + expected decision |
| Manifest | `crates/hq-connectors/tests/corpus/manifests/*.yaml` | valid/invalid + error code |
| Replay | `crates/hq-store/tests/corpus/events/*.jsonl` | events → snapshot insta |

Menambah kasus ke corpus lebih baik daripada menambah tes ad hoc. Corpus adalah
kontrak; perubahan output yang disengaja harus memperbarui corpus + changelog.

## 5. Fake server dan connector

- Gunakan `wiremock` untuk setiap connector yang punya HTTP.
- Setiap connector wajib punya tiga tes: sukses, auth gagal (401), dan payload
  rusak (400/timeout).
- Live smoke hanya berjalan bila env kredensial ada; tidak pernah menggagalkan
  CI tanpa kredensial:

```bash
HQ_LIVE_GITHUB=1 GITHUB_TOKEN=... cargo test -p hq-connectors -- --ignored github_live
```

- Webhook test memakai signature valid, signature salah, timestamp kedaluwarsa,
  dan replay nonce.

## 6. E2E

`hq serve --data <tmp> --seed <fixture> --allow-seed` menjalankan server dengan
event fixture deterministik. Flag seed hanya ada saat build debug atau
`HQ_ALLOW_SEED=1`; tidak pernah aktif di rilis.

Setiap alur E2E minimal:

1. Today menampilkan agent + attention rail dari event.
2. Buka detail agent, ganti tab, buka raw log.
3. Ack alert → badge turun tanpa reload.
4. Approve aksi → muncul di audit.
5. Office memuat scene, klik agent membuka detail, 2D fallback bekerja.
6. Integrations menampilkan connector `unconfigured` dengan jujur.
7. Remote tanpa token → 401; dengan token → 200.

## 7. Visual, a11y, perf

- Visual: screenshot 1440×900 light, 1440×900 dark, 390×844 light. Toleransi
  0,1% pixel; review manual untuk perubahan yang disengaja.
- a11y: axe-core 0 violation serious/critical; keyboard-only untuk Today,
  detail, approval; fokus terlihat; `prefers-reduced-motion`.
- Perf: LCP < 1,5 s lokal; bundle awal < 350 KB gzip; API p95 < 100 ms;
  append p95 < 5 ms; 3D ≥ 55 fps rata-rata @ 40 agent di mesin pengembang,
  dan render loop berhenti saat idle (dihitung lewat instrumentasi).

## 8. Security gate

```bash
cargo deny check
hq doctor --scan-secrets --data <tmp>
```

Tes wajib:

- Setiap endpoint kontrol tanpa capability → 403.
- `HQ_TRUST_LOOPBACK=0` + request loopback tanpa token → 401.
- `X-Forwarded-For` saat trust loopback → 401 (fail closed).
- Webhook signature salah/expired/replay → 401/409.
- Mutasi cookie tanpa CSRF → 403.
- Input berisi SQL/HTML/path traversal tidak mengubah query atau path.
- Secret corpus tidak ditemukan di DB dump, log, atau response.

## 9. CI

`.github/workflows/ci.yml`:

```yaml
jobs:
  rust:   fmt, clippy -D warnings, test, cargo deny
  web:    corepack pnpm install --frozen-lockfile, check, test, build
  e2e:    build binary, start with seed, playwright
```

Cache: `Swatinem/rust-cache`, pnpm store. Artefak: `openapi.json`, laporan
bundle, screenshot saat gagal, `tools/parity/report-*.md` bila dijalankan.

## 10. Template gate sesi

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
pnpm -C web check
pnpm -C web test
pnpm -C web build
```

Sesi menambahkan perintah spesifik di bawah ini. Gate tidak lulus bila ada
warning, tes di-skip, atau output `FAIL`.
