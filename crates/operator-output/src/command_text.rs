pub fn human_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_command_normalizes_whitespace() {
        assert_eq!(
            human_command(" vida   status   --json "),
            "vida status --json"
        );
    }
}
