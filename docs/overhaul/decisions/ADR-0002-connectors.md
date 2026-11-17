# ADR-0002 — Connector native plus engine deklaratif

## Konteks

Rencana mencakup GitHub, Telegram, MCP, OTLP, dan puluhan integrasi lain.
Menulis tiap integrasi sebagai kode Rust baru membuat breadth lambat;
mengandalkan hanya konfigurasi kehilangan kontrol atas webhook, pagination,
dan policy. `00-design.md` §11 menetapkan tiga tier dan kontrak trait.

## Keputusan

- Tier 1 memakai trait Rust (`manifest`, `poll`, `handle_webhook`, `action`,
  `health`) untuk integrasi yang butuh kontrol penuh.
- Tier 2 dan 3 memakai satu engine deklaratif (YAML/JSON) untuk auth, poll,
  webhook, mapping JSONPath, aksi HTTP, retry, dan rate limit.
- `connector.event` adalah jalur generik untuk event yang tidak punya enum
  Rust; payload divalidasi terhadap JSON Schema di manifest.
- Supervisor per connector melakukan backoff + jitter, circuit breaker, dan
  mencatat `connector_runs`.
- Rahasia hanya dirujuk sebagai `${secret:path}`, tidak pernah diserialisasi
  atau masuk event/log/error.
- Connector tanpa kredensial berstatus `unconfigured`; tidak ada data palsu.
- Satu connector dianggap selesai hanya dengan manifest, test (sukses, auth
  gagal, payload rusak), dan dokumentasi auth.

## Konsekuensi

- Integrasi baru biasanya cukup satu manifest + test, bukan perubahan Rust.
- Engine deklaratif menjadi komponen kritis yang harus diuji dengan corpus
  manifest valid/invalid.
- Batas capability tetap berlaku untuk aksi connector, termasuk MCP.

## Alternatif ditolak

- Semua connector ditulis sebagai Rust: terlalu mahal untuk 30+ integrasi.
- Semua connector hanya YAML: tidak cukup untuk OAuth, webhook signature, dan
  kontrol kanal yang kompleks.
- Otomasi eksternal (n8n/Zapier): menjadikan runtime bergantung layanan
  jaringan, bertentangan dengan local-first.
- Connector menulis langsung ke SQLite: memotong redaksi, idempotency, dan
  projection.
