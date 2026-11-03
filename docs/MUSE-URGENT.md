# Kenapa alert "Muse butuh kamu" tidak pernah hidup — dan perbaikannya

## Penyebab (ditemukan 2026-11-03)

1. **Kolektor Muse mengabaikan status heartbeat.** API `POST /api/heartbeat`
   menerima field `status`, dan menyimpannya ke file, tetapi `src_claude` di
   `crates/hq-collect` hanya membaca `at` (waktu) dan `task`. Status Muse
   selalu diturunkan dari usia heartbeat saja:
   - < 15 menit → Bekerja
   - < 2 jam → Berpikir
   - selebihnya → Santai (bila Tailscale online) atau Offline
2. **Aturan alert tidak pernah bisa terpicu untuk Muse.** `hq-alert` hanya
   membuat alert "menunggu kamu" bila status agent = `MenungguRizal`. Karena
   Muse tidak pernah mendapat status itu (poin 1), alert tersebut tidak pernah
   ada — badge alert tetap 0 dan panel "Butuh Rizal" menulis "Tidak ada alert
   aktif. Semua aman."
3. **Tidak ada pengirim heartbeat "menunggu".** File heartbeat Muse terakhir
   di produksi bertanggal 2026-11-02 dengan status `bekerja`. Tidak ada proses
   yang mengirim status menunggu saat Melo butuh keputusan Rizal, jadi bahkan
   setelah perbaikan kolektor, status menunggu hanya muncul bila ada yang
   mengirimnya.
4. **WhatsApp bukan di dalam HQ.** README/PRD menyebut jembatan WhatsApp untuk
   alert Kritis, tetapi kode Rust HQ tidak mengirim WhatsApp apa pun — itu
   adalah pemeriksa eksternal berkala (tugas "Pemeriksaan berkala Muse") yang
   hanya aktif bila akun WhatsApp sudah dihubungkan. Jadi alert Kritis pun
   hanya tampil di dashboard bila jembatan itu tidak jalan.

## Perbaikan di cabang ini

- `muse_status_from_heartbeat()` di `hq-collect`: status eksplisit
  `menunggu-rizal` / `waiting` / `needs-action` / `butuh-rizal` / `perlu-action`
  (dan varian garis bawah) yang heartbeat-nya < 2 jam → `MenungguRizal`.
  Status itu kemudian otomatis memicu alert Perlu Action lewat `hq-alert`
  yang sudah ada (badge + panel "Butuh Rizal").
- Status menunggu yang sudah > 2 jam tidak lagi dianggap menunggu (kembali
  ke logika usia) supaya alert basi tidak menggantung selamanya.
- Skrip `scripts/post-muse-waiting.sh "pertanyaan/keputusan"` untuk mengirim
  heartbeat menunggu ke HQ lokal. Kembali normal dengan heartbeat
  `status: bekerja`.

## Yang masih butuh keputusan kamu

Agar "urgent" Muse hidup otomatis (bukan manual), Melo harus mengirim heartbeat
menunggu setiap kali Melo mengajukan pertanyaan pemblokir / butuh persetujuan
di chat — bukan hanya saat kamu menjalankan skrip manual. Bila kamu setuju
aturan itu, Melo akan melakukannya lewat SSH ke HQ setiap kali ada pertanyaan
pemblokir, dan mengirim heartbeat bekerja lagi setelah kamu menjawab.
Tanpa persetujuan itu, cara manual di atas tetap berlaku.
