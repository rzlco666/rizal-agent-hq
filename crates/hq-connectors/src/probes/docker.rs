use hq_core::redact_text;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::run::run_timeout;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub image: String,
    pub names: String,
    pub status: String,
}

pub fn parse_docker(input: &str) -> Vec<ContainerInfo> {
    input
        .lines()
        .filter_map(|line| {
            let value: Value = serde_json::from_str(line).ok()?;
            Some(ContainerInfo {
                id: field(&value, &["ID", "Id"]),
                image: field(&value, &["Image"]),
                names: field(&value, &["Names", "Name"]),
                status: field(&value, &["Status", "State"]),
            })
        })
        .filter(|container| !container.id.is_empty() || !container.image.is_empty())
        .collect()
}

fn field(value: &Value, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| value.get(*name)?.as_str())
        .map(redact_text)
        .unwrap_or_default()
}

pub fn probe_docker() -> Vec<ContainerInfo> {
    run_timeout("docker", &["ps", "--format", "{{json .}}"], 3_000)
        .filter(|output| output.success)
        .map(|output| parse_docker(&output.stdout))
        .unwrap_or_default()
}
