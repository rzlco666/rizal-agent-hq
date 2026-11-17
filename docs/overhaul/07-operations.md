# 07 — Operations

**Status:** kontrak, siap dieksekusi.
**Spec:** [`00-design.md`](00-design.md) §15, §9.5, §11.4, §16 · [`01-architecture.md`](01-architecture.md) §6, §7 · [`09-roadmap.md`](09-roadmap.md) §3
**Sesi pemilik:** S25 (migrasi, cutover), S26 (backup/restore, doctor, rilis). S02 memiliki `Store::backup`.

Dokumen ini mengikat: path, nama file, nama env, bentuk perintah, exit code,
dan urutan runbook. Perintah ditulis supaya bisa ditempel apa adanya; nilai di
dalam `<...>` adalah input operator, bukan placeholder dokumen.

---

## 1. Layout filesystem

### 1.1 Akar XDG

| Peran | Default | Override | Mode |
|---|---|---|---|
| Config | `${XDG_CONFIG_HOME:-$HOME/.config}/rizal-agent-hq` | `HQ_CONFIG` | `0700` |
| Data | `${XDG_DATA_HOME:-$HOME/.local/share}/rizal-agent-hq` | `HQ_DATA` | `0700` |
| State | `${XDG_STATE_HOME:-$HOME/.local/state}/rizal-agent-hq` | `HQ_STATE` | `0700` |
| Binary | `$HOME/.local/bin/rizal-agent-hq` + symlink `hq` | `--prefix` installer | `0755` |
| Unit | `${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/rizal-agent-hq.service` | — | `0644` |

Tidak ada direktori cache: aset UI di-embed, model diunduh sekali ke data dir.
Tidak ada direktori log: tracing ke journald (§8).

### 1.2 Isi config

| Path | Isi | Mode | Rahasia |
|---|---|---|---|
| `config.toml` | setelan operator: `[maintenance]`, `[notifications]`, `[knowledge]`, `repos = ["/path/to/repo"]` | `0644` | tidak |
| `policy.yaml` | policy capability (§04 §3) | `0644` | tidak |
| `automations.yaml` | rule otomasi + `notify` (S11) | `0644` | tidak |
| `shell.yaml` | allowlist shell (§04 §6.5) | `0644` | tidak |
| `connectors/<id>.yaml` | config/manifest preset connector; auth **wajib** `${secret:NAME}` | `0644` | tidak boleh literal |
| `secrets.age` | blob terenkripsi age (XChaCha20-Poly1305) | `0600` | ya |
| `master.key` | age identity (`AGE-SECRET-KEY-1` + 58 karakter bech32) | `0600` | ya |

`config.toml` tidak pernah memuat secret. Connector yang menuliskan token
literal gagal validasi saat start (`connector_config_invalid`), bukan diterima
diam-diam.

```toml
# config.toml — seluruh kunci opsional; default di bawah.
[maintenance]
retention_days = 90            # HQ_RETENTION_DAYS menang
archive_at = "03:30"           # WIB
backup_at = "03:15"            # WIB
vacuum_day = "sun"
vacuum_at = "04:00"
backup_daily_keep = 14
backup_weekly_keep = 8
wal_checkpoint_bytes = 33554432
min_free_bytes = 2147483648
min_free_percent = 10

[notifications]
channels = ["telegram"]
rate_limit_per_minute = 20
dedupe_window_s = 300

[knowledge]
embed = true                   # HQ_EMBED menang
```

### 1.3 Isi data

| Path | Isi | Mode |
|---|---|---|
| `hq.db` | SQLite utama (events + read model + audit), `PRAGMA user_version` | `0640` |
| `hq.db-wal`, `hq.db-shm` | WAL SQLite (dibuat SQLite) | `0640` |
| `models/<name>/` | model embedding ONNX + tokenizer | `0755` |
| `models/MANIFEST.sha256` | checksum semua file model | `0644` |
| `backups/daily/` | backup otomatis harian (`hq-YYYY-MM-DD.db`) | `0700` |
| `backups/weekly/` | backup Minggu (`hq-YYYY-Www.db`) | `0700` |
| `backups/pre-migrate-<from>-<ts>.db` | backup otomatis sebelum migrasi skema | `0600` |
| `legacy/reports/` | salinan `reports/` v2 saat migrasi layout | `0755` |
| `shadow/` | DB kandidat untuk `hq replay --rebuild` dan paritas | `0700` |

File `.db` selalu `0640` (bukan `0600`) karena backup dan `sqlite3` operator
berjalan sebagai user yang sama; `0700` pada direktori adalah batas kerasnya.

### 1.4 Isi state

| Path | Isi | Mode |
|---|---|---|
| `serve.pid` | pid + start time proses `hq serve`; dipakai `restore`/`replay` untuk menolak operasi saat server hidup | `0644` |
| `maintenance.lock` | lock `flock` untuk job kompaksi/backup/vacuum | `0600` |
| `last-backup.json` | `{"at_ms":1763372712000,"path":"backups/daily/hq-2026-11-16.db","sha256":"9f2c41d7b0a58c3e6d14f7a2b9c0d3e5f6081a2b3c4d5e6f708192a3b4c5d6e7","ok":true}` | `0644` |
| `last-vacuum.json` | `{"at_ms":1763370000000,"before_bytes":48234496,"after_bytes":41943040}` | `0644` |
| `last-archive.json` | `{"at_ms":1763371800000,"moved":1204,"oldest_remaining_ms":1760779800000}` | `0644` |
| `doctor.json` | hasil `hq doctor` terakhir (tanpa nilai secret) | `0644` |
| `migrate-<ts>.json` | laporan `hq migrate` | `0644` |
| `parity-<ts>.md` | hanya bila operator memakai `--out` di state dir | `0644` |

### 1.5 Env

Kontrak `01-architecture.md` §6 berlaku apa adanya: `HQ_PORT`, `HQ_BIND`,
`HQ_DATA`, `HQ_CONFIG`, `HQ_INTERVAL`, `HQ_TRUST_LOOPBACK`,
`HQ_RETENTION_DAYS`, `HQ_LOCALE`, `HQ_TZ`, `HQ_LOG`, `HQ_OTLP`,
`HQ_TRACE_SAMPLE`, `HQ_EMBED`.

Tambahan yang dibutuhkan dokumen ini — **wajib masuk `01-architecture.md` §6
pada sesi pertama yang memakainya**, bukan env liar:

| Env | Default | Arti |
|---|---|---|
| `HQ_STATE` | XDG state | direktori state (§1.4) |
| `HQ_MASTER_KEY_FILE` | `$HQ_CONFIG/master.key` | path age identity |
| `HQ_SECRETS_FILE` | `$HQ_CONFIG/secrets.age` | path blob secret |
| `HQ_ALLOW_SEED` | `0` | mengizinkan `--seed`/`dry_run` di build rilis (hanya CI) |

Aturan: env menang atas `config.toml`; secret tidak pernah lewat env kecuali
path file kunci untuk CI (`HQ_MASTER_KEY_FILE` menunjuk file sementara `0600`).

### 1.6 Migrasi dari `~/rizal-agent-hq`

Layout v2: binary `~/rizal-agent-hq/bin/`, data `~/rizal-agent-hq/data/`,
UI `~/rizal-agent-hq/web/`, secrets plaintext di drop-in unit. Langkah
(urutan ini bagian dari installer `install.sh`):

```bash
# 0. Hentikan v2 (paritas sudah PASS, 09 §3.2). Oracle beku tetap ada di tools/parity/oracle/.
systemctl --user stop rizal-agent-hq.service

# 1. Direktori XDG.
install -d -m 0700 "$HOME/.config/rizal-agent-hq" \
  "$HOME/.local/share/rizal-agent-hq" "$HOME/.local/state/rizal-agent-hq"
install -d -m 0755 "$HOME/.local/bin"

# 2. Backup v2 SEBELUM apa pun menyentuhnya (offline copy).
hq backup --from "$HOME/rizal-agent-hq/data/hq.db" \
  --out "$HOME/.local/state/rizal-agent-hq/v2-pre-cutover.db"

# 3. Secret dari drop-in plaintext ke store terenkripsi. Dilakukan SEBELUM
#    migrasi supaya connector yang butuh kredensial bisa dibuat.
hq secret import --from systemd --unit rizal-agent-hq \
  --keys EODHD_API_KEY,FMP_API_KEY,NOTION_API_KEY,TELEGRAM_BOT_TOKEN
hq secret list

# 4. Migrasi DB + file legacy ke DB v3 (sumber dibuka read-only).
hq migrate --from "$HOME/.local/state/rizal-agent-hq/v2-pre-cutover.db" \
  --to "$HOME/.local/share/rizal-agent-hq/hq.db" \
  --legacy-data "$HOME/rizal-agent-hq/data" \
  --config-out "$HOME/.config/rizal-agent-hq" \
  --report "$HOME/.local/state/rizal-agent-hq/migrate-$(date +%Y%m%dT%H%M%S).json"

# 5. Buang drop-in plaintext dan unit lama.
rm -f "$HOME/.config/systemd/user/rizal-agent-hq.service.d/override.conf"
rmdir "$HOME/.config/systemd/user/rizal-agent-hq.service.d" 2>/dev/null || true
```

Pemetaan sisa file:

| v2 | v3 | Tindakan |
|---|---|---|
| `data/hq.db` | `$HQ_DATA/hq.db` | `hq migrate` (baca-saja dari backup) |
| `data/heartbeats/*.json` | event `agent.seen` | `hq migrate` |
| `data/notion.json`, `data/stocks.json` | `$HQ_CONFIG/connectors/*.yaml` | `hq migrate --config-out` |
| `data/stocks_cache.json` | — | dibuang (cache harga) |
| `data/reports/` | `$HQ_DATA/legacy/reports/` | disalin apa adanya |
| `web/`, `web.bak-*` | — | dihapus setelah 7 hari tanpa regresi |
| `bin/rizal-agent-hq` (v2) | `tools/parity/oracle/` | dibekukan sebagai oracle S00 |
| `scripts/` | — | diganti subcommand binary |
| unit v2 | unit v3 (§2) | ditimpa |

Setelah 7 hari cutover stabil, `~/rizal-agent-hq` boleh dihapus:

```bash
rm -rf "$HOME/rizal-agent-hq/web" "$HOME/rizal-agent-hq/web.bak-"*
```

`~/rizal-agent-hq/data` **tidak** dihapus otomatis; ia satu-satunya salinan
data v2 dan dipertahankan sampai operator memutuskan lain.

---

## 2. systemd user unit

### 2.1 Unit v3 (teks persis)

`${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/rizal-agent-hq.service`:

```ini
[Unit]
Description=Rizal Agent HQ v3 — ruang kendali agent
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=300
StartLimitBurst=5

[Service]
Type=simple
WorkingDirectory=%h
ExecStart=%h/.local/bin/rizal-agent-hq serve
Restart=on-failure
RestartSec=5
TimeoutStopSec=20
KillSignal=SIGTERM
Environment=HQ_LOG=info
Environment=HQ_TZ=Asia/Jakarta
MemoryHigh=768M
MemoryMax=1G
TasksMax=1024
LimitNOFILE=8192
NoNewPrivileges=yes
ProtectSystem=full
RestrictSUIDSGID=yes
LockPersonality=yes
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rizal-agent-hq

[Install]
WantedBy=default.target
```

### 2.2 Alasan setiap pagar

| Direktif | Alasan |
|---|---|
| `Restart=on-failure` | crash nyata memicu restart; `systemctl stop` tidak memicu loop |
| `RestartSec=5` + `StartLimitBurst=5`/`300s` | maksimum 5 restart per 5 menit; setelah itu `failed` dan terlihat, bukan restart abadi |
| `MemoryHigh=768M` / `MemoryMax=1G` | batas v2 dipertahankan (insiden OOM 6 Nov: subprocess LLM dihitung di cgroup yang sama) |
| `TasksMax=1024` | 10+ agent + subprocess LLM + worker internal |
| `TimeoutStopSec=20` | `serve` menutup WS, checkpoint WAL, lalu keluar |
| `NoNewPrivileges=yes` | tidak ada eskalasi lewat setuid pada proses agent |
| `ProtectSystem=full` | `/usr`, `/boot`, `/etc` read-only; **`ProtectSystem=strict` tidak dipakai** karena akan membuat `$HOME` read-only dan mematikan agent yang menulis workspace |
| `SystemCallFilter` **tidak dipakai** | subprocess agent (omp, node, browser Cursor) memakai syscall di luar `@system-service`; filter akan mematikan agent, bukan menambah keamanan nyata |
| `PrivateTmp` **tidak dipakai** | agent berbagi `/tmp` dengan tooling lain; isolasi akan memutus koordinasi di luar HQ |
| `ProtectHome` **tidak dipakai** | collector membaca `~/.omp`, `~/.cursor`, `~/.claude`; menyembunyikan home = kehilangan bukti |
| `SyslogIdentifier` | `journalctl --user -u rizal-agent-hq` dan filter `SYSLOG_IDENTIFIER` konsisten |
| tidak ada `Environment=` secret | secret hanya dari `secrets.age`; unit tidak pernah memuat nilai |

### 2.3 Operasi unit

```bash
systemctl --user daemon-reload
systemctl --user enable --now rizal-agent-hq.service
loginctl enable-linger "$USER"
loginctl show-user "$USER" -p Linger
systemctl --user status rizal-agent-hq.service --no-pager
journalctl --user -u rizal-agent-hq.service -n 100 --no-pager
journalctl --user -u rizal-agent-hq.service -f -o cat
curl -fsS http://127.0.0.1:8900/api/v1/health
```

`enable-linger` wajib: tanpa itu service mati saat sesi WSL logout. Installer
menjalankannya dan **gagal** (exit 1) bila `Linger` masih `no`, karena service
yang hanya hidup selama sesi bukan instalasi.

Log JSON: setiap baris journald adalah satu objek (§8.1); `-o cat` menampilkan
JSON mentah, `journalctl --user -u rizal-agent-hq -o json | jq -r
'.MESSAGE | fromjson | .level + " " + .target'` untuk membaca ringkas.

Bila journald membuang burst (`Suppressed N messages`), naikkan batas system
journal (butuh root, satu kali):

```bash
sudo install -d -m 0755 /etc/systemd/journald.conf.d
printf '[Journal]\nRateLimitBurst=20000\nRateLimitIntervalSec=30s\n' \
  | sudo tee /etc/systemd/journald.conf.d/hq.conf
sudo systemctl restart systemd-journald
```

---

## 3. Secret

### 3.1 Bentuk store

- `master.key`: file teks berisi age identity (`AGE-SECRET-KEY-1` + 58 karakter
  bech32), `0600`,
  dibuat `hq secret rotate --new-key-out` atau otomatis saat pertama dipakai.
- `secrets.age`: satu blob age (XChaCha20-Poly1305) berisi JSON:

```json
{"version":1,"secrets":{"TELEGRAM_BOT_TOKEN":{"value":"contoh-bukan-secret","created_ms":1763372712000,"updated_ms":1763372712000}}}
```

- Nama secret: `^[A-Z][A-Z0-9_]{2,63}$`. Nilai: 1–4096 byte. Keduanya
  divalidasi sebelum enkripsi.
- Connector merujuk `${secret:TELEGRAM_BOT_TOKEN}`; nilai diselesaikan di
  memori saat request keluar. Tidak ada jalur yang menulis nilai ke
  `config.toml`, event, audit, log, atau pesan error.

### 3.2 Perintah

```bash
hq secret set TELEGRAM_BOT_TOKEN --stdin          # baca dari stdin, tanpa echo
hq secret list                                     # nama + updated_ms, tanpa nilai
hq secret rm FMP_API_KEY
hq secret rotate --new-key-out /tmp/hq-master-new.key
hq secret import --from systemd --unit rizal-agent-hq --keys A,B,C
```

`hq secret list --json`:

```json
{"schema":"hq.cli/1","command":"secret.list","ok":true,
 "names":[{"name":"TELEGRAM_BOT_TOKEN","updated_ms":1763372712000,"bytes":46}]}
```

`bytes` adalah panjang nilai; ia bukan kebocoran yang berguna tanpa nilai, dan
membantu mendeteksi secret terpotong saat impor.

### 3.3 Migrasi dari drop-in plaintext

```bash
systemctl --user show rizal-agent-hq -p Environment
```

Menampilkan baris `Environment=NAMA=nilai` dari unit + drop-in. `hq secret
import --from systemd` membaca keluaran yang sama, mengambil **hanya** kunci
yang disebut `--keys`, menulis ke `secrets.age`, dan mencetak nama + panjang
(tidak pernah nilai). Kunci yang tidak ada di `--keys` tidak diimpor; kunci
yang bernilai kosong dilewati dengan `warn`.

Verifikasi tidak ada plaintext tersisa:

```bash
hq secret list
hq doctor --scan-secrets --json
grep -rn "API_KEY\|BOT_TOKEN\|ntn_\|AGE-SECRET" \
  "$HOME/.config/systemd/user/" "$HOME/.config/rizal-agent-hq/" || true
systemctl --user show rizal-agent-hq -p Environment   # harus kosong dari secret
```

`hq secret rotate` menulis `master.key.bak-<ts>` (`0600`) di samping kunci
baru, dan menghapusnya otomatis setelah 7 hari pada maintenance harian.

---

## 4. CLI

Binary: `rizal-agent-hq` (nama tetap), dipanggil sebagai `hq` lewat symlink
yang dibuat installer. Subcommand: `serve`, `doctor`,
`migrate`, `replay`, `secret`, `backup`, `restore`, `parity`, `model`, `mcp`,
`device`, `openapi` (dua terakhir diwajibkan design: `hq device add`
`00-design.md` §9.5; `hq openapi --check` §17).

### 4.1 Flag global

```
hq [--json] [--data PATH] [--config PATH] [--state PATH] [--log FILTER] <command> [flag]
```

| Flag | Default | Arti |
|---|---|---|
| `--json` | mati | satu objek JSON di stdout; log tetap di stderr |
| `--data` | `$HQ_DATA` | override data dir |
| `--config` | `$HQ_CONFIG` | override config dir |
| `--state` | `$HQ_STATE` | override state dir |
| `--log` | `$HQ_LOG` | filter tracing (`info,hq_store=debug`) |

Flag global dideklarasikan `global = true` di clap, sehingga boleh ditulis
sebelum **atau** sesudah subcommand: `hq --json doctor` dan
`hq doctor --json` sama-sama sah, dan bentuk yang dipakai
`08-testing-gates.md` (`hq doctor --scan-secrets --data <tmp>`) tetap jalan.

### 4.2 `serve`

```
hq serve [--bind ADDR] [--port PORT] [--seed FILE] [--allow-seed]
```

| Flag | Default | Arti |
|---|---|---|
| `--bind` | `127.0.0.1` | alamat bind (`HQ_BIND`) |
| `--port` | `8900` | port (`HQ_PORT`) |
| `--seed` | — | memuat fixture event deterministik (E2E) |
| `--allow-seed` | mati | wajib bersama `--seed` di build rilis |

`--seed` hanya diterima bila build debug **atau** `HQ_ALLOW_SEED=1`; selain itu
exit 2 dengan pesan `seed tidak tersedia di build rilis`. Selama `--seed`,
store memakai DB sementara di `$HQ_DATA/e2e/hq.db` dan menghapusnya saat
shutdown.

### 4.3 `doctor`

```
hq doctor [--scan-secrets] [--deep]
```

| Flag | Arti |
|---|---|
| `--scan-secrets` | jalankan corpus scan atas DB, config, dan state |
| `--deep` | `PRAGMA integrity_check` penuh + verifikasi checksum model + verifikasi backup terbaru |

Exit 0 bila tidak ada `fail`; exit 3 bila ada `fail`. `warn` tidak mengubah
exit code. `--json` menulis `DoctorReport` (§11.1) dan tetap exit 3 bila gagal.

### 4.4 `migrate`

```
hq migrate --from PATH --to PATH [--legacy-data DIR] [--config-out DIR]
           [--dry-run] [--force] [--report PATH]
```

`--dry-run` menulis laporan ke stdout/`--report` tanpa menyentuh target.
`--force` memindahkan target yang sudah ada ke `<to>.bak-<ts>` sebelum menulis.
Detail §9.

### 4.5 `replay`

```
hq replay [--rebuild] [--shadow PATH] [--from-seq N] [--to-seq N]
          [--agent ID] [--until MS] [--dry-run]
```

| Flag | Arti |
|---|---|
| `--rebuild` | bangun ulang read model ke `--shadow` (default `$HQ_DATA/shadow/hq.db`), lalu swap saat server berhenti |
| `--from-seq`, `--to-seq` | batas replay inklusif |
| `--agent` | replay `GET /agents/:id?as_of` untuk satu subject |
| `--until` | replay sampai `event_ts_ms <= MS` |
| `--dry-run` | hitung jumlah event + hash hasil, tanpa menulis |

`--rebuild` menolak berjalan bila `serve.pid` hidup dan `--shadow` tidak
diberikan (exit 4). `--rebuild --shadow` boleh berjalan saat server hidup; swap
dilakukan setelah server berhenti (§4.14) dengan `mv` file bayangan ke
`hq.db`.

### 4.6 `secret`

Lihat §3.2.

### 4.7 `backup`

```
hq backup [--out PATH] [--from PATH] [--include-config] [--no-verify]
```

| Flag | Arti |
|---|---|
| `--out` | file `.db` atau direktori; direktori → nama otomatis `hq-<YYYY-MM-DDTHHMM>Z.db` |
| `--from` | DB sumber untuk backup offline (mis. `~/rizal-agent-hq/data/hq.db`) |
| `--include-config` | menyalin config dir kecuali `master.key` |
| `--no-verify` | lewati verifikasi pasca-backup (tidak dipakai di gate) |

Tanpa `--from`, backup berjalan online terhadap DB live. Detail §6.

### 4.8 `restore`

```
hq restore --from PATH [--to PATH] [--verify] [--force] [--config-from DIR]
```

Menolak berjalan bila `serve.pid` hidup (exit 4) kecuali `--force`. Detail §6.

### 4.9 `parity`

```
hq parity --oracle URL --candidate URL [--duration S] [--out FILE]
```

Default `--duration 600`. Exit 0 = `PASS`, exit 3 = `FAIL`. Format laporan
mengikuti `08-testing-gates.md` §3.2.

### 4.10 `model`

```
hq model list
hq model fetch [--name NAME] [--out DIR]
hq model verify
```

`fetch` mengunduh ke `$HQ_DATA/models/<name>/` dengan checksum yang sama
dengan `MANIFEST.sha256`; file sementara `.part` dihapus bila checksum gagal.
`verify` menghitung ulang sha256 dan exit 3 bila beda.

### 4.11 `mcp`

```
hq mcp serve [--transport stdio|http] [--bind ADDR] [--port PORT]
hq mcp list-tools [--server ID]
hq mcp call --server ID --tool NAME [--args JSON]
```

`hq mcp serve` adalah MCP server HQ (read-only default, `04-control-plane.md`
§10.2). `hq mcp call` memakai jalur `mcp.control` dengan aktor `owner` lokal:
policy tetap dievaluasi, dan approval interaktif ditampilkan sebagai prompt
`setujui? [y/N]` lalu dicatat `decided_by=owner`. Scope `mcp.control` (§04
§2.3) hanya relevan untuk token remote, bukan untuk proses CLI lokal.

### 4.12 `device`

```
hq device add --name NAME [--scope owner] [--scope mcp.control] [--expires-days 90]
hq device list
hq device revoke ID
```

`add` mencetak token **sekali** dan pairing code `HQ-XXXX-XXXX` (10 menit,
sekali pakai). Token hanya disimpan sebagai hash. `devices` perlu kolom
`expires_ms` (lihat catatan kontrak di §11.4).

### 4.13 `openapi`

```
hq openapi --check [--out PATH]
```

`--check` membandingkan OpenAPI yang dihasilkan dengan
`web/src/lib/api/schema.d.ts` hasil generate; beda → exit 3. `--out` menulis
`openapi.json` ke path.

### 4.14 Exit code

| Code | Arti | Contoh |
|---|---|---|
| 0 | sukses | `doctor` tanpa `fail`, `parity` PASS, `migrate` verifikasi cocok |
| 1 | error runtime | DB tidak bisa dibuka, secret gagal didekripsi, port gagal bind |
| 2 | usage error | flag tidak dikenal, `--seed` di build rilis, nilai di luar batas |
| 3 | verifikasi gagal | `parity` FAIL, checksum model beda, jumlah migrasi beda, `doctor --deep` gagal |
| 4 | prasyarat/konflik | `serve.pid` hidup saat `restore`/`replay --rebuild`, target migrasi sudah ada tanpa `--force`, `user_version` lebih baru dari binary |

Code 5 tidak ada. Semua kegagalan yang tidak terklasifikasi = 1 dan wajib
memuat `request_id` bila berasal dari request HTTP.

### 4.15 Format output

Human (default), ke stdout:

```
$ hq doctor
ok    db.integrity        quick_check ok
ok    db.user_version     9
warn  db.wal              WAL 41 MiB (ambang 32 MiB)
      → jalankan: systemctl --user restart rizal-agent-hq
ok    event.lag           lag 3 seq
fail  backup.age          backup terbaru 5 hari lalu (ambang 48 jam)
      → jalankan: hq backup --out ~/.local/share/rizal-agent-hq/backups/daily
ok    secret.decrypt      tersedia
```

`--json` menulis satu objek, skema versi tetap:

```json
{"schema":"hq.cli/1","command":"doctor","ok":false,"exit_code":3,
 "generated_ms":1763372712000,
 "checks":[{"id":"db.wal","status":"warn","value":"41943040",
            "remediation":"systemctl --user restart rizal-agent-hq"}]}
```

Aturan: `schema` = `hq.cli/1` untuk semua perintah; `command` = path
subcommand bertitik (`backup.create`, `secret.list`); `ok` konsisten dengan
`exit_code == 0`; log tracing selalu di stderr, tidak pernah mencampur stdout.

---

## 5. Instalasi, uninstall, upgrade, downgrade

### 5.1 Installer

Satu skrip di tarball: `install.sh`.

```
./install.sh [--prefix DIR] [--port PORT] [--no-service] [--uninstall [--purge]]
```

Langkah (berurutan, berhenti pada kegagalan pertama):

1. Preflight: `uname -s` = `Linux`, `systemctl --user` tersedia (kecuali
   `--no-service`), `$HOME` writable, dan port target bebas atau dipegang
   proses `rizal-agent-hq` (v2 dihentikan pada langkah 4, v3 yang sedang
   berjalan dihentikan sebelum unit ditimpa).
2. `install -m 0755 rizal-agent-hq "$PREFIX/bin/rizal-agent-hq"` dan
   `ln -sfn rizal-agent-hq "$PREFIX/bin/hq"` (semua perintah di dokumen ini
   memakai nama pendek `hq`).
3. `install -d -m 0700` untuk config/data/state; `0700` pada state.
4. Bila `~/rizal-agent-hq/data/hq.db` ada:
   - bila `tools/parity/report-<ts>.md` terbaru berstatus `PASS`, jalankan
     urutan §1.6 langkah 0–5 (cutover);
   - bila tidak ada laporan `PASS`, **berhenti sebelum langkah 5** dengan
     exit 4 dan pesan: jalankan `09-roadmap.md` §3.2 (kandidat 8901) lebih
     dulu. Ini mencegah installer menghapus oracle v2 di 8900 sebelum paritas
     lulus.
5. Render unit dari `share/systemd/rizal-agent-hq.service` ke
   `$( ${XDG_CONFIG_HOME:-$HOME/.config} )/systemd/user/rizal-agent-hq.service`:
   `%h` dibiarkan (diselesaikan systemd saat runtime), `ExecStart=%h/.local/bin/rizal-agent-hq`
   diganti `$PREFIX/bin/rizal-agent-hq` bila `--prefix` bukan
   `$HOME/.local`, dan bila `--port` bukan `8900` tambahkan baris
   `Environment=HQ_PORT=<port>` setelah `Environment=HQ_TZ=...`. Lalu
   `systemctl --user daemon-reload`.
6. `cp share/policy.example.yaml $HQ_CONFIG/policy.yaml` bila belum ada;
   idem `shell.example.yaml`, `config.example.toml`.
7. `systemctl --user enable --now rizal-agent-hq.service`,
   `loginctl enable-linger "$USER"`.
8. Tunggu maksimum 10 detik `curl -fsS http://127.0.0.1:<port>/api/v1/health`;
   gagal → cetak 20 baris journal terakhir dan exit 1.
9. Cetak URL, path data, dan perintah `hq doctor`.

`--port` hanya mengubah `HQ_PORT` di unit; port 8900 tetap default.

### 5.2 Uninstaller

```
./install.sh --uninstall [--purge]
```

1. `systemctl --user disable --now rizal-agent-hq.service`.
2. Hapus unit + `daemon-reload`. **Tidak** mematikan linger (mungkin dipakai
   unit user lain); cetak perintahnya.
3. Hapus `$PREFIX/bin/rizal-agent-hq` dan symlink `$PREFIX/bin/hq`.
4. Tanpa `--purge`: pertahankan config, data, state; cetak path-nya.
   Dengan `--purge`: hapus ketiga direktori **setelah** mencetak daftar
   backup yang ikut terhapus, dan hanya bila `--purge` disertai
   `HQ_CONFIRM_PURGE=1` (mencegah `--purge` tak sengaja).

### 5.3 Upgrade

1. `hq backup --out "$HQ_DATA/backups/pre-upgrade-$(date +%Y%m%d).db"`.
2. Ganti binary secara atomik:
   `install -m 0755 rizal-agent-hq-v3.x.y "$HOME/.local/bin/rizal-agent-hq"`.
3. `systemctl --user restart rizal-agent-hq.service`.
4. `hq doctor --deep`; exit 3 → rollback (§5.4).

Migrasi skema berjalan otomatis saat start, forward-only. Sebelum menerapkan
migrasi, `hq serve` **wajib**:

1. cek disk ≥ 2× ukuran `hq.db`; gagal → exit 1 (`insufficient_storage`),
2. `VACUUM INTO $HQ_DATA/backups/pre-migrate-<from_version>-<ts>.db`,
3. verifikasi `quick_check` backup,
4. baru menjalankan migrasi + `PRAGMA user_version = <target>`.

### 5.4 Downgrade

Tidak ada migrasi mundur. Bila `PRAGMA user_version` lebih besar dari versi
skema yang didukung binary:

```
hq serve gagal start (exit 1)
pesan: "DB user_version 12 > binary 9. Downgrade hanya lewat restore backup."
```

Prosedur:

```bash
systemctl --user stop rizal-agent-hq.service
mv "$HQ_DATA/hq.db" "$HQ_DATA/hq.db.v12"
hq restore --from "$HQ_DATA/backups/pre-upgrade-<ts>.db" --to "$HQ_DATA/hq.db" --verify
install -m 0755 rizal-agent-hq-<versi-lama> "$HOME/.local/bin/rizal-agent-hq"
systemctl --user start rizal-agent-hq.service
hq doctor
```

### 5.5 Port dan paritas

- Produksi/cutover: **8900** (`HQ_PORT` default).
- Kandidat selama paritas: **8901** dengan data terpisah:

```bash
HQ_PORT=8901 HQ_DATA="$HOME/.local/share/rizal-agent-hq/shadow" \
  "$HOME/.local/bin/rizal-agent-hq" serve
```

- Oracle v2 tetap di 8900 dan binary-nya beku di `tools/parity/oracle/`.
- `hq parity` tidak membuka port; ia hanya klien HTTP.
- Instalasi tidak pernah menjalankan dua `serve` pada `$HQ_DATA` yang sama:
  `serve.pid` + `flock` menolak instance kedua (exit 4).

---

## 6. Backup, restore, retensi

### 6.1 Backup

`hq backup` menghasilkan snapshot konsisten tanpa memblokir writer lama. Dua
jalur: `VACUUM INTO` untuk DB live, SQLite Online Backup API untuk `--from`
(DB offline yang mungkin masih punya isi WAL). Langkah:

1. Resolusi sumber: `--from` (DB offline, dibuka read-only, disalin lewat
   SQLite Online Backup API sehingga isi WAL ikut) atau DB live lewat store
   (`VACUUM INTO`).
2. Cek disk: butuh `free ≥ 2 × size(hq.db) + 64 MiB`; gagal → exit 1
   `insufficient_storage`.
3. Tulis tujuan: `VACUUM INTO '<dest>'` (live) atau backup API ke `<dest>`
   (`--from`).
4. Buka `<dest>` read-only: `PRAGMA quick_check`, `PRAGMA user_version`,
   `SELECT max(seq), count(*) FROM events`.
5. Hitung sha256, tulis `<dest>.sha256` (`<hex>  <basename>`) dan
   `<dest>.json`:

```json
{"schema":"hq.backup/1","created_ms":1763372712000,"seq":184203,
 "events":104231,"user_version":9,"bytes":48234496,
 "sha256":"9f2c41d7b0a58c3e6d14f7a2b9c0d3e5f6081a2b3c4d5e6f708192a3b4c5d6e7",
 "config_included":true,"secrets_included":true}
```

6. `--include-config` menyalin `config.toml`, `policy.yaml`,
   `automations.yaml`, `shell.yaml`, `connectors/`, dan `secrets.age` ke
   `<dest>.config/`. **`master.key` tidak pernah disalin.** Backup tanpa kunci
   tidak bisa membuka secret; itu disengaja.
7. Perbarui `$HQ_STATE/last-backup.json`.

### 6.2 Restore

```bash
systemctl --user stop rizal-agent-hq.service
hq restore --from "$HQ_DATA/backups/daily/hq-2026-11-16.db" \
  --to "$HQ_DATA/hq.db" --verify
systemctl --user start rizal-agent-hq.service
hq doctor
```

Langkah `hq restore`:

1. Tolak bila proses pada `serve.pid` masih hidup (exit 4) kecuali
   `--force`; pid file basi (proses sudah tidak ada) tidak menghalangi.
2. Verifikasi `<from>.sha256` + `<from>.json` (ukuran, sha256, `schema`).
3. `PRAGMA quick_check` pada backup; gagal → exit 3 tanpa menulis apa pun.
4. Salin ke `<to>.restore-tmp`, `fsync`, `rename` atomik.
5. Hapus `hq.db-wal`/`hq.db-shm` lama (mereka milik DB yang diganti).
6. Bila `--config-from <dir>`: salin file config kembali, **kecuali**
   `master.key` (kunci tidak pernah ditimpa oleh restore).
7. `--verify`: buka hasil, `quick_check`, bandingkan `max(seq)` + `user_version`
   dengan manifest; beda → exit 3.

### 6.3 Drill (wajib di gate S25/S26 dan sebelum rilis)

```bash
DRILL="$(mktemp -d)"
hq backup --out "$DRILL/backup.db" --include-config
hq restore --from "$DRILL/backup.db" --to "$DRILL/data/hq.db" --verify --json
hq doctor --data "$DRILL/data" --json
sqlite3 "$DRILL/data/hq.db" "SELECT (SELECT count(*) FROM events),(SELECT count(*) FROM audits);"
rm -rf "$DRILL"
```

Kriteria lulus: backup verify exit 0; restore verify exit 0; `doctor` tanpa
`fail`; jumlah `events` dan `audits` setelah restore sama dengan sumber pada
`seq` manifest. drill dicatat di `docs/overhaul/evidence/S25.md` (cutover) dan
`S26.md` (rilis).

### 6.4 Jadwal maintenance

Berjalan di dalam `hq serve` (tidak ada timer systemd tambahan). Waktu WIB,
diambil dari `config.toml [maintenance]`, dengan catch-up saat start bila
jadwal terlewat dan `last-*.json` lebih tua dari jadwal.

| Waktu | Job | Syarat | Efek |
|---|---|---|---|
| setiap 5 menit | checkpoint WAL | `size(hq.db-wal) > wal_checkpoint_bytes` | `PRAGMA wal_checkpoint(PASSIVE)` |
| 03:15 | backup harian | disk cukup | `backups/daily/hq-YYYY-MM-DD.db` + manifest; simpan 14 |
| 03:30 | arsip | ada event operasional `event_ts_ms < now - retention_days` | pindah ke `events_archive` dalam satu transaksi, lalu `wal_checkpoint(TRUNCATE)`; `last-archive.json` |
| 03:45 | rotasi backup | backup harian > 14, Minggu > 8 | hapus file + manifest terlama |
| Minggu 04:00 | `VACUUM` + `PRAGMA optimize` | disk ≥ 2× DB | `last-vacuum.json` |
| harian, setelah arsip | hapus `master.key.bak-*` > 7 hari | — | — |

Job memakai `flock` pada `$HQ_STATE/maintenance.lock`; job kedua yang
bertabrakan dilewati (`warn`), tidak diantrekan. Job yang gagal menulis
`warn` ke journal + alert `system` level `warning` (dedup 1/jam) dan dicoba
lagi pada jadwal berikutnya, bukan loop rapat.

Yang **tidak** diarsipkan: `audits`, event `action.*`, `alerts`, dan agregat
(`usage_minute`, `usage_daily`). Retensi hanya memindahkan event operasional.

### 6.5 Disk guard

Ambang: `min_free_bytes` (default 2 GiB) dan `min_free_percent` (default 10);
guard aktif bila salah satu dilanggar.

| Area | Perilaku saat guard aktif |
|---|---|
| `/health` | `status="degraded"`, `disk.guard=true` |
| alert | satu `system.health_observed` level `critical`, dedup 1/jam |
| `VACUUM`, arsip, backup harian | dilewati (kecuali backup ke path di luar filesystem penuh) |
| `POST /ingest/otlp/*` | `507 insufficient_storage` |
| `POST /knowledge/sync`, `POST /knowledge/promote` | `507 insufficient_storage` |
| `hq model fetch` | exit 1 sebelum mengunduh |
| `POST /actions` (shell/agent/connector) | tetap jalan — kontrol tidak boleh mati karena disk |
| `hq doctor` | `fail disk.free` + remediasi §7 |

---

## 7. Doctor

### 7.1 `/health` vs `/doctor`

| Aspek | `GET /api/v1/health` | `GET /api/v1/doctor` |
|---|---|---|
| Tujuan | liveness/readiness untuk installer, systemd, monitoring | diagnosis + remediasi untuk operator |
| Biaya | < 20 ms; tidak menyentuh disk besar, tidak mendekripsi ulang | hingga 2 s; `quick_check`, checksum model, verifikasi backup terbaru |
| Isi | status ringkas + angka | `checks[]` dengan `remediation` |
| Status HTTP | 200 `ok`/`degraded`, 503 `fail` | 200 tanpa `fail`, 503 ada `fail` |
| Auth | semua token valid | `owner` |
| Cek berat | tidak ada | semua kecuali `unit.*` dan `secret.scan` |

`/health` memakai hasil dekripsi secret yang di-cache saat start dan saat
`config.changed`; ia tidak mendekripsi tiap request. `checks_omitted` pada
`/doctor` selalu berisi `["unit.active","secret.scan"]` karena keduanya butuh
proses lokal (systemd user bus, corpus scan penuh). `hq doctor` CLI menjalankan
keduanya.

### 7.2 Daftar cek

| id | Cara | Lulus bila | Remediasi (teks yang dicetak) |
|---|---|---|---|
| `db.file` | `stat(hq.db)` | ada, `0640`, milik uid proses | `Pulihkan: hq restore --from $HQ_DATA/backups/daily/hq-YYYY-MM-DD.db --to $HQ_DATA/hq.db --verify` |
| `db.integrity` | `PRAGMA quick_check` (`--deep`: `integrity_check`) | `ok` | `Jalankan hq backup lalu hq replay --rebuild --shadow; bila tetap gagal, restore backup terbaru` |
| `db.user_version` | `PRAGMA user_version` | `≤` versi binary | `Binary lebih lama dari DB. Pasang rilis yang cocok atau restore backup (lihat 07 §5.4)` |
| `db.wal` | `size(hq.db-wal)` | `< 64 MiB` (`warn` ≥ 32 MiB) | `systemctl --user restart rizal-agent-hq` (checkpoint saat start) |
| `db.size` | `size(hq.db)` | informatif | — |
| `event.lag` | `max(seq) - projection_checkpoint('core')` | `≤ 1000` (`warn` > 1000, `fail` > 10000) | `Cek journal untuk projection error; jalankan hq replay --rebuild --shadow` |
| `event.rate` | jumlah event 24 jam / 1 hari | `≤ 20000` (`warn` di atas) | `Perketat dedupe/sampling: cek connector_runs dan telemetry (09 §4)` |
| `event.archive` | event operasional lebih tua dari retensi | 0 baris | `Job arsip terlewat. Jalankan systemctl --user restart rizal-agent-hq; cek journal "archive"` |
| `connector.health` | read model `connectors` | tidak ada `error` pada connector `enabled=1` | `Buka /integrations, jalankan Test pada connector yang error` |
| `connector.auth` | event `connector.auth_failed` 24 jam | 0 | `hq secret set <NAMA>; lalu POST /connectors/<id>/test` |
| `secret.file` | `stat` dua file | `0600`, milik uid proses | `chmod 600 $HQ_CONFIG/master.key $HQ_CONFIG/secrets.age` |
| `secret.decrypt` | dekripsi blob + parse JSON | berhasil, `version=1` | `Bila kunci hilang: hq secret rotate --new-key-out, lalu set ulang semua secret dari daftar hq secret list` |
| `secret.scan` | corpus scan DB/config/state | 0 temuan | `Rotasi secret yang bocor (hq secret rotate + hq secret set), purge event terkait, catat insiden` |
| `model.present` | `models/MANIFEST.sha256` + file | semua file ada | `hq model fetch` |
| `model.checksum` | sha256 (`--deep` atau `hq model verify`) | cocok manifest | `Hapus models/<name> lalu hq model fetch` |
| `port.bind` | coba bind `HQ_BIND:HQ_PORT` | bebas, atau dipegang pid `serve.pid` | `Hentikan proses lain: ss -ltnp 'sport = :8900'` |
| `disk.free` | `statvfs($HQ_DATA)` | `≥ min_free_bytes` dan `≥ min_free_percent` | `Hapus backup lama, pindahkan $HQ_DATA ke disk lain, atau jalankan hq backup ke path eksternal` |
| `backup.age` | `last-backup.json` | `≤ 48 jam` (`fail` > 72 jam) | `hq backup --out $HQ_DATA/backups/daily` |
| `backup.verify` | sha256 + quick_check backup terbaru | cocok | `Jalankan hq backup baru; backup lama rusak, jangan diandalkan` |
| `state.perms` | `stat` tiga direktori | `0700` (config/state), `0700` (data) | `chmod 700 $HQ_CONFIG $HQ_DATA $HQ_STATE` |
| `policy.valid` | parse `policy.yaml` | valid (`warn` bila `rules: []`) | `Perbaiki YAML mengikuti 04 §3.2; policy lama tetap dipakai sampai valid` |
| `shell.valid` | parse `shell.yaml` + `access(argv[0])` | valid, semua executable ada | `Perbaiki entry; hq doctor menampilkan nama entry yang gagal` |
| `approval.stale` | `approvals` state `pending` lebih tua dari TTL | 0 | `Sweeper tidak jalan. Restart service; laporkan bila berulang` |
| `queue.stale` | command `queued` > 15 menit | 0 | `Agent tidak mengambil command. Cek token agent + konektivitas` |
| `clock.skew` | `now - max(ingest_ts_ms)` | `|skew| < 60 s` | `Samakan jam WSL: sudo hwclock -s` |
| `unit.active` | `systemctl --user is-active rizal-agent-hq.service` | `active` (hanya CLI) | `systemctl --user start rizal-agent-hq.service` |
| `otel.export` | TCP connect ke endpoint OTLP bila diset | reachable (`warn` bila tidak) | `Cek OTEL_EXPORTER_OTLP_ENDPOINT atau hapus env untuk mematikan export` |

---

## 8. Observability

### 8.1 Tracing

Format: satu objek JSON per baris ke journald (stdout saat TTY memakai format
manusia; journald selalu JSON). Field wajib:

| Field | Tipe | Ada di span |
|---|---|---|
| `ts` | RFC3339 ms UTC | semua |
| `level` | `error`/`warn`/`info`/`debug`/`trace` | semua |
| `target` | modul Rust (`hq_api::actions`) | semua |
| `span` | nama span | semua |
| `request_id` | `req_<ulid>` | `request`, `action.execute` |
| `trace_id`, `span_id` | hex W3C | semua bila tracing aktif |
| `seq` | integer | `append`, `project`, `connector.webhook` |
| `subject` | string kanonik (`agent:omp`) | `append`, `project` |
| `capability`, `target` | string wire | `action.execute` |
| `action_id` | ULID | `action.execute` |
| `connector_id` | string | `connector.poll`, `connector.webhook` |
| `duration_ms` | integer | span yang selesai |
| `result` | `ok`/`error`/`denied`/`timeout` | span yang selesai |

Contoh:

```json
{"ts":"2026-11-16T15:12:31.412Z","level":"INFO","target":"hq_api::actions","span":"action.execute","request_id":"req_01JB9K2N7Q","trace_id":"4bf92f3577b34da6a3ce929d0e0e4736","span_id":"00f067aa0ba902b7","action_id":"01JB9K2N7Q4X8T3R5V6W7Y8Z9A","capability":"agent.prompt","target":"agent:omp","duration_ms":412,"result":"ok"}
```

Yang **tidak** pernah masuk tracing: nilai secret, `input` mentah, `output`
executor, header `Authorization`, cookie, body webhook. Output eksekusi hanya
hidup di event (setelah redaksi), bukan di log.

### 8.2 Metrik

`GET /metrics` (Prometheus text). Delapan metrik dari
`01-architecture.md` §7 adalah kontrak:

| Metrik | Tipe | Label |
|---|---|---|
| `hq_events_total` | counter | `kind` |
| `hq_append_ms` | histogram | — (bucket ms: 1,2,5,10,25,50,100,250,500,1000) |
| `hq_projection_lag_seq` | gauge | — |
| `hq_connector_up` | gauge | `id` |
| `hq_connector_poll_ms` | histogram | `id` (bucket: 50,100,250,500,1000,2500,5000,10000) |
| `hq_ws_clients` | gauge | — |
| `hq_alerts_open` | gauge | `level` |
| `hq_actions_total` | counter | `result` (`executed`,`failed`,`denied`,`pending`) |

Tambahan yang dibutuhkan halaman self-health dan doctor — wajib ditambahkan
ke `01-architecture.md` §7, bukan metrik liar:

| Metrik | Tipe | Label |
|---|---|---|
| `hq_approval_pending` | gauge | — |
| `hq_db_size_bytes` | gauge | — |
| `hq_wal_size_bytes` | gauge | — |
| `hq_disk_free_bytes` | gauge | — |
| `hq_backup_age_seconds` | gauge | — |
| `hq_otlp_datapoints_total` | counter | `signal`, `result` |

Kardinalitas: label hanya id yang sudah ada di read model (`connector.id`,
`kind` enum, `level` 3 nilai). Tidak ada label `agent_id` pada metrik (bisa
ratusan); per-agent ada di API/`usage`, bukan di Prometheus.

### 8.3 OTLP

**Terima** (mati kecuali `HQ_OTLP=1`):

```
POST /api/v1/ingest/otlp/v1/metrics
POST /api/v1/ingest/otlp/v1/traces
POST /api/v1/ingest/otlp/v1/logs
```

- Body: protobuf atau JSON (`Content-Type: application/x-protobuf` atau
  `application/json`), `Content-Encoding: gzip` didukung.
- Batas: 4 MiB per request, 1000 datapoint/span/log per request; lebih →
  `413`, dicatat `hq_otlp_datapoints_total{result="rejected"}`.
- Trace: sampling `HQ_TRACE_SAMPLE` (default 0,1) sebelum persist.
- Log: pesan diredaksi lalu dipotong 512 byte; sampling 10%; maksimum 100
  event `telemetry.log` per menit; kelebihan dihitung `result="dropped"`.
- Metrik: diagregasi per menit per `(service, metric)` menjadi satu event
  `telemetry.metric`; tidak ada penyimpanan mentah per datapoint.
- Autentikasi: `owner` atau token `read`; tanpa auth hanya saat trust loopback.

**Ekspor** (opsional, mati default): memakai env standar OpenTelemetry, bukan
env `HQ_*` baru:

| Env | Arti |
|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | basis URL; bila kosong, ekspor mati |
| `OTEL_EXPORTER_OTLP_HEADERS` | header auth (`k=v,k2=v2`) |
| `OTEL_SERVICE_NAME` | default `rizal-agent-hq` |
| `OTEL_SDK_DISABLED` | `true` mematikan ekspor walau endpoint diset |

Batch 5 detik, timeout 10 detik, retry 3× dengan backoff; kegagalan hanya
`warn` (maksimum 1 per menit) dan **tidak pernah** memblokir request. Yang
diekspor hanya span metadata §8.1 — tanpa payload event, tanpa secret.

### 8.4 Level log

`HQ_LOG` menerima `error|warn|info|debug|trace` dan filter per-target
(`info,hq_store=debug,hq_connectors::github=trace`). Nilai tidak valid → satu
`warn` saat start, lalu `info`. Default `info`; `debug` hanya untuk investigasi
dan tidak boleh aktif di unit produksi (gate S26 memeriksa unit).

### 8.5 Halaman self-health

Route `/settings`, panel "Kesehatan" (design §14: settings memuat retensi,
tema, bahasa, backup, device). Isi:

- status keseluruhan (`ok`/`degraded`/`fail`) + versi + uptime;
- DB: ukuran, WAL, `user_version`, lag projection, jumlah event hari ini;
- connector: `up`/`down`/`unconfigured` (angka, klik → `/integrations`);
- secret: nama yang terdaftar, nilai tidak pernah dirender;
- model: ada/checksum cocok;
- disk: bebas (byte + persen) dan status guard;
- backup: umur backup terbaru + hasil verifikasi manifest; panel hanya
  menampilkan, backup dijalankan oleh jadwal internal atau operator lewat CLI;
- approval pending + tautan `/automations`.

Data: `GET /health` (poll 10 detik hanya saat tab terlihat) dan `GET /doctor`
saat panel dibuka. Tidak ada aksi destruktif dari panel ini.

---

## 9. Migrasi data v2 (CLI)

### 9.1 Input dan output

Input:

- `--from <v2 hq.db>`: dibuka dengan URI read-only (`mode=ro`, tanpa
  `immutable=1` supaya isi WAL v2 ikut terbaca); tidak pernah ditulis.
- `--legacy-data <dir>`: untuk `heartbeats/`, `notion.json`, `stocks.json`,
  `reports/`.
- `--config-out <dir>`: menulis connector config hasil migrasi.

Output:

- `--to <v3 hq.db>`: dibuat baru; bila ada dan tanpa `--force` → exit 4.
- `--report <file>`: JSON `hq.migrate/1` (juga stdout dengan `--json`).
- `--config-out`: `connectors/notion-<source_id>.yaml` (3 file), satu
  `connectors/eodhd.yaml` bila `EODHD_API_KEY` ada di secret store atau
  `connectors/fmp.yaml` bila hanya `FMP_API_KEY` yang ada (v2 tidak menyimpan
  provider di `stocks.json`; ia memilih dari env), dan
  `connectors/telegram.yaml` hanya bila `TELEGRAM_BOT_TOKEN` sudah ada. Yang
  tidak punya kredensial tidak dibuat — tidak ada connector `unconfigured`
  karangan.

### 9.2 Pemetaan

| v2 | v3 | Event / tabel | Verifikasi |
|---|---|---|---|
| `snapshots` 33.436 | event `usage.recorded`, subject `usage:global`, `source='legacy'`, `attribution='global'` | `events` | jumlah event = 33.436 |
| `notion_docs` 220 | `knowledge_pages` + `knowledge_chunks` + event `knowledge.page_synced` | `knowledge_pages`, `events` | 220 halaman; 220 event; `status` = `sync_status` v2 (`synced`/`quarantined`), `content_hash` 20 sampel sama |
| `notion_docs.sync_status='quarantined'` | event `knowledge.page_quarantined` | `events` | jumlah = jumlah baris quarantined |
| `notion_sources` 3 | `connectors/notion-<id>.yaml` + baris `connectors` | config | 3 file, `root_page_id` sama |
| `llm_summaries` 102 | tabel `summaries` | `summaries` | 102 baris; hash kunci sama |
| `alert_log` | event `alert.raised` dengan `fingerprint = legacy:<id>`, payload `legacy_id` | `events`, `alerts` | jumlah sama; level + subject sama; bukan fingerprint v3 |
| `alert_state` 5 | `alert_decisions` + event `alert.acknowledged`/`alert.snoozed`/`alert.resolved`, subject `alert:legacy:<id>` | `alert_decisions`, `events` | 5 baris; `legacy_id` + status sama; tidak ada alert v2 yang dibawa sebagai open |
| `heartbeats/*.json` | event `agent.observed` + `agent.seen`, subject `agent:<slug>` (slug = nama file tanpa `.json`) | `events` | jumlah = jumlah file; `event_ts_ms = at` |
| `stocks.json`, `notion.json` | connector config | config | simbol + `root_page_id` sama |
| `portfolio_snapshots` 0 | tidak dimigrasikan | — | 0 |
| `memory_entries`, `memory_events` | **tidak dimigrasikan** | — | dicatat sebagai `warning` di laporan |

Payload heartbeat legacy:

```json
{"display_name":"Muse","role":"asisten","legacy_status":"bekerja",
 "task":"menunggu jadwal","evidence":"L1","legacy_row":true}
```

`role`/`task` diredaksi ulang oleh v3 sebelum append (v2 sudah meredaksi, v3
tidak mempercayainya). Status dipetakan persis: `bekerja`→`Bekerja`, `berpikir`→`Berpikir`,
`menunggu-rizal`/`menunggu rizal`→`MenungguRizal`, `santai`→`Santai`,
`offline`→`Offline`, `error`→`Error`; nilai lain → `Santai` dengan
`legacy_status` tetap disimpan untuk audit.

Dua rekonsiliasi nama dari `00-design.md` §16 ke kontrak yang mengikat:

1. Subject usage legacy: design menulis `system:usage-total`; bentuk kanonik
   `01-architecture.md` §2 mengikat `usage:global`. Migrasi memakai
   `usage:global`; alias `system:usage-total → usage:global` ditambahkan ke
   `tools/parity/fixtures/aliases.json` supaya normalizer paritas tetap bisa
   membandingkan.
2. `alert.decision` dan `agent.heartbeat` di design adalah nama lama; enum yang
   mengikat ada di `01-architecture.md` §2 (`alert.acknowledged`,
   `alert.snoozed`, `alert.resolved`, `agent.observed`, `agent.seen`).

Payload sintetis `usage.recorded` (JSON kanonik: kunci terurut, tanpa spasi,
float dibulatkan 6 desimal):

```json
{"active_agents":12,"at_ms":1763370000000,"attribution":"global",
 "cache_tokens":123,"cash_usd":1.234567,"input_tokens":1000,
 "legacy_row":true,"output_tokens":500,"requests":10,"shadow_usd":2.345678}
```

`event_ts_ms = snapshots.at` (sudah epoch ms), `ingest_ts_ms` = waktu migrasi,
`idempotency_key = "legacy:snapshot:<at>"`, `schema_version = 1`.

### 9.3 Verifikasi

Dijalankan otomatis di akhir `hq migrate` dan ulang oleh operator:

```bash
SRC="$HOME/.local/state/rizal-agent-hq/v2-pre-cutover.db"
DST="$HOME/.local/share/rizal-agent-hq/hq.db"

sqlite3 -readonly "$SRC" "SELECT 'snapshots',count(*) FROM snapshots
 UNION ALL SELECT 'notion_docs',count(*) FROM notion_docs
 UNION ALL SELECT 'notion_sources',count(*) FROM notion_sources
 UNION ALL SELECT 'llm_summaries',count(*) FROM llm_summaries
 UNION ALL SELECT 'alert_state',count(*) FROM alert_state
 UNION ALL SELECT 'alert_log',count(*) FROM alert_log
 UNION ALL SELECT 'portfolio_snapshots',count(*) FROM portfolio_snapshots;"

sqlite3 -readonly "$DST" "SELECT 'usage_events',count(*) FROM events WHERE source='legacy' AND kind='usage.recorded'
 UNION ALL SELECT 'knowledge_pages',count(*) FROM knowledge_pages
 UNION ALL SELECT 'summaries',count(*) FROM summaries
 UNION ALL SELECT 'alert_decisions',count(*) FROM alert_decisions
 UNION ALL SELECT 'legacy_agent_events',count(*) FROM events WHERE source='legacy' AND kind IN ('agent.observed','agent.seen');"
```

Angka yang harus cocok:

| Nama | Nilai |
|---|---|
| snapshot usage | 33.436 |
| dokumen Notion | 220 |
| sumber Notion | 3 |
| ringkasan LLM | 102 |
| alert state | 5 |
| portfolio snapshot | 0 (tidak ada target) |

Bila salah satu beda → exit 3, laporan memuat `expected`, `actual`, dan
`query` yang dipakai; target tetap ada untuk diperiksa (tidak dihapus
otomatis).

Sampel hash: 20 snapshot, offset `floor(i*(n-1)/19)` untuk `i=0..19` pada
`ORDER BY at ASC`; hash = sha256 atas payload JSON kanonik §9.2. Hash yang
sama harus muncul pada dua kali migrasi dari sumber yang sama.

### 9.4 Dry-run

`hq migrate --dry-run` membuka sumber read-only, menghitung semua angka §9.3,
membangun payload di memori untuk 20 sampel hash, menulis laporan, dan
**tidak** membuat/mengubah `--to`, `--config-out`, atau state. Exit 0 bila
semua angka sumber terbaca; exit 3 bila tabel wajib tidak ada (mis. DB v2
tidak lengkap) dan laporan menyebut tabel yang hilang.

### 9.5 Idempotensi

- Setiap event legacy memakai `idempotency_key` deterministik
  (`legacy:snapshot:<at>`, `legacy:notion:<page_id>`,
  `legacy:summary:<task_id>`, `legacy:heartbeat:<slug>:<last_seen_ms>`).
- Menjalankan ulang migrasi ke DB yang sama = no-op tercatat:
  `connector_runs`-style counter di laporan (`inserted`, `skipped`), exit 0.
- Menjalankan ulang ke DB baru = hasil identik (jumlah + 20 hash sama).
- `--force` pada target yang ada hanya memindahkan file lama ke
  `<to>.bak-<ts>`; ia tidak menggabungkan dua DB.
- `--config-out` idempoten: file connector yang sudah ada tidak ditimpa
  (dihitung `skipped_config`), sehingga `hq migrate` boleh dijalankan ulang
  setelah secret baru ditambahkan untuk membuat connector yang sebelumnya
  tidak bisa dibuat.

### 9.6 Rollback

- Sumber tidak pernah ditulis, jadi rollback tidak butuh "un-migrate".
- Rollback cutover = stop v3, kembalikan unit + binary v2, start v2 dengan DB
  v2 asli (`09-roadmap.md` §3.4).
- Bila migrasi gagal di tengah (crash), target parsial dihapus dan dijalankan
  ulang; karena target adalah file baru, tidak ada state setengah yang
  tersaji ke UI.
- `hq migrate` tidak pernah menyentuh `$HQ_DATA/hq.db` yang sedang dipakai
  `serve` (ditolak exit 4 bila `--to` = DB live dan `serve.pid` hidup).

---

## 10. Proses rilis

### 10.1 Build

```bash
corepack enable
pnpm -C web install --frozen-lockfile
pnpm -C web build
cargo build --release --locked -p hq-server
strip target/release/rizal-agent-hq
```

Build frontend lebih dulu karena `rust-embed` mengambil `web/build/` saat
kompilasi; `cargo build` tanpa langkah itu gagal dengan pesan
`web/build tidak ditemukan`.

### 10.2 Tarball

Nama: `rizal-agent-hq-v3.0.0-x86_64-unknown-linux-gnu.tar.gz`

```
rizal-agent-hq-v3.0.0-x86_64-unknown-linux-gnu/
  rizal-agent-hq                      # binary, 0755
  install.sh                          # installer/uninstaller (§5)
  README.md
  CHANGELOG.md
  LICENSE
  share/systemd/rizal-agent-hq.service
  share/policy.example.yaml
  share/shell.example.yaml
  share/config.example.toml
```

```bash
STAGE="rizal-agent-hq-v3.0.0-x86_64-unknown-linux-gnu"
rm -rf "dist/$STAGE" && mkdir -p "dist/$STAGE/share/systemd"
install -m 0755 target/release/rizal-agent-hq "dist/$STAGE/rizal-agent-hq"
install -m 0755 scripts/install.sh "dist/$STAGE/install.sh"
install -m 0644 README.md CHANGELOG.md LICENSE "dist/$STAGE/"
install -m 0644 systemd/rizal-agent-hq.service "dist/$STAGE/share/systemd/"
install -m 0644 deploy/policy.example.yaml \
  deploy/shell.example.yaml deploy/config.example.toml \
  "dist/$STAGE/share/"
tar -C dist -czf "dist/$STAGE.tar.gz" "$STAGE"
```

Tidak ada aset UI, font, ikon, atau model di tarball: UI di-embed di binary;
model diunduh lewat `hq model fetch`.

### 10.3 Checksum dan release

```bash
cd dist
sha256sum rizal-agent-hq-v3.0.0-x86_64-unknown-linux-gnu.tar.gz \
  > SHA256SUMS
gpg --armor --detach-sign --output SHA256SUMS.asc SHA256SUMS   # opsional
gh release create v3.0.0 \
  --title "Rizal Agent HQ v3.0.0" \
  --notes-file "notes/v3.0.0.md" \
  rizal-agent-hq-v3.0.0-x86_64-unknown-linux-gnu.tar.gz \
  SHA256SUMS SHA256SUMS.asc
```

Verifikasi unduhan:

```bash
sha256sum -c SHA256SUMS
```

Tanda tangan asimetris tidak wajib untuk rilis personal; bila dipakai,
`SHA256SUMS.asc` ikut diunggah dan dicantumkan di release notes.

### 10.4 CHANGELOG

Format Keep a Changelog; versi teratas harus sama dengan tag. Entri wajib
memuat: perubahan kontrak (event, capability, API), migrasi data, dan batas
yang dihapus. Contoh kerangka:

```markdown
## [3.0.0] - <tanggal rilis>

### Added
- Event log append-only + projection; API `/api/v1`; control plane capability,
  policy, approval, audit.

### Changed
- Binary tunggal dengan subcommand `serve|doctor|migrate|replay|secret|backup|restore|parity|model|mcp|device|openapi`.
- Data pindah ke XDG; secret pindah ke `secrets.age`.

### Removed
- API v2, UI `web/` lama, collector snapshot 6 detik, unit report harian.
```

Gate rilis (`09-roadmap.md` §6) dijalankan sebelum tag; tarball tanpa
`docs/overhaul/evidence/S26.md` tidak dirilis.

---

## 11. Interfaces dan tes

### 11.1 Tipe yang diproduksi S25/S26

```rust
// crates/hq-store/src/backup.rs
pub struct BackupOptions {
    pub db: PathBuf,
    pub out: PathBuf,
    pub include_config: Option<PathBuf>,
    pub verify: bool,
}
pub struct BackupManifest {
    pub schema: &'static str, // "hq.backup/1"
    pub created_ms: Ms,
    pub seq: i64,
    pub events: i64,
    pub user_version: u32,
    pub bytes: u64,
    pub sha256: String,
    pub config_included: bool,
}
pub async fn backup(opts: &BackupOptions) -> Result<BackupManifest, BackupError>;
pub async fn verify(path: &Path) -> Result<BackupManifest, BackupError>;

pub struct RestoreReport { pub seq: i64, pub events: i64, pub user_version: u32, pub verified: bool }
pub struct RestoreOptions {
    pub from: PathBuf,
    pub to: PathBuf,
    pub verify: bool,
    pub config_from: Option<PathBuf>,
    pub force: bool,
}
pub async fn restore(opts: &RestoreOptions) -> Result<RestoreReport, BackupError>;
```

```rust
// crates/hq-observe/src/doctor.rs
pub enum Severity { Ok, Warn, Fail }
pub struct Check { pub id: &'static str, pub status: Severity, pub value: String, pub remediation: Option<String> }
pub struct DoctorReport { pub ok: bool, pub worst: Severity, pub checks: Vec<Check>, pub omitted: Vec<&'static str>, pub generated_ms: Ms }
pub async fn run(ctx: &DoctorCtx<'_>) -> DoctorReport;
```

```rust
// crates/hq-store/src/legacy/mod.rs
pub struct MigrateOptions {
    pub from: PathBuf,
    pub to: PathBuf,
    pub legacy_data: Option<PathBuf>,
    pub config_out: Option<PathBuf>,
    pub dry_run: bool,
    pub force: bool,
    pub report: Option<PathBuf>,
}
pub struct CountPair { pub expected: Option<i64>, pub actual: i64 }
pub struct MigrateReport {
    pub schema: &'static str, // "hq.migrate/1"
    pub counts: BTreeMap<String, CountPair>,
    pub sample_hashes: Vec<String>,
    pub inserted: u64,
    pub skipped: u64,
    pub skipped_config: Vec<String>,
    pub warnings: Vec<String>,
    pub ok: bool,
}
pub fn migrate(opts: &MigrateOptions) -> Result<MigrateReport, MigrateError>;
```

```rust
// crates/hq-server/src/cli/parity.rs — tooling, bukan domain
pub struct ParityOptions { pub oracle: Url, pub candidate: Url, pub duration_s: u64 }
pub struct ParityReport { pub pass: bool, pub per_agent: Vec<ParityRow>, pub usage_delta_pct: f64, pub alerts_equal: bool }
pub async fn run(opts: &ParityOptions) -> Result<ParityReport, ParityError>;
```

### 11.2 Nama tes S25

| Tes | Assertion |
|---|---|
| `migrate_legacy_counts_match_expected` | 33.436 / 220 / 3 / 102 / 5 seperti §9.3 |
| `migrate_sample_hashes_stable` | dua kali migrasi → 20 hash identik |
| `migrate_dry_run_writes_nothing` | tidak ada file di `--to`, `--config-out`, state |
| `migrate_source_is_read_only` | URI `mode=ro`; menulis ke sumber gagal |
| `migrate_is_idempotent` | run kedua: `inserted=0`, `skipped` = jumlah event legacy |
| `migrate_refuses_existing_target` | exit 4 tanpa `--force` |
| `migrate_alert_state_maps_to_decisions` | 5 baris `alert_decisions`, `legacy_id` + status sama |
| `migrate_heartbeats_become_agent_seen` | jumlah event = jumlah file |
| `migrate_usage_total_matches_global` | total `requests`/`cash_usd` sama dalam 0 (integer) dan 1e-6 (float) |
| `parity_report_passes_on_fixture` | fixture beku → PASS |
| `parity_fails_on_status_regression` | 1 agent dipaksa beda → FAIL + exit 3 |
| `cutover_rollback_restores_v2_db` | drill restore + start v2 (scripted) |

### 11.3 Nama tes S26

| Tes | Assertion |
|---|---|
| `backup_vacuum_into_is_consistent` | manifest `seq` = `max(seq)` DB sumber saat backup |
| `backup_manifest_checksum_verifies` | ubah 1 byte → `verify` exit 3 |
| `backup_excludes_master_key` | `--include-config` tidak menyalin `master.key` |
| `restore_roundtrip_preserves_counts` | `events` + `audits` sama setelah restore |
| `restore_refuses_when_service_running` | `serve.pid` hidup → exit 4 |
| `restore_atomic_rename` | crash simulasi sebelum rename → DB lama utuh |
| `doctor_detects_wal_bloat` | WAL 40 MiB → `warn db.wal` |
| `doctor_fails_on_expired_backup` | `last-backup.json` 5 hari → `fail backup.age` |
| `doctor_reports_remediation_text` | setiap `fail` punya `remediation` tidak kosong |
| `health_never_decrypts_per_request` | 100 request `/health` → 0 dekripsi (counter) |
| `metrics_exposition_has_required_names` | 8 metrik kontrak ada di `/metrics` |
| `otlp_ingest_rejects_oversize` | body 5 MiB → `413` |
| `maintenance_archives_old_events_only` | event 100 hari pindah; `action.*`/`audits` tetap |
| `archive_delete_requires_allowlist` | `DELETE FROM events` biasa → SQLite abort |
| `wal_checkpoint_runs_at_threshold` | WAL > ambang → checkpoint, ukuran turun |
| `disk_guard_blocks_heavy_writes` | free < ambang → OTLP `507`, shell tetap jalan |
| `log_contains_no_secret_corpus` | corpus secret → tidak ada di journal fixture |
| `release_tarball_contains_expected_paths` | 9 path §10.2 ada, tidak ada `web/` |
| `unit_memory_max_is_1g` | unit di tarball memuat `MemoryMax=1G` |
| `installer_fails_without_linger` | `Linger=no` → exit 1 dengan pesan |

### 11.4 Catatan kontrak yang harus ditutup sebelum sesi terkait

- `devices` di `01-architecture.md` §3 tidak punya kolom expiry; design §9.5
  menuntut expiry. Tambahkan `expires_ms INTEGER` (nullable = tanpa expiry)
  lewat migrasi yang sama dengan tabel `devices`, dan perbarui 01 §3.
- Tabel `summaries` (dipakai migrasi 102 ringkasan, `00-design.md` §16) belum
  ada di daftar tabel 01 §3. Tambahkan
  `summaries(task_id TEXT PRIMARY KEY, hash INTEGER NOT NULL, summary TEXT NOT NULL, at_ms INTEGER NOT NULL)`
  lewat migrasi S24/S25, dan perbarui 01 §3.
- Env `HQ_STATE`, `HQ_MASTER_KEY_FILE`, `HQ_SECRETS_FILE`, `HQ_ALLOW_SEED`
  (§1.5) dan enam metrik tambahan (§8.2) harus masuk 01 §6/§7.

---

## Diserahkan ke builder

1. **Tabel `summaries` dan kolom `devices.expires_ms`.** Keduanya dibutuhkan
   kontrak ini tetapi belum ada di `01-architecture.md` §3; dokumen ini memilih
   bentuk minimum di §11.4. Nomor migrasi dan penamaan file diserahkan ke S24
   (migrasi baru, bukan mengubah `0001_init.sql`).
2. **Memori B.5 v2 (`memory_entries`, `memory_events`).** `00-design.md` §16
   tidak memetakannya; default dokumen ini: tidak dimigrasikan, dipertahankan
   di backup v2, dan dicatat sebagai `warning` di laporan migrasi. Bila ingin
   dimigrasikan ke `knowledge_pages` (status `candidate`), itu keputusan
   desain baru, bukan penyimpangan builder.
3. **Tanda tangan artefak rilis** (GPG/cosign/sha256 saja). §10.3 memakai
   `SHA256SUMS` wajib + `.asc` opsional; pilih satu dan catat di
   `S26.md`.
4. **Angka `min_free_bytes`, `min_free_percent`, dan retensi backup**
   (2 GiB / 10% / 14 harian / 8 mingguan) adalah default operasional awal;
   kalibrasi setelah 30 hari data nyata dan catat di CHANGELOG bila berubah.
5. **Jadwal maintenance internal vs systemd timer.** Dokumen ini memilih
   scheduler internal (`serve`) dengan catch-up, tanpa timer tambahan.
   Alternatif timer terpisah ditolak karena menambah unit yang bisa mati
   diam-diam; bila builder menemukan kebutuhan operasional yang tidak
   tertangani, itu perubahan kontrak.
