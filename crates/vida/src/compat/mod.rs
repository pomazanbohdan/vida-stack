use std::process::ExitCode;

use serde::Serialize;

use crate::ProxyArgs;

pub(crate) const LEGACY_ROOT_ALIAS_RECEIPT_CODE: &str = "legacy_root_alias_used";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LegacyRootAliasSpec {
    pub(crate) alias: &'static str,
    pub(crate) canonical_family: &'static str,
    pub(crate) canonical_prefix: &'static str,
    pub(crate) deprecated_since: &'static str,
    pub(crate) removal_target: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LegacyRootAliasReceipt {
    pub(crate) receipt_code: &'static str,
    pub(crate) alias: &'static str,
    pub(crate) canonical_command: String,
    pub(crate) deprecated_since: &'static str,
    pub(crate) removal_target: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyRootAliasResolution {
    pub(crate) canonical_args: ProxyArgs,
    pub(crate) deprecation_notice: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyRootAliasProblem {
    pub(crate) blocker_code: &'static str,
    pub(crate) message: String,
    pub(crate) exit_code: ExitCode,
}

const LEGACY_ROOT_ALIASES: [LegacyRootAliasSpec; 3] = [
    LegacyRootAliasSpec {
        alias: "consume",
        canonical_family: "taskflow",
        canonical_prefix: "consume",
        deprecated_since: "vida-command-v1",
        removal_target: "Use `vida taskflow consume ...`.",
    },
    LegacyRootAliasSpec {
        alias: "recovery",
        canonical_family: "taskflow",
        canonical_prefix: "recovery",
        deprecated_since: "vida-command-v1",
        removal_target: "Use `vida taskflow recovery ...`.",
    },
    LegacyRootAliasSpec {
        alias: "route",
        canonical_family: "taskflow",
        canonical_prefix: "route",
        deprecated_since: "vida-command-v1",
        removal_target: "Use `vida taskflow route ...`.",
    },
];

pub(crate) fn retained_root_aliases() -> &'static [LegacyRootAliasSpec] {
    &LEGACY_ROOT_ALIASES
}

pub(crate) fn resolve_legacy_root_alias(
    alias: &'static str,
    args: ProxyArgs,
) -> Result<LegacyRootAliasResolution, LegacyRootAliasProblem> {
    let spec = retained_root_aliases()
        .iter()
        .find(|spec| spec.alias == alias)
        .ok_or_else(|| LegacyRootAliasProblem {
            blocker_code: "unknown_legacy_root_alias",
            message: format!("Unknown legacy root alias `{alias}`."),
            exit_code: ExitCode::from(2),
        })?;

    if let Some(nested) = args
        .args
        .first()
        .filter(|arg| ambiguous_nested_root_token(arg))
    {
        return Err(LegacyRootAliasProblem {
            blocker_code: "ambiguous_legacy_root_alias",
            message: format!(
                "Legacy root alias `{alias}` received nested command `{nested}`; use `vida {} {} ...`.",
                spec.canonical_family, spec.canonical_prefix
            ),
            exit_code: ExitCode::from(2),
        });
    }

    let mut canonical_args = Vec::with_capacity(args.args.len() + 1);
    canonical_args.push(spec.canonical_prefix.to_string());
    canonical_args.extend(args.args);

    let receipt = LegacyRootAliasReceipt {
        receipt_code: LEGACY_ROOT_ALIAS_RECEIPT_CODE,
        alias: spec.alias,
        canonical_command: format!("vida {} {}", spec.canonical_family, spec.canonical_prefix),
        deprecated_since: spec.deprecated_since,
        removal_target: spec.removal_target,
    };

    Ok(LegacyRootAliasResolution {
        canonical_args: ProxyArgs {
            args: canonical_args,
        },
        deprecation_notice: legacy_root_alias_deprecation_notice(&receipt),
    })
}

fn ambiguous_nested_root_token(token: &str) -> bool {
    token == "taskflow"
        || retained_root_aliases()
            .iter()
            .any(|spec| spec.alias == token)
}

fn legacy_root_alias_deprecation_notice(receipt: &LegacyRootAliasReceipt) -> String {
    let receipt_json =
        serde_json::to_string(receipt).expect("legacy root alias receipt should serialize to JSON");
    format!("warning: {LEGACY_ROOT_ALIAS_RECEIPT_CODE} {receipt_json}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_root_aliases_canonicalize_to_taskflow_prefixes() {
        for spec in retained_root_aliases() {
            let resolution = resolve_legacy_root_alias(
                spec.alias,
                ProxyArgs {
                    args: vec!["status".to_string(), "--json".to_string()],
                },
            )
            .expect("legacy root alias should resolve");

            assert_eq!(
                resolution.canonical_args.args,
                vec![
                    spec.canonical_prefix.to_string(),
                    "status".to_string(),
                    "--json".to_string()
                ]
            );
            assert!(resolution
                .deprecation_notice
                .contains(LEGACY_ROOT_ALIAS_RECEIPT_CODE));
            assert!(resolution.deprecation_notice.contains(spec.alias));
            assert!(resolution.deprecation_notice.contains(&format!(
                "vida {} {}",
                spec.canonical_family, spec.canonical_prefix
            )));
        }
    }

    #[test]
    fn ambiguous_nested_root_alias_input_fails_before_proxy_dispatch() {
        let error = resolve_legacy_root_alias(
            "consume",
            ProxyArgs {
                args: vec!["taskflow".to_string(), "consume".to_string()],
            },
        )
        .expect_err("nested taskflow token should fail closed");

        assert_eq!(error.blocker_code, "ambiguous_legacy_root_alias");
        assert!(error.message.contains("vida taskflow consume"));
    }
}
