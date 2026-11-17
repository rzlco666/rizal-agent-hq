use crate::Locale;
use hq_core::Sentence;

/// Template keys with a static rendering in every supported locale.
pub mod templates {
    pub const KEYS: &[&str] = &[
        "agent.started_task",
        "agent.running_tests",
        "agent.building",
        "agent.editing_file",
        "agent.reading",
        "agent.searching",
        "agent.committing",
        "agent.waiting_user",
        "agent.completed",
        "agent.failed",
        "agent.idle",
        "agent.thinking",
        "agent.meeting",
        "agent.serving",
        "agent.logged",
    ];
}

/// Render a stored template plus params for the requested locale. Missing
/// params stay visible as `{name}` so contract tests catch them immediately.
pub fn render(sentence: &Sentence, locale: Locale) -> String {
    let template = locale
        .template(&sentence.key)
        .unwrap_or(sentence.key.as_str());
    substitute(template, &sentence.params)
}

fn substitute(template: &str, params: &std::collections::BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match params.get(name) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
