use hq_humanize::{Locale, Sentence, render, templates};
use std::collections::BTreeMap;

fn params_for(_key: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("agent".to_string(), "OMP".to_string()),
        ("object".to_string(), "strut-stack-sql".to_string()),
        ("detail".to_string(), "188 tes hijau".to_string()),
        ("outcome".to_string(), "semua hijau".to_string()),
    ])
}

#[test]
fn every_key_renders_in_every_locale() {
    for key in templates::KEYS {
        for loc in [Locale::Id, Locale::En, Locale::Zh] {
            let s = Sentence::new(*key, params_for(key)).unwrap();
            let out = render(&s, loc);
            assert!(!out.is_empty());
            assert_ne!(out, *key, "missing template {key} for {loc:?}");
            assert!(
                !out.contains('{'),
                "unfilled placeholder in {key} for {loc:?}: {out}"
            );
        }
    }
}

#[test]
fn missing_param_renders_placeholder_not_panic() {
    let s = Sentence::new("agent.running_tests", BTreeMap::new()).unwrap();
    assert!(render(&s, Locale::Id).contains("{object}"));
    assert!(render(&s, Locale::En).contains("{agent}"));
    assert!(render(&s, Locale::Zh).contains("{detail}"));
}

#[test]
fn locale_parse_defaults_to_id_and_rejects_unknown() {
    assert_eq!(Locale::parse("id").unwrap(), Locale::Id);
    assert_eq!(Locale::parse("en").unwrap(), Locale::En);
    assert_eq!(Locale::parse("zh").unwrap(), Locale::Zh);
    assert_eq!(Locale::default(), Locale::Id);
    assert!(Locale::parse("xx").is_err());
}

#[test]
fn unknown_key_renders_key_without_panic() {
    let s = Sentence::new("agent.unknown", BTreeMap::new()).unwrap();
    assert_eq!(render(&s, Locale::En), "agent.unknown");
}
