# Performa Portofolio Saham di Rizal Agent HQ

Seksi Portofolio di dalam panel Saham: posisi yang kamu miliki + nilainya dari
harga live. HQ di sini **mengukur, bukan menyarankan** — tidak ada rekomendasi
beli/jual, prediksi harga, atau auto-trading (keputusan SA, hanya Syahrizal
yang bisa membatalkan dengan alasan tertulis).

## Prinsip

- **Posisi ≠ pantauan.** `stocks.json` adalah daftar pantauan (simbol yang
  harganya diambil tiap hari). Posisi yang kamu miliki hidup terpisah di
  `data/portfolio.json` — satu file satu pemilik. Menghapus posisi tidak
  menyentuh daftar pantauan.
- **Cost basis = input manual.** `stocks.json` punya field `price` yang artinya
  ambigu (harga beli / harga catatan / penutupan kemarin). Portofolio memakai
  field eksplisit `avg_buy_price` di `portfolio.json`. Bila kosong, untung/rugi
  tampil "—" (belum tersedia), **bukan** ditebak dari `price`.
- **Harga dari cache, tanpa panggilan tambahan.** Semua nilai dihitung dari
  `stocks_cache.json` (modul harga). Modul portofolio tidak memanggil EODHD/FMP,
  tidak memakai kuota, tidak memegang kunci API.
- **Tidak ada angka karangan.** Data yang tidak ada tampil "—" + alasan
  ("harga belum tersedia", "harga beli belum diisi", "tanpa previous close
  (EOD)"), bukan 0. 0 adalah angka yang berarti "tidak ada nilai" — dan itu
  salah bila datanya sebenarnya "tidak diketahui".
- **File tidak ada = portofolio kosong yang jujur.** File rusak = pesan error
  yang jelas. `portfolio.json` adalah data user, tidak pernah di-commit
  (sudah di `.gitignore` bersama seluruh `data/`).

## Format `data/portfolio.json`

```json
{
  "usd_idr_rate": 16250,
  "rate_asof": "2026-11-07",
  "holdings": [
    {"symbol": "BBCA", "shares": 1000, "avg_buy_price": 9200, "bought_at": "2026-03-14", "note": "lot 10, beli bertahap"},
    {"symbol": "AAPL.US", "shares": 5, "avg_buy_price": 175.5, "bought_at": "2025-11-02"}
  ]
}
```

- `holdings[].symbol`: wajib. Harus ada di `stocks.json` (kalau tidak, API
  menolak: "Tambahkan dulu di panel Saham"). `BBCA` dan `BBCA.JK` setara.
- `holdings[].shares`: wajib, angka > 0, boleh desimal. Jumlah lembar yang
  **dimiliki**. Di UI bisa diisi sebagai lot (1 lot = 100 lembar) atau lembar;
  disimpan selalu sebagai lembar.
- `holdings[].avg_buy_price`: harga beli rata-rata per lembar, dalam mata uang
  simbol itu (diwarisi dari `stocks.json`). Boleh kosong → P&L "belum tersedia".
- `holdings[].bought_at`: tanggal beli (YYYY-MM-DD, tidak boleh masa depan) —
  informasi saja, bukan input perhitungan di v1.
- `holdings[].note`: catatan bebas, maks 200 karakter (lewat `redact()`).
- `usd_idr_rate` + `rate_asof`: kurs manual USD→IDR + tanggalnya. Tanpa ini
  tidak ada grand total IDR (lihat bawah).

## Rumus (semua dihitung server, JS hanya menampilkan)

Per holding, dalam mata uang simbol itu:

| Ukuran | Rumus | Bila data tidak ada |
|---|---|---|
| Nilai saat ini | `lembar × harga live` | Simbol tidak ada di cache → "—" |
| Modal | `lembar × avg_buy_price` | `avg_buy_price` kosong → "—" |
| Untung/rugi | `(harga live − avg_buy_price) × lembar` | Salah satu kosong → "—" |
| Untung/rugi (%) | `(harga live − avg_buy_price) / avg_buy_price × 100` | Sama |
| Perubahan harian | `(harga live − previous_close) × lembar` | `previous_close` kosong (jalur EOD) → "—" |
| Perubahan harian (%) | `(harga live − previous_close) / previous_close × 100` | Sama |
| Bobot | `nilai holding / total (mata uang sama) × 100` | Total 0 / tidak ada → "—" |

**Total per mata uang** (selalu tampil): nilai, modal, untung/rugi, perubahan
harian — dijumlahkan hanya atas holding dengan mata uang itu DAN harga
tersedia. Holding yang harganya belum tersedia dikeluarkan dari penjumlahan
dan ditandai ("1 posisi belum ada harga — tidak masuk total"). Total modal /
P&L / harian menjadi "—" bila ada satu saja holding di mata uang itu yang
datanya tidak lengkap untuk ukuran tersebut.

**Grand total IDR:** hanya bila `usd_idr_rate` + `rate_asof` terisi.
Rumus: `total_IDR + subtotal_USD × kurs`. v1 hanya mengonversi USD; mata uang
lain tampil sebagai subtotal sendiri tanpa masuk grand total. Tanggal kurs
selalu ditampilkan di samping grand total ("kurs 16.250 per 2026-11-07").

**Pembulatan:** disimpan `f64` penuh; tampil bulat untuk IDR dan 2 desimal
untuk USD/persentase. Tidak pernah membulatkan sebelum penjumlahan.

**Belum termasuk dividen.** Total v1 = capital gain saja; panel menuliskan itu
agar tidak ada yang mengira total return. Dividen, holding period, pajak, dan
fee broker tidak dihitung di v1.

## Histori (snapshot harian)

- Satu baris per (tanggal, mata uang) di tabel `portfolio_snapshots` (`hq.db`).
- Dibuat otomatis pukul **16:30 WIB** (setelah jendela EOD 16:05) + bisa
  dipicu manual (`POST /api/portfolio/snapshot`).
- Idempotent: jalan dua kali di hari yang sama menghasilkan satu baris per
  mata uang (yang terakhir menang).
- **Hanya bila ada fetch segar hari itu** (`last_fetch_wib_date == hari ini`).
  Akhir pekan / libur / hari tanpa fetch → tidak ada baris baru (grafik v2
  menunjukkan gap, bukan garis datar palsu).
- Mengubah `avg_buy_price` tidak menulis ulang snapshot lama; menghapus
  holding tidak menghapus snapshot lama (agregat masa lalu tetap apa adanya).

## Endpoint

| Endpoint | Fungsi |
|---|---|
| `GET /api/portfolio` | Posisi + hitung + total + kurs + catatan (satu-satunya sumber hitung panel) |
| `POST /api/portfolio/holdings` | Tambah posisi (simbol ganda → 409, pakai PATCH) |
| `PATCH /api/portfolio/holdings/:symbol` | Ubah posisi (field yang tidak diisi dipertahankan) |
| `DELETE /api/portfolio/holdings/:symbol` | Hapus posisi (konfirmasi di UI) |
| `GET /api/portfolio/history?days=30&currency=IDR` | Baris snapshot harian (untuk grafik v2) |
| `POST /api/portfolio/snapshot` | Trigger manual snapshot hari itu (idempotent) |
| `POST /api/portfolio/config` | Isi/ubah kurs manual USD→IDR (kurs + tanggal harus sepasang) |

Validasi (400 + pesan jelas bila gagal): simbol harus ada di `stocks.json`;
lembar/lot > 0; isi lot ATAU lembar, bukan keduanya; `avg_buy_price` > 0 bila
diisi; `bought_at` tanggal valid ≤ hari ini; catatan ≤ 200 karakter.

## Batas modul

- Modul portofolio **hanya membaca** `stocks.json` dan `stocks_cache.json` —
  tidak pernah menulis keduanya (dibuktikan checksum sebelum/sesudah operasi).
- Tidak ada panggilan keluar ke EODHD/FMP selama operasi portofolio.
- `stocks.json` tetap ditulis manual oleh user; modul ini tidak pernah
  menebak posisi dari `stocks.json:shares`.
- Localhost-only sampai auth B.7 (sama seperti seluruh HQ — data uang ikut
  terekspos lewat funnel tanpa auth; jawabannya B.7, bukan menonaktifkan modul).
