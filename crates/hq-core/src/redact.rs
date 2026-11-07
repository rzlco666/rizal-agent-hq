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

/// Nilai bentuk assignment (`token=...`, `password: ...`) tampak seperti
/// rahasia bila panjang >= 6 DAN mengandung digit, ATAU panjang >= 16
/// dengan huruf besar+ kecil (token panjang campuran). Prosa TIDAK lolos:
/// "password = kata sandi" (nilai 4 huruf), "password = minimal 8
/// karakter" (7 huruf tanpa digit), "token = potongan teks" (8 huruf
/// tanpa digit), "token = JWT" (3 huruf).
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
    let len = value.chars().count();
    if len >= 6 && value.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    len >= 16
        && value.chars().any(|c| c.is_ascii_uppercase())
        && value.chars().any(|c| c.is_ascii_lowercase())
}

/// Nilai bentuk lepas (`--api-key <nilai>`, `token <nilai>`) tampak seperti
/// rahasia bila panjang >= 12 DAN mengandung digit DAN huruf besar, dan
/// hanya berisi karakter token (alnum + `_-.+/=`). Prosa gagal di minimal
/// satu syarat: "token harus" (5 huruf), "token Telegram" (8 huruf, tanpa
/// digit), "token diperlakukan" (12 huruf, tanpa digit), "password hanya"
/// (5 huruf), "secret (siapa" (mengandung `(`).
fn secret_like_bare(value: &str) -> bool {
    let len = value.chars().count();
    len >= 12
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.+/=".contains(c))
        && value.chars().any(|c| c.is_ascii_digit())
        && value.chars().any(|c| c.is_ascii_uppercase())
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

static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let defs: Vec<(&str, &'static str)> = vec![
        (r"(crsr_[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(gho_[A-Za-z0-9]{4})[A-Za-z0-9]+", "$1•••"),
        (r"(ghp_[A-Za-z0-9]{4})[A-Za-z0-9]+", "$1•••"),
        (r"(github_pat_[A-Za-z0-9_]{4})[A-Za-z0-9_]+", "$1•••"),
        (r"(ntn_[A-Za-z0-9]{4})[A-Za-z0-9]+", "$1•••"),
        (r"(sk-ant-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(sk-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(xai-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"AKIA[A-Z0-9]{12,}", "AKIA•••"),
        (r"(?i)(Authorization:\s*)\S+", "$1•••"),
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
            if secret_like_assignment(&caps[2]) {
                format!("{}•••", &caps[1])
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    // Aturan 1b: sama untuk bentuk lepas.
    s = RULE1_BARE
        .replace_all(&s, |caps: &Captures| {
            if secret_like_bare(&caps[2]) {
                format!("{}•••", &caps[1])
            } else {
                caps[0].to_string()
            }
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
        assert!(!redact("Authorization: Bearer abc.def.ghi").contains("abc.def.ghi"));
    }

    #[test]
    fn gerbang_rahasia_mendeteksi_semua_pola() {
        // Tiap pola di RULES harus memicu gerbang tulis.
        let rahasia = [
            "token=AbCdEf1234567890",
            "api-key crsr_1234567890abcdef",
            "api_key=AKIA123456789012",
            "token=sk-ant-abc1234567890",
            "secret=ghp_1234567890abcdef1234567890abcdef1234",
            "password=sk-abc1234567890",
            "xai-abc1234567890",
            "AKIA1234567890123456",
            "Bearer abc.def.ghi123",
            "Authorization: abcdef",
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
}
