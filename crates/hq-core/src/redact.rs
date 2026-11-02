//! Sensor rahasia: API key/token tidak pernah tampil atau tersimpan.
//! Port Rust dari redact v1 yang sudah teruji di lapangan.

use once_cell::sync::Lazy;
use regex::Regex;

static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let defs: Vec<(&str, &'static str)> = vec![
        (
            r#"(?i)(--?(?:api[-_]?key|token|password|passwd|secret|auth|credential)[= ]+)[^\s"']+"#,
            "$1•••",
        ),
        (r"(crsr_[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(gho_[A-Za-z0-9]{4})[A-Za-z0-9]+", "$1•••"),
        (r"(github_pat_[A-Za-z0-9_]{4})[A-Za-z0-9_]+", "$1•••"),
        (r"(sk-ant-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(sk-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"(xai-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+", "$1•••"),
        (r"AKIA[A-Z0-9]{12,}", "AKIA•••"),
        (r"(?i)(Bearer\s+)[A-Za-z0-9._~+/-]+=*", "$1•••"),
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
    for (re, rep) in RULES.iter() {
        s = re.replace_all(&s, *rep).into_owned();
    }
    s
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
        assert!(!redact("gho_1234567890abcdefghij").contains("1234567890abcdefghij"));
        assert!(!redact("Authorization: Bearer abc.def.ghi").contains("abc.def.ghi"));
    }
}
