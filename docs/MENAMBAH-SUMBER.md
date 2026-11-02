# Menambah Sumber Data / Konektor Baru

Rizal Agent HQ dirancang **dinamis**: agent, perkakas, atau layanan baru bisa
muncul di kantor tanpa mengubah UI, model data, atau server. Ada **tiga jalur**,
dari yang paling tanpa-kode sampai yang paling dalam.

## Jalur 1 — Heartbeat API (tanpa kode, untuk agent/perkakas apa pun)

Agent eksternal cukup melapor berkala:

```bash
curl -X POST http://127.0.0.1:8900/api/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"agent":"Nama Agent","role":"Perannya","status":"bekerja","task":"Sedang mengerjakan apa, kalimat manusiawi"}'
```

Laporan tersimpan sebagai berkas heartbeat dan agent-nya tampil berstatus
*Bekerja* selama laporannya segar (< 10 menit). Cocok untuk: bot, cron,
layanan di mesin lain, atau agent yang belum punya kolektor khusus.

## Jalur 2 — Sumber kolektor baru (Rust, ±30 baris)

1. Tulis satu fungsi di `crates/hq-collect/src/lib.rs` (atau berkas baru)
   mengikuti pola sumber yang ada:

   ```rust
   fn src_contoh(_state: &mut CollectState, pr: &Probes) -> SourceOut {
       let mut out = SourceOut::default();
       // Baca sumbermu secara READ-ONLY dan defensif:
       // kegagalan di sini tidak boleh menjatuhkan snapshot.
       out.agents.push(Agent {
           id: "contoh".into(),          // id stabil, huruf kecil
           name: "Contoh".into(),
           emoji: "🧩".into(),
           role: "Peran singkat".into(),
           kind: AgentKind::Utama,        // Utama | Worker | Mcp
           parent: None,                  // id induk untuk worker
           status: Status::Bekerja,       // HANYA 6 status seragam
           activity: "Satu kalimat manusiawi tentang yang sedang dikerjakan".into(),
           provider: None, model: None,
           usage: Usage::default(),
           processes: 0, cpu_percent: 0.0, active_alerts: 0,
       });
       out
   }
   ```

2. Daftarkan **satu baris** di `registry()`:

   ```rust
   ("contoh", src_contoh),
   ```

3. Selesai. Agent baru otomatis tampil di kantor 3D, kartu, view detail,
   alert, dan laporan harian — karena semua panel membaca model seragam
   `Snapshot` dari `hq-core`.

Yang tersedia untuk sumbermu: `Probes` (proses sistem terklasifikasi, tmux,
docker, git, info sistem — dihitung sekali bersama), `CollectState` (cache
antar-siklus), dan helper `run_timeout`, `redact`, `humanize`.

**Aturan main sumber baru:**
- Read-only. Jangan pernah mengubah keadaan mesin dari kolektor.
- Defensif: bungkus pembacaan yang bisa gagal; kembalikan yang ada.
- Sensor: semua teks bebas wajib lewat `hq_core::redact::redact`.
- Manusiawi: `activity` selalu SATU kalimat yang dipahami manusia.
- Status hanya enam nilai `Status` — tidak ada status khusus sumber.

## Jalur 3 — Matikan / nyalakan sumber tanpa kompilasi

```bash
HQ_SOURCES=omp,cursor,Muse        # hanya sumber ini yang jalan
HQ_DISABLE_SOURCES=mcp,coding-agents # semua kecuali ini
```

Id sumber saat ini: `omp`, `cursor`, `Muse`, `coding-agents`, `mcp`
(lihat `known_sources()` / registry untuk yang terbaru).
