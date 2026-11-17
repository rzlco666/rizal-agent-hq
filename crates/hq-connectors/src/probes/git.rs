use hq_core::redact_text;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::run::run_timeout;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoCommit {
    pub repo: String,
    pub hash: String,
    pub subject: String,
    pub committed_at: i64,
}

pub fn parse_git(input: &str) -> Vec<RepoCommit> {
    input
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '|');
            let hash = fields.next()?.trim();
            let subject = fields.next()?.trim();
            let committed_at = fields.next()?.trim().parse().ok()?;
            if hash.is_empty() {
                return None;
            }
            Some(RepoCommit {
                repo: String::new(),
                hash: hash.to_owned(),
                subject: redact_text(subject),
                committed_at,
            })
        })
        .collect()
}

pub fn probe_git(repos: &[String]) -> Vec<RepoCommit> {
    repos
        .iter()
        .filter_map(|repo| {
            let path = Path::new(repo);
            if !path.is_dir() {
                return None;
            }
            let output = run_timeout(
                "git",
                &["-C", repo, "log", "-1", "--format=%h|%s|%ct"],
                3_000,
            )?;
            if !output.success {
                return None;
            }
            parse_git(&output.stdout)
                .into_iter()
                .next()
                .map(|mut commit| {
                    commit.repo = redact_text(repo);
                    commit
                })
        })
        .collect()
}
