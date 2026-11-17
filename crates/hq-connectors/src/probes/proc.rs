use hq_core::redact_text;
use serde::{Deserialize, Serialize};

use super::run::run_timeout;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub pcpu: f32,
    pub pmem: f32,
    pub etime: String,
    pub comm: String,
    pub args: String,
}

pub fn parse_ps(input: &str) -> Vec<ProcessInfo> {
    input
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let mut fields = trimmed.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            let pcpu = fields.next()?.parse().ok()?;
            let pmem = fields.next()?.parse().ok()?;
            let etime = fields.next()?.to_owned();
            let comm = fields.next()?.to_owned();
            let mut end = 0;
            for _ in 0..6 {
                end = trimmed[end..]
                    .find(char::is_whitespace)
                    .map(|i| end + i)
                    .unwrap_or(trimmed.len());
                end = trimmed[end..].len() - trimmed[end..].trim_start().len() + end;
            }
            let args = trimmed[end..].trim();
            if args.is_empty() {
                return None;
            }
            Some(ProcessInfo {
                pid,
                ppid,
                pcpu,
                pmem,
                etime,
                comm,
                args: redact_text(args),
            })
        })
        .collect()
}

pub fn probe_processes() -> Vec<ProcessInfo> {
    run_timeout(
        "ps",
        &["-eo", "pid=,ppid=,pcpu=,pmem=,etime=,comm=,args="],
        2_000,
    )
    .filter(|output| output.success)
    .map(|output| parse_ps(&output.stdout))
    .unwrap_or_default()
}
