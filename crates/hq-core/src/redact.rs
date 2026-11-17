//! Redaksi v2. Pola diterapkan berurutan; RegexSet untuk deteksi cepat.
//! Pola yang diganti menjadi `[REDACTED:<kind>]`. Aturan entropy hanya untuk
//! token >=32 karakter dengan minimal 3 kelas karakter dan bukan hex murni,
//! sehingga git SHA/UUID/ULID tetap utuh.

use regex::Regex;
use regex::RegexSet;
use serde_json::Value;

static QUICK: std::sync::OnceLock<RegexSet> = std::sync::OnceLock::new();
static PATTERNS: std::sync::OnceLock<Vec<(Regex, &'static str)>> = std::sync::OnceLock::new();

fn quick() -> &'static RegexSet {
    QUICK.get_or_init(|| {
        RegexSet::new([
            r"AKIA[0-9A-Z]{16}",
            r"ghp_[A-Za-z0-9_]{4,}",
            r"github_pat_[A-Za-z0-9_]{20,}",
            r"sk-(?:proj-)?[A-Za-z0-9_-]{20,}",
            r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            r"(?i)Authorization:\s*Bearer\s+\S+",
            r"(?i)password\s*[=:]\s*\S+",
            r"[a-zA-Z][a-zA-Z0-9+.-]*://[^\s]+",
            r"[A-Za-z0-9+/=_-]{32,}",
        ])
        .unwrap_or_else(|_| RegexSet::empty())
    })
}

fn patterns() -> &'static Vec<(Regex, &'static str)> {
    PATTERNS.get_or_init(|| {
        let candidates: [(&str, &str); 9] = [
            (r"AKIA[0-9A-Z]{16}", "aws"),
            (r"ghp_[A-Za-z0-9_]{4,}", "github"),
            (r"github_pat_[A-Za-z0-9_]{20,}", "github_pat"),
            (r"sk-(?:proj-)?[A-Za-z0-9_-]{20,}", "openai"),
            (r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+", "jwt"),
            (
                r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
                "private_key",
            ),
            (r"(?i)(Authorization:\s*Bearer\s+)\S+", "bearer"),
            (r"(?i)(password\s*[=:]\s*)\S+", "password"),
            (r"[a-zA-Z][a-zA-Z0-9+.-]*://[^\s]+", "dsn"),
        ];
        let mut out = Vec::new();
        for (pat, kind) in candidates {
            if let Ok(re) = Regex::new(pat) {
                out.push((re, kind));
            }
        }
        out
    })
}

fn is_high_entropy(tok: &str) -> bool {
    if tok.len() < 32 {
        return false;
    }
    // UUID
    if tok.chars().filter(|c| *c == '-').count() == 4 && tok.len() == 36 {
        return false;
    }
    // not pure hex
    if tok.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let mut classes = 0;
    if tok.chars().any(|c| c.is_ascii_lowercase()) {
        classes += 1;
    }
    if tok.chars().any(|c| c.is_ascii_uppercase()) {
        classes += 1;
    }
    if tok.chars().any(|c| c.is_ascii_digit()) {
        classes += 1;
    }
    if tok.chars().any(|c| !c.is_ascii_alphanumeric()) {
        classes += 1;
    }
    classes >= 3
}

pub fn redact_text(input: &str) -> String {
    if !quick().is_match(input) {
        return input.to_string();
    }
    let mut out = input.to_string();
    for (re, kind) in patterns() {
        if *kind == "bearer" {
            out = re
                .replace_all(&out, |caps: &regex::Captures| {
                    format!("{}[REDACTED:bearer]", &caps[1])
                })
                .to_string();
        } else if *kind == "password" {
            out = re
                .replace_all(&out, |caps: &regex::Captures| {
                    format!("{}[REDACTED:password]", &caps[1])
                })
                .to_string();
        } else {
            let repl = format!("[REDACTED:{}]", kind);
            out = re.replace_all(&out, repl.as_str()).to_string();
        }
    }
    // high entropy
    let he = Regex::new(r"[A-Za-z0-9+/=_-]{32,}").ok();
    if let Some(he) = he {
        out = he
            .replace_all(&out, |caps: &regex::Captures| {
                let tok = &caps[0];
                if is_high_entropy(tok) {
                    "[REDACTED:high_entropy]".to_string()
                } else {
                    tok.to_string()
                }
            })
            .to_string();
    }
    out
}

pub fn redact_json(v: &mut Value) {
    match v {
        Value::String(s) => {
            let r = redact_text(s);
            *s = r;
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_json(item);
            }
        }
        Value::Object(map) => {
            for (_k, val) in map.iter_mut() {
                redact_json(val);
            }
        }
        _ => {}
    }
}
