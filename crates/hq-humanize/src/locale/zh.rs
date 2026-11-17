pub(crate) fn template(key: &str) -> Option<&'static str> {
    match key {
        "agent.started_task" => Some("{agent} 开始处理 {object}"),
        "agent.running_tests" => Some("{agent} 正在运行 {object} 的测试：{detail}"),
        "agent.building" => Some("{agent} 正在构建 {object}"),
        "agent.editing_file" => Some("{agent} 正在编辑 {object}"),
        "agent.reading" => Some("{agent} 正在阅读 {object}"),
        "agent.searching" => Some("{agent} 正在搜索 {object}"),
        "agent.committing" => Some("{agent} 正在提交 {object}"),
        "agent.waiting_user" => Some("{agent} 正在等待 Rizal：{detail}"),
        "agent.completed" => Some("{agent} 已完成 {object}：{outcome}"),
        "agent.failed" => Some("{agent} 在 {object} 上失败：{outcome}"),
        "agent.idle" => Some("{agent} 空闲中"),
        "agent.thinking" => Some("{agent} 正在思考：{detail}"),
        "agent.meeting" => Some("{agent} 正在开会讨论 {object}"),
        "agent.serving" => Some("{agent} 正在处理请求：{detail}"),
        "agent.logged" => Some("{agent} 记录：{detail}"),
        _ => None,
    }
}
