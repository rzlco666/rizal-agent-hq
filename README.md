<p align="center">
  <img src="web/logo.svg" width="128" alt="Logo Rizal Agent HQ">
</p>

<h1 align="center">Rizal Agent HQ</h1>

<p align="center">
  <b>Markas 3D untuk semua agent AI yang bekerja untukmu.</b><br>
  Satu ruangan, semua agent terlihat bekerja — dengan tugas &amp; log <i>manusiawi</i>,<br>
  usage per agent, dan alert saat ada yang <i>butuh kamu</i>.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.99-dea584?logo=rust&logoColor=white">
  <img alt="Axum" src="https://img.shields.io/badge/Axum-0.8-2f3b52">
  <img alt="Three.js" src="https://img.shields.io/badge/Three.js-3D-049ef4?logo=threedotjs&logoColor=white">
  <img alt="Lisensi MIT" src="https://img.shields.io/badge/License-MIT-3ddc97">
  <img alt="Satu binary" src="https://img.shields.io/badge/Deploy-1%20binary%20~5MB-ffb454">
</p>

---

<p align="center">
  <img src="docs/screenshot.png" width="880" alt="Kantor 3D Rizal Agent HQ dengan data asli: agent utama di meja depan, worker robot di belakang, panel alert Kritis dan kuota di kanan">
</p>
<p align="center"><i>Kantor 3D dengan data asli — OMP, Cursor, Muse di baris depan; worker robot &amp; MCP di belakang; alert Kritis dan kuota provider di panel kanan.</i></p>

## ✨ Kenapa ada proyek ini?

Agent AI modern tidak bekerja sendirian lagi: ada asisten utama, mesin coding
multi-provider, subagent yang dipanggil-panggil, editor AI, server MCP — semua
jalan bersamaan, dan pemiliknya **tidak bisa melihat apa-apa**. Terminal penuh
log mentah, biaya token tercecer, dan tidak ada yang memberi tahu saat seorang
agent diam-diam menunggu keputusanmu selama dua jam.

**Rizal Agent HQ** mengubah semua itu menjadi satu **kantor 3D yang hidup**:

| | |
|---|---|
| 🏢 **Per agent, bukan per mesin** | Yang tampil adalah *siapa yang bekerja*: tiap agent & worker punya meja, karakter, dan status sendiri. Mesin (WSL/Docker/Git) cuma panel pendukung. |
| 🧑‍🤝‍🧑 **Karakter campuran** | Agent utama = manusia low-poly; worker & MCP = robot kecil sewarna induknya. Sekali lirik tahu siapa pegawai tetap, siapa pekerja panggilan. |
| 🗣️ **Log & tugas manusiawi** | Bukan `bash -c CARGO_BUILD_JOBS=2 cargo test …`, melainkan *"OMP menjalankan tes strut-stack-sql — 188 tes hijau."* Log mentah tetap ada, di balik toggle. |
| 👆 **View detail beneran** | Klik karakter → detail 5 tab: Ringkasan, Tugas, Log, Usage, Sesi & File. |
| 🚨 **Alert "Butuh Kamu"** | Kuota penuh, saldo menipis, agent mati mendadak, sesi menunggu input — muncul sebagai alert bertingkat, yang **Kritis** diteruskan ke WhatsApp. |
| 💸 **Biaya per agent, jujur** | Token & biaya diatribusikan per agent/sesi, selalu dipisah **TUNAI vs bayangan** (langganan). Tidak ada lagi "total $16" yang tidak jelas punya siapa. |
| 🔌 **Dinamis & plug & play** | Sumber data = registry: tambah konektor baru cukup satu fungsi + satu baris. Agent luar bisa ikut tampil hanya dengan POST heartbeat. |

## 🚀 Pasang (plug & play)

Butuh: Linux dengan `systemd --user`, Rust (untuk build), dan itu saja.
OMP/Cursor/agent lain **opsional** — yang tidak ada akan tampil *Offline*,
sisanya jalan.

```bash
git clone https://github.com/rzlco666/rizal-agent-hq.git
cd rizal-agent-hq
./scripts/install.sh
```

Selesai. Buka **http://localhost:8900/** — kantor 3D-mu langsung terisi dari
mesinmu sendiri. Copot kapan saja dengan `./scripts/uninstall.sh`.

Jalan manual tanpa systemd:

```bash
cargo run --release -p hq-server      # http://127.0.0.1:8900/
cargo test --workspace                # 13+ test
```

## ⚙️ Konfigurasi (semua lewat env, tanpa file wajib)

| Env | Bawaan | Arti |
|---|---|---|
| `HQ_PORT` | `8900` | Port server |
| `HQ_BIND` | `127.0.0.1` | Alamat dengar |
| `HQ_INTERVAL` | `30` | Interval kumpul data (detik) |
| `HQ_DATA` | `data/` | Folder SQLite + heartbeat |
| `HQ_WEB` | `web/` | Folder UI statis |
| `HQ_SOURCES` | *(semua)* | Daftar putih sumber, mis. `omp,cursor,Muse` |
| `HQ_DISABLE_SOURCES` | — | Matikan sumber tertentu, mis. `mcp` |

## 🧩 Dinamis: menambah agent & konektor baru

Tiga jalur, sedalam yang kamu butuhkan — panduan lengkap di
**[docs/MENAMBAH-SUMBER.md](docs/MENAMBAH-SUMBER.md)**:

1. **Heartbeat API (tanpa kode)** — agent/perkakas apa pun POST ke
   `/api/heartbeat`, langsung tampil di kantor.
2. **Sumber kolektor (±30 baris Rust)** — satu fungsi + satu baris registrasi
   di `registry()`; UI, alert, dan laporan mengikut otomatis berkat model
   seragam.
3. **On/off via env** — `HQ_SOURCES` / `HQ_DISABLE_SOURCES`, tanpa kompilasi ulang.

Sumber bawaan: `omp` (stats, kuota, sesi & worker per sesi), `cursor`,
`Muse` (heartbeat + Tailscale), `coding-agents` (Claude Code/Codex/Devin
bila jalan), `mcp`.

## 🏗️ Arsitektur

```
            ┌───────────────────────── SATU BINARY RUST ─────────────────────────┐
 sumber ───▶│ hq-collect (registry) ─▶ hq-core (model seragam + Humanizer)       │
 (omp,      │        │                                  │                         │
  cursor,   │        ▼                                  ▼                         │
  Muse…)   │   hq-alert (mesin aturan)          hq-server (Axum)                 │
            │        │                       REST + WebSocket + SQLite            │
            └────────┼───────────────────────────────────┬───────────────────────┘
                     ▼                                   ▼
              Alert Center                       web/ — kantor 3D (Three.js)
              + WhatsApp (Kritis)                manusia & robot, live via WS
```

Prinsip yang tidak bisa ditawar:

- **6 status seragam** untuk semua agent: `Bekerja · Berpikir · Menunggu Rizal · Santai · Offline · Error`.
- **Manusiawi dulu** — pola kalimat tetap: `[Agent] [kata kerja] [objek] — [detail]`.
- **Read-only & aman** — kolektor tidak mengubah apa pun di mesin; semua teks bebas disensor dari rahasia (API key/token tidak pernah tampil/tersimpan).
- **Satu sumber kebenaran** — semua panel membaca `Snapshot` yang sama dari `hq-core`.

Spesifikasi produk lengkap: **[PRD.md](PRD.md)**.

## 🗺️ Status

| Tahap | Isi | Status |
|---|---|---|
| M1 | Kerangka Rust + kolektor + SQLite + API | ✅ selesai, terverifikasi data asli |
| M2 | Humanisasi sesi + worker per sesi + WebSocket | ✅ selesai, terverifikasi data asli |
| M3 | Kantor 3D + view detail 5 tab | ✅ selesai, terverifikasi tangkapan layar data asli |
| M4 | Alert Center + jembatan WhatsApp | ✅ Alert Center live; jembatan Kritis jalan tiap ±15 mnt (saluran WhatsApp aktif begitu akun tertaut) |
| M5 | Rilis GitHub + pasang systemd + laporan harian | ✅ live di WSL (port 8900) + repo publik; panel kuota menampilkan rantai fallback OpenCode Go → DeepSeek |
| M6 | Redesign kantor virtual isometrik terang | ✅ kamera ortografik isometrik, ruangan kaca berperabot per tim, lingkungan hidup (jalan, zebra cross, mobil berjalan, pohon), chip nama melayang, UI terang pill + pencarian agen |
| M7 | Fitur kantor virtual lengkap | ✅ **Panggil Meeting** (agen utama berjalan ke ruang meeting ±30 dtk), **Papan Tugas** kanban 4 fase, **Fasilitas** (host/Docker/Tailscale/kuota jebol/fallback), interior bisa diklik (sofa, tenis meja, rak server, layar meeting, papan pintu), label versi v2.1 |

## 🤝 Berkontribusi

Sumber data baru adalah kontribusi paling berharga: kalau agent/perkakas
favoritmu belum punya kolektor, ikuti [docs/MENAMBAH-SUMBER.md](docs/MENAMBAH-SUMBER.md)
— satu fungsi, satu baris registrasi, selesai.

## 📄 Lisensi

MIT © 2026 Syahrizal Hanif ([@rzlco666](https://github.com/rzlco666)) — dibangun dari nol.
