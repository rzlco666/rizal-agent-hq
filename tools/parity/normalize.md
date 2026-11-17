# Normalisasi oracle v2 ↔ kandidat v3

Fixture beku di `tools/parity/` dipakai untuk menguji normalizer dan humanizer.
Uji yang mengikat tetap memakai oracle live v2 di `127.0.0.1:8900` dan kandidat
di `127.0.0.1:8901` dengan sampel simultan; fixture bukan pengganti uji live.

## Artifact beku

- `oracle/rizal-agent-hq-v2`: binary v2 yang berjalan di oracle.
- `oracle/web-v2/`: UI statis v2.
- `oracle/unit-v2.txt`: unit systemd v2 + override, nilai secret diganti
  `<redacted>` (aturan repo: secret tidak boleh masuk file atau commit).
- `oracle/version-v2.txt`: keluaran apa adanya dari
  `rizal-agent-hq --version`. v2 tidak punya flag versi: perintah itu menyalakan
  server dan panik karena port 8900 dipakai oracle yang hidup. Rekaman ini adalah
  bukti perilaku v2, bukan nomor versi.
- `fixtures/state-<UTC timestamp>.json`: 200 tangkapan
  `GET http://127.0.0.1:8900/api/state`, interval 6 detik selama 1200 detik.
- `fixtures/evidence-<UTC timestamp>/`: bukti mentah untuk 20 sampel pertama:
  `ps.txt` (tanpa argumen command line), `omp-stats.txt`, `tmux.txt`,
  `docker.txt`, `git.txt`, `sessions.txt` (nama file sesi, bukan isinya).
- `fixtures/MANIFEST.sha256`: checksum semua artifact di `oracle/` dan
  `fixtures/`, relatif terhadap `tools/parity/`.

`freeze.sh --duration 1200 --interval 6` menghasilkan artifact di atas.
`verify.sh` memeriksa checksum, jumlah state tepat 200, dan jumlah evidence
tepat 20. Jalankan `bash tools/parity/tests/run.sh` untuk menguji logika
verifikasi terhadap fixture sintetis.

## Aturan perbandingan

1. Petakan id v2 ke subject v3 memakai `aliases.json`; id yang tidak dikenal
   pada kandidat dipertahankan apa adanya, bukan dibuang.
   - `omp` → `agent:omp`, `cursor` → `agent:cursor`, `Muse` → `agent:muse`
   - `mcp` → `agent:mcp/*`, `omp-workers-agg` → `agent:omp/worker/*`
   - `coding-agents` → `agent:*`
2. Urutkan agent berdasarkan id sebelum membandingkan.
3. Buang field waktu tampilan (relative time, label lokal) dari kedua sisi.
4. Bulatkan usage ke 3 angka signifikan; bandingkan total global dalam jendela
   waktu yang sama (toleransi ±15%).
5. Normalisasi status ke enam nilai: `bekerja`, `santai`, `offline`, `error`,
   `unconfigured`, `unknown`.
6. Bandingkan alert berdasarkan `(rule_id, subject, level)`, bukan id acak.
7. Cakupan: setiap agent v2 harus punya padanan v3. Agent tambahan hanya
   diterima bila ada bukti runtime di evidence (proses atau file sesi).
8. Status harus Cocok pada ≥90% sampel untuk agent yang berpasangan.
