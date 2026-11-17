# ADR-0004 — Secret terenkripsi dengan master key

## Konteks

Connector GitHub, Telegram, dan sejumlah preset memerlukan token. v2 menyimpan
beberapa kunci API langsung di unit systemd, yang membuat backup unit dan log
menjadi tempat kebocoran. S00 sudah menghapus nilai tersebut dari salinan
oracle; `00-design.md` §11.4 dan K14 menetapkan target penyimpanan.

## Keputusan

- Satu file terenkripsi `~/.config/rizal-agent-hq/secrets.age` memakai age
  (XChaCha20), dengan master key pada file berizin `0600`.
- Environment variable hanya sebagai override untuk CI dan uji; tidak pernah
  disimpan di repo, unit, atau fixture.
- CLI `hq secret set|list|rm|rotate` mengelola nilai; rotasi menghasilkan event
  `secret.rotated` tanpa nilai.
- Referensi `${secret:path}` di manifest connector tidak pernah diserialisasi
  sebagai nilai.
- Redaksi berjalan sebelum persist: string mirip API key/JWT/token tidak boleh
  masuk event, log, error, API, WS, atau pencarian.
- Akses remote memakai hash token + scope + expiry; cookie memakai
  `HttpOnly; SameSite=Strict; Secure` saat HTTPS.

## Konsekuensi

- Menyalin konfigurasi atau fixture tidak membocorkan kredensial.
- Operator harus menjaga master key; kehilangannya berarti rotasi semua
  kredensial.
- Setiap output connector dan agen perlu corpus uji secret sebelum naik ke
  produksi.

## Alternatif ditolak

- Token plaintext di `.env` yang dikomit atau di unit systemd: mudah bocor via
  backup dan log.
- Hanya OS keyring: tidak portabel untuk CI dan proses non-interaktif.
- Menyimpan secret di event log agar mudah diaudit: bertentangan dengan
  aturan redaksi sebelum append.
