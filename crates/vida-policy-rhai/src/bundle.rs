use std::{collections::BTreeMap, sync::Arc};

use rhai::AST;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::PolicyEngine;

pub const POLICY_BUNDLE_SCHEMA: u16 = 1;
pub const POLICY_ENGINE_ABI: &str = "rhai-policy-engine-v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PolicyBundleIdentity {
    pub policy_id: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyBundle {
    pub schema: u16,
    pub policy_id: String,
    pub version: u32,
    pub engine_abi: String,
    pub source: String,
}

impl PolicyBundle {
    pub fn from_json(raw: &str) -> Result<Self, PolicyBundleError> {
        let mut bundle: Self = serde_json::from_str(raw)?;
        if bundle.schema != POLICY_BUNDLE_SCHEMA {
            return Err(PolicyBundleError::UnsupportedSchema(bundle.schema));
        }
        if bundle.engine_abi != POLICY_ENGINE_ABI {
            return Err(PolicyBundleError::UnsupportedAbi(bundle.engine_abi));
        }
        if bundle.policy_id.trim().is_empty() || bundle.source.is_empty() {
            return Err(PolicyBundleError::InvalidField(
                "policy_id and source must be non-empty".to_string(),
            ));
        }
        bundle.source = normalize_newlines(&bundle.source);
        Ok(bundle)
    }

    pub fn identity(&self) -> PolicyBundleIdentity {
        PolicyBundleIdentity {
            policy_id: self.policy_id.clone(),
            version: self.version,
        }
    }

    pub fn canonical_json(&self) -> Result<Vec<u8>, PolicyBundleError> {
        serde_json::to_vec(self).map_err(PolicyBundleError::Canonicalization)
    }

    pub fn digest(&self) -> Result<String, PolicyBundleError> {
        let canonical_source =
            serde_json::to_vec(&self.source).map_err(PolicyBundleError::Canonicalization)?;
        Ok(blake3::hash(&canonical_source).to_hex().to_string())
    }
}

#[derive(Debug, Error)]
pub enum PolicyBundleError {
    #[error("malformed policy bundle: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("unsupported policy bundle schema {0}")]
    UnsupportedSchema(u16),
    #[error("unsupported policy engine ABI '{0}'")]
    UnsupportedAbi(String),
    #[error("invalid policy bundle field: {0}")]
    InvalidField(String),
    #[error("policy bundle canonicalization failed: {0}")]
    Canonicalization(serde_json::Error),
    #[error("policy bundle compilation failed: {0}")]
    Compile(String),
    #[error(
        "policy bundle identity {policy_id}@{version} conflicts with digest {existing_digest}; received {received_digest}"
    )]
    DigestConflict {
        policy_id: String,
        version: u32,
        existing_digest: String,
        received_digest: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleCacheStatus {
    Compiled,
    Hit,
}

pub struct CachedPolicyBundle {
    bundle: PolicyBundle,
    digest: String,
    canonical_json: Vec<u8>,
    ast: Arc<AST>,
}

impl CachedPolicyBundle {
    pub fn bundle(&self) -> &PolicyBundle {
        &self.bundle
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn canonical_json(&self) -> &[u8] {
        &self.canonical_json
    }

    pub fn ast(&self) -> &AST {
        &self.ast
    }
}

#[derive(Default)]
pub struct PolicyBundleCache {
    bundles: BTreeMap<PolicyBundleIdentity, CachedPolicyBundle>,
    ast_by_digest: BTreeMap<String, Arc<AST>>,
    compile_count: usize,
}

impl PolicyBundleCache {
    pub fn import_json<'a>(
        &'a mut self,
        engine: &PolicyEngine,
        raw: &str,
    ) -> Result<(&'a CachedPolicyBundle, BundleCacheStatus), PolicyBundleError> {
        let bundle = PolicyBundle::from_json(raw)?;
        let identity = bundle.identity();
        let digest = bundle.digest()?;

        if let Some(existing_digest) = self
            .bundles
            .get(&identity)
            .map(|existing| existing.digest.clone())
        {
            if existing_digest == digest {
                let existing = self.bundles.get(&identity).expect("existing bundle");
                return Ok((existing, BundleCacheStatus::Hit));
            }
            return Err(PolicyBundleError::DigestConflict {
                policy_id: identity.policy_id,
                version: identity.version,
                existing_digest,
                received_digest: digest,
            });
        }

        let (ast, status) = match self.ast_by_digest.get(&digest) {
            Some(ast) => (Arc::clone(ast), BundleCacheStatus::Hit),
            None => {
                let ast = Arc::new(
                    engine
                        .compile_source(&bundle.source)
                        .map_err(|error| PolicyBundleError::Compile(error.to_string()))?,
                );
                self.ast_by_digest.insert(digest.clone(), Arc::clone(&ast));
                self.compile_count += 1;
                (ast, BundleCacheStatus::Compiled)
            }
        };

        let canonical_json = bundle.canonical_json()?;
        self.bundles.insert(
            identity.clone(),
            CachedPolicyBundle {
                bundle,
                digest,
                canonical_json,
                ast,
            },
        );
        Ok((
            self.bundles.get(&identity).expect("inserted bundle"),
            status,
        ))
    }

    pub fn compile_count(&self) -> usize {
        self.compile_count
    }

    pub fn ast_cache_len(&self) -> usize {
        self.ast_by_digest.len()
    }

    pub fn identities(&self) -> impl Iterator<Item = &PolicyBundleIdentity> {
        self.bundles.keys()
    }
}

fn normalize_newlines(source: &str) -> String {
    let mut normalized = String::with_capacity(source.len());
    let mut chars = source.chars();
    while let Some(character) = chars.next() {
        if character == '\r' {
            if chars.clone().next() == Some('\n') {
                chars.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_bundle(policy_id: &str, version: u32, source: &str) -> String {
        serde_json::json!({
            "schema": 1,
            "policy_id": policy_id,
            "version": version,
            "engine_abi": POLICY_ENGINE_ABI,
            "source": source,
        })
        .to_string()
    }

    fn engine() -> PolicyEngine {
        crate::build_policy_engine(Default::default())
    }

    #[test]
    fn lf_crlf_and_cr_sources_have_equal_digest() {
        let digest = |source| {
            PolicyBundle::from_json(&raw_bundle("authority", 1, source))
                .unwrap()
                .digest()
                .unwrap()
        };
        assert_eq!(digest("let x = 1;\n x"), digest("let x = 1;\r\n x"));
        assert_eq!(digest("let x = 1;\n x"), digest("let x = 1;\r x"));
    }

    #[test]
    fn malformed_schema_and_abi_are_rejected() {
        assert!(matches!(
            PolicyBundle::from_json("{\"schema\":1}"),
            Err(PolicyBundleError::Malformed(_))
        ));
        assert!(matches!(
            PolicyBundle::from_json(
                &raw_bundle("x", 1, "1").replace("\"schema\":1", "\"schema\":2")
            ),
            Err(PolicyBundleError::UnsupportedSchema(2))
        ));
        assert!(matches!(
            PolicyBundle::from_json(&raw_bundle("x", 1, "1").replace(POLICY_ENGINE_ABI, "other")),
            Err(PolicyBundleError::UnsupportedAbi(_))
        ));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let raw = raw_bundle("x", 1, "1").replace("}", ",\"extra\":true}");
        assert!(matches!(
            PolicyBundle::from_json(&raw),
            Err(PolicyBundleError::Malformed(_))
        ));
    }

    #[test]
    fn same_identity_and_digest_is_idempotent_hit() {
        let raw = raw_bundle("authority", 1, "1");
        let mut cache = PolicyBundleCache::default();
        assert_eq!(
            cache.import_json(&engine(), &raw).unwrap().1,
            BundleCacheStatus::Compiled
        );
        assert_eq!(
            cache.import_json(&engine(), &raw).unwrap().1,
            BundleCacheStatus::Hit
        );
        assert_eq!(cache.compile_count(), 1);
    }

    #[test]
    fn same_identity_with_different_digest_fails_closed() {
        let mut cache = PolicyBundleCache::default();
        cache
            .import_json(&engine(), &raw_bundle("authority", 1, "1"))
            .unwrap();
        assert!(matches!(
            cache.import_json(&engine(), &raw_bundle("authority", 1, "2")),
            Err(PolicyBundleError::DigestConflict { .. })
        ));
    }

    #[test]
    fn equal_source_reuses_ast_by_digest_and_orders_identities() {
        let mut cache = PolicyBundleCache::default();
        cache
            .import_json(&engine(), &raw_bundle("z", 1, "1"))
            .unwrap();
        cache
            .import_json(&engine(), &raw_bundle("a", 1, "1"))
            .unwrap();
        assert_eq!(cache.compile_count(), 1);
        assert_eq!(cache.ast_cache_len(), 1);
        let ids = cache
            .identities()
            .map(|identity| identity.policy_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["a", "z"]);
    }
}
