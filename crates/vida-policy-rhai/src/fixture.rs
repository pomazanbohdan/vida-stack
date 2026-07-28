use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{PolicyBundle, PolicyEngine, PolicyError, PolicyErrorCode};

pub const MAX_FIXTURE_CORPUS_BYTES: usize = 1024 * 1024;
pub const MAX_FIXTURE_ROWS: usize = 1024;
pub const MAX_FIXTURE_CONTEXT_BYTES: usize = 16 * 1024;
const MAX_FIXTURE_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureCorpusErrorCode {
    CorpusTooLarge,
    EmptyCorpus,
    TooManyRows,
    MalformedJsonl,
    UnknownField,
    MissingFixtureId,
    InvalidFixtureId,
    DuplicateFixtureId,
    MissingContext,
    MissingExpectation,
    AmbiguousExpectation,
    InvalidExpectedErrorCode,
    ContextTooLarge,
}

impl FixtureCorpusErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CorpusTooLarge => "fixture_corpus_too_large",
            Self::EmptyCorpus => "fixture_corpus_empty",
            Self::TooManyRows => "fixture_row_count_exceeded",
            Self::MalformedJsonl => "fixture_jsonl_malformed",
            Self::UnknownField => "fixture_unknown_field",
            Self::MissingFixtureId => "fixture_id_missing",
            Self::InvalidFixtureId => "fixture_id_invalid",
            Self::DuplicateFixtureId => "fixture_id_duplicate",
            Self::MissingContext => "fixture_context_missing",
            Self::MissingExpectation => "fixture_expectation_missing",
            Self::AmbiguousExpectation => "fixture_expectation_ambiguous",
            Self::InvalidExpectedErrorCode => "fixture_expected_error_code_invalid",
            Self::ContextTooLarge => "fixture_context_too_large",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureCorpusError {
    pub code: FixtureCorpusErrorCode,
    pub line: Option<usize>,
    pub fixture_id: Option<String>,
    pub detail: String,
}

impl FixtureCorpusError {
    fn new(
        code: FixtureCorpusErrorCode,
        line: Option<usize>,
        fixture_id: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            line,
            fixture_id,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for FixtureCorpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.detail)?;
        if let Some(line) = self.line {
            write!(formatter, " (line {line}")?;
            if let Some(fixture_id) = &self.fixture_id {
                write!(formatter, ", fixture_id {fixture_id}")?;
            }
            write!(formatter, ")")?;
        }
        Ok(())
    }
}

impl std::error::Error for FixtureCorpusError {}

#[derive(Debug, Error)]
pub enum FixtureRunError {
    #[error(transparent)]
    Corpus(#[from] FixtureCorpusError),
    #[error("policy source preparation failed with code {}: {source}", source.code().as_str())]
    Policy {
        #[source]
        source: PolicyError,
    },
}

impl From<PolicyError> for FixtureRunError {
    fn from(source: PolicyError) -> Self {
        Self::Policy { source }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureFailureCode {
    OutputMismatch,
    UnexpectedPolicyError,
    WrongErrorCode,
    UnexpectedPolicySuccess,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureFailure {
    pub code: FixtureFailureCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_error_code: Option<PolicyErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_error_code: Option<PolicyErrorCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureResult {
    pub fixture_id: String,
    pub line: usize,
    pub status: FixtureStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<FixtureFailure>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<FixtureResult>,
}

impl FixtureReport {
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        self.failed == 0
    }
}

#[derive(Debug, Clone, Copy)]
struct RunnerLimits {
    max_corpus_bytes: usize,
    max_rows: usize,
    max_context_bytes: usize,
}

impl Default for RunnerLimits {
    fn default() -> Self {
        Self {
            max_corpus_bytes: MAX_FIXTURE_CORPUS_BYTES,
            max_rows: MAX_FIXTURE_ROWS,
            max_context_bytes: MAX_FIXTURE_CONTEXT_BYTES,
        }
    }
}

#[derive(Debug, Clone)]
enum Presence<T> {
    Missing,
    Present(T),
}

impl<T> Default for Presence<T> {
    fn default() -> Self {
        Self::Missing
    }
}

fn deserialize_presence<'de, D, T>(deserializer: D) -> Result<Presence<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Presence::Present)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureRow {
    fixture_id: String,
    context: Value,
    #[serde(default, deserialize_with = "deserialize_presence")]
    expected: Presence<Value>,
    #[serde(default, deserialize_with = "deserialize_presence")]
    expected_error_code: Presence<PolicyErrorCode>,
}

enum FixtureExpectation {
    Output(Value),
    Error(PolicyErrorCode),
}

struct ValidatedFixture {
    line: usize,
    fixture_id: String,
    context: Value,
    expectation: FixtureExpectation,
}

pub fn run_fixture_jsonl(
    engine: &PolicyEngine,
    bundle: &PolicyBundle,
    jsonl: &str,
) -> Result<FixtureReport, FixtureRunError> {
    run_fixture_jsonl_with_limits(engine, bundle, jsonl, RunnerLimits::default())
}

fn run_fixture_jsonl_with_limits(
    engine: &PolicyEngine,
    bundle: &PolicyBundle,
    jsonl: &str,
    limits: RunnerLimits,
) -> Result<FixtureReport, FixtureRunError> {
    let fixtures = validate_corpus(jsonl, limits)?;
    let ast = engine.compile_source(&bundle.source)?;
    let mut results = Vec::with_capacity(fixtures.len());
    let mut passed = 0;

    for fixture in fixtures {
        let evaluation = engine.evaluate_ast(&ast, fixture.context);
        let failure = compare_expectation(fixture.expectation, evaluation);
        let status = if failure.is_none() {
            passed += 1;
            FixtureStatus::Passed
        } else {
            FixtureStatus::Failed
        };
        results.push(FixtureResult {
            fixture_id: fixture.fixture_id,
            line: fixture.line,
            status,
            failure,
        });
    }

    Ok(FixtureReport {
        total: results.len(),
        passed,
        failed: results.len() - passed,
        results,
    })
}

fn validate_corpus(
    jsonl: &str,
    limits: RunnerLimits,
) -> Result<Vec<ValidatedFixture>, FixtureCorpusError> {
    if jsonl.len() > limits.max_corpus_bytes {
        return Err(FixtureCorpusError::new(
            FixtureCorpusErrorCode::CorpusTooLarge,
            None,
            None,
            format!(
                "fixture corpus is {} bytes; limit is {} bytes",
                jsonl.len(),
                limits.max_corpus_bytes
            ),
        ));
    }

    let mut fixtures = Vec::new();
    let mut fixture_ids = BTreeSet::new();
    for (index, raw_line) in jsonl.lines().enumerate() {
        let line = index + 1;
        if fixtures.len() == limits.max_rows {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::TooManyRows,
                Some(line),
                None,
                format!("fixture row count exceeds limit {}", limits.max_rows),
            ));
        }
        if raw_line.trim().is_empty() {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::MalformedJsonl,
                Some(line),
                None,
                "blank JSONL row",
            ));
        }

        let value: Value = serde_json::from_str(raw_line).map_err(|error| {
            FixtureCorpusError::new(
                FixtureCorpusErrorCode::MalformedJsonl,
                Some(line),
                None,
                error.to_string(),
            )
        })?;
        let object = value.as_object().ok_or_else(|| {
            FixtureCorpusError::new(
                FixtureCorpusErrorCode::MalformedJsonl,
                Some(line),
                None,
                "fixture row must be a JSON object",
            )
        })?;
        let fixture_context = object
            .get("fixture_id")
            .and_then(Value::as_str)
            .map(str::to_owned);

        const ALLOWED_FIELDS: [&str; 4] =
            ["fixture_id", "context", "expected", "expected_error_code"];
        if let Some(unknown) = object
            .keys()
            .find(|key| !ALLOWED_FIELDS.contains(&key.as_str()))
        {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::UnknownField,
                Some(line),
                fixture_context,
                format!("unknown fixture field `{unknown}`"),
            ));
        }
        if !object.contains_key("fixture_id") {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::MissingFixtureId,
                Some(line),
                None,
                "fixture_id is required",
            ));
        }
        if !object.contains_key("context") {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::MissingContext,
                Some(line),
                fixture_context,
                "context is required",
            ));
        }
        let has_expected = object.contains_key("expected");
        let has_expected_error = object.contains_key("expected_error_code");
        if has_expected == has_expected_error {
            let code = if has_expected {
                FixtureCorpusErrorCode::AmbiguousExpectation
            } else {
                FixtureCorpusErrorCode::MissingExpectation
            };
            return Err(FixtureCorpusError::new(
                code,
                Some(line),
                fixture_context,
                "exactly one of expected or expected_error_code is required",
            ));
        }

        let row: FixtureRow = serde_json::from_value(value).map_err(|error| {
            let code = if has_expected_error {
                FixtureCorpusErrorCode::InvalidExpectedErrorCode
            } else {
                FixtureCorpusErrorCode::MalformedJsonl
            };
            FixtureCorpusError::new(code, Some(line), fixture_context, error.to_string())
        })?;
        if !valid_fixture_id(&row.fixture_id) {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::InvalidFixtureId,
                Some(line),
                Some(row.fixture_id),
                format!(
                    "fixture_id must be 1..={MAX_FIXTURE_ID_BYTES} bytes of ASCII letters, digits, '.', '_', or '-'"
                ),
            ));
        }
        if !fixture_ids.insert(row.fixture_id.clone()) {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::DuplicateFixtureId,
                Some(line),
                Some(row.fixture_id),
                "fixture_id must be unique within the corpus",
            ));
        }
        let context_bytes = serde_json::to_vec(&row.context).map_err(|error| {
            FixtureCorpusError::new(
                FixtureCorpusErrorCode::MalformedJsonl,
                Some(line),
                Some(row.fixture_id.clone()),
                error.to_string(),
            )
        })?;
        if context_bytes.len() > limits.max_context_bytes {
            return Err(FixtureCorpusError::new(
                FixtureCorpusErrorCode::ContextTooLarge,
                Some(line),
                Some(row.fixture_id),
                format!(
                    "fixture context is {} bytes; limit is {} bytes",
                    context_bytes.len(),
                    limits.max_context_bytes
                ),
            ));
        }
        let expectation = match (row.expected, row.expected_error_code) {
            (Presence::Present(expected), Presence::Missing) => {
                FixtureExpectation::Output(expected)
            }
            (Presence::Missing, Presence::Present(expected)) => FixtureExpectation::Error(expected),
            _ => unreachable!("expectation presence validated before deserialization"),
        };
        fixtures.push(ValidatedFixture {
            line,
            fixture_id: row.fixture_id,
            context: row.context,
            expectation,
        });
    }

    if fixtures.is_empty() {
        return Err(FixtureCorpusError::new(
            FixtureCorpusErrorCode::EmptyCorpus,
            None,
            None,
            "fixture corpus must contain at least one row",
        ));
    }
    Ok(fixtures)
}

fn valid_fixture_id(fixture_id: &str) -> bool {
    !fixture_id.is_empty()
        && fixture_id.len() <= MAX_FIXTURE_ID_BYTES
        && fixture_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn compare_expectation(
    expectation: FixtureExpectation,
    evaluation: Result<Value, PolicyError>,
) -> Option<FixtureFailure> {
    match (expectation, evaluation) {
        (FixtureExpectation::Output(expected), Ok(actual)) if expected == actual => None,
        (FixtureExpectation::Output(expected), Ok(actual)) => Some(FixtureFailure {
            code: FixtureFailureCode::OutputMismatch,
            expected: Some(expected),
            actual: Some(actual),
            expected_error_code: None,
            actual_error_code: None,
        }),
        (FixtureExpectation::Output(expected), Err(error)) => Some(FixtureFailure {
            code: FixtureFailureCode::UnexpectedPolicyError,
            expected: Some(expected),
            actual: None,
            expected_error_code: None,
            actual_error_code: Some(error.code()),
        }),
        (FixtureExpectation::Error(expected), Err(error)) if expected == error.code() => None,
        (FixtureExpectation::Error(expected), Err(error)) => Some(FixtureFailure {
            code: FixtureFailureCode::WrongErrorCode,
            expected: None,
            actual: None,
            expected_error_code: Some(expected),
            actual_error_code: Some(error.code()),
        }),
        (FixtureExpectation::Error(expected), Ok(actual)) => Some(FixtureFailure {
            code: FixtureFailureCode::UnexpectedPolicySuccess,
            expected: None,
            actual: Some(actual),
            expected_error_code: Some(expected),
            actual_error_code: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;
    use crate::bundle::{POLICY_BUNDLE_SCHEMA, POLICY_ENGINE_ABI};
    use crate::{Limits, build_policy_engine};

    fn bundle(source: &str) -> PolicyBundle {
        PolicyBundle {
            schema: POLICY_BUNDLE_SCHEMA,
            policy_id: "fixture-policy".to_string(),
            version: 1,
            engine_abi: POLICY_ENGINE_ABI.to_string(),
            source: source.to_string(),
        }
    }

    fn engine() -> PolicyEngine {
        build_policy_engine(Limits::default())
    }

    fn corpus(rows: &[Value]) -> String {
        rows.iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn corpus_error(error: FixtureRunError) -> FixtureCorpusError {
        match error {
            FixtureRunError::Corpus(error) => error,
            FixtureRunError::Policy { source } => {
                panic!(
                    "expected corpus error, got policy error {}",
                    source.code().as_str()
                )
            }
        }
    }

    #[test]
    fn fixture_all_pass_mixes_exact_output_and_typed_error_and_compiles_once() {
        let engine = engine();
        let jsonl = corpus(&[
            json!({"fixture_id":"output","context":{"ok":true,"value":42},"expected":42}),
            json!({"fixture_id":"error","context":{"ok":false},"expected_error_code":"evaluation"}),
            json!({"fixture_id":"null-output","context":{"ok":true,"value":null},"expected":null}),
        ]);

        let report = run_fixture_jsonl(
            &engine,
            &bundle("if ctx.ok { ctx.value } else { ctx.missing }"),
            &jsonl,
        )
        .unwrap();

        assert!(report.is_pass());
        assert_eq!((report.total, report.passed, report.failed), (3, 3, 0));
        assert_eq!(engine.compile_count(), 1);
        assert_eq!(
            report
                .results
                .iter()
                .map(|result| result.fixture_id.as_str())
                .collect::<Vec<_>>(),
            vec!["output", "error", "null-output"]
        );
    }

    #[test]
    fn fixture_output_mismatch_is_a_failed_result() {
        let report = run_fixture_jsonl(
            &engine(),
            &bundle("ctx.value"),
            &corpus(&[json!({"fixture_id":"mismatch","context":{"value":1},"expected":2})]),
        )
        .unwrap();

        assert_eq!((report.passed, report.failed), (0, 1));
        let failure = report.results[0].failure.as_ref().unwrap();
        assert_eq!(failure.code, FixtureFailureCode::OutputMismatch);
        assert_eq!(failure.expected, Some(json!(2)));
        assert_eq!(failure.actual, Some(json!(1)));
    }

    #[test]
    fn fixture_oversized_output_is_not_retained_in_the_report() {
        let engine = build_policy_engine(Limits {
            max_context_size: 64,
            ..Limits::default()
        });
        let report = run_fixture_jsonl(
            &engine,
            &bundle("[ctx.value, ctx.value, ctx.value]"),
            &corpus(&[json!({
                "fixture_id":"oversized-output",
                "context":{"value":"xxxxxxxxxxxxxxxxxxxxxxxx"},
                "expected":1
            })]),
        )
        .unwrap();

        let failure = report.results[0].failure.as_ref().unwrap();
        assert_eq!(failure.code, FixtureFailureCode::UnexpectedPolicyError);
        assert_eq!(failure.actual, None);
        assert_eq!(
            failure.actual_error_code,
            Some(PolicyErrorCode::OutputTooLarge)
        );
    }

    #[test]
    fn fixture_wrong_error_code_is_a_failed_result() {
        let report = run_fixture_jsonl(
            &engine(),
            &bundle("ctx.missing"),
            &corpus(&[
                json!({"fixture_id":"wrong-code","context":{},"expected_error_code":"unsupported_value"}),
            ]),
        )
        .unwrap();

        let failure = report.results[0].failure.as_ref().unwrap();
        assert_eq!(failure.code, FixtureFailureCode::WrongErrorCode);
        assert_eq!(
            failure.expected_error_code,
            Some(PolicyErrorCode::UnsupportedValue)
        );
        assert_eq!(failure.actual_error_code, Some(PolicyErrorCode::Evaluation));
    }

    #[test]
    fn fixture_malformed_unknown_and_invalid_error_code_fail_closed() {
        let malformed = corpus_error(
            run_fixture_jsonl(&engine(), &bundle("1"), r#"{"fixture_id":"broken""#).unwrap_err(),
        );
        assert_eq!(malformed.code, FixtureCorpusErrorCode::MalformedJsonl);
        assert_eq!(malformed.line, Some(1));

        let unknown = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                r#"{"fixture_id":"unknown","context":{},"expected":1,"extra":true}"#,
            )
            .unwrap_err(),
        );
        assert_eq!(unknown.code, FixtureCorpusErrorCode::UnknownField);
        assert_eq!(unknown.fixture_id.as_deref(), Some("unknown"));

        let invalid_code = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                r#"{"fixture_id":"bad-code","context":{},"expected_error_code":"other"}"#,
            )
            .unwrap_err(),
        );
        assert_eq!(
            invalid_code.code,
            FixtureCorpusErrorCode::InvalidExpectedErrorCode
        );
    }

    #[test]
    fn fixture_duplicate_ambiguous_missing_expectation_and_invalid_id_fail_closed() {
        let duplicate = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                &corpus(&[
                    json!({"fixture_id":"same","context":{},"expected":1}),
                    json!({"fixture_id":"same","context":{},"expected":1}),
                ]),
            )
            .unwrap_err(),
        );
        assert_eq!(duplicate.code, FixtureCorpusErrorCode::DuplicateFixtureId);
        assert_eq!(duplicate.line, Some(2));
        assert_eq!(duplicate.fixture_id.as_deref(), Some("same"));

        let ambiguous = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                r#"{"fixture_id":"ambiguous","context":{},"expected":1,"expected_error_code":"evaluation"}"#,
            )
            .unwrap_err(),
        );
        assert_eq!(ambiguous.code, FixtureCorpusErrorCode::AmbiguousExpectation);

        let missing_expectation = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                r#"{"fixture_id":"missing","context":{}}"#,
            )
            .unwrap_err(),
        );
        assert_eq!(
            missing_expectation.code,
            FixtureCorpusErrorCode::MissingExpectation
        );

        let missing_id = corpus_error(
            run_fixture_jsonl(&engine(), &bundle("1"), r#"{"context":{},"expected":1}"#)
                .unwrap_err(),
        );
        assert_eq!(missing_id.code, FixtureCorpusErrorCode::MissingFixtureId);
        assert_eq!(missing_id.line, Some(1));

        let empty_id = corpus_error(
            run_fixture_jsonl(
                &engine(),
                &bundle("1"),
                r#"{"fixture_id":"","context":{},"expected":1}"#,
            )
            .unwrap_err(),
        );
        assert_eq!(empty_id.code, FixtureCorpusErrorCode::InvalidFixtureId);
        assert_eq!(empty_id.line, Some(1));
        assert_eq!(empty_id.fixture_id.as_deref(), Some(""));
    }

    #[test]
    fn fixture_report_preserves_source_order() {
        let report = run_fixture_jsonl(
            &engine(),
            &bundle("ctx.value"),
            &corpus(&[
                json!({"fixture_id":"z","context":{"value":1},"expected":1}),
                json!({"fixture_id":"a","context":{"value":2},"expected":2}),
                json!({"fixture_id":"m","context":{"value":3},"expected":3}),
            ]),
        )
        .unwrap();

        assert_eq!(
            report
                .results
                .iter()
                .map(|result| result.fixture_id.as_str())
                .collect::<Vec<_>>(),
            vec!["z", "a", "m"]
        );
        assert_eq!(
            report
                .results
                .iter()
                .map(|result| result.line)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn fixture_corpus_row_and_context_bounds_fail_before_compilation() {
        let engine = engine();
        let base_limits = RunnerLimits {
            max_corpus_bytes: 32,
            max_rows: 8,
            max_context_bytes: 32,
        };
        let too_large = corpus_error(
            run_fixture_jsonl_with_limits(&engine, &bundle("1"), &"x".repeat(33), base_limits)
                .unwrap_err(),
        );
        assert_eq!(too_large.code, FixtureCorpusErrorCode::CorpusTooLarge);

        let too_many = corpus_error(
            run_fixture_jsonl_with_limits(
                &engine,
                &bundle("1"),
                &corpus(&[
                    json!({"fixture_id":"one","context":{},"expected":1}),
                    json!({"fixture_id":"two","context":{},"expected":1}),
                ]),
                RunnerLimits {
                    max_corpus_bytes: 1024,
                    max_rows: 1,
                    max_context_bytes: 32,
                },
            )
            .unwrap_err(),
        );
        assert_eq!(too_many.code, FixtureCorpusErrorCode::TooManyRows);
        assert_eq!(too_many.line, Some(2));

        let context = corpus_error(
            run_fixture_jsonl_with_limits(
                &engine,
                &bundle("1"),
                &corpus(&[
                    json!({"fixture_id":"large-context","context":{"value":"123456789"},"expected":1}),
                ]),
                RunnerLimits {
                    max_corpus_bytes: 1024,
                    max_rows: 1,
                    max_context_bytes: 8,
                },
            )
            .unwrap_err(),
        );
        assert_eq!(context.code, FixtureCorpusErrorCode::ContextTooLarge);
        assert_eq!(context.fixture_id.as_deref(), Some("large-context"));
        assert_eq!(engine.compile_count(), 0);
    }

    #[test]
    fn fixture_runner_does_not_create_files_or_state() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sentinel = std::env::temp_dir().join(format!(
            "vida-policy-fixture-runner-{}-{nonce}",
            std::process::id()
        ));
        assert!(!sentinel.exists());
        let report = run_fixture_jsonl(
            &engine(),
            &bundle("ctx.allowed"),
            &corpus(&[json!({
                "fixture_id":"no-io",
                "context":{"allowed":true,"path":sentinel},
                "expected":true
            })]),
        )
        .unwrap();

        assert!(report.is_pass());
        assert!(!sentinel.exists());
    }
}
