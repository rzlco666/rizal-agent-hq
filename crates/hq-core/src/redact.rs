//! Sensor rahasia: API key/token tidak pernah tampil atau tersimpan.
//! Port Rust dari redact v1 yang sudah teruji di lapangan.

use once_cell::sync::Lazy;
use regex::{Captures, Regex};

// Aturan 1 (kata kunci + nilai) TIDAK bisa satu regex: crate `regex` tidak
// mendukung look-around, padahal "nilai tampak seperti rahasia" butuh syarat
// gabungan (panjang + kandungan karakter). Jadi aturan 1 dipecah dua tahap:
// regex menangkap kandidat `kata-kunci + pemisah + nilai`, lalu predikat
// Rust di bawah menilai nilainya. Aturan 1a/1b jalan SEBELUM tabel RULES.
static RULE1_ASSIGN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)(-{0,2}(?:api[-_]?key|token|password|passwd|secret|auth|credential)\s*[:=]\s*)([^\s\"']+)"#,

    )
    .expect("pola redact 1a valid")
});
static RULE1_BARE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)(-{0,2}(?:api[-_]?key|token|password|passwd|secret|auth|credential)\s+)([^\s\"']+)"#,
    )
    .expect("pola redact 1b valid")
});

/// Nama standar yang muncul sebagai NILAI di dokumen spesifikasi —
/// protokol auth dan algoritma kripto. Nilai yang seluruh segmennya
/// terdiri dari kata ini (atau angka versi/kekuatan) BUKAN rahasia.
/// Bukti probe live ronde 3 (2026-11-07): `auth: OIDC/OAuth2;` dan
/// `secret: AES-256-GCM` mengarantina halaman PRD StrutStack V2 dan
/// Analisis SA UMPKU A.5. JANGAN tambah kata generik
/// (KEY/TOKEN/SECRET/PASSWORD) tanpa review terpisah: `SECRET-KEY-1234`
/// akan lolos sebagai bukan-rahasia.
const KNOWN_VALUE_SEGMENTS: &[&str] = &[
    // protokol
    "oidc", "oauth", "oauth2", "saml", "saml2", "jwt", "openid", // kripto
    "aes", "des", "rsa", "ecc", "ecdsa", "ed25519", "chacha20", "poly1305", "gcm", "cbc", "ecb",
    "ctr", "sha1", "sha256", "sha384", "sha512", "md5", "hmac",
];

/// True bila SEMUA segmen nilai (dipisah `- _ / + . =`) adalah kata di
/// KNOWN_VALUE_SEGMENTS (case-insensitive) atau angka murni. Segmen
/// kosong dianggap tidak dikenal supaya nilai aneh tetap dinilai.
fn all_known_segments(value: &str) -> bool {
    value.split(['-', '_', '/', '+', '.', '=']).all(|seg| {
        let lower = seg.to_ascii_lowercase();
        !seg.is_empty()
            && (KNOWN_VALUE_SEGMENTS.contains(&lower.as_str())
                || seg.chars().all(|c| c.is_ascii_digit()))
    })
}

/// R3-1: batas kata kiri pada KATA KUNCI aturan 1a/1b. Crate `regex`
/// tidak mendukung look-behind, jadi dicek di Rust: posisi kata kunci =
/// awal match + jumlah `-` di depan (grup 1 memuat `-{0,2}`). Karakter
/// sebelum KATA KUNCI (bukan sebelum match) yang dinilai, supaya
/// `X-API-KEY:` (didahului `-`) dan `--api-key` tetap tertangkap,
/// sementara `tiktoken==0.14.0` (didahului `k`) dilewati.
/// Trade-off diterima: `mytoken=…` / `authtoken=…` juga dilewati; kunci
/// ber-prefix dikenal tetap ditangkap tabel RULES.
fn keyword_left_boundary_ok(haystack: &str, caps: &Captures) -> bool {
    let m = caps.get(0).expect("grup 0 selalu ada");
    let leading_dashes = m.as_str().bytes().take_while(|b| *b == b'-').count();
    let kw_start = m.start() + leading_dashes;
    haystack[..kw_start]
        .chars()
        .next_back()
        .map(|c| !c.is_ascii_alphanumeric())
        .unwrap_or(true)
}

/// Nilai bentuk assignment (`token=...`, `password: ...`) tampak seperti
/// rahasia bila panjang >= 6 DAN mengandung digit, ATAU panjang >= 16
/// dengan huruf besar+ kecil (token panjang campuran). Prosa TIDAK lolos:
/// "password = kata sandi" (nilai 4 huruf), "password = minimal 8
/// karakter" (7 huruf tanpa digit), "token = potongan teks" (8 huruf
/// tanpa digit), "token = JWT" (3 huruf).
/// R3-4: pangkas tanda baca ujung dulu (paritas `secret_like_bare`), lalu
/// nilai yang seluruh segmennya nama standar/angka bukan rahasia.
fn secret_like_assignment(value: &str) -> bool {
    // Bukan rahasia (bukti probe live ronde 2, 2026-11-07): placeholder
    // sintaks ("mode[:source1|source2]" — kurung siku menandai sintaks,
    // bukan nilai) dan pin versi paket ("tiktoken==0.14.0" — "==" di
    // TENGAH nilai; padding base64 "==" yang sah hanya ada di AKHIR).
    if value.contains('[') || value.contains(']') {
        return false;
    }
    if value.contains("==") && !value.ends_with('=') {
        return false;
    }
    // R3-4: tanda baca ujung (`,` `;` `)` `"` …) bukan bagian nilai.
    let value = value.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || " _-./+=".contains(c)));
    // R3-4: `OIDC/OAuth2`, `AES-256-GCM`, `1.0`, `2048` bukan rahasia.
    if all_known_segments(value) {
        return false;
    }
    let len = value.chars().count();
    if len >= 6 && value.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    len >= 16
        && value.chars().any(|c| c.is_ascii_uppercase())
        && value.chars().any(|c| c.is_ascii_lowercase())
}

/// Nilai bentuk lepas (`--api-key <nilai>`, `token <nilai>`) tampak seperti
/// rahasia bila panjang >= 12 DAN mengandung digit, dan hanya berisi
/// karakter token (alnum + `_-.+/=`). Syarat huruf besar DIHAPUS (ronde
/// perbaikan PR #11, F1): kunci hex huruf-kecil seperti `abcdef1234567890`
/// adalah bentuk kunci API yang lazim dan harus tertangkap; semua contoh
/// prosa tetap gugur oleh syarat lain: "token harus" (5 huruf),
/// "token Telegram" (8 huruf, tanpa digit), "token diperlakukan" (12 huruf,
/// tanpa digit), "password hanya" (5 huruf), "secret (siapa" (mengandung
/// `(`). Tanda baca di ujung nilai (koma/titik-koma setelah nilai dalam
/// kalimat) dipangkas dulu sebelum dinilai, supaya `AbCdEf1234567890,`
/// tetap tertangkap.
fn secret_like_bare(value: &str) -> bool {
    // Pangkas tanda baca di ujung nilai: regex menangkap nilai sampai
    // spasi/kutip, jadi `token AbCdEf1234567890,` ikut menangkap koma —
    // tanpa pemangkasan, `.all(charset)` gagal pada `,` dan rahasia lolos.
    let value =
        value.trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || "_-.+/=".contains(c)));
    let len = value.chars().count();
    len >= 12
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.+/=".contains(c))
        && value.chars().any(|c| c.is_ascii_digit())
}

/// Nilai sesudah `Bearer` tampak seperti rahasia bila panjang >= 10 DAN
/// mengandung digit ATAU titik (JWT punya 2 titik; token opaque panjang
/// punya digit). Frasa generik pendek ("token", "legacy", "TOKEN-1" —
/// 5–7 karakter, tanpa titik) BUKAN rahasia: itu penyebutan skema auth
/// dalam prosa/dokumen, bukan kredensial. Aturan Bearer lama (di tabel
/// RULES) menyensor KATA APA PUN sesudah "Bearer" — itu yang mengkarantina
/// PRD V2, Lampiran, dan PRD SQL V2 (bukti probe live ronde 2).
/// Kalibrasi ambang 10 (bukan 12): nilai "abc.def.ghi" (11 karakter,
/// 2 titik — fragmen JWT) HARUS tetap tertangkap; ambang 12 terbukti
/// meloloskannya (tes sensor_token_gh_dan_bearer gagal). Ambang 10
/// duduk di antara klaster generik (<= 7) dan rahasia terpendek (11).
static RULE_BEARER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(Bearer\s+)([A-Za-z0-9._~+/-]+=*)").expect("pola redact bearer valid")
});

fn secret_like_bearer(value: &str) -> bool {
    let len = value.chars().count();
    len >= 10 && (value.chars().any(|c| c.is_ascii_digit()) || value.contains('.'))
}

/// R3-3: aturan `Authorization:` sadar-nilai. Aturan lama menyensor KATA
/// APA PUN sesudah `Authorization:` — yang di sini adalah kata skema
/// (`Bearer`) atau placeholder (`<key>`), bukan kredensial. Pola
/// menangkap kata skema sebagai bagian prefix opsional sehingga yang
/// dinilai adalah tokennya (berlaku juga untuk `Proxy-Authorization:`).
/// Placeholder (`<…>`) dan kata skema murni diloloskan; token dinilai
/// dengan predikat Bearer ATAU campuran huruf besar-kecil len >= 8.
/// Celah sisa yang diterima: token seluruhnya huruf kecil tanpa digit
/// yang panjang lolos dari aturan ini (dan dari aturan Bearer).
static RULE_AUTH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(Authorization:\s*(?:Bearer\s+|Basic\s+|Digest\s+)?)(\S+)")
        .expect("pola redact authorization valid")
});

fn secret_like_auth(value: &str) -> bool {
    if value.starts_with('<') {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    if matches!(lower.as_str(), "bearer" | "basic" | "digest" | "token") {
        return false;
    }
    secret_like_bearer(value)
        || (value.chars().count() >= 8
            && value.chars().any(|c| c.is_ascii_uppercase())
            && value.chars().any(|c| c.is_ascii_lowercase()))
}

static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let defs: Vec<(&str, &'static str)> = vec![
        // R3-2: `(^|[^A-Za-z0-9])` menolak awalan yang menempel di tengah
        // kata majemuk (`kiosk-second`, `X-Caddy-Ask-Secret`). Grup 2 =
        // prefix yang dipertahankan; grup 1 = pemisah yang ditulis ulang.
        (
            r"(^|[^A-Za-z0-9])(crsr_[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(gho_[A-Za-z0-9]{4})[A-Za-z0-9]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(ghp_[A-Za-z0-9]{4})[A-Za-z0-9]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(github_pat_[A-Za-z0-9_]{4})[A-Za-z0-9_]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(ntn_[A-Za-z0-9]{4})[A-Za-z0-9]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(sk-ant-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(sk-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+",
            "$1$2•••",
        ),
        (
            r"(^|[^A-Za-z0-9])(xai-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+",
            "$1$2•••",
        ),
        (r"(^|[^A-Za-z0-9])(AKIA)[A-Z0-9]{12,}", "$1$2•••"),
        (r"(\w+://[^:\s/]+:)[^@\s/]+(@)", "$1•••$2"),
        (
            r"(?i)([?&](?:key|token|secret|password|api_key)=)[^&\s]+",
            "$1•••",
        ),
    ];
    defs.into_iter()
        .map(|(p, r)| (Regex::new(p).expect("pola redact valid"), r))
        .collect()
});

pub fn redact(input: &str) -> String {
    let mut s = input.to_string();
    // Aturan 1a: sensor nilainya hanya bila tampak seperti rahasia.
    s = RULE1_ASSIGN
        .replace_all(&s, |caps: &Captures| {
            if !keyword_left_boundary_ok(&s, caps) || !secret_like_assignment(&caps[2]) {
                return caps[0].to_string();
            }
            format!("{}•••", &caps[1])
        })
        .into_owned();
    // Aturan 1b: sama untuk bentuk lepas.
    s = RULE1_BARE
        .replace_all(&s, |caps: &Captures| {
            if !keyword_left_boundary_ok(&s, caps) || !secret_like_bare(&caps[2]) {
                return caps[0].to_string();
            }
            format!("{}•••", &caps[1])
        })
        .into_owned();
    // Aturan Bearer: sensor nilainya hanya bila tampak seperti rahasia
    // (lihat secret_like_bearer).
    s = RULE_BEARER
        .replace_all(&s, |caps: &Captures| {
            if secret_like_bearer(&caps[2]) {
                format!("{}•••", &caps[1])
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    // Aturan Authorization sadar-nilai (R3-3).
    s = RULE_AUTH
        .replace_all(&s, |caps: &Captures| {
            if secret_like_auth(&caps[2]) {
                format!("{}•••", &caps[1])
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    for (re, rep) in RULES.iter() {
        s = re.replace_all(&s, *rep).into_owned();
    }
    s
}

/// Gerbang tulis: true bila teks mengandung pola rahasia.
/// Memakai SATU sumber pola yang sama dengan `redact()` — implementasinya
/// membandingkan hasil `redact()`: setiap aturan menyisipkan `•••`, jadi
/// teks berubah HANYA bila ada pola yang cocok. Dinilai pada teks ASLI
/// (sebelum redact) — teks yang mengandung rahasia DITOLAK, bukan disimpan
/// dalam keadaan tersensor (B.5 §7.1).
pub fn contains_secret(input: &str) -> bool {
    redact(input) != input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_api_key_cursor() {
        let out = redact(
            "cursor-agent --api-key crsr_1234567890abcdef --endpoint https://api2.cursor.sh",
        );
        assert!(!out.contains("crsr_1234567890abcdef"), "{out}");
        assert!(out.contains("cursor-agent"));
    }

    #[test]
    fn sensor_token_gh_dan_bearer() {
        assert!(!redact("ghp_1234567890abcdefghij").contains("1234567890abcdefghij"));
        assert!(!redact("gho_1234567890abcdefghij").contains("1234567890abcdefghij"));
        assert!(!redact("Authorization: Bearer abc.def.ghi").contains("abc.def.ghi"));
    }

    #[test]
    fn gerbang_rahasia_mendeteksi_semua_pola() {
        // Tiap pola di RULES harus memicu gerbang tulis.
        let rahasia = [
            "token=AbCdEf1234567890",
            "api-key crsr_1234567890abcdef",
            "api-key gho_1234567890abcdef",
            "api-key github_pat_1234567890abcdef",
            "api_key=AKIA123456789012",
            "token=sk-ant-abc1234567890",
            "secret=ghp_1234567890abcdef1234567890abcdef1234",
            "password=sk-abc1234567890",
            "xai-abc1234567890",
            "AKIA1234567890123456",
            "Bearer abc.def.ghi123",
            // R3-3: token pendek huruf kecil tanpa digit sesudah
            // `Authorization:` bukan lagi rahasia (perubahan perilaku
            // yang disengaja); fixture memakai token yang tetap
            // tampak-rahasia supaya intent "tiap pola memicu gerbang"
            // terjaga.
            "Authorization: AbCdEf123456",
            "https://user:pass123@example.com/x",
            "https://api.example.com/?token=abcdef",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "pola lolos: {s}");
        }
    }

    #[test]
    fn gerbang_rahasia_tidak_false_positive_teks_biasa() {
        let biasa = [
            "Rapat mingguan membahas roadmap dan anggaran kuota.",
            "Lihat PR #7 dan migrasi 0065 di repo strut-stack.",
            "Dokumen ini membahas arsitektur sinkronisasi dan jadwal.",
            "",
        ];
        for s in biasa {
            assert!(!contains_secret(s), "false positive: {s}");
        }
    }

    #[test]
    fn gerbang_prosa_kata_kunci_tidak_terkarantina() {
        // Bukti deploy PR #10 (A1/A5 2026-11-07): kalimat-kalimat ini
        // mengkarantina 54/106 halaman padahal semuanya prosa biasa.
        let prosa = [
            "token harus diperlakukan sebagai rahasia",
            "token diperlakukan seperti kunci rumah",
            "token Telegram harus dijaga kerahasiaannya",
            "password hanya diketahui admin",
            "secret (siapa yang boleh melihatnya?)",
            "hemat token untuk agen AI",
            "password = kata sandi yang kuat",
            "password = minimal 8 karakter",
            "token = potongan teks",
            "token = JWT yang dikirim",
        ];
        for s in prosa {
            assert!(!contains_secret(s), "prosa terkarantina: {s}");
        }
    }

    #[test]
    fn gerbang_nilai_rahasia_tetap_tertangkap() {
        // Bentuk assignment (aturan 1a) + bentuk lepas (aturan 1b) +
        // prefix token baru (ntn_ Notion, ghp_ GitHub klasik).
        let rahasia = [
            "token=AbCdEf1234567890",
            "password=hunter2",
            "api_key=sk_live_abc123",
            "secret: mysecretvalue99",
            "token: abcdef123456",
            "--api-key AbCdEfGh123456",
            "token AbCdEf1234567890",
            "ntn_1234567890abcdefghij1234567890ab",
            "ghp_1234567890abcdefghij1234567890abcd",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "rahasia lolos: {s}");
        }
    }

    #[test]
    fn redact_menyensor_nilai_tapi_bukan_prosa() {
        let out = redact("password=hunter2 dan token harus dijaga");
        assert!(!out.contains("hunter2"), "{out}");
        assert!(out.contains("password="), "{out}");
        assert!(out.contains("token harus dijaga"), "{out}");
    }

    #[test]
    fn gerbang_bearer_generik_tidak_terkarantina_tapi_jwt_tetap_tertangkap() {
        // Bukti probe live ronde 2 (2026-11-07): PRD V2, Lampiran, dan
        // PRD SQL V2 terkarantina oleh aturan Bearer lama yang menyensor
        // KATA APA PUN sesudah "Bearer" — nilainya frasa generik pendek
        // (5–7 huruf, tanpa titik/digit). Predikat baru: panjang >= 10
        // DAN (digit ATAU titik); "abc.def.ghi" (11, 2 titik) adalah
        // batas bawah rahasia — fragmen JWT, harus tertangkap.
        let generik = [
            "PAT Bearer token",
            "atau bearer token legacy",
            "Bearer TOKEN-1",
            "OAuth 2.1/OIDC atau bearer token",
        ];
        for s in generik {
            assert!(!contains_secret(s), "bearer generik terkarantina: {s}");
        }
        let rahasia = [
            "Bearer abc.def.ghi",
            "Bearer abc.def.ghi123",
            "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
            "Bearer AbCdEfGh1234567890",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "bearer rahasia lolos: {s}");
        }
    }

    #[test]
    fn gerbang_assignment_sintaks_dan_pin_versi_tidak_terkarantina() {
        // Bukti probe live ronde 2: "token=tiktoken==0.14.0" (pin versi
        // paket) dan "token:mode[:source1|source2]" (placeholder sintaks
        // CLI) terkarantina oleh aturan 1a lama. Keduanya bukan rahasia.
        let bukan_rahasia = [
            "token=tiktoken==0.14.0",
            "token:mode[:source1|source2]",
            "secret: [nilai]",
        ];
        for s in bukan_rahasia {
            assert!(!contains_secret(s), "sintaks terkarantina: {s}");
        }
        assert!(contains_secret("token=AbCdEf1234567890"));
    }

    #[test]
    fn gerbang_rahasia_huruf_kecil_dan_koma() {
        // F1 (ronde perbaikan PR #11): rahasia huruf-kecil tanpa huruf
        // besar HARUS tertangkap; koma/titik-koma di ujung nilai tidak
        // boleh meloloskan.
        assert!(contains_secret("token abcdef1234567890"));
        assert!(contains_secret("--api-key abcdef1234567890abcdef"));
        assert!(contains_secret("token AbCdEf1234567890,"));
        assert!(contains_secret("secret abcdef1234567890;"));
        // Prosa tetap lolos (alasan: panjang < 12 atau tanpa digit).
        assert!(!contains_secret("token harus"));
        assert!(!contains_secret("token Telegram"));
        assert!(!contains_secret("token di-pin")); // placeholder (fixture R3-1)
        assert!(!contains_secret("token diperlakukan"));
        assert!(!contains_secret("password hanya"));
    }

    #[test]
    fn gerbang_batas_kata_kiri_kata_kunci() {
        // R3-1 — teks blok halaman 7 dan 8 (Draf Publikasi Benchmark
        // Token + Perbaikan Harness): `token=` di dalam `tiktoken==`
        // bukan kata kunci; prosa ini harus lolos.
        let prosa = [
            "Determinisme: …; versi tiktoken di-pin (tiktoken==0.14.0).",
            "(SEDANG) tiktoken di-pin. token-benchmark-requirements.txt baru: tiktoken==0.14.0 (terverifikasi …). … (pip install --user --break-system-packages 'tiktoken==0.14.0') …",
        ];
        for s in prosa {
            assert!(!contains_secret(s), "prosa kata kunci terkarantina: {s}");
        }
        // Batas kiri kata kunci tetap menangkap bentuk header ber-hubung
        // dan opsi CLI ber-dash, juga kunci ber-prefix di posisi rawan.
        let rahasia = [
            "X-API-KEY: AbCdEf1234567890",
            "--api-key abcdef1234567890abcdef",
            "token=AbCdEf1234567890",
            " token=AbCdEf1234567890",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "rahasia kata kunci lolos: {s}");
        }
    }

    #[test]
    fn gerbang_batas_kata_kiri_prefix_rules() {
        // R3-2 — `sk-` di tengah kata majemuk bukan awalan kunci.
        assert!(!contains_secret(
            "(6) Phone-first, kiosk-second. Semua yang dibutuhkan P0–P1 bisa jalan tanpa hardware baru."
        ));
        assert!(!contains_secret(
            "defense-in-depth header secret bersama X-Caddy-Ask-Secret. Cache jawaban 60 dtk."
        ));
        // Prefix di awal string / sesudah spasi / `=` / kutip tetap sensor.
        let rahasia = [
            "sk-abcdef1234567890abcdef",
            " sk-abcdef1234567890abcdef",
            "=sk-abcdef1234567890abcdef",
            "\"sk-abcdef1234567890abcdef\"",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "kunci prefix lolos: {s}");
        }
        assert!(!redact("=sk-abcdef1234567890abcdef").contains("abcdef1234567890abcdef"));
    }

    #[test]
    fn gerbang_authorization_sadar_nilai() {
        // R3-3 — placeholder + kata skema bukan kredensial (halaman 2/3).
        let lolos = [
            "Header auth | — | Authorization: Bearer <key>  • X-Tenant-Slug tidak boleh mengubah tenant key",
            "API: GET/POST /api/v1/api-keys, …; auth Authorization: Bearer <key> — tenant diambil dari key",
            "Authorization: Bearer",
            "Authorization: token",
        ];
        for s in lolos {
            assert!(!contains_secret(s), "placeholder/skema terkarantina: {s}");
        }
        let rahasia = [
            "Authorization: Bearer abcdef123456",
            "Authorization: Basic dXNlcjpwYXNz",
            "Proxy-Authorization: Bearer abcdef123456",
            "Authorization: AbCdEf123456",
        ];
        for s in rahasia {
            assert!(contains_secret(s), "authorization rahasia lolos: {s}");
        }
        let out = redact("Authorization: Bearer <key>");
        assert!(out.contains("<key>"), "{out}");
    }

    #[test]
    fn gerbang_nama_protokol_dan_algoritma_bukan_rahasia() {
        // R3-4 — nama protokol/algoritma standar + pin angka (halaman 1/9).
        let lolos = [
            "SSO | Non-goal | Core auth: OIDC/OAuth2; SAML = modul enterprise",
            "Kunci + secret: AES-256-GCM + key terpisah (D1); tidak ada kunci di repo",
            "auth: OIDC/OAuth2; SAML",
            "secret: AES-256-GCM",
        ];
        for s in lolos {
            assert!(!contains_secret(s), "nama standar terkarantina: {s}");
        }
        // Segmen tak dikenal tetap rahasia (daftar kata TIDAK boleh jadi
        // celah umum) + koma ujung tetap dipangkas (F1).
        assert!(contains_secret("secret: SAML-1234-ABCD"));
        assert!(contains_secret("token: AbCdEf1234567890,"));
        // Halaman 4 (Rencana Demo 60 Detik) tetap terkarantina: bentuk
        // sintetis 19 karakter, campuran huruf besar-kecil, pemisah
        // `_` dan `/`, tanpa digit — sama bentuk dengan nilai asli.
        assert!(contains_secret("password=AbCd/EfGh_IjKlMnOpQr"));
    }
}
