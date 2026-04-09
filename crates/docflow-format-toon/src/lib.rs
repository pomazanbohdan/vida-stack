pub fn render_summary(title: &str, body: &str) -> String {
    common_format_toon::render_compact_block(title, body)
}

#[cfg(test)]
mod tests {
    use super::render_summary;

    #[test]
    fn renders_compact_summary() {
        assert_eq!(render_summary("docflow", "ready"), "docflow\n  ready");
    }

    #[test]
    fn summary_matches_golden_fixture() {
        let expected = include_str!("../../../tests/golden/docflow/summary.toon").trim_end();
        assert_eq!(render_summary("docflow", "ready"), expected);
    }
}
