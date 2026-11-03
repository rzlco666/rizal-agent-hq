# Saham di Rizal Agent HQ — dari file portofolio kamu

## Pendapat Melo (kenapa begini, bukan harga live / contoh)

Saham adalah data uang. Mengisi modul ini dengan ticker dan harga contoh akan
menyesatkan — kamu bisa mengira itu portofolio nyata. Mengambil harga live
tanpa sumber yang kamu pilih juga berisiko: API bisa salah, tertunda, atau
berubah format tanpa pemberitahuan.

Jadi langkah terbaik sekarang: **portofolio dari file yang kamu isi sendiri**.

- Sumber kebenaran = file `stocks.json` di folder data HQ (`HQ_DATA`, default `data/`).
- Harga di file itu adalah harga yang kamu isi — bisa harga beli, harga terakhir
  yang kamu catat, atau harga penutupan kemarin. HQ menghitung nilai = lembar × harga
  dan total portofolio dari situ, dan selalu memberi label "Harga dari file kamu,
  bukan harga live pasar."
- File tidak ada = panel kosong yang jujur, bukan angka contoh.
- File rusak = pesan error yang jelas, bukan angka palsu.

## Format

```json
[
  {"symbol": "BBCA", "name": "Bank Central Asia", "shares": 100, "price": 9500, "currency": "IDR"},
  {"symbol": "TLKM", "name": "Telkom Indonesia", "shares": 500, "price": 3100, "currency": "IDR"}
]
```

- `symbol`: wajib, string.
- `name`: opsional, string.
- `shares`: wajib, angka (boleh desimal).
- `price`: wajib, angka, dalam mata uang `currency`.
- `currency`: opsional, default `IDR`. Selain IDR, HQ menampilkan format `$`.

## Langkah berikutnya (butuh keputusan kamu)

Bila kamu mau harga live, pilih sumber kuotasi untuk simbol di file ini
(mis. Yahoo Finance, IDX, atau API broker). HQ kemudian bisa menambahkan kolom
"Harga live (sumber, waktu)" di samping harga file — tanpa pernah menimpa harga
file secara diam-diam.
