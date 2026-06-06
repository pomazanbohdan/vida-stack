pub(crate) fn human_command(command: &str) -> String {
    command
        .split_whitespace()
        .filter(|token| *token != "--json")
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::human_command;

    #[test]
    fn human_command_removes_json_flag_without_changing_other_tokens() {
        assert_eq!(
            human_command("vida taskflow consume continue --json"),
            "vida taskflow consume continue"
        );
        assert_eq!(
            human_command("vida taskflow consume --json continue"),
            "vida taskflow consume continue"
        );
        assert_eq!(
            human_command("--json vida taskflow consume continue"),
            "vida taskflow consume continue"
        );
        assert_eq!(
            human_command("vida task show CaseSensitiveID --json --fields status,title"),
            "vida task show CaseSensitiveID --fields status,title"
        );
        assert_eq!(
            human_command("vida task show --json-output fixture --json"),
            "vida task show --json-output fixture"
        );
    }
}
