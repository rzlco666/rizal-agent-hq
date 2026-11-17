use hq_core::*;

#[test]
fn subject_keys_are_canonical() {
    assert_eq!(
        Subject::Agent(AgentId::parse("agent:omp").unwrap()).key(),
        "agent:omp"
    );
    assert_eq!(
        Subject::Session(SessionId::parse("agent:omp/session/s1").unwrap()).key(),
        "agent:omp/session/s1"
    );
    assert_eq!(Subject::Usage(UsageScope::Global).key(), "usage:global");
    assert_eq!(
        Subject::Usage(UsageScope::Agent(AgentId::parse("agent:omp").unwrap())).key(),
        "usage:agent:omp"
    );
    assert_eq!(Subject::System.key(), "system");
}

#[test]
fn subject_rejects_bad_ids() {
    assert!(AgentId::parse("omp").is_err());
    assert!(AgentId::parse("agent:OMP").is_err());
    assert!(AgentId::parse("agent:omp/../../etc").is_err());
}
