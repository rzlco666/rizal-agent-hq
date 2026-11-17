# Catatan Eksekusi Overhaul v3

## 2026-11-17 — Fakta lingkungan yang memengaruhi S25

- Oracle v2 hidup di `127.0.0.1:8900`, binary aktual
  `/home/rizal/rizal-agent-hq/bin/rizal-agent-hq` (bukan path di bawah
  `~/projects`), unit `~/.config/systemd/user/rizal-agent-hq.service`.
- Count DB v2 aktual saat eksekusi dimulai (dibaca read-only):
  `snapshots=34450`, `notion_docs=220`, `notion_sources=3`,
  `llm_summaries=103`, `alert_state=5`, `portfolio_snapshots=0`.
  Rencana (`00-design.md` §16 / `09-roadmap.md` §3.2) menulis 33436 snapshot
  dan 102 ringkasan — angka itu baseline saat dokumen dibuat, bukan angka DB
  hidup. S25 wajib memigrasikan **semua** baris sumber aktual dan membuktikan
  jumlahnya; jangan memalsukan angka dokumen dengan membuang baris.
- `pnpm`/`corepack` di PATH awal berasal dari `/mnt/d/laragon/...` (Windows).
  S00 memasang pnpm 10 dari toolchain WSL; gate tidak boleh bergantung pada
  Node Windows.
- `cargo-deny` tidak terpasang awalnya; dipasang di `~/.cargo/bin` oleh
  integrator pada 2026-11-17.

## 2026-11-17 — S02 minor (catatan reviewer, tidak memblokir gate)

- Failpoint `test_fail_after_insert` tetap publik (tanpa `#[cfg(test)]`) karena
  tes integrasi di `tests/append.rs` dan `tests/recovery.rs` memanggilnya;
  `#[cfg(test)]` pada src/lib.rs tidak aktif saat kompilasi integration test.
  Failpoint tidak dipanggil di jalur produksi (hanya tes); field `fail_after_insert`
  di `StoreInner` hanya dibaca di `do_append`. Ini keputusan builder: failpoint
  = test-only API publik, bukan hidden cfg. Tag s02-done tetap.
- Rebuild shadow+swap memakai `std::fs::rename`; handle writer thread tetap
  menunjuk file lama (POSIX rename aman). WAL/shm lama berpotensi yatim pada
  beban produksi; layak diperbaiki sebelum S03 bila waktu. Tidak memblokir gate.
- Dependency `sha2 = 0.10` ditambahkan ke hq-store (floor README §2 menyebut
  sha2 0.10, sesuai). Tidak ada dependency lain di luar kontrak.

## 2026-11-17 — S03 blocker resolved

- Temuan final ditutup oleh commits `088ff3b` (bounded SQLite replay),
  `477ca12` (watermark concurrent write), `a0417bf` (typed event deltas),
  `bff8e9c` (future SSE cursor watermark), serta follow-up fixes/tests
  `5cd3369`, `fc9e0b0`, `a062b49`, `ce47da4` dan prior review commits.
- Full review `EasternStingray` menemukan hanya future-cursor watermark;
  scoped reviewer `RealisticAntelope` APPROVE setelah `bff8e9c`.
  Reviewer sebelumnya mengonfirmasi SSE auth/cursor/gap, WS replay/lag/error,
  PatchRing continuity, task/log/alert event deltas, serta Axum 0.8.
- Gate integrator pada current code: fmt/clippy/tests `hq-store` + `hq-api`
  exit 0; 49 passed, 14 suites.
- RED evidence tercatat pada history S03FinalFix/S03LiveGap/S03BoundedReplay:
  backlog tepat 1000, lag forwarder/WS, cursor `i64::MAX`, task/log delta,
  malformed row di luar read limit, archive read error, dan future cursor.
  Bukti diringkas di `docs/overhaul/evidence/S03.md`.
- TDD urutan awal memiliki deviasi: tests awal dikomit setelah feature commits;
  RED/GREEN untuk regressions dan fixes berikutnya dibuktikan dan dicatat.
- `docs/overhaul/sessions/S03.md` Hasil telah diisi; tag `s03-done` dibuat
  setelah evidence commit. S04 menjadi sesi berikutnya.
