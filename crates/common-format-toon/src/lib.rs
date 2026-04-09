pub fn render_compact_block(title: &str, body: &str) -> String {
    format!("{title}\n  {body}")
}

#[cfg(test)]
mod tests {
    use super::render_compact_block;

    #[test]
    fn renders_compact_block() {
        assert_eq!(render_compact_block("common", "ready"), "common\n  ready");
    }
}
