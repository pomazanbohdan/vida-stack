use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorToonField {
    pub key: String,
    pub value: String,
}

impl OperatorToonField {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

pub fn render(surface: &str, fields: &[OperatorToonField]) -> String {
    let mut output = String::new();
    output.push_str(surface);
    output.push_str(":\n");
    for field in fields {
        output.push_str(&field.key);
        output.push_str(": ");
        output.push_str(&field.value);
        output.push('\n');
    }
    output
}

pub fn print(surface: &str, fields: &[OperatorToonField]) {
    print!("{}", render(surface, fields));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_outputs_compact_key_value_lines() {
        let rendered = render("status", &[OperatorToonField::new("state", "pass")]);
        assert_eq!(rendered, "status:\nstate: pass\n");
    }
}
