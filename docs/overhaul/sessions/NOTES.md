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
