#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use serde_json::Value;

pub mod bundle;

pub use bundle::{BundleCacheStatus, PolicyBundle, PolicyBundleCache, PolicyBundleError};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    ScriptTooLarge { actual: usize, limit: usize },
    ContextTooLarge { actual: usize, limit: usize },
    Compile(String),
    Evaluation(String),
    UnsupportedValue(String),
    Json(String),
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
    PolicyEngine { engine, limits }
}

impl PolicyEngine {
    pub(crate) fn compile_source(&self, script: &str) -> Result<rhai::AST, PolicyError> {
        let mut scope = Scope::new();
        scope.push_dynamic("ctx", Dynamic::UNIT);
        self.engine
            .compile_with_scope(&scope, script)
            .map_err(|error| PolicyError::Compile(error.to_string()))
    }

    pub fn evaluate(&self, script: &str, ctx: Value) -> Result<Value, PolicyError> {
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
            .eval_with_scope::<Dynamic>(&mut scope, script)
            .map_err(|error| PolicyError::Evaluation(error.to_string()))?;
        dynamic_to_json(result)
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

fn dynamic_to_json(value: Dynamic) -> Result<Value, PolicyError> {
    if value.is_unit() {
        return Ok(Value::Null);
    }
    if let Some(value) = value.clone().try_cast::<bool>() {
        return Ok(Value::Bool(value));
    }
    if let Some(value) = value.clone().try_cast::<rhai::INT>() {
        return Ok(Value::Number(value.into()));
    }
    if let Some(value) = value.clone().try_cast::<rhai::FLOAT>() {
        let number = serde_json::Number::from_f64(value as f64)
            .ok_or_else(|| PolicyError::UnsupportedValue("non-finite float".to_string()))?;
        return Ok(Value::Number(number));
    }
    if let Some(value) = value.clone().try_cast::<rhai::ImmutableString>() {
        return Ok(Value::String(value.to_string()));
    }
    if let Some(values) = value.clone().try_cast::<rhai::Array>() {
        return values.into_iter().map(dynamic_to_json).collect();
    }
    if let Some(values) = value.try_cast::<rhai::Map>() {
        let entries = values
            .into_iter()
            .map(|(key, value)| dynamic_to_json(value).map(|value| (key.to_string(), value)))
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
        assert!(engine()
            .evaluate(r#"import \"blocked\" as blocked; 1"#, json!({}))
            .is_err());
    }

    #[test]
    fn sandbox_enforces_operation_limit() {
        assert!(engine()
            .evaluate("let n = 0; while n < 100000 { n += 1; } n", json!({}))
            .is_err());
    }

    #[test]
    fn sandbox_enforces_recursion_call_depth() {
        assert!(engine()
            .evaluate("fn recurse() { recurse(); } recurse()", json!({}))
            .is_err());
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
}
