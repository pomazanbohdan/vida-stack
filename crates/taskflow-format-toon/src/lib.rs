pub fn render_section(title: &str, body: &str) -> String {
    common_format_toon::render_compact_block(title, body)
}

#[cfg(test)]
mod tests {
    use super::render_section;

    #[test]
    fn renders_compact_section() {
        assert_eq!(render_section("taskflow", "ready"), "taskflow\n  ready");
    }

    #[test]
    fn section_matches_golden_fixture() {
        let expected = include_str!("../../../tests/golden/taskflow/section.toon").trim_end();
        assert_eq!(render_section("taskflow", "ready"), expected);
    }
}
