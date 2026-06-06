pub use common_format_toon::sanitize_toon_scalar;

pub fn render_section(title: &str, body: &str) -> String {
    common_format_toon::render_compact_block(title, body)
}

pub fn render_value_section<T: serde::Serialize>(title: &str, value: &T) -> String {
    common_format_toon::render_toon_value_block(title, value)
}

#[cfg(test)]
mod tests {
    use super::{render_section, render_value_section, sanitize_toon_scalar};

    #[test]
    fn renders_compact_section() {
        assert_eq!(render_section("taskflow", "ready"), "taskflow\n  ready");
    }

    #[test]
    fn reexports_shared_toon_scalar_sanitizer() {
        assert_eq!(
            sanitize_toon_scalar("task\nready\x1b[31m"),
            r"task\nready\u{1b}[31m"
        );
    }

    #[test]
    fn section_matches_golden_fixture() {
        let expected = include_str!("../../../tests/golden/taskflow/section.toon").trim_end();
        assert_eq!(render_section("taskflow", "ready"), expected);
    }

    #[test]
    fn value_section_uses_tabular_headers() {
        let value = serde_json::json!({
            "tasks": [
                {
                    "id": "task-1",
                    "status": "open",
                },
            ],
        });

        assert_eq!(
            render_value_section("taskflow", &value),
            "taskflow\n  tasks[1]{id,status}:\n    \"task-1\",open"
        );
    }
}
