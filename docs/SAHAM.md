# Saham di Rizal Agent HQ — portofolio dari file + harga live opsional

## Prinsip

Saham adalah data uang. Mengisi modul ini dengan ticker dan harga contoh akan
menyesatkan — kamu bisa mengira itu portofolio nyata.

- Sumber kebenaran portofolio = file `stocks.json` di folder data HQ (`HQ_DATA`,
  default `data/`). Harga di file adalah harga yang kamu isi sendiri (harga beli,
  harga catatan terakhir, atau penutupan kemarin). Nilai = lembar × harga, total
  portofolio selalu dihitung dari file.
- Harga live hanya **kolom tampilan** di samping harga file. HQ **tidak pernah**
  menimpa `stocks.json` secara diam-diam; cache harga live disimpan terpisah di
  `stocks_cache.json`.
- Setiap harga live selalu tampil dengan tanggal (dan waktu ambil). Tidak ada
  harga tanpa tanggal, tidak ada angka karangan.
- File tidak ada = panel kosong yang jujur. File rusak = pesan error yang jelas.

## Format `stocks.json`

```json
[
  {"symbol": "BBCA", "name": "Bank Central Asia", "shares": 100, "price": 9500, "currency": "IDR"},
  {"symbol": "TLKM", "name": "Telkom Indonesia", "shares": 500, "price": 3100, "currency": "IDR"},
  {"symbol": "AAPL.US", "name": "Apple", "shares": 2, "price": 180, "currency": "USD", "priority": true}
]
```

- `symbol`: wajib, string. Tanpa sufiks atau `.JK` = bursa Indonesia (EODHD);
  sufiks lain (mis. `.US`) = bursa luar (FMP). `BBCA` dan `BBCA.JK` setara.
- `name`: opsional, string.
- `shares`: wajib, angka (boleh desimal).
- `price`: wajib, angka, dalam mata uang `currency`.
- `currency`: opsional, default `IDR`. Selain IDR, HQ menampilkan format `$`.
- `priority`: opsional, boolean, default `false`. Lihat "Simbol prioritas".

## Sumber harga live dan urutan fallback

| Lapis | Sumber | Dipakai untuk | Kapan |
|---|---|---|---|
| L1 | EODHD Live (`/api/real-time`) | simbol IDX (`.JK`/tanpa sufiks) | sekali sehari >= 15:00 WIB, satu permintaan batch |
| L1b | EODHD EOD (`/api/eod`) | simbol IDX | hanya bila Live gagal **bukan** karena kuota (jaringan/5xx): satu percobaan >= 16:05 WIB hari yang sama |
| L2 | FMP (`/stable/quote`) | simbol non-IDX (mis. `.US`) | jadwal harian yang sama; FMP **tidak pernah** dipanggil untuk simbol `.JK` (fitur berbayar) |
| L3 | cache terakhir | semua | kuota habis (402), gagal total, 429 menetap, akhir pekan/libur |

Aturan waktu:

- Hari bursa Senin–Jumat. Akhir pekan tidak ada pengambilan sama sekali.
- Gerbang sekali-sehari (`last_fetch_wib_date` di cache): setelah pengambilan
  tuntas (sukses, kuota habis, token salah), hari itu tidak diambil lagi.
- Gagal karena jaringan/5xx **tidak** menutup gerbang, supaya percobaan EOD
  16:05 bisa jalan; setelah percobaan EOD, hari itu ditutup.
- Libur bursa tanpa data baru (EOD mengembalikan tanggal lama): cache lama
  dipakai dan tidak ada pengambilan ulang hari itu.

## Kuota EODHD

- Paket gratis: 20 panggilan/hari (reset tengah malam GMT) + bonus sekali pakai
  500. Bonus bukan bagian rencana harian; pemakaiannya dicatat di log.
- Sebelum **setiap** pengambilan, HQ memeriksa
  `GET https://eodhd.com/api/user?api_token=...&fmt=json` (0 panggilan) dan
  membaca `apiRequests` (terpakai hari ini), `dailyRateLimit`, `extraLimit`
  (sisa bonus).
- 1 simbol = 1 panggilan, apa pun bentuknya (batch `s=` menghemat HTTP, bukan kuota).
- Bila sisa kuota < jumlah simbol: simbol `priority` diambil lebih dulu, sisanya
  ditunda ke hari berikutnya. Bila sisa 0: tunda.
- Kode balasan: `402` kuota habis (tidak memakan kuota, jangan diulang hari itu),
  `429` terlalu banyak per menit (`Retry-After` dihormati, dicoba sekali; antar
  permintaan EOD beri jeda >=1 detik), `401` token salah (log hanya "token salah"),
  `404` simbol tidak dikenal (tetap memakan 1 panggilan; ditandai di log).

## Simbol prioritas dan rotasi (> 20 simbol)

- <= 20 simbol: semua diambil setiap hari bursa.
- `> 20` simbol: maksimal 20 simbol per hari (batas kuota).
  - Simbol dengan `"priority": true` (maks 20) selalu ikut setiap hari.
  - Sisa kuota hari itu diisi simbol lain secara bergilir (round-robin satu batch
    per hari). Tanpa bendera prioritas, batch bergilir mulai dari 20 simbol
    pertama pada urutan file, lalu 20 berikutnya, dan seterusnya.
- Konsekuensi yang disengaja: simbol di luar batch harian menjadi "data terakhir"
  (stale) 1–2 hari atau lebih. Panel menampilkan label tanggal cache-nya, bukan
  angka baru.

## Label waktu di panel

- Live: `Live · delay 15–20 mnt, per 15:00 WIB, diambil 15:01 WIB (EODHD)`.
- EOD/FMP: `penutupan 05 Nov 2026, diambil 16:05 WIB (EODHD)`.
- Cache lama: ditambah `data terakhir 05 Nov 2026`.
- Tanpa kunci atau tanpa cache: kolom live kosong dan panel menulis
  "harga live belum dikonfigurasi" — bukan error, bukan angka palsu.

## Endpoint

- `GET /api/stocks` — bentuk lama tetap: `{stocks, source, error}`. Tambahan:
  `quotes` (peta simbol -> `{price, asof, fetched_at, source, stale}`) dan
  `quotes_updated_at`. `stocks[].price` tidak berubah.
- `POST /api/stocks/refresh` — coba ambil harga sekarang juga. Tetap menaati
  akhir pekan, gerbang sekali-sehari, dan kuota; boleh dipakai di luar jendela
  15:00 (mis. untuk QA). Balasan: `{"refreshed": bool, "reason": "..."}`.

```bash
curl -s -X POST http://127.0.0.1:8900/api/stocks/refresh
curl -s http://127.0.0.1:8900/api/stocks | python3 -m json.tool
```

## Menyiapkan kunci API

Kunci hanya dibaca dari environment saat server mulai: `EODHD_API_KEY` dan
`FMP_API_KEY`. Kunci **tidak pernah** ditulis ke log, cache, respons API, atau
dokumen — dan **jangan** dimasukkan ke berkas service di repo ini.

Untuk layanan systemd user, pakai drop-in terpisah:

```bash
systemctl --user edit rizal-agent-hq.service
# di editor, isi:
# [Service]
# Environment=EODHD_API_KEY=isi-kunci-anda
# Environment=FMP_API_KEY=isi-kunci-anda
systemctl --user daemon-reload
systemctl --user restart rizal-agent-hq.service
```

Alternatif manual: `export EODHD_API_KEY=...` sebelum menjalankan server.
Tanpa kunci, panel tetap menampilkan harga file dan kolom live kosong.

Variabel uji (jangan dipakai di produksi):

- `EODHD_BASE_URL` / `FMP_BASE_URL` — arahkan ke mock lokal (default
  `https://eodhd.com` dan `https://financialmodelingprep.com`).
- `HQ_STOCKS_FAKE_WIB_DATE=YYYY-MM-DD` (+ opsional `HQ_STOCKS_FAKE_WIB_TIME=HH:MM`)
  — simulasikan tanggal/jam WIB, mis. akhir pekan.

## Endpoint yang sengaja tidak dibangun

Polling real-time, order book, broker summary, KSEI, Yahoo/Twelve Data/Alpha
Vantage, Invezgo, upgrade EODHD berbayar, aksi trading, notifikasi harga, dan
tampilan publik. Harga yang tampil adalah delay 15–20 menit (EODHD) atau
penutupan (EOD/FMP), bukan umpan real-time.
