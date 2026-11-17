pub(crate) fn template(key: &str) -> Option<&'static str> {
    match key {
        "agent.started_task" => Some("{agent} started {object}"),
        "agent.running_tests" => Some("{agent} running tests for {object}: {detail}"),
        "agent.building" => Some("{agent} building {object}"),
        "agent.editing_file" => Some("{agent} is editing {object}"),
        "agent.reading" => Some("{agent} is reading {object}"),
        "agent.searching" => Some("{agent} is searching for {object}"),
        "agent.committing" => Some("{agent} committing {object}"),
        "agent.waiting_user" => Some("{agent} waiting for Rizal: {detail}"),
        "agent.completed" => Some("{agent} completed {object}: {outcome}"),
        "agent.failed" => Some("{agent} failed on {object}: {outcome}"),
        "agent.idle" => Some("{agent} is idle"),
        "agent.thinking" => Some("{agent} is thinking: {detail}"),
        "agent.meeting" => Some("{agent} in a meeting about {object}"),
        "agent.serving" => Some("{agent} serving requests: {detail}"),
        "agent.logged" => Some("{agent} logged: {detail}"),
        _ => None,
    }
}
