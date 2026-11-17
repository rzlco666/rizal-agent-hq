pub(crate) fn template(key: &str) -> Option<&'static str> {
    match key {
        "agent.started_task" => Some("{agent} mulai mengerjakan {object}"),
        "agent.running_tests" => Some("{agent} menjalankan tes {object}: {detail}"),
        "agent.building" => Some("{agent} membangun {object}"),
        "agent.editing_file" => Some("{agent} sedang menulis {object}"),
        "agent.reading" => Some("{agent} membaca {object}"),
        "agent.searching" => Some("{agent} mencari {object}"),
        "agent.committing" => Some("{agent} menyimpan perubahan {object}"),
        "agent.waiting_user" => Some("{agent} menunggu Rizal: {detail}"),
        "agent.completed" => Some("{agent} menyelesaikan {object}: {outcome}"),
        "agent.failed" => Some("{agent} gagal pada {object}: {outcome}"),
        "agent.idle" => Some("{agent} sedang menganggur"),
        "agent.thinking" => Some("{agent} sedang berpikir: {detail}"),
        "agent.meeting" => Some("{agent} dalam rapat {object}"),
        "agent.serving" => Some("{agent} melayani permintaan: {detail}"),
        "agent.logged" => Some("{agent} mencatat: {detail}"),
        _ => None,
    }
}
