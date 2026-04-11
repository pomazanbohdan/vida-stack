use super::*;

impl StateStore {
    pub async fn seed_framework_instruction_bundle(&self) -> Result<(), StateStoreError> {
        let existing_runtime_state: Option<InstructionRuntimeStateRow> = self
            .db
            .select(("instruction_runtime_state", "primary"))
            .await?;
        let active_root_artifact_id = existing_runtime_state
            .as_ref()
            .map(|row| row.active_root_artifact_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "framework-agent-definition".to_string());
        let runtime_mode = existing_runtime_state
            .as_ref()
            .map(|row| row.runtime_mode.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "framework_seed".to_string());

        let query = r#"
UPSERT instruction_artifact:framework-agent-definition CONTENT {
  artifact_id: 'framework-agent-definition',
  artifact_kind: 'agent_definition',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-agent-definition-v1',
  activation_class: 'always_on',
  required_follow_on: ['framework-instruction-contract', 'framework-prompt-template-config']
};

UPSERT instruction_artifact:framework-instruction-contract CONTENT {
  artifact_id: 'framework-instruction-contract',
  artifact_kind: 'instruction_contract',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-instruction-contract-v1',
  activation_class: 'always_on',
  required_follow_on: []
};

UPSERT instruction_artifact:framework-prompt-template-config CONTENT {
  artifact_id: 'framework-prompt-template-config',
  artifact_kind: 'prompt_template_configuration',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-prompt-template-config-v1',
  activation_class: 'always_on',
  required_follow_on: []
};

UPSERT instruction_dependency_edge:framework-agent-definition__framework-instruction-contract CONTENT {
  from_artifact: 'framework-agent-definition',
  to_artifact: 'framework-instruction-contract',
  edge_kind: 'mandatory_follow_on'
};

UPSERT instruction_dependency_edge:framework-agent-definition__framework-prompt-template-config CONTENT {
  from_artifact: 'framework-agent-definition',
  to_artifact: 'framework-prompt-template-config',
  edge_kind: 'mandatory_follow_on'
};

UPSERT instruction_migration_receipt:framework-bundle-v1 CONTENT {
  receipt_id: 'framework-bundle-v1',
  bundle_version: 1,
  state_schema_version: 1,
  instruction_schema_version: 1,
  receipt_kind: 'seed',
  applied: true
};

UPSERT instruction_sidecar:framework-sidecar-substrate CONTENT {
  sidecar_id: 'framework-sidecar-substrate',
  target_artifact_id: 'framework-instruction-contract',
  patch_format: 'structured_diff',
  active: false,
  sidecar_kind: 'seed_substrate'
};

UPSERT instruction_diff_patch:framework-diff-substrate CONTENT {
  patch_id: 'framework-diff-substrate',
  target_artifact_id: 'framework-instruction-contract',
  active: false,
  patch_kind: 'seed_substrate',
  operations: []
};

UPSERT source_tree_config:instruction CONTENT {
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT source_tree_config:project CONTENT {
  slice: 'project_memory',
  source_root: 'docs/project-memory',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT source_tree_config:framework CONTENT {
  slice: 'framework_memory',
  source_root: 'vida/config/instructions/bundles/framework-memory-source',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT instruction_source_artifact:framework-agent-definition-source CONTENT {
  source_artifact_id: 'framework-agent-definition-source',
  artifact_id: 'framework-agent-definition',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/agent-definition.md',
  content_hash: 'seed-framework-agent-definition-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_source_artifact:framework-instruction-contract-source CONTENT {
  source_artifact_id: 'framework-instruction-contract-source',
  artifact_id: 'framework-instruction-contract',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/instruction-contract.md',
  content_hash: 'seed-framework-instruction-contract-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_source_artifact:framework-prompt-template-config-source CONTENT {
  source_artifact_id: 'framework-prompt-template-config-source',
  artifact_id: 'framework-prompt-template-config',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/prompt-template-config.md',
  content_hash: 'seed-framework-prompt-template-config-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_ingest_receipt:framework-bundle-seed CONTENT {
  receipt_id: 'framework-bundle-seed',
  source_root: 'vida/config/instructions/bundles/framework-source',
  product_version: '__VIDA_PRODUCT_VERSION__',
  ingest_kind: 'seed',
  applied: true
};

"#
        .replace("__VIDA_PRODUCT_VERSION__", VIDA_PRODUCT_VERSION);

        self.db.query(query).await?;
        let _: Option<InstructionRuntimeStateRow> = self
            .db
            .upsert(("instruction_runtime_state", "primary"))
            .content(InstructionRuntimeStateRow {
                state_id: "primary".to_string(),
                active_root_artifact_id,
                runtime_mode,
            })
            .await?;
        Ok(())
    }

    pub async fn source_tree_summary(&self) -> Result<String, StateStoreError> {
        let row: Option<SourceTreeConfigRow> = self
            .db
            .select(("source_tree_config", "instruction"))
            .await?;
        let row = row.ok_or(StateStoreError::MissingSourceTreeConfig)?;
        Ok(format!("{} -> {}", row.source_root, row.slice))
    }

    pub async fn active_instruction_root(&self) -> Result<String, StateStoreError> {
        let row: Option<InstructionRuntimeStateRow> = self
            .db
            .select(("instruction_runtime_state", "primary"))
            .await?;
        let row = row.ok_or(StateStoreError::MissingInstructionRuntimeState)?;
        if row.active_root_artifact_id.trim().is_empty() {
            return Err(StateStoreError::InvalidInstructionRuntimeState {
                reason: "active root artifact id is empty".to_string(),
            });
        }
        Ok(row.active_root_artifact_id)
    }

    pub async fn ingest_instruction_source_tree(
        &self,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        self.ingest_tree("instruction_memory", source_root).await
    }

    pub async fn ingest_framework_memory_source_tree(
        &self,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        self.ingest_tree("framework_memory", source_root).await
    }

    async fn ingest_tree(
        &self,
        slice: &str,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        let root = repo_root().join(source_root);
        if !root.exists() {
            return Err(StateStoreError::MissingSourceRoot {
                slice: slice.to_string(),
                path: root,
            });
        }

        let files = collect_markdown_files(&root)?;
        let mut imported = 0usize;
        let mut unchanged = 0usize;
        let mut updated = 0usize;

        for path in files {
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| StateStoreError::InvalidSourcePath(path.clone()))?;
            let body = fs::read_to_string(&path)?;
            let hash = blake3::hash(body.as_bytes()).to_hex().to_string();
            let metadata = parse_source_metadata(&body);
            let artifact_id = metadata
                .artifact_id
                .clone()
                .unwrap_or_else(|| artifact_id_from_path(relative));
            let hierarchy = if metadata.hierarchy.is_empty() {
                hierarchy_from_path(relative)
            } else {
                metadata.hierarchy.clone()
            };
            let artifact_kind = metadata
                .artifact_kind
                .clone()
                .unwrap_or_else(|| infer_artifact_kind(slice, relative));
            let version = metadata.version.unwrap_or(1);
            let ownership_class = metadata
                .ownership_class
                .clone()
                .unwrap_or_else(|| infer_ownership_class(slice).to_string());
            let mutability_class = metadata
                .mutability_class
                .clone()
                .unwrap_or_else(|| infer_mutability_class(slice).to_string());
            let activation_class = metadata.activation_class.clone().unwrap_or_default();
            let source_record_id = record_id_for_slice_source(slice, relative);

            let existing: Option<SourceArtifactRow> = self
                .db
                .select(("instruction_source_artifact", source_record_id.as_str()))
                .await?;

            let status = match existing {
                Some(existing_row) if existing_row.content_hash == hash => {
                    unchanged += 1;
                    "unchanged"
                }
                Some(_) => {
                    updated += 1;
                    "updated"
                }
                None => {
                    imported += 1;
                    "imported"
                }
            };

            let _: Option<SourceArtifactContent> = self
                .db
                .upsert(("instruction_source_artifact", source_record_id.as_str()))
                .content(SourceArtifactContent {
                    source_artifact_id: source_record_id.clone(),
                    artifact_id: artifact_id.clone(),
                    artifact_kind: artifact_kind.clone(),
                    version,
                    ownership_class: ownership_class.clone(),
                    mutability_class: mutability_class.clone(),
                    slice: slice.to_string(),
                    source_root: source_root.to_string(),
                    source_path: normalize_path(path.as_path()),
                    content_hash: hash.clone(),
                    ingest_status: status.to_string(),
                    hierarchy: hierarchy.clone(),
                })
                .await?;

            let _: Option<InstructionArtifactContent> = self
                .db
                .upsert(("instruction_artifact", artifact_id.as_str()))
                .content(InstructionArtifactContent {
                    artifact_id: artifact_id.clone(),
                    artifact_kind: artifact_kind.clone(),
                    version,
                    ownership_class: ownership_class.clone(),
                    mutability_class: mutability_class.clone(),
                    activation_class,
                    source_hash: hash,
                    body: body.clone(),
                    hierarchy: hierarchy.clone(),
                    required_follow_on: metadata.required_follow_on.clone(),
                })
                .await?;

            self.replace_dependency_edges(&artifact_id, &metadata.required_follow_on)
                .await?;
        }

        let receipt_id = format!("{}-ingest-{}", slice, unix_timestamp());
        let _: Option<InstructionIngestReceiptContent> = self
            .db
            .upsert(("instruction_ingest_receipt", receipt_id.as_str()))
            .content(InstructionIngestReceiptContent {
                receipt_id,
                slice: slice.to_string(),
                source_root: source_root.to_string(),
                product_version: VIDA_PRODUCT_VERSION.to_string(),
                ingest_kind: "scan".to_string(),
                applied: true,
                imported_count: imported,
                unchanged_count: unchanged,
                updated_count: updated,
            })
            .await?;

        Ok(InstructionIngestSummary {
            source_root: source_root.to_string(),
            imported_count: imported,
            unchanged_count: unchanged,
            updated_count: updated,
        })
    }

    async fn replace_dependency_edges(
        &self,
        artifact_id: &str,
        required_follow_on: &[String],
    ) -> Result<(), StateStoreError> {
        self.db
            .query(format!(
                "DELETE instruction_dependency_edge WHERE from_artifact = '{}';",
                artifact_id
            ))
            .await?;

        for target in required_follow_on {
            let edge_id = format!("{}__{}", artifact_id, target);
            let _: Option<InstructionDependencyEdgeContent> = self
                .db
                .upsert(("instruction_dependency_edge", edge_id.as_str()))
                .content(InstructionDependencyEdgeContent {
                    from_artifact: artifact_id.to_string(),
                    to_artifact: target.clone(),
                    edge_kind: "mandatory_follow_on".to_string(),
                })
                .await?;
        }

        Ok(())
    }

    pub async fn project_instruction_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<InstructionProjection, StateStoreError> {
        self.project_instruction_artifact_internal(artifact_id, true)
            .await
    }

    pub async fn inspect_instruction_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<InstructionProjection, StateStoreError> {
        self.project_instruction_artifact_internal(artifact_id, false)
            .await
    }

    async fn project_instruction_artifact_internal(
        &self,
        artifact_id: &str,
        persist_receipt: bool,
    ) -> Result<InstructionProjection, StateStoreError> {
        let base: Option<InstructionArtifactRow> = self
            .db
            .select(("instruction_artifact", artifact_id))
            .await?;
        let base = base.ok_or_else(|| StateStoreError::MissingInstructionArtifact {
            artifact_id: artifact_id.to_string(),
        })?;

        let mut sidecar_query = self
            .db
            .query(format!(
                "SELECT * FROM instruction_diff_patch WHERE target_artifact_id = '{}' AND active = true ORDER BY patch_precedence ASC, patch_id ASC;",
                artifact_id
            ))
            .await?;
        let patches: Vec<InstructionDiffPatchRow> = sidecar_query.take(0)?;

        if let Err(error) = validate_patch_bindings(&base, &patches) {
            if persist_receipt {
                self.write_projection_receipt(
                    artifact_id,
                    &base,
                    &base.body,
                    &[],
                    collect_patch_ids(&patches),
                    error.to_string(),
                )
                .await?;
            }
            return Err(error);
        }
        if let Err(error) = validate_patch_conflicts(&patches) {
            if persist_receipt {
                self.write_projection_receipt(
                    artifact_id,
                    &base,
                    &base.body,
                    &[],
                    collect_patch_ids(&patches),
                    error.to_string(),
                )
                .await?;
            }
            return Err(error);
        }

        let mut lines = split_lines(&base.body);
        let mut applied_patch_ids = Vec::new();
        let mut skipped_patch_ids = Vec::new();
        let mut failed_reason = String::new();

        for (index, patch) in patches.iter().enumerate() {
            for operation in &patch.operations {
                if let Err(error) = apply_patch_operation(&mut lines, operation) {
                    failed_reason = error.to_string();
                    skipped_patch_ids.extend(
                        patches
                            .iter()
                            .skip(index)
                            .map(|remaining| remaining.patch_id.clone()),
                    );

                    let projected_body = join_lines(&lines);
                    if persist_receipt {
                        self.write_projection_receipt(
                            artifact_id,
                            &base,
                            &projected_body,
                            &applied_patch_ids,
                            skipped_patch_ids.clone(),
                            failed_reason,
                        )
                        .await?;
                    }

                    return Err(error);
                }
            }
            applied_patch_ids.push(patch.patch_id.clone());
        }

        let projected_body = join_lines(&lines);
        let projection_hash = if persist_receipt {
            self.write_projection_receipt(
                artifact_id,
                &base,
                &projected_body,
                &applied_patch_ids,
                skipped_patch_ids.clone(),
                failed_reason,
            )
            .await?
        } else {
            blake3::hash(projected_body.as_bytes()).to_hex().to_string()
        };

        Ok(InstructionProjection {
            artifact_id: artifact_id.to_string(),
            body: projected_body,
            projected_hash: projection_hash,
            applied_patch_ids,
            skipped_patch_ids,
        })
    }

    #[allow(dead_code)]
    pub async fn upsert_instruction_diff_patch(
        &self,
        patch: InstructionDiffPatchContent,
    ) -> Result<(), StateStoreError> {
        let _: Option<InstructionDiffPatchContent> = self
            .db
            .upsert(("instruction_diff_patch", patch.patch_id.as_str()))
            .content(patch)
            .await?;
        Ok(())
    }

    async fn write_projection_receipt(
        &self,
        artifact_id: &str,
        base: &InstructionArtifactRow,
        projected_body: &str,
        applied_patch_ids: &[String],
        skipped_patch_ids: Vec<String>,
        failed_reason: String,
    ) -> Result<String, StateStoreError> {
        let projected_hash = blake3::hash(projected_body.as_bytes()).to_hex().to_string();
        let receipt_id = format!("projection-{}-{}", artifact_id, unix_timestamp());

        let _: Option<InstructionProjectionReceiptContent> = self
            .db
            .upsert(("instruction_projection_receipt", receipt_id.as_str()))
            .content(InstructionProjectionReceiptContent {
                receipt_id,
                artifact_id: artifact_id.to_string(),
                base_version: base.version,
                base_hash: base.source_hash.clone(),
                projected_hash: projected_hash.clone(),
                applied_patch_ids: applied_patch_ids.to_vec(),
                skipped_patch_ids,
                failed_reason,
                line_count: split_lines(projected_body).len(),
            })
            .await?;

        Ok(projected_hash)
    }

    pub async fn resolve_effective_instruction_bundle(
        &self,
        root_artifact_id: &str,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        self.resolve_effective_instruction_bundle_internal(root_artifact_id, true)
            .await
    }

    pub async fn inspect_effective_instruction_bundle(
        &self,
        root_artifact_id: &str,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        self.resolve_effective_instruction_bundle_internal(root_artifact_id, false)
            .await
    }

    async fn resolve_effective_instruction_bundle_internal(
        &self,
        root_artifact_id: &str,
        persist_receipt: bool,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        let ordered_ids = self.resolve_mandatory_chain(root_artifact_id).await?;
        let mut projected_artifacts = Vec::new();
        let mut source_version_tuple = Vec::new();

        for artifact_id in &ordered_ids {
            let projection = if persist_receipt {
                self.project_instruction_artifact(artifact_id).await?
            } else {
                self.inspect_instruction_artifact(artifact_id).await?
            };
            let base: Option<InstructionArtifactRow> = self
                .db
                .select(("instruction_artifact", artifact_id.as_str()))
                .await?;
            let base = base.ok_or_else(|| StateStoreError::MissingInstructionArtifact {
                artifact_id: artifact_id.clone(),
            })?;

            projected_artifacts.push(EffectiveInstructionArtifact {
                artifact_id: artifact_id.clone(),
                version: base.version,
                source_hash: base.source_hash,
                projected_hash: projection.projected_hash,
                body: projection.body,
            });
            source_version_tuple.push(format!("{}@v{}", artifact_id, base.version));
        }

        let receipt_id = if persist_receipt {
            let receipt_id = format!(
                "effective-bundle-{}-{}",
                root_artifact_id,
                unix_timestamp_nanos()
            );
            let _: Option<EffectiveInstructionBundleReceiptContent> = self
                .db
                .upsert(("effective_instruction_bundle_receipt", receipt_id.as_str()))
                .content(EffectiveInstructionBundleReceiptContent {
                    receipt_id: receipt_id.clone(),
                    root_artifact_id: root_artifact_id.to_string(),
                    mandatory_chain_order: ordered_ids.clone(),
                    source_version_tuple: source_version_tuple.clone(),
                    // Reserved for later trigger-matrix runtime work; B04/B05 keeps this explicit and empty.
                    optional_triggered_reads: Vec::new(),
                    artifact_count: projected_artifacts.len(),
                })
                .await?;
            receipt_id
        } else {
            "not-persisted".to_string()
        };

        Ok(EffectiveInstructionBundle {
            root_artifact_id: root_artifact_id.to_string(),
            mandatory_chain_order: ordered_ids,
            source_version_tuple,
            projected_artifacts,
            receipt_id,
        })
    }

    pub(crate) async fn resolve_mandatory_chain(
        &self,
        root_artifact_id: &str,
    ) -> Result<Vec<String>, StateStoreError> {
        let root_exists: Option<InstructionArtifactRow> = self
            .db
            .select(("instruction_artifact", root_artifact_id))
            .await?;
        if root_exists.is_none() {
            return Err(StateStoreError::MissingInstructionArtifact {
                artifact_id: root_artifact_id.to_string(),
            });
        }

        let mut reachable = BTreeSet::new();
        let mut frontier = vec![root_artifact_id.to_string()];

        while let Some(current) = frontier.pop() {
            if !reachable.insert(current.clone()) {
                continue;
            }

            let mut query = self
                .db
                .query(format!(
                    "SELECT to_artifact, edge_kind FROM instruction_dependency_edge WHERE from_artifact = '{}' ORDER BY to_artifact ASC;",
                    current
                ))
                .await?;
            let edges: Vec<InstructionDependencyEdgeRow> = query.take(0)?;
            for edge in edges {
                if edge.edge_kind == "mandatory_follow_on" {
                    let target_exists: Option<InstructionArtifactRow> = self
                        .db
                        .select(("instruction_artifact", edge.to_artifact.as_str()))
                        .await?;
                    if target_exists.is_none() {
                        return Err(StateStoreError::MissingInstructionArtifact {
                            artifact_id: edge.to_artifact,
                        });
                    }
                    frontier.push(edge.to_artifact);
                }
            }
        }

        let mut indegree: BTreeMap<String, usize> =
            reachable.iter().map(|id| (id.clone(), 0usize)).collect();
        let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for from in &reachable {
            let mut query = self
                .db
                .query(format!(
                    "SELECT to_artifact, edge_kind FROM instruction_dependency_edge WHERE from_artifact = '{}' ORDER BY to_artifact ASC;",
                    from
                ))
                .await?;
            let edges: Vec<InstructionDependencyEdgeRow> = query.take(0)?;
            for edge in edges {
                if edge.edge_kind == "mandatory_follow_on" && reachable.contains(&edge.to_artifact)
                {
                    adjacency
                        .entry(from.clone())
                        .or_default()
                        .push(edge.to_artifact.clone());
                    *indegree.entry(edge.to_artifact).or_default() += 1;
                }
            }
        }

        let mut ready: Vec<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        ready.sort();

        let mut ordered = Vec::new();
        while let Some(current) = ready.first().cloned() {
            ready.remove(0);
            ordered.push(current.clone());

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if let Some(degree) = indegree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            ready.push(neighbor.clone());
                        }
                    }
                }
                ready.sort();
            }
        }

        if ordered.len() != reachable.len() {
            let cycle_nodes: Vec<String> = indegree
                .into_iter()
                .filter_map(|(id, degree)| if degree > 0 { Some(id) } else { None })
                .collect();
            return Err(StateStoreError::InstructionDependencyCycle {
                cycle_path: cycle_nodes.join(" -> "),
            });
        }

        Ok(ordered)
    }
}
