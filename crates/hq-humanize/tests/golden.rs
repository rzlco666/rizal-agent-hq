use hq_humanize::{Fact, Locale, Sentence, Verb, humanize, render};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn running_tests_fact_becomes_sentence() {
    let f = Fact {
        agent: "OMP".into(),
        verb: Verb::RunningTests,
        object: Some("strut-stack-sql".into()),
        detail: Some("188 tes hijau".into()),
        outcome: None,
    };
    let s = humanize(&f);
    assert_eq!(s.key, "agent.running_tests");
    assert_eq!(s.params["agent"], "OMP");
    assert_eq!(s.params["object"], "strut-stack-sql");
}

#[derive(Debug, Deserialize)]
struct Golden {
    fact: Fact,
    sentence: Sentence,
    rendered: BTreeMap<Locale, String>,
}

fn corpus_lines() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/golden");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("golden corpus directory")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).unwrap();
        for line in text.lines() {
            if !line.trim().is_empty() {
                out.push(line.to_string());
            }
        }
    }
    out
}

#[test]
fn golden_corpus_matches_exactly() {
    for line in corpus_lines() {
        let g: Golden = serde_json::from_str(&line).unwrap();
        assert_eq!(humanize(&g.fact), g.sentence, "corpus line: {line}");
        for (loc, expected) in &g.rendered {
            assert_eq!(render(&g.sentence, *loc), *expected, "corpus line: {line}");
        }
    }
}

#[test]
fn golden_corpus_covers_all_verbs_and_locales() {
    let rows = corpus_lines();
    assert!(
        rows.len() >= 20,
        "need at least 20 golden rows, got {}",
        rows.len()
    );
    let mut keys = BTreeMap::new();
    let mut chinese = false;
    let mut long_detail = false;
    let mut empty_object = false;
    for line in &rows {
        let g: Golden = serde_json::from_str(line).unwrap();
        keys.insert(g.sentence.key.clone(), true);
        assert_eq!(
            g.rendered.keys().copied().collect::<Vec<_>>(),
            vec![Locale::Id, Locale::En, Locale::Zh],
            "rendered must carry exactly id/en/zh: {line}"
        );
        if g.fact
            .object
            .as_deref()
            .is_some_and(|s| s.chars().any(|c| c as u32 > 0x7f))
            || g.fact
                .detail
                .as_deref()
                .is_some_and(|s| s.chars().any(|c| c as u32 > 0x7f))
        {
            chinese = true;
        }
        if g.fact.detail.as_deref().is_some_and(|s| s.len() > 80) {
            long_detail = true;
        }
        if g.fact.object.as_deref().is_none_or(str::is_empty) {
            empty_object = true;
        }
    }
    for verb in Verb::ALL {
        assert!(
            keys.contains_key(verb.key()),
            "corpus missing verb {verb:?}"
        );
    }
    assert!(chinese, "corpus needs a Unicode row");
    assert!(long_detail, "corpus needs a long detail row");
    assert!(empty_object, "corpus needs a null/empty object row");
}

#[test]
fn all_free_text_params_are_redacted() {
    let f = Fact {
        agent: "OMP".into(),
        verb: Verb::Serving,
        object: None,
        detail: Some("token ghp_abcdefghijklmnop".into()),
        outcome: None,
    };
    let s = humanize(&f);
    assert_eq!(s.params["detail"], "token [REDACTED:github]");
}
