pub fn render_compact_block(title: &str, body: &str) -> String {
    format!("{title}\n  {body}")
}

pub fn render_toon_value_block<T: serde::Serialize>(title: &str, value: &T) -> String {
    let body = toon_format::encode_default(value).expect("TOON value should encode");
    render_indented_block(title, &body)
}

fn render_indented_block(title: &str, body: &str) -> String {
    let indented = body
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}\n{indented}")
}

#[cfg(test)]
mod tests {
    use super::{render_compact_block, render_toon_value_block};
    use serde::Serialize;

    #[test]
    fn renders_compact_block() {
        assert_eq!(render_compact_block("common", "ready"), "common\n  ready");
    }

    #[test]
    fn renders_toon_value_block_with_tabular_headers() {
        #[derive(Serialize)]
        struct Row {
            id: &'static str,
            status: &'static str,
        }

        let value = serde_json::json!({
            "rows": [
                Row {
                    id: "task-1",
                    status: "open",
                },
            ],
        });

        assert_eq!(
            render_toon_value_block("common", &value),
            "common\n  rows[1]{id,status}:\n    \"task-1\",open"
        );
    }
}
