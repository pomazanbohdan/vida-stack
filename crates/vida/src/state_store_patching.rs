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
