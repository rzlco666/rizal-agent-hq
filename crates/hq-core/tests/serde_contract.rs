use hq_core::*;

#[test]
fn status_uses_kebab_case() {
    assert_eq!(
        serde_json::to_string(&Status::MenungguRizal).unwrap(),
        "\"menunggu-rizal\""
    );
}

#[test]
fn sentence_requires_key() {
    assert!(Sentence::new("", Default::default()).is_err());
}

#[test]
fn capability_shell_requires_name() {
    assert!(Capability::parse("shell.run:").is_err());
    assert_eq!(
        Capability::parse("shell.run:git-status").unwrap(),
        Capability::ShellRun("git-status".into())
    );
}
