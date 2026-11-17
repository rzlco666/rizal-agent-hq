# ADR-0001 — Event log append-only dan projection

## Konteks

v2 menyimpan state operasional sebagai tabel yang ditimpa. Paritas S00
membutuhkan jejak yang bisa di-replay, sementara UI harus menampilkan state
terkini tanpa menunggu agregat berat. Sumber data (proses, sesi, webhook, usage)
datang terlambat atau duplikat, sehingga state tidak boleh diasumsikan
monoton. Lihat `00-design.md` §9.2 dan `01-architecture.md` §2–§3.

## Keputusan

- `events` adalah satu-satunya sumber kebenaran, append-only; tidak ada
  `UPDATE`/`DELETE` lewat API. Retensi memindahkan baris ke `events_archive`.
- `seq` dari SQLite adalah kursor global untuk WS/SSE dan `?since=`.
- `idempotency_key` unik per `(source, key)`; duplikat menjadi no-op yang
  tercatat di `connector_runs`.
- Payload divalidasi terhadap skema `kind` + `schema_version` sebelum append;
  redaksi berjalan sebelum append.
- Projection inti (agent, session, alert, approval) ditulis dalam transaksi
  yang sama dengan append. Projection turunan (FTS, embedding, agregat) lewat
  `outbox` + `projection_checkpoint` dan bersifat eventual.
- Perubahan skema memakai upcaster saat baca/replay. `hq replay --rebuild`
  berjalan offline atau ke tabel bayangan lalu swap, tidak pernah menulis ke
  `events`.
- Task bukan event sumber, melainkan projection deterministik dari
  `session.*`, `log.recorded`, `usage.recorded`, dan fakta adapter.
- Setiap aksi kontrol menghasilkan minimal tiga event: requested, decided,
  executed/failed.

## Konsekuensi

- State lama selalu bisa dibangun ulang, dan lag projection turunan tampil di
  `/health`.
- Biaya tulis sedikit lebih tinggi karena validasi + projection satu transaksi.
- Semua konsumen harus memakai `seq`, bukan timestamp atau counter patch.

## Alternatif ditolak

- Tabel state mutable saja: kehilangan histori dan tidak ada replay.
- Event store eksternal (Postgres/Kafka): melanggar target bundled local-first.
- Menyimpan kalimat hasil humanizer sebagai fakta: merusak determinisme
  rebuild dan i18n.
