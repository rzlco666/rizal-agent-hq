# ADR-0003 — Frontend Svelte 5 dan Threlte

## Konteks

UI v2 adalah HTML statis plus dua file JavaScript tanpa tipe, i18n, atau
sistem komponen. v3 membutuhkan shell operasional, detail agent, dan view 3D
dalam satu binary tanpa CDN saat runtime. `00-design.md` §8 K3, §14 dan
`README.md` §2 menetapkan stack dan batasnya.

## Keputusan

- SvelteKit 2 + Svelte 5, Vite 7, TypeScript 5.9 strict, Tailwind 4.
- Kantor 3D memakai Threlte 8 di atas Three 0.180 dengan render-on-demand dan
  fallback 2D; scene memakai data nyata, bukan dekorasi.
- uPlot untuk time series; ikon lucide-svelte 16/18 stroke 1.5; tanpa emoji
  sebagai ikon.
- Teks UI lewat i18n ID default, EN, 中文; tidak ada string hardcoded. Waktu
  disimpan UTC ms dan tampil di `Asia/Jakarta` dengan `tabular-nums`.
- UI, font, dan ikon di-embed ke binary; tidak ada CDN saat runtime.
- Manajer paket: **pnpm 10.34.9** terpasang dari nvm WSL (Node v26.2.0),
  `which pnpm` → `/home/rizal/.nvm/versions/node/v26.2.0/bin/pnpm`.
  pnpm Windows di `/mnt/d/laragon/...` tidak dipakai. Lockfile dikomit dan
  pemasangan memakai `ignore-scripts`.

## Konsekuensi

- Satu binary dapat disalin tanpa aset eksternal, dan CI tidak bergantung Node
  Windows.
- Bundle dan anggaran render 3D perlu dijaga oleh gate visual/perf.
- Semua teks baru harus masuk katalog tiga locale sebelum dipakai.

## Alternatif ditolak

- React/Next: menambah runtime dan tidak sesuai kontrak desain yang disetujui.
- Melanjutkan HTML/JS statis v2: tidak ada tipe, komponen, atau i18n.
- Menyajikan Three/font dari CDN: melanggar requirement offline saat runtime.
- Memakai pnpm `/mnt/d`: build bergantung lingkungan Windows dan versi yang
  tidak terkunci.
