# Performance Workspace & zona dinamis — Rizal Agent HQ

Dokumen ini menjelaskan fitur yang ditambahkan di cabang `feat/hq-kantor-cerah-v2`
di atas main v2.5 (84550bd). Tidak ada data contoh yang ditampilkan di mana pun di sini.

## Performance Workspace

- Dibuka dari tombol `Performance` di nav atas, atau `?view=perf`.
- Agent dipilih dari dropdown (default: agent yang sedang bekerja, atau agent terpilih di drawer).
- Donut = tugas selesai / total tugas agent itu. Hitungan:
  - Dikerjakan = fase `berjalan` + `antre`
  - Menunggu review = fase `menunggu-rizal`
  - Selesai = fase `selesai`
  - Perlu diperiksa = fase `gagal`
- Tabel: Tugas (judul + detail) / Status / Diperbarui (waktu selesai, atau waktu mulai,
  atau `—` bila tidak ada) / Aksi `Buka tugas` (menutup workspace dan membuka drawer agent).
- Pencarian memfilter judul + detail tugas agent yang dipilih saja.
- Agent tanpa tugas: donut 0/0, semua hitungan 0, dan pesan "Belum ada tugas tercatat
  untuk agent ini."

## Zona dinamis

Chip zona di bawah panggung menghitung agent dari snapshot live:

- `semua` = semua agent; `Muse` = agent id `Muse`; `utama` = agent id `omp` atau
  kind `utama` (di luar Muse dan Cursor); `workers` = kind `worker` atau
  `omp-workers-agg`; `cursor` = agent id `cursor`; `meeting` = kind `utama`;
  `mcp` = agent id `mcp` atau kind `mcp`.
- Zona kosong (kecuali `semua` dan `meeting`, karena ruang meeting memang ada walau
  kosong) ditampilkan redup dan tidak menyesatkan.

## Label agent

Label melayang kini: nama + peran (`role` dari data agent) + label status + aktivitas
bila sedang bekerja/berpikir. Peran kosong tidak menampilkan pil peran.

## Nav Saham & Pengetahuan — jujur kosong

Kedua panel ini tidak menampilkan data apa pun sampai sumber dipilih. Itu disengaja:
menampilkan harga saham atau dokumen contoh akan menyesatkan. Langkah berikutnya
membutuhkan keputusan sumber dari pemilik HQ, bukan tebakan Melo.

## Setup cepat

- `./scripts/quickstart.sh` — foreground, cocok untuk mencoba.
- `./scripts/install.sh` — service systemd permanen.
- `.env.example` — daftar variabel env yang didukung (`HQ_PORT`, `HQ_BIND`,
  `HQ_INTERVAL`, `HQ_DATA`, `HQ_WEB`, `HQ_SOURCES`, `HQ_DISABLE_SOURCES`, `HQ_LLM`).
