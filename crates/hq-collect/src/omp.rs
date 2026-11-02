//! Parser OMP: `omp stats --json` (usage per model & tipe agent) dan
//! `omp usage` (kuota) — format asli terverifikasi 2 Nov 2026, sama seperti
//! parser v1.0.1 yang sudah terbukti di lapangan.

use hq_core::{Quota, Usage};
use serde_json::Value;
use std::collections::HashSet;

pub fn cash_providers() -> HashSet<&'static str> {
    ["deepseek", "kilo", "openrouter", "cloudflare-ai-gateway", "opencode-zen"]
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct ModelUsage {
    pub provider: String,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Default)]
pub struct OmpStats {
    pub ok: bool,
    pub overall: Usage,
    pub total_cost_raw: f64,
    pub by_model: Vec<ModelUsage>,
    pub main: Usage,
    pub subagent: Usage,
    pub last_ts: i64,
    pub error_rate: f64,
}

fn u(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
}
fn f(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
}

fn usage_of(v: &Value) -> Usage {
    Usage {
        requests: u(v, "totalRequests"),
        ok: u(v, "successfulRequests"),
        failed: u(v, "failedRequests"),
        input_tokens: u(v, "totalInputTokens"),
        output_tokens: u(v, "totalOutputTokens"),
        cache_tokens: u(v, "totalCacheReadTokens"),
        cash_usd: 0.0,
        shadow_usd: 0.0,
    }
}

/// Pisah biaya satu model ke TUNAI vs bayangan.
pub fn split_one(provider: &str, model: &str, cost: f64) -> (f64, f64) {
    if cost == 0.0 || model.to_lowercase().contains("free") {
        (0.0, 0.0)
    } else if cash_providers().contains(provider) {
        (cost, 0.0)
    } else {
        (0.0, cost)
    }
}

pub fn parse_stats(json_text: &str) -> Option<OmpStats> {
    let j: Value = serde_json::from_str(json_text).ok()?;
    let overall_v = j.get("overall")?;
    let mut out = OmpStats {
        ok: true,
        total_cost_raw: f(overall_v, "totalCost"),
        error_rate: f(overall_v, "errorRate"),
        last_ts: overall_v.get("lastTimestamp").and_then(|x| x.as_i64()).unwrap_or(0),
        ..Default::default()
    };
    let mut ov = usage_of(overall_v);
    if let Some(models) = j.get("byModel").and_then(|x| x.as_array()) {
        for m in models {
            let provider = m.get("provider").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let model = m.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let cost = f(m, "totalCost");
            let (cash, shadow) = split_one(&provider, &model, cost);
            let mut mu = usage_of(m);
            mu.cash_usd = cash;
            mu.shadow_usd = shadow;
            ov.cash_usd += cash;
            ov.shadow_usd += shadow;
            out.by_model.push(ModelUsage { provider, model, usage: mu });
        }
    }
    out.by_model.sort_by(|a, b| b.usage.total_cost().partial_cmp(&a.usage.total_cost()).unwrap());
    out.overall = ov;
    if let Some(types) = j.get("byAgentType").and_then(|x| x.as_array()) {
        for t in types {
            // byAgentType tidak membawa rincian sukses/gagal; ambil token+cost & proporsi model tidak tersedia.
            let mut tu = Usage {
                requests: u(t, "totalRequests"),
                input_tokens: u(t, "totalInputTokens"),
                output_tokens: u(t, "totalOutputTokens"),
                cache_tokens: u(t, "totalCacheReadTokens"),
                ..Default::default()
            };
            let cost = f(t, "totalCost");
            // Atribusi kasar: proporsi cost tipe ini dianggap bayangan kecuali terbukti DeepSeek;
            // angka global tetap akurat dari byModel. (Per-sesi presisi = pekerjaan M2 dari file sesi.)
            tu.shadow_usd = cost;
            match t.get("agentType").and_then(|x| x.as_str()) {
                Some("main") => out.main = tu,
                Some("subagent") => out.subagent = tu,
                _ => {}
            }
        }
    }
    Some(out)
}

fn norm_provider(name: &str) -> String {
    let n = name.trim().to_lowercase().replace(' ', "-");
    match n.as_str() {
        "openai" | "openai-codex" => "openai-codex".into(),
        "xai" | "xai-oauth" => "xai-oauth".into(),
        "github" | "github-copilot" => "github-copilot".into(),
        "google" | "google-antigravity" => "google-antigravity".into(),
        "kimi" | "kimi-code" => "kimi-code".into(),
        other => other.to_string(),
    }
}

/// Parser `omp usage` teks (header "Nama — N account", baris "Label ━━ X% used • resets ...").
pub fn parse_usage(text: &str) -> Vec<Quota> {
    let mut out: Vec<Quota> = Vec::new();
    let mut cur = String::from("umum");
    let head_re = regex::Regex::new(r"^([A-Z][A-Za-z ]+?)\s+—\s+\d+\s+account").unwrap();
    let used_re = regex::Regex::new(r"(?i)^(.*?)\s+([\d.,]+)%\s+used\s*(?:·|•)\s*(.*)$").unwrap();
    let req_re = regex::Regex::new(r"(?i)^(.*?)\s+([\d.,]+)\s*/\s*([\d.,]+[KM]?)\s+requests").unwrap();
    let bars = regex::Regex::new(r"[\u{2500}-\u{257F}\u{2580}-\u{259F}]").unwrap();
    for raw in text.lines() {
        let line0 = raw.replace('\u{1b}', "");
        if let Some(c) = head_re.captures(line0.trim()) {
            cur = norm_provider(&c[1]);
            continue;
        }
        let cleaned = bars.replace_all(line0.trim(), "");
        let line = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        let line = line.trim_start_matches('•').trim().to_string();
        if line.is_empty() || line.to_lowercase().starts_with("usage") || line.to_lowercase().starts_with("capacity:") {
            continue;
        }
        if let Some(c) = used_re.captures(&line) {
            out.push(Quota {
                provider: cur.clone(),
                label: hq_core::redact::redact(&c[1]).chars().take(70).collect(),
                percent: c[2].replace(',', ".").parse::<f64>().ok(),
                extra: hq_core::redact::redact(&c[3]).chars().take(60).collect(),
            });
        } else if let Some(c) = req_re.captures(&line) {
            out.push(Quota {
                provider: cur.clone(),
                label: hq_core::redact::redact(&c[1]).chars().take(70).collect(),
                percent: None,
                extra: format!("{} / {} requests", &c[2], &c[3]),
            });
        } else if line.to_lowercase().contains("no usage data") {
            out.push(Quota {
                provider: cur.clone(),
                label: "Tidak ada data usage dari API provider ini".into(),
                percent: None,
                extra: String::new(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_usage_format_asli() {
        let sample = "Usage · fetched 58.1s ago\n\nCursor — 1 account\n  ● mail@x.com\n      ● Cursor Models (Monthly)  ██████  100.0% used · resets in 15d17h\n\nGoogle Antigravity — 1 account\n      ● Claude & GPT (shared) (Weekly)  ███  21.2% used · resets in 6d23h\n      ● Gemini (Weekly)  ░░  0.4% used · resets in 6d23h\n\nOpenai Codex — 1 account\n      ● 7 days   ██  63.0% used · resets in 3d1h\n";
        let q = parse_usage(sample);
        assert_eq!(q.len(), 4);
        assert_eq!(q[0].provider, "cursor");
        assert_eq!(q[0].percent, Some(100.0));
        assert_eq!(q[1].provider, "google-antigravity");
        assert_eq!(q[3].provider, "openai-codex");
    }

    #[test]
    fn split_tunai_vs_bayangan() {
        assert_eq!(split_one("deepseek", "deepseek-v4-flash", 1.5), (1.5, 0.0));
        assert_eq!(split_one("openai-codex", "gpt-6.1-sol", 7.0), (0.0, 7.0));
        assert_eq!(split_one("opencode-zen", "deepseek-v4.1-flash-free", 0.0), (0.0, 0.0));
        // OpenCode Go = langganan flat $10/bln: meteran stats-nya BAYANGAN, bukan tunai.
        assert_eq!(split_one("opencode-go", "glm-5.3-flash", 1.0), (0.0, 1.0));
    }
}
