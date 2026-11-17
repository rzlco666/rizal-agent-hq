use hq_humanize::{Fact, Verb, humanize};

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
