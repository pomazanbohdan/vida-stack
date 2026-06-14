//! Task reconciliation command payload helpers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileScopeRequiredPayload {
    pub surface: String,
    pub status: String,
    pub scope: Option<String>,
    pub dry_run: bool,
    pub close_if_complete: bool,
    pub closed_epics: Vec<String>,
    pub blocked_epics: Vec<String>,
    pub missing_children: Vec<String>,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
}

#[must_use]
pub fn scope_required_payload(
    surface: &str,
    dry_run: bool,
    close_if_complete: bool,
) -> ReconcileScopeRequiredPayload {
    ReconcileScopeRequiredPayload {
        surface: surface.to_string(),
        status: "blocked".to_string(),
        scope: None,
        dry_run,
        close_if_complete,
        closed_epics: Vec::new(),
        blocked_epics: Vec::new(),
        missing_children: Vec::new(),
        blocker_codes: vec!["scope_required".to_string()],
        next_actions: vec!["Run vida task reconcile --epics to inspect open epics.".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::scope_required_payload;

    #[test]
    fn reconcile_scope_required_payload_preserves_public_contract() {
        let payload = scope_required_payload("vida task reconcile", true, false);
        assert_eq!(payload.status, "blocked");
        assert_eq!(payload.scope, None);
        assert_eq!(payload.blocker_codes, vec!["scope_required"]);
        assert_eq!(
            payload.next_actions,
            vec!["Run vida task reconcile --epics to inspect open epics."]
        );
    }
}
