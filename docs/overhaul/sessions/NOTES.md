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

## 2026-11-17 — BLOCKER S03 setelah dua fix rounds

S03 dihentikan sesuai batas dua siklus perbaikan. Jangan mulai S04 sebelum
temuan ini diperbaiki, dites RED→GREEN, direview, dan gate S03 lulus.

- Siklus 1: reviewer menemukan PatchRing didorong dua kali dari router dan WS
  (`068fc7a`, `8bd499f` memperbaiki sumber push dan mapping replay).
- Siklus 2: `86a86c8` memperbaiki proxy guard WS, subscribe-before-replay WS,
  event patch, backlog, disconnect, status kebab-case, SSE payload/cursor,
  ring continuity, dan Axum 0.8; `dd14e93` mengunci Cargo.lock; `23808e4`
  formatting.
- Gate lokal terkini: `cargo fmt --all -- --check`, `cargo clippy -p hq-store
  -p hq-api --all-targets -- -D warnings`, `cargo test -p hq-store -p hq-api`
  exit 0 (20 passed). Gate hijau tidak menutup kontrak yang belum terpenuhi.
- Reviewer final (`UnitedNarwhal`) tetap `CHANGES_REQUESTED`:
  1. `crates/hq-api/src/router.rs:36`: SSE route tidak memakai auth/proxy guard;
     loopback peer dari reverse proxy dapat membaca stream tanpa kredensial.
  2. `crates/hq-api/src/sse.rs:37-42`: receiver dibuat sesudah replay (race
     replay→live); backlog >1000 dan read error bisa terlewat tanpa snapshot.
  3. `crates/hq-api/src/ws.rs:142-147` dan `sse.rs`: entity patch tidak punya
     discriminator `type`/bentuk API lengkap, hanya id untuk alert/usage/
     connector/approval/task.
  4. `crates/hq-store/src/lib.rs:137-143`: `AgentDisappeared` fallback ke
     `Status::Bekerja`, bukan Offline/deletion.
  5. `crates/hq-api/src/ws.rs:46-49`: kegagalan `read_since` ditelan menjadi
     empty replay; `since=0` bisa hello pada seq terkini tanpa state.
  6. `crates/hq-api/tests/sse.rs:31-35`: tes endpoint hanya connect/drop TCP;
     tidak mengirim HTTP atau mengassert SSE. Tes regresi final juga tidak ada
     untuk proxy, patch Alert/Usage, race, backlog, disconnect, cursor, dan ring
     gap yang diperbaiki.
- `docs/overhaul/sessions/S03.md` `## Hasil` dan tag `s03-done` sengaja belum
  diisi/dibuat. S04 belum dimulai.
