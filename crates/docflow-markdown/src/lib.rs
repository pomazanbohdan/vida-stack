use thiserror::Error;

pub const FOOTER_DELIMITER: &str = "-----";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownArtifact {
    pub body: String,
    pub footer: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MarkdownError {
    #[error("footer block must not be empty when delimiter is present")]
    EmptyFooter,
}

pub fn split_footer(input: &str) -> Result<MarkdownArtifact, MarkdownError> {
    if let Some((body, footer)) = input.split_once(FOOTER_DELIMITER) {
        let trimmed_footer = footer.trim();
        if trimmed_footer.is_empty() {
            return Err(MarkdownError::EmptyFooter);
        }
        return Ok(MarkdownArtifact {
            body: body.trim_end().to_string(),
            footer: Some(trimmed_footer.to_string()),
        });
    }

    Ok(MarkdownArtifact {
        body: input.trim_end().to_string(),
        footer: None,
    })
}

pub fn render_artifact(artifact: &MarkdownArtifact) -> String {
    match &artifact.footer {
        Some(footer) => format!(
            "{}\n\n{FOOTER_DELIMITER}\n{footer}\n",
            artifact.body.trim_end()
        ),
        None => format!("{}\n", artifact.body.trim_end()),
    }
}

pub fn append_changelog_row(changelog: &str, row: &str) -> String {
    if changelog.trim().is_empty() {
        format!("{row}\n")
    } else {
        format!("{}\n{row}\n", changelog.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FOOTER_DELIMITER, MarkdownArtifact, MarkdownError, append_changelog_row, render_artifact,
        split_footer,
    };

    #[test]
    fn splits_body_and_footer() {
        let artifact =
            split_footer("body line\n\n-----\nkey: value\n").expect("split should succeed");
        assert_eq!(artifact.body, "body line");
        assert_eq!(artifact.footer.as_deref(), Some("key: value"));
    }

    #[test]
    fn rejects_empty_footer_block() {
        let error = split_footer(&format!("body\n{FOOTER_DELIMITER}\n"))
            .expect_err("empty footer should fail");
        assert_eq!(error, MarkdownError::EmptyFooter);
    }

    #[test]
    fn renders_footer_back_into_markdown() {
        let artifact = MarkdownArtifact {
            body: "body line".into(),
            footer: Some("key: value".into()),
        };
        let rendered = render_artifact(&artifact);
        assert_eq!(rendered, "body line\n\n-----\nkey: value\n");
    }

    #[test]
    fn appends_jsonl_changelog_rows() {
        let rendered = append_changelog_row("{\"a\":1}", "{\"b\":2}");
        assert_eq!(rendered, "{\"a\":1}\n{\"b\":2}\n");
    }
}
