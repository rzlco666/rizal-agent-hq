# 05 — Design System

**Status:** siap dieksekusi. Dokumen ini mengikat S07 dan semua sesi UI sesudahnya.
**Spec induk:** [`00-design.md`](00-design.md) §13–14, [`01-architecture.md`](01-architecture.md) §5.
**Referensi lain:** [`08-testing-gates.md`](08-testing-gates.md) §7, [`10-alerts.md`](10-alerts.md) §7.

Dokumen ini tidak mengulang kontrak API atau model data. Ia menetapkan arah
visual, token, tipe, komponen, layout, copy, dan kriteria uji yang dipakai
`06-frontend.md`.

---

## 1. Audit v2 (unslop)

Bukti diambil dari `web/index.html`, `web/app.js`, dan `docs/screenshot.png`
pada v2.5. Satu temuan, satu baris.

| Temuan | Bukti | Akibat |
|---|---|---|
| Emoji sebagai ikon dan status | `web/app.js:104,108,169,171,599,626,629` (`✨`, `✅`, `⏳`) | Status tidak terbaca mesin, tidak konsisten lintas platform, dan terlihat seperti catatan tempel. |
| Pill soup | 39 aturan `border-radius:99px` di `web/index.html` (nav, badge, search, kamera, zona, tombol, status, toggle) | Semua kontrol punya bobot visual sama; tidak ada hierarki aksi. |
| Label uppercase ber-tracking | `.pcard h2`, `.dhead h2` (`text-transform:uppercase;letter-spacing:.13em`), `.brand h1` (`.05em`), `.ver` | Label berteriak, sulit dibaca, dan melanggar aturan sentence case. |
| Version badge | `web/index.html:309` (`v2.5` dalam pill indigo) | Chrome memakai warna merek untuk informasi yang tidak berguna bagi keputusan. |
| Scrollbar ganda | `body{overflow:hidden}` + `aside#panel` (`:126`) + `.dbody` dua kolom (`:165`) + `#grid2d` (`:92`) + `.overlay` (`:192`) + `#drawer` (`:178`) | Tiga track scrollbar terlihat pada screenshot (rail, tugas, log); lima region scroll di DOM, tanpa hierarki. |
| Bahasa campur | `memuat…`, `menghubungkan…`, `Cari agen atau tim…`, `Tutup` di HTML; `LIHAT MENTAH`, `Usage`, `Performance Workspace`, `Model & Provider` di katalog | String tidak melewati i18n dan register bahasa berubah di tengah layar. |
| Chip melayang di atas 3D | `.alabel`, `#stageCtl`, `#hint`, `#bubbleTip` (`web/index.html:70–103`) | Diorama tertutup kartu putih; label menempel permanen, bukan saat hover atau seleksi. |
| Dependensi CDN | `web/index.html:438` memuat `three@0.160.0` dari `unpkg.com` | Aplikasi tidak offline; versi runtime berubah di luar lockfile. |
| Kepadatan tidak konsisten | 21 aturan `font-size:9–11px`; `9px` rolepill, `9.5px` versi, `12.5px` nav, `14px` body, `17px` KPI | Mikroteks tidak terbaca di 1440px; angka dan label bersaing. |
| Aksen indigo/violet di luar dunia | `--indigo:#4f46e5`, `#4338ca`, `#eef2ff` pada nav, pill peran, tombol aktif | Warna merek dipakai sebagai dekorasi, bukan makna. |
| Ikon tidak satu set | `.ic{width:15px;stroke-width:2}`; campuran simbol SVG buatan dan emoji | Stroke dan ukuran tidak konsisten dengan aturan 16/18px, stroke 1,5. |
| Gerak berulang tanpa alasan | `@keyframes pulse` pada `.wsdot.on`; auto-orbit kamera | Animasi berdenyut dan orbit terus-menerus mengalihkan perhatian dari data. |
| Chrome tembus pandang | `backdrop-filter:blur(6px)` pada empat kontrol melayang | Kontras tidak stabil di atas scene dan menambah biaya render. |

Kesimpulan audit: slop berat, bukan polish. Arah visual dan seluruh chrome
diganti; fitur, urutan informasi, dan data tetap.

---

## 2. Arah (dikunci dari §13.1)

| Sumbu | Putusan | Konsekuensi implementasi |
|---|---|---|
| Product | Ruang kendali satu orang untuk setiap agent AI: apa yang dikerjakan, apa yang butuh keputusan, berapa biayanya | Setiap layar menjawab salah satu dari tiga pertanyaan itu; tidak ada panel dekoratif. |
| Character | Instrumented, quiet, a little warm | Grafit hangat, satu aksen, angka presisi, tanpa glow dan gradient. |
| World | Meja operasi shift malam: grafit hangat, netral pasir, cahaya sinyal; permukaan putih kertas saat siang | Light = day watch, dark = night shift. Warna hanya dari ramp netral + satu aksen + semantik status. |
| Moments | `/office` sebagai diorama yang benar-benar diterangi; `/` sebagai ledger live dengan angka presisi dan attention rail | Dua tempat ini boleh kaya material dan detail; layar lain tenang. |
| Signature | **Attention rail**: satu tepi tetap tempat semua yang butuh kamu mendarat, posisi sama di setiap layar inti | Rail ada di shell, bukan komponen per halaman; posisi, lebar, dan urutannya tidak berubah. |
| Bukan | Gradient ungu/biru, glow, emoji ikon, pill soup, label uppercase ber-tracking, badge `New ✨`, angka tanpa satuan | Dilarang di review; temuan otomatis masuk daftar tolak. |

Satu aksen (`attention`) tidak pernah mengisi tombol. Primary selalu `ink-12`.
Warna semantik hanya untuk status dan hasil, tidak untuk ranking atau dekorasi.

---

## 3. Token

Tema dipasang sebagai `data-theme="day"` atau `data-theme="night"` pada
`<html>`. Default `day`; pilihan `system` menyelesaikan `prefers-color-scheme`
saat boot lalu menulis tema eksplisit. Tidak ada `prefers-color-scheme` murni di
CSS, supaya tema pilihan pengguna tidak bertabrakan dengan tema sistem.

```css
:root,
:root[data-theme="day"] {
  color-scheme: light;

  /* Permukaan */
  --ground: #F4F1EA;
  --surface: #FFFFFF;
  --raised: #FFFFFF;
  --well: #ECE8DF;
  --scrim: rgb(23 22 20 / 32%);

  /* Garis */
  --hairline: rgb(23 22 20 / 10%);
  --hairline-strong: rgb(23 22 20 / 18%);
  --control-edge: #6F6A62;

  /* Teks */
  --ink-12: #171614;
  --ink-11: #55514A;
  --ink-10: #6F6A62;

  /* Makna */
  --attention: #A65A00;
  --attention-tint: #FCEBD6;
  --critical: #B3261E;
  --success: #1B6B45;
  --info: #245C8A;
  --focus: #55514A;

  /* Interaksi */
  --hover: rgb(23 22 20 / 4%);
  --pressed: rgb(23 22 20 / 8%);
  --primary-bg: #171614;
  --primary-fg: #F4F1EA;
  --primary-hover: #2A2824;
  --on-critical: #FFFFFF;

  /* Bayangan: ring + drop crisp, tanpa blur besar */
  --shadow-color: 23 22 20;
  --shadow-ring: 0 0 0 1px var(--hairline);
  --shadow-card: var(--shadow-ring),
    0 1px 2px rgb(var(--shadow-color) / 8%),
    0 3px 4px -2px rgb(var(--shadow-color) / 6%);
  --shadow-button: var(--shadow-ring),
    0 1px 2px rgb(var(--shadow-color) / 10%);
  --shadow-menu: var(--shadow-ring),
    0 4px 8px -4px rgb(var(--shadow-color) / 16%);
  --shadow-modal: var(--shadow-ring),
    0 6px 8px -4px rgb(var(--shadow-color) / 18%);
  --shadow-sheet: 0 -1px 2px rgb(var(--shadow-color) / 8%),
    0 -8px 16px -8px rgb(var(--shadow-color) / 18%);

  /* Bentuk */
  --radius-control: 6px;
  --radius-card: 8px;
  --radius-modal: 12px;
  --radius-keycap: 4px;
  --radius-dot: 999px;

  /* Jarak: grid 4px */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* Gerak */
  --ease: cubic-bezier(.23, 1, .32, 1);
  --dur-1: 150ms;
  --dur-2: 200ms;
  --dur-3: 240ms;
  --dur-4: 300ms;

  /* Tipografi */
  --font-cjk: "Noto Sans SC", "Source Han Sans SC", "Microsoft YaHei",
    "PingFang SC", sans-serif;
  --font-sans: "IBM Plex Sans", var(--font-cjk), system-ui, sans-serif;
  --font-mono: "IBM Plex Mono", ui-monospace, "SFMono-Regular", monospace;
}

:root[data-theme="night"] {
  color-scheme: dark;

  --ground: #0C0C0B;
  --surface: #151514;
  --raised: #1D1D1B;
  --well: #101010;
  --scrim: rgb(0 0 0 / 72%);

  --hairline: rgb(242 239 233 / 12%);
  --hairline-strong: rgb(242 239 233 / 20%);
  --control-edge: #8E8980;

  --ink-12: #F2EFE9;
  --ink-11: #B5B0A7;
  --ink-10: #8E8980;

  --attention: #F2A63C;
  --attention-tint: #3A2A12;
  --critical: #FF6B5E;
  --success: #4CC38A;
  --info: #6FA8DC;
  --focus: #B5B0A7;

  --hover: rgb(242 239 233 / 5%);
  --pressed: rgb(242 239 233 / 9%);
  --primary-bg: #F2EFE9;
  --primary-fg: #0C0C0B;
  --primary-hover: #E4E0D8;
  --on-critical: #0C0C0B;

  --shadow-color: 0 0 0;
  --shadow-ring: 0 0 0 1px var(--hairline-strong);
  --shadow-card: var(--shadow-ring), 0 1px 2px rgb(0 0 0 / 60%);
  --shadow-button: var(--shadow-ring), 0 1px 2px rgb(0 0 0 / 60%);
  --shadow-menu: var(--shadow-ring), 0 4px 8px -4px rgb(0 0 0 / 70%);
  --shadow-modal: var(--shadow-ring), 0 6px 8px -4px rgb(0 0 0 / 80%);
  --shadow-sheet: 0 -1px 2px rgb(0 0 0 / 50%),
    0 -8px 16px -8px rgb(0 0 0 / 70%);
}
```

Aturan pemakaian token:

- `ground` adalah latar aplikasi. `surface` untuk panel dan tabel. `raised`
  hanya untuk satu unit yang benar-benar naik di atas permukaannya; light
  membedakannya lewat ring + drop, dark lewat satu langkah terang.
- `well` untuk field, track meter, keycap, dan area tenggelam. Teks di atas
  `well` hanya `ink-12` atau `ink-11`.
- `hairline` untuk pemisah dan tepi kartu; `control-edge` hanya untuk batas
  kontrol yang harus terbaca (field, select, switch, checkbox).
- `attention` untuk hal yang menunggu Rizal dan fokus grafik; tidak untuk
  tombol, navigasi aktif, atau header.
- `critical`, `success`, `info` hanya untuk status, hasil, dan angka yang
  berubah karena status. Tidak pernah untuk kategori atau dekorasi.
- `focus` adalah `ink-11`; ring fokus 2px dengan offset 2px.

### 3.1 Kontras

Angka §13.2 dipertahankan sebagai target tercatat.

| Pasangan | Rasio | Peran |
|---|---:|---|
| `ink-12` / `ground` | 16,03:1 | teks utama, isi tabel |
| `ink-11` / `ground` | 6,99:1 | teks sekunder |
| `ink-10` / `ground` | 4,76:1 | meta, timestamp, label kecil |
| `attention` / `ground` | 4,56:1 | teks yang menunggu keputusan |
| tombol primary | 16,60:1 | teks `primary-fg` di atas `primary-bg` |

Angka 16,60:1 adalah catatan §13.2; pengukuran ulang pasangan token aktual
menghasilkan 16,03:1. Keduanya jauh di atas ambang. Yang mengikat adalah
4,5:1 untuk setiap pasangan teks.

Pengukuran ulang pasangan token aktual (dipakai sebagai pagar review):

| Pasangan | Light | Dark |
|---|---:|---:|
| `ink-12` / `surface` | 18,08:1 | 15,92:1 |
| `ink-11` / `surface` | 7,89:1 | 8,47:1 |
| `ink-10` / `surface` | 5,37:1 | 5,26:1 |
| `attention` / `surface` | 5,14:1 | 8,96:1 |
| `critical` / `ground` | 5,79:1 | 7,00:1 |
| `success` / `ground` | 5,75:1 | 8,83:1 |
| `info` / `ground` | 6,26:1 | 7,74:1 |
| `focus` / `ground` | 6,99:1 | 9,07:1 |
| `focus` / `surface` | 7,89:1 | 8,47:1 |
| `control-edge` / `ground` | 4,76:1 | 5,63:1 |
| teks putih di `critical` | 6,54:1 | — |
| `ink-12` di `attention-tint` | 15,49:1 | 12,04:1 |
| `attention` di `attention-tint` | **4,41:1** | 6,77:1 |
| `ink-10` di `well` | **4,39:1** | 5,47:1 |

Pagar yang mengikat:

1. Setiap pasangan teks harus ≥4,5:1. Ring fokus dan batas kontrol ≥3:1.
2. `attention` tidak boleh menjadi warna teks di atas `attention-tint` pada
   tema light (4,41:1 gagal). Badge memakai tint + `ink-12` + dot hue.
3. `ink-10` tidak boleh dipakai di atas `well` pada tema light (4,39:1 gagal).
   Gunakan `ink-11`.
4. Teks berwarna semantik hanya di atas `ground` atau `surface`; di atas tint,
   teks selalu `ink-12` atau `ink-11`.
5. Hover dan selected tidak boleh menurunkan kontras teks di bawah ambang;
   selected memakai `attention-tint` + teks `ink-12`, bukan teks attention.

---

## 4. Tipografi

IBM Plex Sans 400/500/600; IBM Plex Mono 400/500 untuk angka, ID, kode, dan
satuan. Semua di-embed sebagai subset WOFF2; tidak ada CDN.

| Peran | Ukuran / line-height | Berat | Keluarga | Tracking | Pakai |
|---|---|---|---|---:|---|
| Display | 32/40 | 500 | Sans | −0,02em | tanggal atau periode di Today; satu per layar |
| Seksi | 20/28 | 500 | Sans | 0 | judul halaman, judul seksi besar |
| Judul kartu | 16/24 | 500 | Sans | 0 | judul panel, dialog |
| Body | 13/20 | 400 | Sans | 0 | isi, sel tabel, paragraf |
| Body kuat | 13/20 | 500 | Sans | 0 | nilai utama, label kontrol |
| Meta | 12/16 | 400 | Sans | 0 | timestamp, satuan, catatan |
| Label tabel | 12/16 | 500 | Sans | 0 | header tabel, sentence case |
| Angka | 13/20 | 500 | Mono | 0 | angka tabel, biaya, token |
| Angka besar | 24/32 | 500 | Mono | 0 | KPI |
| Sumbu grafik | 11/16 | 400 | Mono | 0 | tick dan tooltip grafik |
| Keycap | 11/16 | 500 | Mono | 0 | `kbd` |
| Body ponsel | 15/22 | 400 | Sans | 0 | isi dan kontrol di <768px |
| Field ponsel | 16/24 | 400 | Sans | 0 | input di layar sentuh |

Aturan:

- Sentence case di semua tempat. Tidak ada `text-transform:uppercase`, tidak
  ada Title Case, tidak ada italic.
- Tidak ada tracking positif. Display memakai −0,02em; label tombol −0,01em.
- `font-variant-numeric: tabular-nums` pada setiap angka, termasuk KPI, tabel,
  grafik, keycap, dan hitungan di badge.
- Angka memakai Plex Mono; teks lain memakai Plex Sans. Satu tabel tidak
  mencampur dua keluarga di kolom teks.
- Tidak ada teks di bawah 11px. 11px hanya untuk tick grafik dan keycap.
- Teks bebas dipotong pada 32 karakter dengan ellipsis dan `title`; nilai
  hilang memakai `—`, bukan `0` atau `N/A`.
- Ponsel: lantai body 15px, field 16px. Tidak ada teks 12px untuk instruksi.
- Default CJK memakai `--font-cjk` (font sistem) karena IBM Plex tidak memuat
  glyph Han; angka dan ID tetap Plex Mono. Tidak ada unduhan font saat runtime.
  Builder boleh mengganti default ini dengan subset Noto yang di-embed bila
  budget §14.2 design tetap terpenuhi (lihat “Diserahkan ke builder”).

---

## 5. Jarak, bentuk, dan state

### 5.1 Jarak

Grid 4px. Skala yang dipakai: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64.
Aturan grouping:

- Di dalam kontrol: 6–8px horizontal, 4px vertikal.
- Antar item dalam grup: 8–12px.
- Antar seksi: 24–32px.
- Padding kartu: 16px; panel besar 20–24px; tepi halaman 24–32px.
- Baris nav: 32px tinggi, 2–4px jarak, ikon pada satu kolom, label satu baris.
- Header halaman: 8px di bawah judul untuk deskripsi; 24px ke konten.
- Angka rata kanan pada satu kolom; label mulai pada satu garis; ikon tidak
  pernah menggeser label dengan margin ganjil.

### 5.2 Radius, garis, bayangan

| Elemen | Radius | Garis | Bayangan |
|---|---|---|---|
| Kontrol (button, field, select, tab aktif) | 6px | `control-edge` 1px untuk field/select; tombol memakai ring | `--shadow-button` |
| Kartu, panel, tabel | 8px | `hairline` 1px | `--shadow-card` |
| Modal | 12px | `hairline` | `--shadow-modal` |
| Sheet ponsel | 12px atas | `hairline` | `--shadow-sheet` |
| Keycap | 4px | `hairline` | tidak ada |
| Dot status | penuh | tidak ada | tidak ada |
| Badge | 4px | ring tipis warna teksnya | `--shadow-button` |

Nested radius = radius luar − padding, minimum 4px. Tidak ada radius 16px+
untuk kontrol. Tidak ada `border-radius:999px` untuk tombol, nav, search, atau
badge; hanya dot dan track meter yang penuh.

Bayangan selalu ring + drop 1px; tidak ada blur >8px, tidak ada colored glow,
tidak ada lit top edge di dark.

### 5.3 State

| State | Aturan visual | Aturan perilaku |
|---|---|---|
| Hover | `--hover` overlay atau satu langkah surface; warna teks tidak berubah | Hanya pointer halus; tidak menempel di sentuh |
| Pressed | `scale(.97)` 150ms; warna `--pressed` | Tidak menggeser layout |
| Focus-visible | ring `--focus` 2px, offset 2px, selalu terlihat | Hanya keyboard; tidak dihapus tanpa pengganti |
| Selected | `attention-tint` + teks `ink-12` + rule kiri 2px `attention` | Terlihat tanpa warna saja (rule + teks) |
| Disabled | opacity .45; kursor `not-allowed`; tetap terbaca | `aria-disabled`, tidak menerima fokus kecuali perlu diumumkan |
| Loading | label tetap, spinner 14px di kiri, lebar tidak berubah | `aria-busy`, klik diabaikan, tidak ada toast ganda |
| Error | ring `critical` 1px + pesan 12/16 `critical` | `aria-invalid`, `aria-describedby`, fokus ke field pertama |
| Read-only | `well` + teks `ink-11`; tanpa ring fokus | Tetap bisa dipilih dan dibaca |
| Empty | dua baris, ikon 18px, satu aksi sekunder | Tidak ada data palsu |

Transisi state memakai `--dur-1` dan `--ease`, hanya warna, opacity, atau
transform. Tidak ada animasi pada angka, lebar, tinggi, atau posisi.

---

## 6. Komponen

Semua komponen berada di `web/src/lib/components/ui/` kecuali tiga komponen
domain (`AttentionRail`, `AgentMonogram`, `LedgerChart`) di
`web/src/lib/components/domain/`. Semua teks melalui `t()`; tidak ada string
hardcoded. Semua tombol `type="button"` kecuali submit form.

Kontrak bersama:

- Menerima `class?: string` dan atribut HTML yang relevan.
- Ikon 16px (18px hanya empty state dan header halaman), stroke 1,5,
  `currentColor`.
- Satu primary per permukaan. Destructive tidak pernah primary sampai dialog
  konfirmasi.
- Fokus selalu memakai ring token, tidak pernah `outline:none` tanpa pengganti.
- Ukuran sentuh minimum 44×44px di ponsel, termasuk tombol 28px yang diberi
  hit area tak terlihat.

### 6.1 Button

`web/src/lib/components/ui/Button.svelte`

Anatomi: ikon kiri opsional 16px, label 13/20 500, ikon kanan opsional; gap 8;
padding 0 14px (12px bila ada ikon); tinggi 36px (`md`), 32px (`sm`), 44px
(`lg`, ponsel); radius 6.

```ts
import type { Component, Snippet } from 'svelte';
export interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  disabled?: boolean;
  icon?: Component;
  iconAfter?: Component;
  type?: 'button' | 'submit';
  onclick?: (event: MouseEvent) => void;
  children: Snippet;
  class?: string;
}
```

Varian: `primary` = fill `primary-bg`, teks `primary-fg`, hover
`primary-hover`, ring surface 1px di light / ring 0,5px + drop di dark;
`secondary` = `raised` + `shadow-button`, teks `ink-12`; `ghost` = transparan,
teks `ink-11`, hover `--hover`; `destructive` = teks `critical` di surface,
hanya berubah menjadi fill `critical` + `on-critical` di dalam modal konfirmasi.

State: hover, pressed, focus-visible, disabled, loading. Loading mengganti ikon
kiri dengan spinner dan mempertahankan label; `aria-busy="true"`.

Keyboard: native Enter/Space. A11y: label teks selalu ada; ikon tidak menjadi
satu-satunya isi (gunakan `IconButton`).

### 6.2 IconButton

`web/src/lib/components/ui/IconButton.svelte`

36×36 (32 `sm`), radius 6, ikon 16; hit area 44×44 di ponsel. `label` wajib.

```ts
export interface IconButtonProps {
  icon: Component;
  label: string;                 // aria-label, juga title bila perlu
  variant?: 'ghost' | 'secondary';
  size?: 'sm' | 'md';
  pressed?: boolean;
  disabled?: boolean;
  onclick?: (event: MouseEvent) => void;
}
```

State: hover, pressed, focus-visible, disabled, `aria-pressed` untuk toggle.
Keyboard: Enter/Space. A11y: `title` hanya pelengkap; `aria-label` tidak boleh
kosong dan harus berupa aksi, bukan ikon.

### 6.3 Field (input + label)

`web/src/lib/components/ui/Field.svelte`

Anatomi: label 13/20 `ink-11` di atas 6px; kontrol `well`, 1px
`control-edge`, radius 6, tinggi 36 (ponsel 44, teks 16); hint/error 12/16 di
bawah 4px. Placeholder bukan label.

```ts
export interface FieldProps {
  label: string;
  value: string;
  type?: 'text' | 'search' | 'number' | 'password' | 'url' | 'email';
  placeholder?: string;
  hint?: string;
  error?: string;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  mono?: boolean;
  name?: string;
  autocomplete?: string;
  oninput?: (value: string) => void;
  onblur?: () => void;
  onsubmit?: () => void;
}
```

State: hover border `ink-11`; focus `control-edge` `ink-11` + ring 2px; error
ring `critical` + pesan `critical`; disabled opacity .45; read-only `well` +
`ink-11`. Validasi saat blur dan submit, bukan setiap karakter.

Keyboard: native; `Enter` men-submit form; `Escape` hanya mengosongkan search.
A11y: `<label for>`, `aria-describedby` untuk hint/error, `aria-invalid`,
penanda wajib berupa teks "wajib", bukan asterisk sendiri.

### 6.4 Select

`web/src/lib/components/ui/Select.svelte`

Select native yang ditata, bukan listbox buatan. Tinggi 36/44, radius 6,
`well` + `control-edge`; chevron 16px. Opsi dikelompokkan bila perlu.

```ts
export interface SelectOption { value: string; label: string; disabled?: boolean; }
export interface SelectProps {
  label: string;
  value: string;
  options: SelectOption[];
  hint?: string;
  error?: string;
  disabled?: boolean;
  onchange?: (value: string) => void;
}
```

State: sama dengan Field; buka/tutup memakai perilaku native. Keyboard: native
(Arrow, Home/End, Enter, Escape). A11y: label eksplisit; jangan mengandalkan
opsi pertama sebagai placeholder.

### 6.5 Tabs

`web/src/lib/components/ui/Tabs.svelte`

Baris 40px, tab 13/20 500, aktif `ink-12` + underline 2px `ink-12`; tidak aktif
`ink-10`, hover `ink-11`; hitungan memakai mono 12. Tab tidak berbentuk pill.
Garis bawah grup 1px `hairline`.

```ts
export interface TabItem { id: string; label: string; count?: number; disabled?: boolean; }
export interface TabsProps {
  tabs: TabItem[];
  value: string;
  ariaLabel: string;
  onchange?: (id: string) => void;
}
```

State: hover, selected, focus-visible, disabled. Keyboard: roving tabindex,
ArrowLeft/ArrowRight, Home/End; `Enter`/`Space` memilih. A11y:
`role=tablist/tab/tabpanel`, `aria-selected`, `aria-controls`; URL menyimpan
tab aktif di route yang punya `?tab=`.

### 6.6 Table

`web/src/lib/components/ui/Table.svelte`

Baris 38px desktop; sel 13/20; header 12/16 500 sentence case `ink-10`;
pemisah horizontal 1px `hairline`; kolom pertama `ink-12`; angka mono rata
kanan; tanpa zebra kecuali tabel panjang (>20 baris, selang-seling 2%).

```ts
export interface Column<T> {
  key: keyof T & string;
  label: string;
  align?: 'left' | 'right';
  mono?: boolean;
  width?: string;
  render?: Snippet<[T]>;
}
export interface TableProps<T> {
  rows: T[];
  columns: Column<T>[];
  rowKey: (row: T) => string;
  caption: string;
  loading?: boolean;
  empty?: Snippet;
  selectedKey?: string;
  onrowclick?: (row: T) => void;
  density?: 'compact' | 'default';
  sort?: { key: string; dir: 'asc' | 'desc' } | null;
  onsort?: (key: string) => void;
}
```

State: header, hover, selected (`attention-tint` + rule kiri 2px), focus row,
loading skeleton 8 baris, empty. Sort header berupa tombol dengan `aria-sort`.
Keyboard: `Tab` berpindah antar kontrol (link sel pertama, tombol sort);
`j`/`k` memindahkan fokus baris bila tabel berada di route yang mendaftarkan
shortcut daftar; `Enter` membuka baris terfokus. A11y: `<table>` semantik,
`<caption>` (boleh sr-only), `th scope="col"`, baris dapat dibuka lewat link di
sel pertama; `onrowclick` hanya pelengkap, bukan satu-satunya jalan. Ponsel:
tabel berubah menjadi daftar baris 56px (label/value), bukan tabel yang
diperkecil; teks panjang dipotong 32 karakter.

### 6.7 StatusDot

`web/src/lib/components/ui/StatusDot.svelte`

Dot 6px radius penuh + label 12/16 opsional. Pemetaan status:

| Status | Token |
|---|---|
| `bekerja` | `success` |
| `berpikir` | `info` |
| `menunggu-rizal` | `attention` |
| `santai` | `ink-10` |
| `offline` | `ink-10` dengan ring 1px `hairline-strong` |
| `error` | `critical` |

```ts
export interface StatusDotProps { status: Status; label?: string; size?: 6 | 8; }
```

Tidak berdenyut. A11y: dot `aria-hidden`; bila label tidak terlihat, sediakan
`sr-only` teks status. Warna tidak pernah satu-satunya pembeda; selalu ada
teks status di dekatnya atau di tooltip yang dapat difokus.

### 6.8 Badge

`web/src/lib/components/ui/Badge.svelte`

Tinggi 20, padding 0 6, radius 4, teks 12/16 500. Bukan tombol; bila dapat
diklik, gunakan Button ghost. Tone: `neutral` (well + `ink-11`), `attention`,
`critical`, `success`, `info` (tint + `ink-12` + dot 6px hue). Hitungan memakai
mono.

```ts
export interface BadgeProps {
  tone?: 'neutral' | 'attention' | 'critical' | 'success' | 'info';
  label: string;
  count?: number;
  dot?: boolean;
}
```

Tidak ada badge "baru", tidak ada emoji, tidak ada `beta`. Jangan memakai badge
untuk kategori yang tidak punya status. A11y: teks badge adalah teks nyata,
bukan `aria-label` pada dot; hitungan memakai `sr-only` satuan ("3 alert"),
dan badge tidak pernah menjadi tombol.

### 6.9 AttentionRail

`web/src/lib/components/domain/AttentionRail.svelte`

Signature. Lebar 320, menempel di tepi kanan di bawah top bar, latar `ground`,
garis kiri `hairline`. Header 40px: judul "Butuh kamu" + hitungan mono. Grup:
"Perlu keputusan" (approval, kedaluwarsa terdekat) lalu "Alert" (critical,
warning). Info tidak masuk rail. Maksimal 8 item terlihat; per agent maksimal
3 alert (aturan `10-alerts.md` §6–7); sisanya "+N". Item minimum 72px: dot
level, judul satu baris 13/20 `ink-12`, meta 12/16 `ink-10` (subjek + umur),
satu aksi utama ghost 28px dan menu.

```ts
export interface AttentionItem {
  id: string;
  kind: 'approval' | 'alert';
  level?: 'warning' | 'critical';
  title: string;
  meta: string;
  href?: string;
  minutesLeft?: number;
}
export interface AttentionRailProps {
  items: AttentionItem[];
  collapsed?: boolean;
  loading?: boolean;
  onopen: (item: AttentionItem) => void;
  onack?: (item: AttentionItem) => void;
  onsnooze?: (item: AttentionItem) => void;
}
```

State: default, hover, selected, collapsed (44px strip dengan ikon + hitungan),
loading (3 skeleton baris), empty (dua baris: "Tidak ada yang menunggu" /
"Alert baru muncul di sini."). Keyboard: setiap item dapat difokus; `Enter`
membuka; `a` ack dan `s` snooze hanya saat fokus di rail. A11y:
`<aside aria-label>`, hitungan di `aria-live="polite"` (bukan setiap patch),
item adalah link/tombol, bukan div. Ponsel: rail tidak dirender; top bar
menampilkan tombol hitungan yang membuka sheet.

### 6.10 AgentMonogram

`web/src/lib/components/domain/AgentMonogram.svelte`

Kotak 28px (20/32 tersedia), radius 6, `well`, inisial 12/16 mono 500
`ink-12`, dot status 6px di sudut kanan bawah dengan ring 2px `ground`. Tidak
ada warna per agent.

```ts
export interface AgentMonogramProps {
  name: string;
  status?: Status;
  size?: 20 | 28 | 32;
}
```

Inisial: dua grapheme pertama dari dua kata pertama; bila satu kata, dua
grapheme pertama; untuk nama CJK, dua karakter pertama. A11y: `aria-hidden`
bila nama tampil di sebelahnya; bila berdiri sendiri, `role="img"` +
`aria-label` berisi nama dan status.

### 6.11 LedgerChart

`web/src/lib/components/domain/LedgerChart.svelte`

Satu-satunya grafik uPlot. Tinggi 240 desktop, 160 ponsel, 320 hanya di
`/costs`. Header berisi legenda teks + sampel garis, bukan kotak warna.

```ts
export interface ChartPoint {
  ts_ms: number;
  cash_usd: number;
  shadow_usd: number;
  requests?: number;
}
export interface LedgerChartProps {
  points: ChartPoint[];
  series?: ('cash' | 'shadow')[];
  height?: 160 | 240 | 320;
  unit?: 'usd' | 'requests' | 'tokens';
  window?: '24h' | '7d' | '30d';
  state?: 'ready' | 'loading' | 'empty' | 'error';
  summary: string;
  onretry?: () => void;
}
```

State: ready, loading (skeleton setinggi grafik), empty (dua baris, sumbu
tidak digambar), error (dua baris + "Coba lagi"), stale (watermark "Data
terakhir 19.53"). A11y: `<figure>` dengan caption ringkasan, tombol "Tabel"
menampilkan `<table>` data yang sama, grafik dapat difokus dan digerakkan
dengan ArrowLeft/ArrowRight. Spesifikasi render ada di §8.

### 6.12 Drawer / Sheet

`web/src/lib/components/ui/Drawer.svelte`

Desktop: panel kanan 480px, penuh tinggi di bawah top bar, `surface`, garis
kiri `hairline`, `--shadow-sheet`; tanpa scrim. Ponsel: bottom sheet maksimal
85vh, radius 12px atas, grab handle 36×5 dengan hit area 44px, scrim.
Header 56px (judul 16/24 + IconButton tutup), body scroll, footer 64px dengan
garis atas `hairline`.

```ts
export interface DrawerProps {
  open: boolean;
  title: string;
  onclose: () => void;
  footer?: Snippet;
  children: Snippet;
  side?: 'right' | 'bottom';
}
```

State: terbuka, menutup, loading footer, error inline. Keyboard: `Escape`
menutup, fokus masuk ke elemen pertama, Tab terperangkap, fokus kembali ke
pemicu. A11y: `role="dialog"`, `aria-modal="true"`, `aria-labelledby`; scroll
body dikunci saat terbuka; grab handle adalah tombol tutup yang terlihat.

### 6.13 Modal

`web/src/lib/components/ui/Modal.svelte`

Konfirmasi 460px, form 620px, radius 12, `raised`, `--shadow-modal`, scrim.
Header 20/20/16, body 16, footer 12; aksi sekunder di kiri, aksi utama di
kanan. Fokus awal: field pertama untuk form; tombol batal untuk konfirmasi
destruktif.

```ts
export interface ModalProps {
  open: boolean;
  title: string;
  size?: 'confirm' | 'form';
  confirmLabel?: string;
  confirmTone?: 'primary' | 'destructive';
  busy?: boolean;
  onclose: () => void;
  onconfirm?: () => void;
  children: Snippet;
}
```

State: default, busy (tombol konfirmasi loading, tombol batal tetap aktif),
error inline. Keyboard: `Escape` menutup bila tidak busy; Tab terperangkap;
`Enter` tidak pernah men-submit aksi destruktif secara implisit. A11y:
`role="dialog"`, `aria-modal`, deskripsi konsekuensi terhubung dengan
`aria-describedby`. Konfirmasi destruktif menyebut objek dan akibatnya; untuk
aksi ireversibel (misalnya cabut device) pengguna mengetik nama objek sebelum
tombol aktif.

### 6.14 Toast

`web/src/lib/components/ui/Toaster.svelte` + `web/src/lib/stores/toast.svelte.ts`

Satu toast terlihat; antrean maksimal 3, toast sukses tertua dibuang. Posisi
bawah tengah, 24px dari bawah, lebar 320–480, `raised` + `--shadow-menu`.
Tone: `success` (ikon check 16, auto 4 detik), `critical` (ikon alert, tetap
sampai ditutup), `info` (auto 4 detik). Aksi opsional satu, misalnya "Lihat".

```ts
export interface ToastOptions {
  tone: 'success' | 'critical' | 'info';
  message: string;
  action?: { label: string; run: () => void };
  timeoutMs?: number;
}
export function showToast(options: ToastOptions): string;
export function dismissToast(id: string): void;
```

A11y: `role="status"` untuk sukses/info, `role="alert"` untuk critical; tidak
mengambil fokus; dapat ditutup dengan tombol dan `Escape`. Tidak ada toast
untuk copy yang berhasil (gunakan perubahan ikon), dan tidak ada toast untuk
setiap patch realtime.

### 6.15 CommandPalette

`web/src/lib/components/ui/CommandPalette.svelte`

Lebar 560, top 12vh, `raised` radius 12, `--shadow-modal`. Input 44px dengan
ikon search; hasil dikelompokkan (Navigasi, Agent, Aksi, Tampilan), maksimal 8
baris per grup; baris 40px. Skor: prefix > substring; recent action menang
dalam grup yang sama; tanpa fuzzy library.

```ts
export interface PaletteAction {
  id: string;
  label: string;
  group: 'navigate' | 'agent' | 'alert' | 'approval' | 'view' | 'setting';
  keywords?: string[];
  shortcut?: string;
  disabledReason?: string;
  run: () => void | Promise<void>;
}
export interface CommandPaletteProps {
  open: boolean;
  actions: PaletteAction[];
  recent: string[];
  onclose: () => void;
}
```

State: kosong ("Tidak ada hasil"), loading untuk aksi async, disabled dengan
alasan. Keyboard: ArrowUp/Down, Enter, Escape, Home/End; input selalu fokus.
A11y: `role="dialog"` + combobox/listbox, `aria-activedescendant`, label
eksplisit; aksi destruktif tidak pernah dijalankan dari palette tanpa dialog
konfirmasi.

### 6.16 EmptyState

`web/src/lib/components/ui/EmptyState.svelte`

Dua baris: keadaan dan tindakan. Ikon 18px `ink-10`; judul 13/20 `ink-11`;
instruksi 13/20 `ink-10`; maksimal satu aksi sekunder; lebar maksimal 360;
padding 32; tanpa ilustrasi dan tanpa emoji.

```ts
export interface EmptyStateProps {
  title: string;
  body: string;
  icon?: Component;
  action?: { label: string; onclick: () => void };
}
```

A11y: teks tidak diawali "Oops" atau "Maaf"; aksi adalah tombol nyata.

### 6.17 Skeleton

`web/src/lib/components/ui/Skeleton.svelte`

Blok `well` statis dengan radius 4/6, tanpa shimmer. Tinggi dan lebar harus
sama dengan konten akhir agar tidak ada layout shift.

```ts
export interface SkeletonProps {
  shape?: 'text' | 'block' | 'row';
  width?: string;
  height?: string;
  lines?: number;
}
```

A11y: `aria-hidden`; region pembungkus memakai `aria-busy="true"` dan label
"Memuat". Maksimal satu skeleton per region.

### 6.18 Keycap

`web/src/lib/components/ui/Keycap.svelte`

`<kbd>` 18px tinggi, min-width 18, padding 0 4, radius 4, `well` + hairline,
mono 11/16 `ink-11`; grup dengan pemisah spasi.

```ts
export interface KeycapProps { keys: string[]; label?: string; }
```

A11y: teks keycap adalah isi `kbd`; `label` menjelaskan aksi untuk pembaca
layar, bukan pengganti.

### 6.19 PageHeader

`web/src/lib/components/ui/PageHeader.svelte`

Judul halaman 20/28; deskripsi 13/20 `ink-11`; meta 12/16 `ink-10`; aksi di
kanan (ponsel: di bawah judul, lebar penuh). Tanpa border bawah dan tanpa
ikon seksi.

```ts
export interface PageHeaderProps {
  title: string;
  description?: string;
  meta?: string;
  actions?: Snippet;
}
```

A11y: satu `<h1>` per halaman; aksi memakai urutan tab setelah judul.

### 6.20 Section

`web/src/lib/components/ui/Section.svelte`

Judul 20/28 (level 2) atau 16/24 (level 3); deskripsi 13/20 `ink-11`; aksi di
kanan judul; jarak 32px atas dan bawah; tanpa ikon dan tanpa eyebrow.

```ts
export interface SectionProps {
  title: string;
  description?: string;
  level?: 2 | 3;
  actions?: Snippet;
  children: Snippet;
}
```

### 6.21 KpiFigure

`web/src/lib/components/ui/KpiFigure.svelte`

Label 12/16 `ink-10`; angka 24/32 mono 500 `ink-12` tabular; satuan 12/16
`ink-11`; delta 12/16 dengan pembanding eksplisit ("+12% vs kemarin"). Tanpa
sparkline, ikon, atau mini bar.

```ts
export interface KpiFigureProps {
  label: string;
  value: string;
  unit?: string;
  delta?: string;
  compare?: string;
  href?: string;
}
```

A11y: angka dan label dalam satu grup; bila `href`, seluruh kartu adalah link
dengan nama yang jelas.

### 6.22 Meter

`web/src/lib/components/ui/Meter.svelte`

Track 6px `well` radius penuh; fill `ink-12`; `attention` bila >80%;
`critical` bila ≥100%. Angka persentase di luar track, mono.

```ts
export interface MeterProps {
  value: number;
  max: number;
  label: string;
  showValue?: boolean;
}
```

A11y: `role="progressbar"` dengan `aria-valuenow/min/max` dan label; angka
tetap terlihat, bukan warna saja.

### 6.23 Switch

`web/src/lib/components/ui/Switch.svelte`

Desktop 40×24 dengan knob 20; ponsel 51×31 dengan knob 23; hit area 44.
Track aktif `ink-12`, nonaktif `well` + `control-edge`; knob `surface` dengan
ring.

```ts
export interface SwitchProps {
  checked: boolean;
  label: string;
  description?: string;
  disabled?: boolean;
  onchange?: (checked: boolean) => void;
}
```

State: on/off, hover, focus-visible, disabled. Keyboard: Space men-toggle;
label dapat diklik. A11y: `role="switch"` + `aria-checked`, deskripsi terhubung
`aria-describedby`; tidak ada animasi spring, knob bergerak 150ms dengan
`--ease`.

### 6.24 Pagination

`web/src/lib/components/ui/Pagination.svelte`

Di bawah tabel, di luar kartu tabel. Teks "1–25 dari 120" mono 12/16
`ink-10`; tombol Sebelumnya/Berikutnya secondary 32px. Tanpa nomor halaman
dan tanpa tombol di dalam kartu tabel.

```ts
export interface PaginationProps {
  page: number;
  pageSize: number;
  total: number;
  onchange: (page: number) => void;
}
```

A11y: `nav aria-label="Halaman"`; tombol disabled saat batas; fokus tetap di
tombol setelah pindah halaman.

---

## 7. Layout

### 7.1 Shell

| Bagian | Desktop ≥1280 | Tablet 768–1279 | Ponsel <768 |
|---|---:|---:|---:|
| Sidebar | 248px; 64px saat diciutkan | 64px ikon + tooltip | tidak ada; bottom nav 56px |
| Top bar | 52px | 52px | 52px |
| Attention rail | 320px, menempel kanan | drawer dari top bar | sheet dari top bar |
| Gutters halaman | 32px (≥1440), 24px (1024–1439) | 24px | 20px |
| Lebar konten | maksimal 1408px | maksimal 1024px | 100% |
| Body | 13/20 | 13/20 | 15/22 |
| Field | 36px | 36px | 44px, teks 16px |
| Target sentuh | 32px minimum | 44px untuk kontrol utama | 44px |

Sidebar: baris nav 32px, ikon 16px pada kolom 24px, label satu baris, jarak
4px; section label 12/16 `ink-10` dengan 24px di atas dan 8px di bawah; item
aktif `raised` + teks `ink-12` + rule kiri 2px `attention`; tidak ada pill.
Logo berupa wordmark teks, bukan gambar berbingkai.

Top bar: wordmark 13/20 500, pencarian global 36px (lebar 240 desktop), status
koneksi (dot + teks), tombol alert dengan hitungan, pemilih bahasa sebagai
teks `ID / EN / 中文` tanpa pill. Tidak ada version badge; versi hanya di
`/settings` bagian Tentang.

Ponsel: top bar berisi judul halaman, tombol kembali bila perlu, tombol
attention (hitungan), dan tombol palette. Bottom nav 56px berisi Hari ini,
Alert, Kantor, Biaya, Lainnya; "Lainnya" membuka sheet berisi Timeline,
Automations, Integrations, Knowledge, Settings. Konten diberi padding bawah
72px.

### 7.2 Layar

| Layar | Desktop | Ponsel |
|---|---|---|
| **Today** | PageHeader (tanggal sebagai display 32/40 + meta) → strip 4 `KpiFigure` (grid 4, tinggi 88, gap 16) → `Table` agent (min 320px, tumbuh) → split bawah: `LedgerChart` 240px (2fr) + daftar biaya per agent (1fr, bukan grafik kedua). Rail 320px di kanan. | Urutan: KPI 2×2 (72px per sel), tombol attention + hitungan, daftar agent (baris 56), chart 160, tautan biaya. Filter agent di sheet. |
| **Agent detail** | Kepala 72px: monogram 32, nama 20/28 + dot status, meta runtime/model/last seen 12/16, aksi kanan (pause/stop/overflow). Tabs 40px. Ringkasan 2 kolom (2fr aktivitas + 1fr meta/usage); tab lain full width. | Kepala bertumpuk, aksi di overflow, tabs dapat digulir horizontal dengan target 44px, tabel menjadi baris, log 44px per baris. |
| **Office** | Full-bleed di bawah top bar; bar kamera 44px sebagai bagian layout (bukan melayang); zone bar 48px docked di bawah; inspector 360px saat agent dipilih; rail disembunyikan. | Default 2D floor plan; toggle 3D di overflow; zone membuka sheet daftar agent; tidak ada kontrol melayang di atas canvas. |
| **Alerts** | Toolbar filter (Tabs Open/History, level select, pencarian subjek) → daftar baris 56px → detail drawer 480px saat dipilih. | Daftar penuh; detail bottom sheet; ack/snooze di footer sheet. |
| **Integrations** | Seksi per tier (Native, Preset, Generic); tabel connector; detail drawer 480px dengan bagian Status, Auth, Test, Actions, Manifest. | Kartu connector 88px; detail sheet; tombol aksi lebar penuh. |
| **Knowledge** | Kolom baca maksimal 760px untuk halaman; daftar hasil di kiri 360px saat lebar ≥1280, else hasil di atas halaman. | Satu kolom: pencarian, hasil, halaman; 20px gutter. |
| **Costs** | Header + pemilih jendela (24 jam/7 hari/30 hari) → ringkasan angka → `LedgerChart` 320px → tabel breakdown (agent/model/provider). Lebar data maksimal 1200px. | Chart 180px, ringkasan 2×2, breakdown sebagai baris kartu; pemilih jendela full width. |
| **Settings** | Subnav 200px + form 720px; seksi Tampilan, Bahasa & waktu, Retensi, Backup, Perangkat, Tentang. Simpan per seksi di kanan bawah seksi. | Daftar seksi → detail; field 44px; tombol simpan lebar penuh. |

Aturan layout tambahan:

- Satu primary per layar. Di Today primary adalah aksi yang benar-benar
  dibutuhkan (misalnya "Tandai ditangani" pada item teratas rail); bila tidak
  ada keputusan, tidak ada primary.
- Angka rata kanan; label rata kiri; ikon tidak pernah menjadi kolom
  tersendiri pada tabel.
- Tabel tidak berada di dalam kartu ber-scroll sendiri; satu region scroll per
  layar. Rail dan tabel boleh scroll, tetapi bukan tiga scrollbar bersarang.
- Tidak ada panel melayang dengan blur di atas konten. Tooltip dan menu boleh
  overlay; chrome utama selalu docked.
- Ponsel tidak memakai hover sebagai satu-satunya jalan; semua aksi terlihat
  atau berada di menu.

---

## 8. Grafik

uPlot 1.6; tidak ada library chart lain. Spesifikasi render:

| Aspek | Nilai |
|---|---|
| Tinggi | 240 desktop; 160 ponsel; 320 hanya `/costs` |
| Lebar | mengikuti container via ResizeObserver; redraw saat resize |
| Titik | maksimal 720; server mengagregasi 24h → bucket 5 menit, 7d → 1 jam, 30d → 6 jam |
| Garis | 1,5px; cash solid `ink-12`; shadow dashed `[4,3]` `ink-10`; tanpa spline |
| Area | tidak ada fill; titik terpilih 3px `attention` |
| Grid | horizontal saja, 1px `hairline`; tanpa garis vertikal |
| Sumbu | tick 11/16 mono `ink-10`; 4 tick Y; waktu WIB di X |
| Crosshair | garis 1px dashed `ink-10`; tooltip `raised` + hairline, angka mono |
| Legenda | teks 12/16 `ink-11` + sampel garis 12×1,5; bukan kotak warna |
| Animasi | tidak ada; data baru digambar langsung |
| Loading | skeleton setinggi grafik, tanpa sumbu palsu |
| Empty | dua baris, tanpa sumbu |
| Error | dua baris + "Coba lagi", tanpa data terakhir yang menyesatkan |
| Stale | watermark "Data terakhir 19.53" bila koneksi terputus |

Hierarki: satu grafik utama per layar.

| Layar | Grafik utama | Data sekunder |
|---|---|---|
| Today | ledger cash vs shadow 24 jam | daftar biaya per agent dengan bar tipis (bukan grafik kedua) |
| Costs | cash vs shadow sesuai jendela | tabel breakdown, bukan grafik kedua |
| Agent detail | usage agent pada tab Usage | tabel sesi, bukan grafik |
| Office, Alerts, Timeline, Integrations, Knowledge, Automations, Settings | tidak ada | angka, meter, dan tabel |

Warna seri: cash `ink-12`, shadow `ink-10`; `attention` hanya untuk titik
terpilih, ambang, atau penanda "hari ini". Tidak ada seri pelangi. Bila suatu
layar benar-benar butuh identitas seri, gunakan langkah `ink` + pola garis,
bukan hue.

A11y: `<figure>` dengan caption ringkasan yang bisa dibaca tanpa grafik,
tombol "Tabel" untuk data tabular, fokus keyboard dengan panah, dan
`aria-describedby` ke ringkasan. Canvas tidak menjadi satu-satunya sumber
informasi.

---

## 9. Ikon

- Hanya `lucide-svelte`. Tidak ada emoji, tidak ada SVG buatan untuk ikon
  antarmuka (kecuali logo).
- Ukuran 16px untuk kontrol dan nav; 18px hanya empty state dan header
  halaman. Stroke 1,5. Satu warna: mengikuti peran teks di sebelahnya.
- Tidak ada ikon di setiap judul seksi. Ikon dipakai untuk aksi, status
  (lewat dot, bukan ikon), empty state, dan objek kantor.
- Tombol ikon wajib punya `aria-label`; ikon dekoratif `aria-hidden`.
- Ikon arah (chevron) berputar 180° saat menu terbuka, 150ms; tidak ada ikon
  yang berdenyut.

Peta ikon utama:

| Konteks | Lucide |
|---|---|
| Today | `layout-dashboard` |
| Office | `building-2` |
| Timeline | `history` |
| Alerts | `bell` |
| Automations | `workflow` |
| Integrations | `plug` |
| Knowledge | `book-open` |
| Costs | `chart-no-axes-column` |
| Settings | `settings` |
| Cari | `search` |
| Tutup | `x` |
| Menu | `ellipsis` |
| Muat ulang | `refresh-cw` |
| Setujui | `check` |
| Tolak | `x` |
| Jeda | `pause` |
| Lanjut | `play` |
| Hentikan | `square` |
| Salin | `copy` |
| Buka tautan | `external-link` |
| Filter | `filter` |
| Konektor | `server` |
| Kunci/secret | `key` |
| Folder | `folder` |
| File | `file-text` |

---

## 10. Copy

Voice: tenang, langsung, spesifik. Tidak menjual, tidak menyapa, tidak
berseru.

| Aturan | Contoh salah | Contoh benar |
|---|---|---|
| Tanpa tanda seru | `Berhasil!` | `Tersimpan` |
| Tanpa emoji | `✨ Tugas selesai` | `Tugas selesai` |
| Sentence case | `Ringkasan Hari Ini` | `Ringkasan hari ini` |
| Tanpa hype | `Supercharge alur kerjamu` | `Semua agent dalam satu layar` |
| Tanpa sapaan | `Selamat datang kembali, Rizal!` | `Hari ini, 16 Nov 2026` |
| Label pendek | `Cari apa saja di seluruh HQ...` | `Cari` |
| Angka dengan satuan | `2.942` | `2.942 request` |
| Pembanding eksplisit | `+12%` | `+12% vs kemarin` |
| Nama objek di konfirmasi | `Hapus?` | `Cabut device "laptop-rizal"?` |
| Empty state dua baris | `Tidak ada data.` | `Belum ada agent terdeteksi` + `Jalankan collector atau tambahkan heartbeat.` |

Aturan angka dan waktu:

- Satu format per layar. Ribuan memakai pemisah locale (`Intl.NumberFormat`).
- Token dan request ditampilkan penuh di bawah 1.000.000; di atasnya memakai
  notasi compact locale dengan satu desimal, nilai persis ada di `title` dan
  tabel rinci.
- USD memakai `Intl.NumberFormat` dengan 2 desimal; nilai <1 memakai 3 desimal
  agar biaya kecil tidak menjadi `$0,00`. Simbol `$`, bukan `USD`.
- Persentase bilangan bulat, kecuali kuota yang butuh satu desimal.
- Waktu `HH.mm` zona `Asia/Jakarta` + `WIB` hanya bila konteks lintas zona;
  tanggal `16 Nov 2026`; relatif `3 menit lalu`, `2 jam lalu`; setelah 24 jam
  memakai tanggal absolut.
- Tidak ada em dash di copy UI; pecah kalimat atau pakai titik dua. Rentang
  memakai en dash.
- Tidak ada data palsu. Nilai tidak diketahui `—`. Connector tanpa kredensial
  `Belum dikonfigurasi`, bukan `0` atau `Sehat`.
- Error: apa yang terjadi + apa yang bisa dilakukan + `request id` di detail.
  Tidak ada stack trace, path internal, atau isi secret.

Label aksi baku: `Simpan`, `Batal`, `Tutup`, `Tandai ditangani`, `Tunda 1 jam`,
`Setujui`, `Tolak`, `Hubungkan`, `Uji`, `Muat ulang`, `Cabut`, `Salin`.
Tidak ada `OK`, `Submit`, `Klik di sini`, atau `Selengkapnya` tanpa objek.

---

## 11. Kantor 3D: spec visual

Bukan game. Batas dari §14.1 tetap: tanpa build mode, needs, kustomisasi
karakter, atau multiplayer.

### 11.1 Ruang dan material

- Satu lantai 24×16 m; grid nav 0,5 m (48×32 sel); enam zona masing-masing
  8×8 m dalam susunan 3×2: OMP, Muse, Cursor (baris belakang); Workers,
  Meeting, MCP (baris depan).
- Dinding dipotong 1,2 m pada sisi selatan dan timur agar interior terbaca
  dari kamera isometrik.
- Material: lantai grafit hangat (roughness .85, metalness 0), dinding pasir,
  meja putih kertas, kursi grafit, tanaman hijau desaturasi. Tidak ada tekstur
  yang diunduh; tekstur papan digambar lokal.
- Tanpa bloom, tanpa neon, tanpa warna hue pada material struktural. Status
  hanya LED kecil di meja dan layar monitor.

### 11.2 Cahaya

| Mode | Key | Fill | Ambien | Catatan |
|---|---|---|---|---|
| Day (06.00–17.59 WIB) | directional hangat 3000K, intensitas 1,0 | hemisphere netral 0,45 | 0,35 | latar `#EFEBE2`; bayangan lembut 1024 |
| Night (18.00–05.59 WIB) | directional hangat 2700K, intensitas 0,65 | hemisphere 0,25 | 0,2 | latar `#0C0C0B`; lampu meja 6 point light tanpa shadow |

Mode mengikuti jam `Asia/Jakarta`, dihitung ulang saat melewati batas. Tidak
ada kontrol manual cuaca atau waktu. Tidak ada glow; LED status memakai
emissive rendah tanpa bloom.

### 11.3 Kamera

| Preset | Azimuth | Polar | Zoom | Catatan |
|---|---:|---:|---:|---|
| Isometrik (default) | 45° | 54,7° | fit 26 m | ortho; seluruh lantai terlihat |
| Atas | 0° | 0° | fit 26 m | untuk melihat zoning |
| OMP / Muse / Cursor / Workers / Meeting / MCP | 45° | 54,7° | 1,4× | pusat zona |
| Follow agent | 45° | 54,7° | 1,8× | offset 8 m; hanya aktif bila dipilih |

Pan dan zoom dibatasi agar lantai tidak keluar frame. Tidak ada auto-orbit.
Transisi preset 300ms dengan `--ease`; reduced motion memotong transisi.
Memilih agent tidak memindahkan kamera kecuali pengguna menekan "Fokus".

### 11.4 Label dan objek

- Label agent hanya muncul saat hover atau seleksi, satu label pada satu
  waktu, sebagai overlay DOM 12/16 dengan `surface` + hairline. Tidak ada
  chip peran, aksi, atau status permanen.
- Objek interaktif membawa data nyata:

| Objek | Data | Aksi |
|---|---|---|
| Papan putih | maksimal 5 task hari ini + "N lainnya" | buka drawer task; tautan ke detail agent |
| Papan alert | alert `open` critical/warning + hitungan | buka drawer alert; ack/snooze |
| Rak server | connector MCP + health | buka drawer connector; tautan Integrations |
| Meja rapat | approval `pending` + sisa waktu | buka drawer approval; setujui/tolak |

- Objek yang tidak punya data tidak diberi ikon interaktif. Hover memakai
  outline 1px `attention`; seleksi memakai outline 2px.
- Canvas bukan satu-satunya jalan: sediakan tombol "Daftar agent" yang membuka
  panel DOM berisi link dan status, serta 2D fallback.

### 11.5 2D fallback

- SVG floor plan 24×16 dengan enam zona berlabel, dot/card agent, status dot,
  dan hitungan per zona. Klik agent membuka detail; klik zona memfilter.
- Dipakai otomatis bila WebGL gagal, dan menjadi default di ponsel.
- Data, filter, dan aksi sama dengan 3D. Tidak ada ilustrasi dekoratif.
- Keyboard: setiap agent/zona adalah tombol dengan fokus terlihat; tidak ada
  informasi yang hanya ada di canvas.

### 11.6 Batas performa

- Maksimal 40 agent terlihat; sisanya agregat "+N" per zona.
- 60 fps target di GPU terintegrasi, gate ≥55 fps rata-rata pada 40 agent
  (`08-testing-gates.md` §7).
- Draw call ≤120; triangle ≤120k; satu shadow map 1024; tanpa post-processing.
- Render-on-demand: loop berjalan hanya saat ada walk/meet atau interaksi;
  animasi ambient (type/think/wait) dibatasi 10 fps; saat semua agent
  `santai`/`offline`, frame berhenti setelah settle.
- Statistik tersedia untuk tes: `window.__hqOfficeStats`.

---

## 12. Aksesibilitas dan gerak

### 12.1 Checklist a11y

- [ ] Satu `<h1>` per halaman; heading berurutan; landmark `header`, `nav`,
  `main`, `complementary`.
- [ ] Skip link ke konten; fokus terlihat 2px pada semua kontrol.
- [ ] Semua tombol/tautan punya nama aksesibel; ikon saja memakai `aria-label`.
- [ ] Dialog, drawer, dan palette memerangkap fokus, menutup dengan `Escape`,
  dan mengembalikan fokus ke pemicu.
- [ ] Tabel memakai `caption`, `th scope`, `aria-sort`; baris dapat dibuka
  tanpa klik baris.
- [ ] Status tidak hanya warna: dot + teks status (terlihat atau sr-only).
- [ ] Form: label eksplisit, `aria-invalid`, `aria-describedby`, fokus ke
  error pertama saat submit.
- [ ] Grafik: ringkasan teks + tabel data; dapat difokus dan digerakkan panah.
- [ ] Live region: hitungan attention `polite`, toast critical `alert`,
  banner offline diumumkan sekali.
- [ ] Kontras teks ≥4,5:1; fokus dan batas kontrol ≥3:1.
- [ ] Reflow 390px tanpa scroll horizontal; region scroll (log mentah, tabel
  lebar) dapat difokus dan punya `aria-label`.
- [ ] Zoom 200% tidak memotong kontrol atau menumpuk teks.
- [ ] `lang` benar (`id`, `en`, `zh-Hans`); `dir="ltr"`.
- [ ] Reduced motion dihormati; animasi kantor dapat dihentikan.
- [ ] Target sentuh 44×44px di ponsel; tidak ada informasi hover-only.

### 12.2 Gerak

| Peristiwa | Durasi | Sifat |
|---|---:|---|
| Hover/warna | 150ms | warna/opacity saja |
| Press | 150ms | `scale(.97)` |
| Menu | masuk 150ms, keluar 150ms | fade + `scale(.97)`, origin di pemicu |
| Modal | masuk 240ms, keluar 200ms | fade + rise 16px |
| Sheet | masuk 300ms, keluar 200ms | translate Y; solid |
| Chart update | 0ms | langsung, tanpa tween |
| Angka berubah | 0ms | langsung, tanpa tween |
| Office preset | 300ms | kamera; reduced motion instan |

Satu kurva `--ease`; hanya `transform`, `opacity`, `color`, `background-color`,
`border-color`. Tidak ada `transition:all`, animasi lebar/tinggi/top/left,
looping pulse, shimmer, parallax, atau hover scale pada kartu. Reduced motion:
travel/scale menjadi crossfade 150ms; walk cycle kantor mati; posisi karakter
berubah dengan crossfade, bukan interpolasi.

---

## 13. Interface untuk S07 dan S13–S15

### 13.1 S07 — design system + shell

File yang dibuat:

- `web/src/app.css` (token §3, reset, `@font-face`)
- `web/src/lib/components/ui/{Button,IconButton,Field,Select,Tabs,Table,StatusDot,Badge,Drawer,Modal,Toaster,CommandPalette,EmptyState,Skeleton,Keycap,PageHeader,Section,KpiFigure,Meter,Switch,Pagination}.svelte`
- `web/src/lib/components/domain/{AttentionRail,AgentMonogram,LedgerChart}.svelte`
- `web/src/lib/components/shell/{AppShell,Sidebar,TopBar,OfflineBanner,TokenGate}.svelte`
- `web/src/lib/stores/{ui,toast}.svelte.ts`
- `web/src/lib/i18n/{index.ts,id.json,en.json,zh.json}`
- `web/src/lib/format/{number,datetime,duration}.ts`
- `web/tests/visual/shell.spec.ts`, `web/tests/a11y/shell.spec.ts`

Export yang dipakai S08+:

```ts
// $lib/components/ui/index.ts
export { Button, IconButton, Field, Select, Tabs, Table, StatusDot, Badge,
  Drawer, Modal, Toaster, CommandPalette, EmptyState, Skeleton, Keycap,
  PageHeader, Section, KpiFigure, Meter, Switch, Pagination };
// $lib/components/domain/index.ts
export { AttentionRail, AgentMonogram, LedgerChart };
// $lib/components/shell/index.ts
export { AppShell, Sidebar, TopBar, OfflineBanner, TokenGate };
// $lib/stores/toast.svelte.ts
export { showToast, dismissToast };
// $lib/stores/ui.svelte.ts
export const ui: {
  theme: 'day' | 'night' | 'system';
  locale: 'id' | 'en' | 'zh';
  railOpen: boolean;
  sidebarOpen: boolean;
  paletteOpen: boolean;
  reducedMotion: boolean;
  applyDocumentTheme(): void;
};
```

Tes S07 (nama exact):

- `web/tests/visual/shell.spec.ts`
  - `shell matches day snapshot at 1440x900`
  - `shell matches night snapshot at 1440x900`
  - `shell matches phone snapshot at 390x844`
- `web/tests/a11y/shell.spec.ts`
  - `shell has no serious or critical axe violations`
  - `focus ring is visible on every interactive control`
  - `attention rail count is announced politely`
- `web/tests/unit/components.test.ts`
  - `button renders exactly one primary per surface contract`
  - `status dot maps six statuses to roles`
  - `attention rail caps items and groups critical before warning`
  - `agent monogram uses initials without per-agent hue`
  - `empty state always renders two lines`
  - `keycap renders kbd semantics`
  - `table row is reachable without clicking the row`

Snapshot: 1440×900 light, 1440×900 dark, 390×844 light (toleransi 0,1%,
`08-testing-gates.md` §7). axe 0 violation serious/critical.

### 13.2 S13 — Office core

File:

- `web/src/routes/office/{+page.ts,+page.svelte}`
- `web/src/lib/office/{OfficeCanvas.svelte,Scene.svelte,Lighting.svelte,Floor.svelte,camera.ts,stats.ts}`
- `web/tests/e2e/office-scene.spec.ts`
- `web/tests/perf/office.spec.ts`

Signature:

```ts
export interface CameraPreset {
  id: 'iso' | 'top' | 'omp' | 'muse' | 'cursor' | 'workers' | 'meeting' | 'mcp';
  azimuth: number;
  polar: number;
  zoom: number;
  target: [number, number, number];
}
export interface OfficeHandle {
  setCamera(preset: CameraPreset['id']): void;
  focusAgent(id: string): void;
  invalidate(reason: string): void;
  stats(): OfficeStats;
  dispose(): void;
}
export interface OfficeStats {
  fps: number;
  frameMsP95: number;
  drawCalls: number;
  triangles: number;
  agentsVisible: number;
  mode: '3d' | '2d';
  framesRendered: number;
  lastInvalidate: string;
}
```

Tes:

- `office-scene.spec.ts`
  - `office renders scene only after data snapshot arrives`
  - `office uses orthographic isometric camera by default`
  - `office camera preset changes framing without auto orbit`
  - `office render loop stops when all agents are idle`
  - `office chunk is requested only on the office route`
- `office.spec.ts`
  - `office averages 55fps with 40 agents`
  - `office draw calls stay under 120`

### 13.3 S14 — Karakter dan navigasi

File:

- `web/src/lib/office/{nav.ts,characters.ts,rig.ts}`
- `web/tests/unit/office-nav.test.ts`
- `web/tests/e2e/office-nav.spec.ts`

Signature:

```ts
export interface NavGrid { cols: number; rows: number; cell: 0.5; }
export function buildNavGrid(zones: ZoneSpec[], obstacles: Rect[]): NavGrid;
export function findPath(grid: NavGrid, from: Vec2, to: Vec2): Vec2[];
export type CharacterState =
  | 'idle' | 'type' | 'think' | 'wait'
  | 'sleep' | 'error' | 'walk' | 'meet';
export function animationFor(status: Status, moving: boolean, inMeeting: boolean): CharacterState;
export function rigFor(kind: AgentKind): 'human' | 'robot';
```

Tes:

- `office-nav.test.ts`
  - `a star routes around furniture`
  - `separation keeps agents apart on the same path`
  - `status priority prefers error over walk`
- `office-nav.spec.ts`
  - `agent walks around obstacles to its desk`
  - `forty agents keep walking without overlap`
  - `reduced motion disables the walk cycle`
  - `meeting state overrides typing at the meeting table`

### 13.4 S15 — Objek interaktif dan 2D

File:

- `web/src/lib/office/{objects.ts,DataBoardDrawer.svelte,Fallback2D.svelte}`
- `web/src/routes/office/+page.svelte` (integrasi drawer/fallback)
- `web/tests/e2e/office-bindings.spec.ts`
- `web/tests/visual/office.spec.ts`
- `web/tests/a11y/office.spec.ts`

Signature:

```ts
export interface OfficeBinding {
  id: 'whiteboard' | 'alert-board' | 'server-rack' | 'meeting-table';
  count: number;
  items: { id: string; label: string; href?: string }[];
  onopen: () => void;
}
export function bindingsFrom(
  state: { tasks: TaskView[]; alerts: AlertView[]; connectors: ConnectorView[]; approvals: ApprovalView[] }
): OfficeBinding[];
```

Tes:

- `office-bindings.spec.ts`
  - `whiteboard lists today tasks and opens the task drawer`
  - `alert board opens open alerts and acknowledges one`
  - `server rack lists mcp connectors with health`
  - `meeting table lists pending approvals`
  - `clicking an object never opens a fake detail`
- `office.spec.ts` (visual)
  - `office matches day snapshot at 1440x900`
  - `office matches night snapshot at 1440x900`
  - `office 2d matches phone snapshot at 390x844`
- `office.spec.ts` (a11y)
  - `office has no serious or critical axe violations`
  - `every agent is reachable from the agent list`
  - `2d fallback exposes the same actions as 3d`

---

## Diserahkan ke builder

- Daftar glyph final untuk subset IBM Plex setelah katalog i18n beku, termasuk
  keputusan apakah CJK memakai font sistem (default dokumen ini) atau subset
  Noto Sans SC yang di-embed; keputusan harus lulus budget §14.2 design.
- Penyesuaian framing dan zoom final untuk delapan preset kamera di §11.3,
  tanpa mengubah jumlah atau urutannya, serta proporsi
  rig karakter (tinggi, panjang lengan/kaki) selama tetap low-poly dan
  memenuhi budget triangle.
- Apakah papan data di kantor memakai CanvasTexture yang digambar saat runtime
  atau tekstur yang dipanggang saat build; keduanya lokal dan harus punya
  jalur update data tanpa rebuild.
- Komposisi visual 2D fallback (SVG floor plan dengan dot versus kartu per
  zona); data, aksi, dan aksesibilitas sudah diikat di §11.5.
- Peninjauan terjemahan 中文 oleh penutur; kunci dan struktur sudah diikat,
  pemilihan kata bukan keputusan teknis.
