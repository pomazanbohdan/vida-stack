//! Pure task update command policy helpers.

pub fn task_update_semantics_arg<'a>(
    value: Option<&'a str>,
    clear: bool,
) -> Result<Option<Option<&'a str>>, String> {
    if value.is_some() && clear {
        return Err(
            "Use either the value flag or the matching clear flag for execution semantics, not both."
                .to_string(),
        );
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

pub fn task_update_parent_arg<'a>(
    value: Option<&'a str>,
    clear: bool,
) -> Result<Option<Option<&'a str>>, String> {
    if value.is_some() && clear {
        return Err("Use either --parent-id or --clear-parent-id, not both.".to_string());
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

#[must_use]
pub fn parse_label_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
}

#[must_use]
pub fn parse_optional_label_value(value: Option<&str>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    })
}

#[must_use]
pub fn parse_proof_target_values(values: &[String]) -> Vec<String> {
    normalize_proof_target_commands(parse_label_values(values))
}

#[must_use]
pub fn normalize_proof_target_commands(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|value| normalize_proof_target_command(&value))
        .collect()
}

fn normalize_proof_target_command(value: &str) -> Vec<String> {
    let command = normalize_stale_proof_target_command(value);
    split_cargo_test_proof_target(&command).unwrap_or_else(|| vec![command])
}

fn normalize_stale_proof_target_command(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed == "vida diagnostics --json" {
        return "vida diagnostics post-commit --json".to_string();
    }
    if trimmed.starts_with("vida docflow protocol-coverage-check ") {
        let mut tokens = trimmed.split_whitespace().peekable();
        let mut normalized = Vec::new();
        while let Some(token) = tokens.next() {
            if token == "--format" {
                let _ = tokens.next();
                continue;
            }
            normalized.push(token);
        }
        return normalized.join(" ");
    }
    trimmed.to_string()
}

fn split_cargo_test_proof_target(command: &str) -> Option<Vec<String>> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 4 || tokens[0] != "cargo" || tokens[1] != "test" {
        return None;
    }

    let separator_index = tokens
        .iter()
        .position(|token| *token == "--")
        .unwrap_or(tokens.len());
    let mut base = vec![tokens[0], tokens[1]];
    let mut filters = Vec::new();
    let mut index = 2;
    while index < separator_index {
        let token = tokens[index];
        if token.starts_with('-') {
            base.push(token);
            if cargo_test_option_takes_value(token) && index + 1 < separator_index {
                index += 1;
                base.push(tokens[index]);
            }
        } else {
            filters.push(token);
        }
        index += 1;
    }

    if filters.len() <= 1 {
        return None;
    }

    let tail = &tokens[separator_index..];
    Some(
        filters
            .into_iter()
            .map(|filter| {
                base.iter()
                    .chain(std::iter::once(&filter))
                    .chain(tail.iter())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect(),
    )
}

fn cargo_test_option_takes_value(option: &str) -> bool {
    matches!(
        option,
        "-p" | "--package"
            | "--exclude"
            | "--features"
            | "--bin"
            | "--bench"
            | "--example"
            | "--test"
            | "--target"
            | "--target-dir"
            | "--manifest-path"
            | "--message-format"
            | "--profile"
            | "--jobs"
            | "-j"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_proof_target_commands, parse_label_values, parse_optional_label_value,
        parse_proof_target_values, task_update_parent_arg, task_update_semantics_arg,
    };

    #[test]
    fn task_update_semantics_arg_rejects_value_and_clear_together() {
        let error = task_update_semantics_arg(Some("parallel_safe"), true)
            .expect_err("value plus clear should fail");

        assert!(error.contains("not both"));
    }

    #[test]
    fn task_update_clearable_args_preserve_set_and_clear_intent() {
        assert_eq!(
            task_update_semantics_arg(Some("parallel_safe"), false).expect("set should pass"),
            Some(Some("parallel_safe"))
        );
        assert_eq!(
            task_update_semantics_arg(None, true).expect("clear should pass"),
            Some(None)
        );
        assert_eq!(
            task_update_parent_arg(Some("parent"), false).expect("parent set should pass"),
            Some(Some("parent"))
        );
        assert_eq!(
            task_update_parent_arg(None, true).expect("parent clear should pass"),
            Some(None)
        );
    }

    #[test]
    fn task_update_parent_arg_rejects_parent_and_clear_together() {
        let error = task_update_parent_arg(Some("parent"), true)
            .expect_err("parent plus clear should fail");

        assert!(error.contains("--parent-id"));
    }

    #[test]
    fn parse_label_values_splits_commas_trims_and_drops_empty_values() {
        let values = parse_label_values(&[
            "alpha, beta".to_string(),
            " ".to_string(),
            "gamma".to_string(),
        ]);

        assert_eq!(values, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn parse_optional_label_value_splits_single_flag() {
        assert_eq!(
            parse_optional_label_value(Some("alpha, beta")),
            Some(vec!["alpha".to_string(), "beta".to_string()])
        );
        assert_eq!(parse_optional_label_value(None), None);
    }

    #[test]
    fn proof_targets_normalize_stale_commands_and_split_multi_filter_cargo_tests() {
        assert_eq!(
            parse_proof_target_values(&["vida diagnostics --json".to_string()]),
            vec!["vida diagnostics post-commit --json"]
        );
        assert_eq!(
            normalize_proof_target_commands(vec![
                "cargo test -p vida alpha beta -- --nocapture".to_string()
            ]),
            vec![
                "cargo test -p vida alpha -- --nocapture".to_string(),
                "cargo test -p vida beta -- --nocapture".to_string(),
            ]
        );
        assert_eq!(
            normalize_proof_target_commands(vec![
                "vida docflow protocol-coverage-check docs --format json".to_string()
            ]),
            vec!["vida docflow protocol-coverage-check docs"]
        );
    }
}
