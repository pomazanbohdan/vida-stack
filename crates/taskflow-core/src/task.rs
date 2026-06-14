//! Task command module skeleton for future TaskFlow core extraction.

pub mod attempts;
pub mod block;
pub mod close;
pub mod closure_ready;
pub mod create;
pub mod dependencies;
pub mod handoff;
pub mod import_export;
pub mod note;
pub mod progress;
pub mod reconcile;
pub mod spawn_blocker;
pub mod split;
pub mod takeover;
pub mod update;
pub mod verify;

#[cfg(test)]
mod tests {
    #[test]
    fn task_module_surface_is_present() {
        let modules = [
            "attempts",
            "block",
            "close",
            "closure_ready",
            "create",
            "dependencies",
            "handoff",
            "import_export",
            "note",
            "progress",
            "reconcile",
            "split",
            "spawn_blocker",
            "takeover",
            "update",
            "verify",
        ];

        assert_eq!(modules.len(), 16);
    }
}
