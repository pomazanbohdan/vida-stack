pub(crate) fn canonical_activation_status(
    status: Option<&str>,
    activation_pending: bool,
) -> &'static str {
    let normalized = status.map(|value| value.trim().to_ascii_lowercase());
    if activation_pending
        || matches!(
            normalized.as_deref(),
            Some("pending") | Some("pending_activation")
        )
    {
        "pending"
    } else {
        "ready_enough_for_normal_work"
    }
}

pub(crate) fn activation_status_is_pending(status: Option<&str>) -> bool {
    matches!(
        status
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("pending") | Some("pending_activation")
    )
}
