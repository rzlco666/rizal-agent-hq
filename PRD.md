# PRD — Rizal Agent HQ v2

**Produk:** Rizal Agent HQ — markas 3D untuk memantau *setiap agent* yang bekerja untuk Rizal.
**Pengganti total dari:** mason-agent-hq v1 (Node/JS) — repo lama dihapus, diganti repo baru `rzlco666/rizal-agent-hq`.
**Bahasa stack:** Rust (backend + logika), web untuk tampilan 3D.
**Status dokumen:** ✅ **FINAL — disetujui Rizal 2 November 2026** (dibahas & diputuskan per bagian).
**Tanggal:** 2 November 2026.

**Rekap keputusan final:** K1-A (Three.js + Rust API) · 3D-C (karakter campuran:
utama manusia, worker robot) · K2-C (humanisasi hybrid) · K3-C (alert dashboard +
WhatsApp khusus Kritis) · K4-A (repo lama dihapus begitu v2 live) · K5 (port 8900).
Bagian 1 (daftar agent & kontrak seragam) dan Bagian 6 (usage per agent) diterima
tanpa perubahan; bawaan Bagian 1 berlaku (GitHub Actions = konteks; worker > 8
dikelompokkan).

---

## 1. Kenapa dibuat ulang (breakdown v1 → v2)

| # | Masalah di v1 | Kebutuhan v2 (dari Rizal) |
|---|---|---|
| 1 | Tampilan **per mesin** (WSL, Docker, GitHub jadi "penghuni" setara agent) | Tampilan **per AGENT**: yang ditampilkan adalah agent-nya (Muse, OMP, Cursor, dst). Mesin/WSL/Docker turun kelas jadi *info lingkungan*, bukan tokoh utama |
| 2 | Kantor hanya animasi 2D canvas sederhana | **Desain 3D yang bagus**: kantor 3D beneran, karakter per agent, bisa diputar/di-zoom, klik karakter untuk fokus |
| 3 | **Log mentah** teknis (argumen proses, teks ps, dst) — tidak enak dibaca | Log **manusiawi**: kalimat bahasa manusia ("OMP mulai menulis ulang modul pembayaran…"), log mentah hanya via toggle "lihat aslinya" |
| 4 | **Task mentah** (nama proses, pid, perintah shell) | Task **manusiawi**: judul kalimat wajar + sub-langkah + progres + kapan mulai + perkiraan selesai, seperti daftar tugas tim |
| 5 | Tidak ada cara **view detail beneran** per agent | Halaman/panel **detail per agent**: ringkasan, tugas aktif & riwayat, log (manusiawi + mentah), usage token/biaya, sesi & file yang diubah, kebutuhan action |
| 6 | Tidak ada pemberitahuan bila ada yang **butuh Rizal** | **Alert center**: Muse/OMP/Cursor/WSL butuh action (login, approval, kuota penuh, saldo habis, error, mati) → banner mencolok + daftar alert + status penanganan |
| 7 | Stack Node/JS | **Rust**: satu binary ringan, cepat, jalan terus berbulan-bulan tanpa bocor memori |
| 8 | Nama "Mason" | Namanya **Rizal** → `rizal-agent-hq`, judul "Kantor Rizal" |

## 2. Prinsip SERAGAM (berlaku untuk semua agent, semua panel, semua pilihan)

Agar konsisten, semua agent diperlakukan dengan *kontrak tampilan yang sama*:

1. **Status seragam** — hanya 6 status ini, untuk semua agent:
   `Bekerja` · `Berpikir` · `Menunggu Rizal` · `Santai` · `Offline` · `Error`
   (warna & ikon status sama di kantor 3D, kartu, detail, dan alert).
2. **Kartu agent seragam** — setiap agent selalu punya: avatar 3D, nama, peran,
   status, *satu kalimat* tugas manusiawi saat ini, token & biaya hari ini,
   dan lencana jumlah alert aktif.
3. **Kalimat seragam** — log & task ditulis dengan pola yang sama:
   `[Agent] [kata kerja manusiawi] [objek] — [detail singkat]`.
   Contoh: "Cursor selesai memperbaiki 3 error TypeScript di sample-next — lanjut menjalankan tes."
4. **Satuan seragam** — token selalu format ringkas (rb/jt/M), biaya selalu
   dipisah `TUNAI` vs `bayangan`, waktu selalu WIB, tanggal selalu format Indonesia.
5. **Pilihan seragam** — setiap keputusan desain di PRD ini disajikan dengan
   format yang sama: *Opsi — ringkasan — untung — rugi — rekomendasi*, supaya
   Rizal bisa memilih dengan cara yang konsisten di semua bagian.
6. **Satu sumber kebenaran** — semua panel membaca model data agent yang sama
   dari API Rust; tidak ada panel yang menghitung sendiri-sendiri.

## 3. Siapa saja "agent" yang ditampilkan (per agent, bukan per mesin)

| Agent | Peran tampil | Sumber bukti aktivitas |
|---|---|---|
| **Muse** ✨ | Asisten utama Rizal (di VM terpisah) | Heartbeat API ke server HQ + kehadiran Tailscale + sesi aktif |
| **OMP — Lead** 🧠 | Mesin coding utama di WSL | Proses `omp`, sesi & transkrip OMP, `omp stats` |
| **OMP — Workers** (per sesi subagent) | Anggota tim yang di-spawn OMP Lead | Sub-sesi OMP: model/provider, tugas, token, biaya per worker |
| **Cursor** 🖱️ | Editor + agent worker | cursor-server, cursor-agent worker, aktivitas file `.cursor` |
| **Claude Code / Codex / Devin** (bila jalan) | Agent coding lain yang terdeteksi | Proses + file sesi masing-masing |
| **MCP Servers** 🔌 | Pelayan perkakas (SQL, Figma, memory, dst) | Proses MCP yang aktif + induknya siapa |
| *(bukan agent)* WSL, Docker, Git, Tailscale | Turun jadi **panel Lingkungan**: kesehatan mesin, kontainer, commit terbaru | ps/docker/git — hanya pendukung konteks |

Setiap agent yang baru terdeteksi otomatis dapat meja di kantor 3D dan kartu
dengan kontrak seragam di atas — tanpa konfigurasi manual.

## 4. Fitur utama

### 4.1 Kantor 3D
- **Gaya karakter: ✅ DIPUTUSKAN 2 Nov 2026 — Opsi 3D-C (Campuran).**
  Agent utama (Muse, OMP Lead, Cursor, dan agent coding mandiri lain) =
  karakter **manusia low-poly** duduk di meja. OMP Workers (subagent) dan
  MCP Servers = **robot kecil** dengan warna identitas induknya — kelihatan
  jelas beda "pegawai tetap" vs "pekerja yang dipanggil".
- Kantor 3D (Three.js): lantai, meja per agent, warna identitas per agent,
  papan nama di meja.
- Animasi mengikuti status seragam: mengetik saat `Bekerja`, awan pikiran saat
  `Berpikir`, tanda seru berdenyut saat `Menunggu Rizal`, 💤 saat `Santai`,
  redup saat `Offline`, kedip merah saat `Error`.
- Balon percakapan di atas kepala berisi *satu kalimat tugas manusiawi* saat ini.
- Kontrol: putar/zoom/geser kamera; tombol preset kamera (Depan, Atas, Fokus Alert).
- **Klik karakter/meja → kamera fokus + panel detail agent terbuka** (lihat 4.2).
- Mode hemat: bila tab tidak aktif atau perangkat lemah, render diturunkan otomatis;
  tersedia juga mode 2D sederhana sebagai cadangan.

### 4.2 View Detail per Agent (view beneran)
Panel detail (drawer kanan / halaman `/agent/:id`) dengan tab seragam:
- **Ringkasan** — status, tugas saat ini (manusiawi), sejak kapan, progres,
  provider/model yang dipakai, alert aktif agent ini.
- **Tugas** — tugas berjalan + riwayat tugas selesai (judul manusiawi,
  sub-langkah dicentang otomatis dari bukti aktivitas, durasi, hasil).
- **Log** — log manusiawi (bawaan) dengan saklar **"Lihat log mentah"**;
  bisa disaring per kata dan per tingkat (info/aktivitas/penting).
- **Usage** — token in/out/cache & biaya (TUNAI vs bayangan) per agent dan per
  sesi/worker, grafik mini 24 jam & 7 hari.
- **Sesi & File** — sesi aktif, file yang baru diubah agent ini, commit yang
  dibuat, proses anak (ditampilkan rapi, tetap disensor rahasianya).

### 4.3 Task Manusiawi
- Setiap tugas ditampilkan sebagai kalimat + konteks:
  *"Memperbaiki bug decode MySQL di strut-stack-sql"* — bukan `cargo test -p ...`.
- Dihasilkan oleh **Mesin Humanisasi** (§5): dari perintah/proses/sesi/commit
  menjadi: kata kerja wajar (menulis, memperbaiki, menjalankan tes, membaca,
  menunggu, …), objek (nama project/fitur/file yang dikenali), dan status langkah.
- Tugas punya siklus seragam: `antre → berjalan → menunggu Rizal → selesai / gagal`,
  lengkap dengan durasi & hasil akhir (mis. "selesai — 188 tes hijau").

### 4.4 Log Manusiawi
- Alur log utama = kalimat manusiawi berurutan waktu, dikelompokkan per agent.
- Contoh transformasi:
  - mentah: `bun /home/rizal/.bun/bin/omp --model openai-codex/gpt-6.1-sol …`
  - manusiawi: "OMP mulai mengerjakan tugas baru memakai GPT (Codex)."
  - mentah: `git commit ebf8117` → "OMP menyelesaikan satu tahap: commit 'release 1.1.0' terbuat."
- Log mentah tetap disimpan & bisa dibuka per baris (toggle), demi audit.

### 4.5 Alert Center — "Butuh Rizal" 🚨
Alert dibuat otomatis oleh **Mesin Aturan Alert** (§5). Jenis seragam:
| Jenis | Contoh pemicu | Tingkat |
|---|---|---|
| Butuh login/akses | Cursor minta login ulang; token `gh` kedaluwarsa; Muse terputus dari Tailscale | Perlu Action |
| Butuh keputusan | Agent berhenti di titik approval; OMP menunggu instruksi di tmux | Perlu Action |
| Kuota penuh/hampir | Provider OMP ≥90% (peringatan) / 100% (kritis) — mis. Cursor & Antigravity Claude hari ini | Peringatan → Kritis |
| Saldo menipis | Estimasi saldo DeepSeek < $1 | Kritis |
| Agent error/mati | Proses agent hilang mendadak, exit code gagal beruntun, error rate naik | Kritis |
| Lingkungan | WSL offline/Tailscale putus, disk ≥90%, RAM ≥95%, kontainer penting mati | Peringatan |

- Tampilan: banner merah/kuning di atas dashboard + lencana angka di judul +
  daftar Alert Center: siapa, apa yang dibutuhkan, sejak kapan, tombol
  **"Sudah saya tangani"** dan **"Tunda 1 jam"**.
- Agent yang sedang `Menunggu Rizal` ikut berdenyut di kantor 3D (§4.1) —
  jadi Rizal cukup melirik kantor untuk tahu ada yang menunggu.

### 4.6 Usage & Biaya (per agent)
- Atribusi usage **per agent & per worker/sesi** (bukan hanya global OMP):
  token in/out/cache, request, biaya TUNAI vs bayangan, kuota per provider.
- Ringkasan harian/mingguan + grafik; saldo DeepSeek terestimasi selalu tampil.
- Laporan harian otomatis tetap ada (Markdown, 23:55 WIB) — kini per agent.

### 4.7 Lingkungan (pendukung, bukan tokoh utama)
Panel kecil: kesehatan WSL (CPU/RAM/disk/uptime), Docker, commit git terbaru,
status Tailscale. Muncul juga sebagai konteks di detail agent terkait.

## 5. Arsitektur Rust

```
rizal-agent-hq/  (Cargo workspace, satu binary akhir: rizal-agent-hq)
├─ crates/
│  ├─ hq-core      model seragam: Agent, Status, Task, LogEvent, Alert, Usage
│  │               + Mesin Humanisasi (aturan kalimat, §5.1)
│  ├─ hq-collect   kolektor per SUMBER (bukan per mesin di UI):
│  │               omp (stats/usage/sesi), cursor, claude-heartbeat,
│  │               proses & tmux, git, docker, sistem — semua read-only,
│  │               semua teks lewat redaksi rahasia
│  ├─ hq-alert     Mesin Aturan Alert (§4.5): evaluasi tiap siklus,
│  │               dedup, tingkat, status penanganan
│  └─ hq-server    Axum: REST + WebSocket (live push), menyajikan UI statis,
│                  penyimpanan SQLite (riwayat, log, alert, snapshot harian)
├─ web/            UI: kantor 3D (Three.js) + panel — file statis, tanpa build rumit
├─ systemd/        service + timer laporan harian (pola sama seperti v1)
└─ scripts/        install-wsl.sh, start/stop/status, migrasi & pencabutan v1
```

Prinsip teknis:
- **Satu binary Rust** (target RSS < 60 MB), tanpa Node/npm di runtime.
- **Dinamis (registry sumber)**: kolektor adalah fungsi-fungsi yang terdaftar
  di `registry()` hq-collect (bawaan: omp, cursor, Muse, coding-agents, mcp);
  sumber baru = satu fungsi + satu baris registrasi; on/off via env
  `HQ_SOURCES`/`HQ_DISABLE_SOURCES`; agent eksternal ikut tampil lewat
  Heartbeat API tanpa kode. Panduan: docs/MENAMBAH-SUMBER.md.
- **Plug & play**: `./scripts/install.sh` membangun, memasang ke
  ~/rizal-agent-hq, dan menyalakan systemd user service; sistem tanpa
  OMP/Cursor tetap jalan (agent yang tidak ada tampil Offline).
- Kolektor defensif: sumber gagal/timeout → agent ditandai dari bukti terakhir,
  dashboard tidak pernah ikut macet (OMP stats tetap di-cache karena berat).
- Live tanpa refresh manual: WebSocket mendorong perubahan status/tugas/alert.
- Read-only & aman: tidak ada aksi menulis ke sistem agent lain; rahasia
  (API key/token di argumen proses) selalu disensor sebelum tampil/simpan.
- Jalan terus: systemd user service (Restart=always + linger), data SQLite
  di `~/rizal-agent-hq/data/`, port default **8900** (baru, agar tidak bentrok sisa v1).

### 5.1 Mesin Humanisasi (log & task jadi manusiawi)
Mengubah bukti mentah → kalimat, berlapis:
1. **Ekstraksi fakta**: agent, aksi mentah (perintah/tool), objek (project/file/fitur
   dari path & riwayat), hasil (exit/sukses/gagal, jumlah tes, hash commit).
2. **Pemetaan kata kerja**: tabel aturan (test→"menjalankan tes", build→"membangun",
   edit file→"menulis/memperbaiki", git commit→"menyelesaikan satu tahap", idle→"santai", dst).
3. **Perakitan kalimat** pola seragam §2.3 + konteks waktu ("baru saja", "sejak 14:02").
4. *(opsional, lihat pilihan §6)* **Perapi LLM**: ringkasan sesi panjang diringkas
   model gratis via OMP — hanya untuk ringkasan, bukan log per baris.

## 6. Keputusan untuk Rizal (format seragam — pilih per nomor)

**K1 — Rangka UI 3D: ✅ DIPUTUSKAN 2 Nov 2026 — Opsi A.**
- A. **Three.js (JS biasa) + Rust API** — DIPILIH Rizal. 3D matang, cepat dibuat, Rust tetap otaknya (API, kolektor, humanisasi, alert).
- B. Full Rust WASM (Leptos/Yew + WebGL) — tidak dipilih (3D lebih lambat & berisiko).
- C. Svelte + Three.js — tidak dipilih (tambah rantai build Node).

**Catatan Bagian 1 (bawaan yang berlaku bila Rizal tidak mengubah):**
GitHub Actions berstatus *konteks lingkungan* (bukan agent). OMP Workers tampil
sebagai penghuni terpisah, dengan pengelompokan otomatis ("+N worker lain")
bila jumlah worker aktif > 8 agar kantor tidak sesak.

**K2 — Mesin Humanisasi: ✅ DIPUTUSKAN 2 Nov 2026 — Opsi C (Hybrid).**
- Aturan lokal untuk semua log/task per baris (deterministik, offline, $0);
  LLM model gratis via OMP *hanya* untuk ringkasan sesi panjang. Bila jalur LLM
  sedang tidak tersedia (kuota penuh/dll), ringkasan jatuh ke template aturan —
  log utama tidak pernah ikut mati. (Opsi A aturan-saja & B LLM-saja tidak dipilih.)

**K3 — Saluran Alert: ✅ DIPUTUSKAN 2 Nov 2026 — Opsi C.**
- Semua tingkat tampil di dashboard (banner + lencana + Alert Center).
- Khusus tingkat **Kritis**, alert juga diteruskan ke **WhatsApp Rizal**.
  Mekanisme jembatan: server HQ menandai alert kritis di API-nya; Muse
  (yang terhubung WhatsApp & Tailscale) memeriksanya berkala (tiap ±15 menit)
  dan meneruskan yang baru ke WhatsApp — dedup mengikuti status alert di HQ,
  berhenti otomatis begitu alert tertangani/pulih. (Opsi A & B tidak dipilih.)

**K4 — Nasib repo & service v1: ✅ DIPUTUSKAN 2 Nov 2026 — Opsi A.**
Repo `mason-agent-hq` **dihapus permanen** begitu v2 live; service v1 di WSL
dicabut pada tahap M5. (Opsi B arsip tidak dipilih.)

**K5 — Port & alamat: ✅ bawaan diterima.** `127.0.0.1:8900`
(dari Windows: http://localhost:8900).

## 7. Tahapan pembangunan (setelah PRD disetujui)

| Tahap | Isi | Selesai bila |
|---|---|---|
| M1 | Kerangka Rust workspace + model seragam + kolektor (OMP/Cursor/proses/tmux/git/docker/sistem) + SQLite | `cargo test` hijau; API `/api/agents` berisi agent nyata dari WSL Rizal |
| M2 | Mesin Humanisasi + task/log manusiawi + toggle mentah | Log & task tampil sebagai kalimat; contoh transformasi §4.4 terbukti di data asli |
| M3 | Kantor 3D + kartu agent + view detail per agent (5 tab) | Klik karakter → detail terbuka; status animasi = status nyata |
| M4 | Mesin Alert + Alert Center + WebSocket live | Alert uji (kuota penuh Cursor yang sedang terjadi) muncul & bisa ditandai selesai |
| M5 | Pasang di WSL (systemd, port 8900), laporan harian, hapus/arsip v1 sesuai K4, README | Dashboard live dari data mesin Rizal; service v1 tercabut; repo lama sesuai keputusan |

## 8. Di luar cakupan (v2.0)
- Mengendalikan agent dari dashboard (tombol stop/start agent) — v2 hanya *memantau & memberi tahu*; aksi tetap Rizal yang melakukan di tempat aslinya.
- Multi-mesin selain WSL Rizal + VM Muse.
- Akun/login multi-pengguna — dashboard lokal untuk Rizal saja.

## 9. Risiko & mitigasi
- *OMP stats/usage berat (±15–60 dtk)* → cache 5 menit + kolektor async; status agent tidak menunggu OMP.
- *Sesi Cursor/OMP tidak selalu punya file transkrip rapi* → sumber bukti berlapis (proses → file aktivitas → heartbeat), status ditandai tingkat keyakinannya.
- *3D berat di laptop* → mode hemat otomatis + cadangan 2D (K1 tetap A).
- *Data sensitif* → redaksi rahasia di kolektor (pola v1 dipertahankan & diuji).
