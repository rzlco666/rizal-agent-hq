# Pengetahuan HQ dari Notion

Modul knowledge menyinkronkan dokumen Notion ke HQ sebagai **cermin baca-saja**
(read-only mirror) yang bisa dicari di panel Pengetahuan. Isi Notion **tidak
pernah otomatis menjadi memori tepercaya** dan **tidak pernah disuntikkan ke
brief agen** — promosi ke `trusted` tetap lewat tangan manusia
(`POST /api/knowledge/promote` → candidate → `POST /api/memory/:id/promote`).

## Cara kerja

- **Sync berkala:** tiap 30 menit (env `NOTION_SYNC_INTERVAL_MIN`, min 5) +
  tombol "Sync sekarang" di panel. Incremental per `last_edited_time`;
  backfill pertama menelusuri subtree tiap sumber dengan checkpoint per
  halaman.
- **Penyimpanan:** tabel `notion_docs` + FTS5 `notion_docs_fts` di `hq.db`
  (terpisah dari `memory_entries` B.5). Halaman yang dihapus/tidak lagi
  dibagikan ditandai `gone`, bukan dihapus diam-diam.
- **Karantina:** halaman yang mengandung pola rahasia (`sk-ant-…`, `gho_…`,
  `token=…`, dsb. — pola yang sama dengan `hq-core/src/redact.rs`) TIDAK
  disimpan isinya; statusnya `quarantined` dan tercatat di panel.
- **Mode mock:** tanpa `NOTION_API_KEY`, modul jalan dengan 3 fixture lokal
  dan banner jujur "Mode mock — bukan data Notion asli" di panel. Semua gate
  hijau tanpa token.

## Setup (sekali saja)

1. Buat **internal integration** di Notion (`notion.so/my-integrations`) —
   beri nama `rizal-agent-hq`. Salin tokennya (`ntn_…`).
2. Di Notion, **bagikan (Share)** tiap halaman root grup ke integrasi itu
   (anak halaman mewarisi akses). Tanpa ini sync mengembalikan 404 yang
   diterjemahkan ke pesan jelas di panel, bukan diam-diam kosong.
3. Pasang token sebagai systemd override di luar repo (token tidak pernah
   masuk repo, file, log, atau chat):
   `systemctl --user edit rizal-agent-hq` → tambahkan
   `Environment=NOTION_API_KEY=...`, lalu `systemctl --user restart rizal-agent-hq`.
4. Buat daftar sumber di `data/notion.json` (di samping `stocks.json`):
   ```json
   { "sources": [
       { "id": "hq", "root_page_id": "40f8b61c-9a0c-81c1-ab14-c75e3c9853aa", "title": "Rizal Agent HQ", "enabled": true },
       { "id": "strutstack", "root_page_id": "40d8b61c-9a0c-8131-a04a-da723c6c4f3f", "title": "StrutStack", "enabled": true },
       { "id": "tracerio", "root_page_id": "4108b61c-9a0c-8108-b1c8-d4ec2e7fd257", "title": "Tracerio", "enabled": true }
   ] }
   ```
   Fallback bila file tidak ada: env `NOTION_ROOT_PAGES`
   (`id:page_id` dipisah koma). Bila keduanya tidak ada: mode mock.
5. Verifikasi: `GET /api/knowledge/sync/status` → `configured: true`,
   `mode: "live"`; lalu `POST /api/knowledge/sync` → halaman mulai masuk.

## Endpoint

| Method | Endpoint | Keterangan |
|---|---|---|
| GET | `/api/knowledge/search?q=&scope=&kind=&status=&limit=` | Pencarian gabungan FTS5 (memory + doc); trusted dulu, lalu synced, lalu candidate |
| GET | `/api/knowledge/pages?group=&status=` | Daftar halaman tersinkron |
| GET | `/api/knowledge/page/:id` | Satu dokumen lengkap + provenance |
| GET | `/api/knowledge/sync/status` | Mode, sync terakhir, per-sumber, rate |
| POST | `/api/knowledge/sync` | Trigger manual satu sync run |
| POST | `/api/knowledge/promote` | Dokumen → kandidat memori (body: `page_id`, `title?`, `body?`) |
| GET | `/api/memory?q=&kind=&scope=&status=` | Daftar/pencarian memori |
| POST | `/api/memory` | Buat kandidat (`trusted` langsung hanya syahrizal/melo; teks berahasia ditolak) |
| POST | `/api/memory/:id/promote` | candidate → trusted (syahrizal/melo) |
| POST | `/api/memory/:id/reject` | candidate → rejected (alasan wajib) |
| POST | `/api/memory/:id/correct` | Buat entri baru yang men-supersede entri lama |
| POST | `/api/memory/:id/delete` | Hard delete HANYA untuk rahasia/PII (alasan wajib) |

## Batasan v1

- Satu arah (Notion → HQ); tidak ada tulis balik ke Notion.
- Baris database/tracker, komentar, dan lampiran Notion tidak diindeks.
- Tidak ada detektor PII otomatis (v2, setelah pengukuran false positive).
- Endpoint localhost-only sampai auth B.7 ada; ikut terekspos lewat tunnel
  seperti seluruh HQ (risiko yang sudah diketahui user).
- Staleness ≤ 30 menit (default). Panel selalu menampilkan `synced_at` +
  label basi bila sync terakhir > 2× interval atau gagal 2× berturut-turut.
