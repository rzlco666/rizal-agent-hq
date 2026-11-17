//! Subset parity comparator for oracle v2 vs candidate v3 snapshots.
//! Only GET is used by `fetch_state`; comparison never writes to either host.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub agents: Vec<AgentState>,
    pub usage: Usage,
    pub alerts: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentState {
    pub id: String,
    pub status: String,
    pub usage: Usage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub requests: f64,
    #[serde(alias = "inputTokens")]
    pub input_tokens: f64,
    #[serde(alias = "outputTokens")]
    pub output_tokens: f64,
    #[serde(alias = "cacheTokens")]
    pub cache_tokens: f64,
    #[serde(alias = "cashUsd")]
    pub cash_usd: f64,
    #[serde(alias = "shadowUsd")]
    pub shadow_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMatch {
    pub id: String,
    pub oracle_status: String,
    pub candidate_status: String,
    pub status_matched: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParityReport {
    pub matched: Vec<AgentMatch>,
    pub missing_from_candidate: Vec<String>,
    pub extra_in_candidate: Vec<String>,
    pub status_match_pct: f64,
    pub usage_diff_pct: f64,
    pub verdict: String,
}

pub fn parse_aliases(text: &str) -> Result<HashMap<String, String>, String> {
    serde_json::from_str(text).map_err(|error| format!("cannot parse aliases: {error}"))
}

pub async fn fetch_state(url: &str) -> Result<State, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| format!("cannot build HTTP client: {error}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("cannot fetch {url}: {error}"))?;
    let response = response
        .error_for_status()
        .map_err(|error| format!("bad response from {url}: {error}"))?;
    response
        .json::<State>()
        .await
        .map_err(|error| format!("cannot decode state from {url}: {error}"))
}

/// Round to three significant figures so rounding noise is not a parity failure.
pub fn round_three_significant(value: f64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return 0.0;
    }
    let magnitude = value.abs().log10().floor();
    let scale = 10_f64.powf(2.0 - magnitude);
    (value * scale).round() / scale
}

pub async fn compare_states(
    oracle: &State,
    candidate: &State,
    aliases: &HashMap<String, String>,
    subset: &[&str],
) -> Result<ParityReport, String> {
    let oracle_agents = selected(&oracle.agents, aliases, subset);
    let candidate_agents = selected(&candidate.agents, aliases, subset);

    let oracle_ids: HashSet<String> = oracle_agents.iter().map(|agent| agent.id.clone()).collect();
    let candidate_ids: HashSet<String> = candidate_agents
        .iter()
        .map(|agent| agent.id.clone())
        .collect();
    let mut missing: Vec<String> = oracle_ids.difference(&candidate_ids).cloned().collect();
    let mut extra: Vec<String> = candidate_ids.difference(&oracle_ids).cloned().collect();
    missing.sort();
    extra.sort();

    let mut matched = Vec::new();
    for oracle_agent in &oracle_agents {
        let Some(candidate_agent) = candidate_agents
            .iter()
            .find(|agent| agent.id == oracle_agent.id)
        else {
            continue;
        };
        let oracle_status = normalize_status(&oracle_agent.status);
        let candidate_status = normalize_status(&candidate_agent.status);
        matched.push(AgentMatch {
            id: oracle_agent.id.clone(),
            oracle_status: oracle_status.clone(),
            candidate_status: candidate_status.clone(),
            status_matched: oracle_status == candidate_status,
        });
    }
    let status_match_pct = if matched.is_empty() {
        0.0
    } else {
        matched.iter().filter(|pair| pair.status_matched).count() as f64 * 100.0
            / matched.len() as f64
    };

    let mut oracle_usage = total_usage(&oracle_agents);
    if oracle_usage == Usage::default() {
        oracle_usage = oracle.usage;
    }
    let mut candidate_usage = total_usage(&candidate_agents);
    if candidate_usage == Usage::default() {
        candidate_usage = candidate.usage;
    }
    let usage_diff_pct = usage_difference(&oracle_usage, &candidate_usage);

    let verdict = if missing.is_empty() && status_match_pct >= 90.0 && usage_diff_pct <= 15.0 {
        "PASS"
    } else {
        "FAIL"
    };
    Ok(ParityReport {
        matched,
        missing_from_candidate: missing,
        extra_in_candidate: extra,
        status_match_pct,
        usage_diff_pct,
        verdict: verdict.to_owned(),
    })
}

fn selected(agents: &[AgentState], aliases: &HashMap<String, String>, subset: &[&str]) -> Vec<AgentState> {
    let mut selected: Vec<AgentState> = agents
        .iter()
        .filter_map(|agent| {
            let id = canonical_id(&agent.id, aliases);
            in_subset(&id, subset).then(|| AgentState {
                id,
                status: agent.status.clone(),
                usage: agent.usage,
            })
        })
        .collect();
    selected.sort_by(|left, right| left.id.cmp(&right.id));
    selected.dedup_by(|left, right| left.id == right.id);
    selected
}

fn canonical_id(id: &str, aliases: &HashMap<String, String>) -> String {
    let alias = aliases
        .get(id)
        .or_else(|| aliases.get(&id.to_ascii_lowercase()));
    match alias.map(String::as_str) {
        Some(target) => match target.strip_suffix('*') {
            Some(prefix) => prefix.trim_end_matches('/').to_owned(),
            None => target.to_owned(),
        },
        None => id.to_owned(),
    }
}

fn in_subset(id: &str, subset: &[&str]) -> bool {
    if subset.is_empty() {
        return true;
    }
    let id = id.to_ascii_lowercase();
    subset.iter().any(|token| {
        let token = token.trim().to_ascii_lowercase();
        id.contains(&token) || id.contains(token.trim_start_matches("agent:"))
    })
}

fn normalize_status(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "bekerja" | "working" | "active" | "work" => "bekerja",
        "berpikir" | "thinking" => "berpikir",
        "menunggu" | "menunggu-rizal" | "waiting" => "menunggu-rizal",
        "santai" | "idle" | "succeeded" => "santai",
        "offline" | "absent" => "offline",
        "error" | "failed" => "error",
        other => other,
    }
    .to_owned()
}

fn total_usage(agents: &[AgentState]) -> Usage {
    let mut total = Usage::default();
    for agent in agents {
        total.requests += agent.usage.requests;
        total.input_tokens += agent.usage.input_tokens;
        total.output_tokens += agent.usage.output_tokens;
        total.cache_tokens += agent.usage.cache_tokens;
        total.cash_usd += agent.usage.cash_usd;
        total.shadow_usd += agent.usage.shadow_usd;
    }
    total
}

fn usage_difference(oracle: &Usage, candidate: &Usage) -> f64 {
    let pairs = [
        (oracle.requests, candidate.requests),
        (oracle.input_tokens, candidate.input_tokens),
        (oracle.output_tokens, candidate.output_tokens),
        (oracle.cache_tokens, candidate.cache_tokens),
        (oracle.cash_usd, candidate.cash_usd),
        (oracle.shadow_usd, candidate.shadow_usd),
    ];
    pairs
        .iter()
        .map(|(left, right)| {
            let left = round_three_significant(*left);
            let right = round_three_significant(*right);
            if left == 0.0 {
                if right == 0.0 { 0.0 } else { 100.0 }
            } else {
                ((right - left).abs() / left.abs()) * 100.0
            }
        })
        .fold(0.0_f64, f64::max)
}
