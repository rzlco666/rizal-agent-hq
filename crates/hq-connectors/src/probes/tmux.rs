use hq_core::redact_text;
use serde::{Deserialize, Serialize};

use super::run::run_timeout;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxSession {
    pub name: String,
    pub windows: u32,
    pub attached: bool,
}

pub fn parse_tmux(input: &str) -> Vec<TmuxSession> {
    input
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '|');
            Some(TmuxSession {
                name: redact_text(fields.next()?),
                windows: fields.next()?.parse().ok()?,
                attached: match fields.next()? {
                    "1" => true,
                    "0" => false,
                    _ => return None,
                },
            })
        })
        .collect()
}

pub fn probe_tmux() -> Vec<TmuxSession> {
    run_timeout(
        "tmux",
        &[
            "list-sessions",
            "-F",
            "#{session_name}|#{session_windows}|#{session_attached}",
        ],
        2_000,
    )
    .filter(|output| output.success)
    .map(|output| parse_tmux(&output.stdout))
    .unwrap_or_default()
}
