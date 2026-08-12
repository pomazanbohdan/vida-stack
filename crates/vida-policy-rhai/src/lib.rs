#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use rhai::{AST, Dynamic, Engine, EvalAltResult, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod bundle;
pub mod fixture;

pub use bundle::{BundleCacheStatus, PolicyBundle, PolicyBundleCache, PolicyBundleError};
pub use fixture::{FixtureReport, FixtureRunError, run_fixture_jsonl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_operations: u64,
    pub max_call_levels: usize,
    pub max_expr_depth: usize,
    pub max_string_size: usize,
    pub max_array_size: usize,
    pub max_map_size: usize,
    pub max_script_size: usize,
    pub max_context_size: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_operations: 10_000,
            max_call_levels: 32,
            max_expr_depth: 64,
            max_string_size: 4_096,
            max_array_size: 64,
            max_map_size: 64,
            max_script_size: 4_096,
            max_context_size: 16 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyErrorCode {
    ScriptTooLarge,
    ContextTooLarge,
    OutputTooLarge,
    Compile,
    Evaluation,
    UnsupportedValue,
    Json,
}

impl PolicyErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScriptTooLarge => "script_too_large",
            Self::ContextTooLarge => "context_too_large",
            Self::OutputTooLarge => "output_too_large",
            Self::Compile => "compile",
            Self::Evaluation => "evaluation",
            Self::UnsupportedValue => "unsupported_value",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    ScriptTooLarge { actual: usize, limit: usize },
    ContextTooLarge { actual: usize, limit: usize },
    OutputTooLarge { actual: usize, limit: usize },
    Compile(String),
    Evaluation(String),
    UnsupportedValue(String),
    Json(String),
}

impl PolicyError {
    #[must_use]
    pub const fn code(&self) -> PolicyErrorCode {
        match self {
            Self::ScriptTooLarge { .. } => PolicyErrorCode::ScriptTooLarge,
            Self::ContextTooLarge { .. } => PolicyErrorCode::ContextTooLarge,
            Self::OutputTooLarge { .. } => PolicyErrorCode::OutputTooLarge,
            Self::Compile(_) => PolicyErrorCode::Compile,
            Self::Evaluation(_) => PolicyErrorCode::Evaluation,
            Self::UnsupportedValue(_) => PolicyErrorCode::UnsupportedValue,
            Self::Json(_) => PolicyErrorCode::Json,
        }
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScriptTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "policy script size {actual} exceeds limit {limit}"
                )
            }
            Self::ContextTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "policy context size {actual} exceeds limit {limit}"
                )
            }
            Self::OutputTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "policy output size exceeds limit {limit} bytes (at least {actual} bytes)"
                )
            }
            Self::Compile(error) => write!(formatter, "policy compile failed: {error}"),
            Self::Evaluation(error) => write!(formatter, "policy evaluation failed: {error}"),
            Self::UnsupportedValue(value) => write!(formatter, "unsupported Rhai value: {value}"),
            Self::Json(error) => write!(formatter, "policy JSON conversion failed: {error}"),
        }
    }
}

impl Error for PolicyError {}

pub struct PolicyEngine {
    engine: Engine,
    limits: Limits,
    #[cfg(test)]
    compile_count: std::cell::Cell<usize>,
}

pub fn build_policy_engine(limits: Limits) -> PolicyEngine {
    let mut engine = Engine::new_raw();
    engine
        .set_strict_variables(true)
        .set_fail_on_invalid_map_property(true)
        .set_max_operations(limits.max_operations)
        .set_max_call_levels(limits.max_call_levels)
        .set_max_expr_depths(limits.max_expr_depth, limits.max_expr_depth)
        .set_max_string_size(limits.max_string_size)
        .set_max_array_size(limits.max_array_size)
        .set_max_map_size(limits.max_map_size);
    for symbol in ["eval", "print", "debug"] {
        engine.disable_symbol(symbol);
    }
    PolicyEngine {
        engine,
        limits,
        #[cfg(test)]
        compile_count: std::cell::Cell::new(0),
    }
}

impl PolicyEngine {
    pub(crate) fn compile_source(&self, script: &str) -> Result<AST, PolicyError> {
        self.validate_script(script)?;
        #[cfg(test)]
        self.compile_count.set(self.compile_count.get() + 1);
        let mut scope = Scope::new();
        scope.push_dynamic("ctx", Dynamic::UNIT);
        self.engine
            .compile_with_scope(&scope, script)
            .map_err(|error| PolicyError::Compile(error.to_string()))
    }

    pub fn evaluate(&self, script: &str, ctx: Value) -> Result<Value, PolicyError> {
        let ast = self.compile_source(script)?;
        self.evaluate_ast(&ast, ctx)
    }

    pub(crate) fn evaluate_ast(&self, ast: &AST, ctx: Value) -> Result<Value, PolicyError> {
        let context_bytes =
            serde_json::to_vec(&ctx).map_err(|error| PolicyError::Json(error.to_string()))?;
        if context_bytes.len() > self.limits.max_context_size {
            return Err(PolicyError::ContextTooLarge {
                actual: context_bytes.len(),
                limit: self.limits.max_context_size,
            });
        }
        let mut scope = Scope::new();
        scope.push_dynamic("ctx", json_to_dynamic(&ctx));
        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, ast)
            .map_err(|error| PolicyError::Evaluation(error.to_string()))?;
        dynamic_to_json(result, &mut OutputBudget::new(self.limits.max_context_size))
    }

    fn validate_script(&self, script: &str) -> Result<(), PolicyError> {
        if script.len() > self.limits.max_script_size {
            return Err(PolicyError::ScriptTooLarge {
                actual: script.len(),
                limit: self.limits.max_script_size,
            });
        }
        if let Some(symbol) = forbidden_construct(script) {
            let error = EvalAltResult::ErrorFunctionNotFound(
                format!("policy construct `{symbol}` denied"),
                rhai::Position::NONE,
            );
            return Err(PolicyError::Evaluation(error.to_string()));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn compile_count(&self) -> usize {
        self.compile_count.get()
    }
}

fn forbidden_construct(script: &str) -> Option<&'static str> {
    let mut token = String::new();
    for character in script.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_alphanumeric() || character == '_' {
            token.push(character.to_ascii_lowercase());
            continue;
        }
        match token.as_str() {
            "eval" => return Some("eval"),
            "import" => return Some("import"),
            _ => token.clear(),
        }
    }
    None
}

fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(value) => Dynamic::from_bool(*value),
        Value::Number(value) => value
            .as_i64()
            .map(|value| Dynamic::from_int(value as rhai::INT))
            .or_else(|| value.as_f64().map(Dynamic::from_float))
            .unwrap_or(Dynamic::UNIT),
        Value::String(value) => Dynamic::from(value.clone()),
        Value::Array(values) => Dynamic::from_array(values.iter().map(json_to_dynamic).collect()),
        Value::Object(values) => Dynamic::from_map(
            values
                .iter()
                .map(|(key, value)| (key.clone().into(), json_to_dynamic(value)))
                .collect(),
        ),
    }
}

struct OutputBudget {
    used: usize,
    limit: usize,
}

impl OutputBudget {
    const fn new(limit: usize) -> Self {
        Self { used: 0, limit }
    }

    fn consume(&mut self, bytes: usize) -> Result<(), PolicyError> {
        self.used = self.used.saturating_add(bytes);
        if self.used > self.limit {
            return Err(PolicyError::OutputTooLarge {
                actual: self.used,
                limit: self.limit,
            });
        }
        Ok(())
    }
}

fn dynamic_to_json(value: Dynamic, budget: &mut OutputBudget) -> Result<Value, PolicyError> {
    if value.is_unit() {
        budget.consume(4)?;
        return Ok(Value::Null);
    }
    if let Some(value) = value.clone().try_cast::<bool>() {
        budget.consume(if value { 4 } else { 5 })?;
        return Ok(Value::Bool(value));
    }
    if let Some(value) = value.clone().try_cast::<rhai::INT>() {
        budget.consume(value.to_string().len())?;
        return Ok(Value::Number(value.into()));
    }
    if let Some(value) = value.clone().try_cast::<rhai::FLOAT>() {
        let number = serde_json::Number::from_f64(value)
            .ok_or_else(|| PolicyError::UnsupportedValue("non-finite float".to_string()))?;
        budget.consume(number.to_string().len())?;
        return Ok(Value::Number(number));
    }
    if let Some(value) = value.clone().try_cast::<rhai::ImmutableString>() {
        let value = value.to_string();
        let serialized_len = serde_json::to_vec(&value)
            .map_err(|error| PolicyError::Json(error.to_string()))?
            .len();
        budget.consume(serialized_len)?;
        return Ok(Value::String(value));
    }
    if let Some(values) = value.clone().try_cast::<rhai::Array>() {
        budget.consume(2 + values.len().saturating_sub(1))?;
        return values
            .into_iter()
            .map(|value| dynamic_to_json(value, budget))
            .collect();
    }
    if let Some(values) = value.try_cast::<rhai::Map>() {
        budget.consume(2 + values.len().saturating_sub(1))?;
        let entries = values
            .into_iter()
            .map(|(key, value)| {
                let key = key.to_string();
                let key_len = serde_json::to_vec(&key)
                    .map_err(|error| PolicyError::Json(error.to_string()))?
                    .len();
                budget.consume(key_len + 1)?;
                dynamic_to_json(value, budget).map(|value| (key, value))
            })
            .collect::<Result<Vec<_>, PolicyError>>()?;
        return Ok(Value::Object(entries.into_iter().collect()));
    }
    Err(PolicyError::UnsupportedValue("custom value".to_string()))
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;
    use serde_json::json;

    fn engine() -> PolicyEngine {
        build_policy_engine(Limits {
            max_operations: 64,
            max_call_levels: 4,
            max_expr_depth: 32,
            max_string_size: 64,
            max_array_size: 4,
            max_map_size: 4,
            max_script_size: 64,
            max_context_size: 64,
        })
    }

    #[test]
    fn sandbox_positive_bounded_evaluation() {
        assert_eq!(
            engine().evaluate("ctx.x + 1", json!({"x": 41})),
            Ok(json!(42))
        );
    }

    #[test]
    fn sandbox_rejects_unknown_property() {
        assert!(engine().evaluate("ctx.missing", json!({"x": 41})).is_err());
    }

    #[test]
    fn sandbox_rejects_dynamic_eval() {
        assert!(engine().evaluate("eval(\"1 + 1\")", json!({})).is_err());
    }

    #[test]
    fn sandbox_rejects_import() {
        assert!(
            engine()
                .evaluate(r#"import \"blocked\" as blocked; 1"#, json!({}))
                .is_err()
        );
    }

    #[test]
    fn sandbox_enforces_operation_limit() {
        assert!(
            engine()
                .evaluate("let n = 0; while n < 100000 { n += 1; } n", json!({}))
                .is_err()
        );
    }

    #[test]
    fn sandbox_enforces_recursion_call_depth() {
        assert!(
            engine()
                .evaluate("fn recurse() { recurse(); } recurse()", json!({}))
                .is_err()
        );
    }

    #[test]
    fn sandbox_rejects_oversized_script_or_context() {
        assert!(matches!(
            engine().evaluate(&"1".repeat(65), json!({})),
            Err(PolicyError::ScriptTooLarge { .. })
        ));
        assert!(matches!(
            engine().evaluate("1", json!({"value": "x".repeat(65)})),
            Err(PolicyError::ContextTooLarge { .. })
        ));
    }

    #[test]
    fn sandbox_rejects_output_larger_than_the_context_budget() {
        let value = "x".repeat(24);
        let engine = build_policy_engine(Limits {
            max_operations: 64,
            max_call_levels: 4,
            max_expr_depth: 32,
            max_string_size: 256,
            max_array_size: 4,
            max_map_size: 4,
            max_script_size: 64,
            max_context_size: 64,
        });
        assert!(matches!(
            engine.evaluate("[ctx.value, ctx.value, ctx.value]", json!({"value": value})),
            Err(PolicyError::OutputTooLarge { limit: 64, .. })
        ));
    }
}
