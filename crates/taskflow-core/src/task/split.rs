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
    let mut parsed = Vec::with_capacity(values.len());
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

        let error =
            parse_split_child_specs(&["child-a:First".to_string(), "child-a:Second".to_string()])
                .expect_err("duplicate child id should fail");
        assert!(error.contains("Duplicate split child task id"));
    }
}
