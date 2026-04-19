pub(crate) fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn shell_quote_joined_labels(labels: &[&str]) -> Option<String> {
    let joined = labels
        .iter()
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    (!joined.is_empty()).then(|| shell_quote(&joined))
}

pub(crate) fn build_task_create_command(
    task_id: &str,
    title: &str,
    task_type: &str,
    parent_id: Option<&str>,
    labels: &[&str],
    description_quoted: Option<&str>,
) -> String {
    let mut command = format!(
        "vida task create {} {} --type {} --status open",
        task_id,
        shell_quote(title),
        task_type
    );
    if let Some(parent_id) = parent_id {
        command.push_str(&format!(" --parent-id {parent_id}"));
    }
    if let Some(labels_arg) = shell_quote_joined_labels(labels) {
        command.push_str(&format!(" --labels {labels_arg}"));
    }
    if let Some(description_quoted) = description_quoted {
        command.push_str(&format!(" --description {description_quoted}"));
    }
    command.push_str(" --json");
    command
}

pub(crate) fn build_task_ensure_command(
    task_id: &str,
    title: &str,
    task_type: &str,
    parent_id: Option<&str>,
    labels: &[&str],
    description_quoted: Option<&str>,
) -> String {
    let mut command = format!(
        "vida task ensure {} {} --type {} --status open",
        task_id,
        shell_quote(title),
        task_type
    );
    if let Some(parent_id) = parent_id {
        command.push_str(&format!(" --parent-id {parent_id}"));
    }
    if let Some(labels_arg) = shell_quote_joined_labels(labels) {
        command.push_str(&format!(" --labels {labels_arg}"));
    }
    if let Some(description_quoted) = description_quoted {
        command.push_str(&format!(" --description {description_quoted}"));
    }
    command.push_str(" --json");
    command
}

pub(crate) fn build_task_show_command(task_id: &str) -> String {
    format!("vida task show {task_id} --json")
}

pub(crate) fn build_task_close_command(task_id: &str, reason: &str) -> String {
    format!(
        "vida task close {} --reason {} --json",
        task_id,
        shell_quote(reason)
    )
}

pub(crate) fn infer_feature_request_slug(request: &str) -> String {
    const STOPWORDS: &[&str] = &[
        "a",
        "an",
        "and",
        "build",
        "code",
        "containing",
        "create",
        "detailed",
        "develop",
        "file",
        "follow",
        "for",
        "full",
        "game",
        "html",
        "implementation",
        "implement",
        "mechanics",
        "page",
        "plan",
        "please",
        "research",
        "single",
        "specifications",
        "steps",
        "the",
        "these",
        "write",
    ];
    let filtered = request
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !STOPWORDS.iter().any(|stop| stop == token))
        .take(6)
        .collect::<Vec<_>>()
        .join("-");
    let slug = super::slugify_project_id(if filtered.is_empty() {
        request
    } else {
        &filtered
    });
    let trimmed = slug.trim_matches('-');
    let bounded = if trimmed.len() > 48 {
        &trimmed[..48]
    } else {
        trimmed
    };
    bounded.trim_matches('-').to_string()
}

pub(crate) fn infer_feature_request_title(request: &str) -> String {
    let trimmed = request.trim();
    let compact = trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if compact.is_empty() {
        "Feature request".to_string()
    } else if compact.chars().count() <= 72 {
        compact
    } else {
        let shortened = compact.chars().take(69).collect::<String>();
        format!("{shortened}...")
    }
}
