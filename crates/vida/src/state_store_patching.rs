use super::{
    InstructionArtifactRow, InstructionDiffPatchRow, InstructionPatchOperation, StateStoreError,
};

#[allow(dead_code)]
pub(super) fn split_lines(body: &str) -> Vec<String> {
    body.lines().map(|line| line.to_string()).collect()
}

#[allow(dead_code)]
pub(super) fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[allow(dead_code)]
pub(super) fn apply_patch_operation(
    lines: &mut Vec<String>,
    operation: &InstructionPatchOperation,
) -> Result<(), StateStoreError> {
    let index = resolve_operation_target(lines, operation)?;

    match operation.op.as_str() {
        "replace_range" => {
            lines.splice(index..=index, operation.with_lines.clone());
        }
        "replace_with_many" => {
            lines.splice(index..=index, operation.with_lines.clone());
        }
        "delete_range" => {
            lines.remove(index);
        }
        "insert_before" => {
            lines.splice(index..index, operation.with_lines.clone());
        }
        "insert_after" => {
            lines.splice(index + 1..index + 1, operation.with_lines.clone());
        }
        "append_block" => {
            lines.extend(operation.with_lines.clone());
        }
        other => {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!("unsupported op: {other}"),
            });
        }
    }

    Ok(())
}

fn resolve_operation_target(
    lines: &[String],
    operation: &InstructionPatchOperation,
) -> Result<usize, StateStoreError> {
    match operation.target_mode.as_str() {
        "exact_text" => lines
            .iter()
            .position(|line| line == &operation.target)
            .ok_or_else(|| StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "anchor not found for op {}: {}",
                    operation.op, operation.target
                ),
            }),
        "line_span" => {
            let line_number = operation.target.parse::<usize>().map_err(|_| {
                StateStoreError::InvalidPatchOperation {
                    reason: format!("invalid line_span target: {}", operation.target),
                }
            })?;
            if line_number == 0 || line_number > lines.len() {
                return Err(StateStoreError::InvalidPatchOperation {
                    reason: format!("line_span out of bounds: {}", operation.target),
                });
            }
            Ok(line_number - 1)
        }
        "anchor_hash" => {
            let target_hash = operation.target.strip_prefix("blake3:").ok_or_else(|| {
                StateStoreError::InvalidPatchOperation {
                    reason: format!("invalid anchor_hash target format: {}", operation.target),
                }
            })?;

            lines
                .iter()
                .position(|line| blake3::hash(line.as_bytes()).to_hex().as_str() == target_hash)
                .ok_or_else(|| StateStoreError::InvalidPatchOperation {
                    reason: format!("anchor hash not found for op {}", operation.op),
                })
        }
        other => Err(StateStoreError::InvalidPatchOperation {
            reason: format!("unsupported target_mode: {other}"),
        }),
    }
}

pub(super) fn validate_patch_conflicts(
    patches: &[InstructionDiffPatchRow],
) -> Result<(), StateStoreError> {
    use std::collections::HashMap;

    let mut claimed: HashMap<(String, String), (u32, String)> = HashMap::new();

    for patch in patches {
        for operation in &patch.operations {
            if matches!(
                operation.op.as_str(),
                "replace_range" | "replace_with_many" | "delete_range"
            ) {
                let key = (operation.target_mode.clone(), operation.target.clone());
                if let Some((existing_precedence, existing_patch_id)) = claimed.get(&key) {
                    if *existing_precedence == patch.patch_precedence {
                        return Err(StateStoreError::PatchConflict {
                            reason: format!(
                                "patches {} and {} target the same anchor with equal precedence",
                                existing_patch_id, patch.patch_id
                            ),
                        });
                    }
                } else {
                    claimed.insert(key, (patch.patch_precedence, patch.patch_id.clone()));
                }
            }
        }
    }

    Ok(())
}

pub(super) fn validate_patch_bindings(
    base: &InstructionArtifactRow,
    patches: &[InstructionDiffPatchRow],
) -> Result<(), StateStoreError> {
    for patch in patches {
        if patch.target_artifact_version != base.version {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "patch {} targets artifact version {} but base version is {}",
                    patch.patch_id, patch.target_artifact_version, base.version
                ),
            });
        }

        if patch.target_artifact_hash != base.source_hash {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "patch {} targets artifact hash {} but base hash is {}",
                    patch.patch_id, patch.target_artifact_hash, base.source_hash
                ),
            });
        }
    }

    Ok(())
}

pub(super) fn collect_patch_ids(patches: &[InstructionDiffPatchRow]) -> Vec<String> {
    patches.iter().map(|patch| patch.patch_id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        InstructionArtifactRow, InstructionDiffPatchRow, InstructionPatchOperation,
        StateStoreError, apply_patch_operation, collect_patch_ids, join_lines, split_lines,
        validate_patch_bindings, validate_patch_conflicts,
    };

    fn operation(
        op: &str,
        target_mode: &str,
        target: &str,
        with_lines: &[&str],
    ) -> InstructionPatchOperation {
        InstructionPatchOperation {
            op: op.to_string(),
            target_mode: target_mode.to_string(),
            target: target.to_string(),
            with_lines: with_lines.iter().map(|line| (*line).to_string()).collect(),
        }
    }

    fn patch(
        patch_id: &str,
        precedence: u32,
        target_version: u32,
        target_hash: &str,
        operations: Vec<InstructionPatchOperation>,
    ) -> InstructionDiffPatchRow {
        InstructionDiffPatchRow {
            patch_id: patch_id.to_string(),
            target_artifact_id: "artifact".to_string(),
            target_artifact_version: target_version,
            target_artifact_hash: target_hash.to_string(),
            patch_precedence: precedence,
            active: true,
            operations,
        }
    }

    #[test]
    fn line_split_and_join_define_the_persisted_trailing_newline_contract() {
        assert_eq!(split_lines("first\nsecond\n"), vec!["first", "second"]);
        assert!(split_lines("").is_empty());
        assert_eq!(
            join_lines(&["first".into(), "second".into()]),
            "first\nsecond\n"
        );
        assert_eq!(join_lines(&[]), "");
    }

    #[test]
    fn patch_operations_cover_anchor_modes_and_fail_closed_unknown_inputs() {
        let mut lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        apply_patch_operation(
            &mut lines,
            &operation("replace_range", "exact_text", "b", &["B"]),
        )
        .expect("replace_range should succeed");
        apply_patch_operation(
            &mut lines,
            &operation("replace_with_many", "line_span", "1", &["A1", "A2"]),
        )
        .expect("replace_with_many should support line spans");
        apply_patch_operation(
            &mut lines,
            &operation("insert_before", "exact_text", "c", &["before"]),
        )
        .expect("insert_before should succeed");
        apply_patch_operation(
            &mut lines,
            &operation("insert_after", "exact_text", "c", &["after"]),
        )
        .expect("insert_after should succeed");
        apply_patch_operation(
            &mut lines,
            &operation("append_block", "exact_text", "c", &["tail"]),
        )
        .expect("append_block should resolve an existing anchor");
        apply_patch_operation(
            &mut lines,
            &operation("delete_range", "exact_text", "before", &[]),
        )
        .expect("delete_range should remove the selected line");
        assert_eq!(lines, vec!["A1", "A2", "B", "c", "after", "tail"]);

        let error = apply_patch_operation(
            &mut lines,
            &operation("unsupported", "exact_text", "A1", &[]),
        )
        .expect_err("unknown patch operation should fail closed");
        assert!(matches!(
            error,
            StateStoreError::InvalidPatchOperation { .. }
        ));
    }

    #[test]
    fn anchor_hash_and_line_span_reject_malformed_or_missing_targets() {
        let mut lines = vec!["alpha".to_string()];
        let hash = blake3::hash(b"alpha").to_hex().to_string();
        apply_patch_operation(
            &mut lines,
            &operation(
                "insert_after",
                "anchor_hash",
                &format!("blake3:{hash}"),
                &["beta"],
            ),
        )
        .expect("matching anchor hash should resolve");

        for (target_mode, target) in [
            ("line_span", "0"),
            ("line_span", "not-a-number"),
            ("line_span", "99"),
            ("anchor_hash", "sha256:wrong"),
            ("anchor_hash", "blake3:missing"),
            ("exact_text", "missing"),
            ("unknown", "alpha"),
        ] {
            let error = apply_patch_operation(
                &mut lines,
                &operation("delete_range", target_mode, target, &[]),
            )
            .expect_err("invalid target should fail closed");
            assert!(matches!(
                error,
                StateStoreError::InvalidPatchOperation { .. }
            ));
        }
    }

    #[test]
    fn conflict_and_binding_validation_preserve_patch_identity_constraints() {
        let conflicting = vec![
            patch(
                "first",
                1,
                3,
                "hash",
                vec![operation("replace_range", "exact_text", "line", &["a"])],
            ),
            patch(
                "second",
                1,
                3,
                "hash",
                vec![operation("delete_range", "exact_text", "line", &[])],
            ),
        ];
        let conflict = validate_patch_conflicts(&conflicting)
            .expect_err("equal-precedence edits to one anchor must conflict");
        assert!(matches!(conflict, StateStoreError::PatchConflict { .. }));
        let higher_precedence = vec![
            conflicting[0].clone(),
            patch(
                "second-higher",
                2,
                3,
                "hash",
                vec![operation("delete_range", "exact_text", "line", &[])],
            ),
        ];
        validate_patch_conflicts(&higher_precedence)
            .expect("different precedence edits should remain ordered");

        let base = InstructionArtifactRow {
            artifact_id: "artifact".to_string(),
            version: 3,
            source_hash: "hash".to_string(),
            body: "body".to_string(),
        };
        validate_patch_bindings(&base, &[conflicting[0].clone()])
            .expect("matching version and source hash should bind");
        let invalid = patch("wrong", 1, 2, "other", Vec::new());
        let error = validate_patch_bindings(&base, &[invalid])
            .expect_err("version/hash drift must fail closed");
        assert!(matches!(
            error,
            StateStoreError::InvalidPatchOperation { .. }
        ));
        let invalid_hash = patch("wrong-hash", 1, 3, "other", Vec::new());
        let hash_error = validate_patch_bindings(&base, &[invalid_hash])
            .expect_err("source hash drift must fail closed");
        assert!(matches!(
            hash_error,
            StateStoreError::InvalidPatchOperation { .. }
        ));

        assert_eq!(collect_patch_ids(&conflicting), vec!["first", "second"]);
    }
}
