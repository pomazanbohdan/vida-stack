//! Core helpers for task split command planning.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSplitChildSpec {
    pub task_id: String,
    pub title: String,
}

pub fn parse_split_child_specs(values: &[String]) -> Result<Vec<ParsedSplitChildSpec>, String> {
    if values.len() < 2 {
        return Err(
            "Use at least two `--child <task-id>:<title>` entries for `vida task split`."
                .to_string(),
        );
    }

    let mut seen = BTreeSet::new();
    let mut parsed = Vec::new();
    for value in values {
        let Some((task_id, title)) = value.split_once(':') else {
            return Err(format!(
                "Invalid `--child` value `{value}`. Expected `<task-id>:<title>`."
            ));
        };
        let task_id = task_id.trim();
        let title = title.trim();
        if task_id.is_empty() || title.is_empty() {
            return Err(format!(
                "Invalid `--child` value `{value}`. Both task id and title are required."
            ));
        }
        if !seen.insert(task_id.to_string()) {
            return Err(format!("Duplicate split child task id `{task_id}`."));
        }
        parsed.push(ParsedSplitChildSpec {
            task_id: task_id.to_string(),
            title: title.to_string(),
        });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::parse_split_child_specs;

    #[test]
    fn split_child_specs_require_two_children() {
        let error = parse_split_child_specs(&[]).expect_err("empty split should fail");
        assert!(error.contains("at least two"));

        let error = parse_split_child_specs(&["child-a:First".to_string()])
            .expect_err("single child split should fail");
        assert!(error.contains("at least two"));
    }

    #[test]
    fn split_child_specs_parse_and_reject_duplicates() {
        let parsed =
            parse_split_child_specs(&["child-a:First".to_string(), "child-b:Second".to_string()])
                .expect("valid children should parse");
        assert_eq!(parsed[0].task_id, "child-a");
        assert_eq!(parsed[0].title, "First");
        assert_eq!(parsed[1].task_id, "child-b");
        assert_eq!(parsed[1].title, "Second");

        let error =
            parse_split_child_specs(&["child-a:First".to_string(), "child-a:Second".to_string()])
                .expect_err("duplicate child id should fail");
        assert!(error.contains("Duplicate split child task id"));
    }

    #[test]
    fn split_child_specs_trim_fields_and_split_only_at_the_first_colon() {
        let parsed = parse_split_child_specs(&[
            " child-a : First: retain the rest ".to_string(),
            "child-b: Second".to_string(),
            "child-c:Third".to_string(),
        ])
        .expect("trimmed children with a colon in a title should parse");

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].task_id, "child-a");
        assert_eq!(parsed[0].title, "First: retain the rest");
        assert_eq!(parsed[1].title, "Second");
        assert_eq!(parsed[2].task_id, "child-c");
    }

    #[test]
    fn split_child_specs_reject_missing_separator_and_empty_fields() {
        for value in ["child-a", ":Title", "child-a:", " : "] {
            let error = parse_split_child_specs(&[value.to_string(), "child-b:Second".to_string()])
                .expect_err("malformed child should fail");
            assert!(
                error.contains("Expected `<task-id>:<title>`")
                    || error.contains("Both task id and title are required"),
                "unexpected error for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn split_child_specs_reject_duplicate_ids_after_trimming() {
        let error =
            parse_split_child_specs(&["child-a:First".to_string(), " child-a :Second".to_string()])
                .expect_err("trim-equivalent child ids should fail");

        assert_eq!(error, "Duplicate split child task id `child-a`.");
    }
}
