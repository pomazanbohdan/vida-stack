#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SemanticRouteCacheKey {
    pub(crate) request_fingerprint: String,
    pub(crate) normalized_request_hash: String,
    pub(crate) task_class: String,
    pub(crate) runtime_role: String,
    pub(crate) route_key: String,
    pub(crate) compiled_bundle_revision: String,
    pub(crate) carrier_runtime_hash: String,
    pub(crate) worker_strategy_updated_at: String,
    pub(crate) price_catalog_snapshot_id: Option<String>,
    pub(crate) semantic_routing_config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SemanticRouteCacheValue {
    pub(crate) feature_vector: serde_json::Value,
    pub(crate) candidate_score_summary: serde_json::Value,
    pub(crate) selected_candidate_hint: Option<SemanticRouteCandidateHint>,
    pub(crate) created_at: String,
    pub(crate) ttl_seconds: u64,
    pub(crate) validity_scope: SemanticRouteCacheValidityScope,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SemanticRouteCandidateHint {
    pub(crate) carrier_id: String,
    pub(crate) model_profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SemanticRouteCacheValidityScope {
    pub(crate) diagnostic_only: bool,
    pub(crate) not_runtime_authority: bool,
    pub(crate) not_proof: bool,
    pub(crate) not_receipt: bool,
    pub(crate) not_closure_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SemanticRouteCacheEntry {
    pub(crate) key: SemanticRouteCacheKey,
    pub(crate) value: SemanticRouteCacheValue,
}

impl SemanticRouteCacheEntry {
    pub(crate) fn selected_candidate_hint_after_hard_filters(
        &self,
        current_key: &SemanticRouteCacheKey,
        hard_constraints_revalidated: bool,
    ) -> Option<&SemanticRouteCandidateHint> {
        if !hard_constraints_revalidated {
            return None;
        }
        if &self.key != current_key {
            return None;
        }
        if !self.value.validity_scope.is_derived_only() {
            return None;
        }
        self.value.selected_candidate_hint.as_ref()
    }
}

impl SemanticRouteCacheValidityScope {
    pub(crate) fn derived_only() -> Self {
        Self {
            diagnostic_only: true,
            not_runtime_authority: true,
            not_proof: true,
            not_receipt: true,
            not_closure_truth: true,
        }
    }

    pub(crate) fn is_derived_only(&self) -> bool {
        self.diagnostic_only
            && self.not_runtime_authority
            && self.not_proof
            && self.not_receipt
            && self.not_closure_truth
    }
}

pub(crate) fn build_semantic_route_cache_key(
    request_text: &str,
    task_class: &str,
    runtime_role: &str,
    route_key: &str,
    invalidation: &SemanticRouteInvalidationInputs<'_>,
) -> SemanticRouteCacheKey {
    let normalized_request = normalize_request(request_text);
    SemanticRouteCacheKey {
        request_fingerprint: stable_hex_hash(&format!(
            "{}|{}|{}|{}",
            normalized_request, task_class, runtime_role, route_key
        )),
        normalized_request_hash: stable_hex_hash(&normalized_request),
        task_class: task_class.trim().to_string(),
        runtime_role: runtime_role.trim().to_string(),
        route_key: route_key.trim().to_string(),
        compiled_bundle_revision: invalidation.compiled_bundle_revision.trim().to_string(),
        carrier_runtime_hash: invalidation.carrier_runtime_hash.trim().to_string(),
        worker_strategy_updated_at: invalidation.worker_strategy_updated_at.trim().to_string(),
        price_catalog_snapshot_id: invalidation
            .price_catalog_snapshot_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        semantic_routing_config_hash: invalidation.semantic_routing_config_hash.trim().to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SemanticRouteInvalidationInputs<'a> {
    pub(crate) compiled_bundle_revision: &'a str,
    pub(crate) carrier_runtime_hash: &'a str,
    pub(crate) worker_strategy_updated_at: &'a str,
    pub(crate) price_catalog_snapshot_id: Option<&'a str>,
    pub(crate) semantic_routing_config_hash: &'a str,
}

pub(crate) fn normalize_request(request_text: &str) -> String {
    request_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

pub(crate) fn stable_hex_hash(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalidation<'a>() -> SemanticRouteInvalidationInputs<'a> {
        SemanticRouteInvalidationInputs {
            compiled_bundle_revision: "bundle-1",
            carrier_runtime_hash: "carrier-1",
            worker_strategy_updated_at: "2026-05-24T18:00:00Z",
            price_catalog_snapshot_id: Some("prices-1"),
            semantic_routing_config_hash: "semantic-config-1",
        }
    }

    #[test]
    fn cache_key_is_stable_for_whitespace_and_case_only_changes() {
        let left = build_semantic_route_cache_key(
            " Implement   Runtime Cache ",
            "implementation",
            "worker",
            "worker",
            &invalidation(),
        );
        let right = build_semantic_route_cache_key(
            "implement runtime cache",
            "implementation",
            "worker",
            "worker",
            &invalidation(),
        );

        assert_eq!(left.request_fingerprint, right.request_fingerprint);
        assert_eq!(left.normalized_request_hash, right.normalized_request_hash);
    }

    #[test]
    fn invalidation_inputs_are_part_of_the_key() {
        let first = build_semantic_route_cache_key(
            "implement runtime cache",
            "implementation",
            "worker",
            "worker",
            &invalidation(),
        );
        let second = build_semantic_route_cache_key(
            "implement runtime cache",
            "implementation",
            "worker",
            "worker",
            &SemanticRouteInvalidationInputs {
                compiled_bundle_revision: "bundle-2",
                ..invalidation()
            },
        );

        assert_ne!(first, second);
        assert_ne!(
            first.compiled_bundle_revision,
            second.compiled_bundle_revision
        );
    }

    #[test]
    fn hint_is_unavailable_until_hard_constraints_are_revalidated() {
        let key = build_semantic_route_cache_key(
            "implement runtime cache",
            "implementation",
            "worker",
            "worker",
            &invalidation(),
        );
        let entry = SemanticRouteCacheEntry {
            key: key.clone(),
            value: SemanticRouteCacheValue {
                feature_vector: serde_json::json!({"complexity_band": "medium"}),
                candidate_score_summary: serde_json::json!({"candidate_count": 2}),
                selected_candidate_hint: Some(SemanticRouteCandidateHint {
                    carrier_id: "junior".to_string(),
                    model_profile_id: "codex_low".to_string(),
                }),
                created_at: "2026-05-24T18:00:00Z".to_string(),
                ttl_seconds: 300,
                validity_scope: SemanticRouteCacheValidityScope::derived_only(),
            },
        };

        assert!(entry
            .selected_candidate_hint_after_hard_filters(&key, false)
            .is_none());
        assert_eq!(
            entry
                .selected_candidate_hint_after_hard_filters(&key, true)
                .map(|hint| hint.carrier_id.as_str()),
            Some("junior")
        );
    }

    #[test]
    fn non_diagnostic_scope_cannot_supply_candidate_hint() {
        let key = build_semantic_route_cache_key(
            "implement runtime cache",
            "implementation",
            "worker",
            "worker",
            &invalidation(),
        );
        let entry = SemanticRouteCacheEntry {
            key: key.clone(),
            value: SemanticRouteCacheValue {
                feature_vector: serde_json::json!({}),
                candidate_score_summary: serde_json::json!({}),
                selected_candidate_hint: Some(SemanticRouteCandidateHint {
                    carrier_id: "junior".to_string(),
                    model_profile_id: "codex_low".to_string(),
                }),
                created_at: "2026-05-24T18:00:00Z".to_string(),
                ttl_seconds: 300,
                validity_scope: SemanticRouteCacheValidityScope {
                    diagnostic_only: true,
                    not_runtime_authority: false,
                    not_proof: true,
                    not_receipt: true,
                    not_closure_truth: true,
                },
            },
        };

        assert!(entry
            .selected_candidate_hint_after_hard_filters(&key, true)
            .is_none());
    }
}
