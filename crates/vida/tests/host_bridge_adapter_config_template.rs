use std::path::PathBuf;

use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

fn repo_file(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative_path)
}

fn canonical_template() -> YamlValue {
    serde_yaml::from_str(
        &std::fs::read_to_string(repo_file(
            "docs/framework/templates/vida.config.yaml.template",
        ))
        .expect("canonical config template should read"),
    )
    .expect("canonical config template should parse")
}

fn project_config() -> YamlValue {
    serde_yaml::from_str(
        &std::fs::read_to_string(repo_file("vida.config.yaml"))
            .expect("project config should read"),
    )
    .expect("project config should parse")
}

fn command_schema() -> JsonValue {
    serde_json::from_str(
        &std::fs::read_to_string(repo_file(
            "vida/config/schemas/host_tool_bridge_adapter_command.schema.json",
        ))
        .expect("adapter command schema should read"),
    )
    .expect("adapter command schema should parse")
}

fn yaml_field<'a>(value: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    value.as_mapping()?.get(&YamlValue::String(key.to_string()))
}

fn yaml_string(value: &YamlValue) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn yaml_string_list(value: &YamlValue) -> Option<Vec<&str>> {
    Some(
        value
            .as_sequence()?
            .iter()
            .map(|item| item.as_str().map(str::trim))
            .collect::<Option<Vec<_>>>()?,
    )
}

fn host_bridge_system(template: &YamlValue) -> &YamlValue {
    template["host_environment"]["systems"]
        .as_mapping()
        .and_then(|systems| {
            systems
                .values()
                .find(|system| yaml_field(system, "host_tool_bridge").is_some())
        })
        .expect("template should declare one host bridge system")
}

fn placeholder_occurrences(token: &str, open: &str, close: &str) -> (usize, bool) {
    let mut count = 0;
    let mut offset = 0;
    while let Some(relative_start) = token[offset..].find(open) {
        let start = offset + relative_start;
        let Some(relative_end) = token[start + open.len()..].find(close) else {
            return (count, false);
        };
        let end = start + open.len() + relative_end;
        assert!(
            !token[start + open.len()..end].trim().is_empty(),
            "placeholder body must be non-empty"
        );
        count += 1;
        offset = end + close.len();
    }
    (count, true)
}

#[test]
fn canonical_adapter_command_is_schema_shaped_and_placeholder_complete() {
    let template = canonical_template();
    let schema = command_schema();
    let system = yaml_field(host_bridge_system(&template), "host_tool_bridge")
        .expect("host bridge system should contain host_tool_bridge");
    let command =
        yaml_field(system, "adapter_command").expect("template should declare adapter_command");
    let schema_required = schema["required"]
        .as_array()
        .expect("schema required must be an array");
    let schema_properties = schema["properties"]
        .as_object()
        .expect("schema properties must be an object");
    let command_map = command
        .as_mapping()
        .expect("adapter_command must be a mapping");
    let contract = yaml_field(system, "adapter_command_contract")
        .expect("template should declare adapter_command_contract");

    for required in schema_required {
        let key = required
            .as_str()
            .expect("schema required key must be a string");
        assert!(
            yaml_field(command, key).is_some(),
            "template adapter_command must include schema field {key}"
        );
    }
    for key in command_map.keys() {
        let key = key
            .as_str()
            .expect("template adapter_command keys must be strings");
        assert!(
            schema_properties.contains_key(key),
            "template adapter_command field {key} is not declared by schema"
        );
    }
    let supported_fields = yaml_string_list(yaml_field(contract, "supported_fields").unwrap())
        .expect("supported_fields must be a string sequence");
    assert_eq!(supported_fields.len(), schema_properties.len());
    for field in supported_fields {
        assert!(schema_properties.contains_key(field));
    }
    assert!(
        !yaml_string_list(yaml_field(contract, "supported_capabilities").unwrap())
            .expect("supported_capabilities must be a string sequence")
            .is_empty()
    );

    let executable = yaml_string(yaml_field(command, "executable").unwrap())
        .expect("adapter executable must be non-empty");
    assert!(!executable.contains('\0'));
    assert!(!executable.contains('\r'));
    assert!(!executable.contains('\n'));
    let subcommands = yaml_string_list(yaml_field(command, "subcommands").unwrap())
        .expect("subcommands must be a string sequence");
    let args = yaml_string_list(yaml_field(command, "args").unwrap())
        .expect("args must be a string sequence");
    for token in subcommands.iter().chain(args.iter()) {
        assert!(!token.is_empty(), "command tokens must be non-empty");
        assert!(!token.contains('\0'), "command tokens must be single-line");
        assert!(!token.contains('\r'), "command tokens must be single-line");
        assert!(!token.contains('\n'), "command tokens must be single-line");
    }

    let open = yaml_string(yaml_field(contract, "placeholder_open").unwrap())
        .expect("placeholder_open must be declared");
    let close = yaml_string(yaml_field(contract, "placeholder_close").unwrap())
        .expect("placeholder_close must be declared");
    let request_placeholder = yaml_string(yaml_field(contract, "request_placeholder").unwrap())
        .expect("request_placeholder must be declared");
    let (placeholder_count, placeholder_terminated) =
        placeholder_occurrences(request_placeholder, open, close);
    assert!(placeholder_terminated);
    assert_eq!(placeholder_count, 1);
    assert!(yaml_string(yaml_field(contract, "placeholder_scope").unwrap()).is_some());

    let mut occurrences = 0;
    for token in subcommands
        .iter()
        .chain(args.iter())
        .chain(std::iter::once(&executable))
    {
        let (count, terminated) = placeholder_occurrences(token, open, close);
        assert!(terminated, "command recipe placeholders must be terminated");
        occurrences += count;
    }
    assert_eq!(
        occurrences, 1,
        "command recipe must have exactly one request placeholder"
    );
    assert!(args.iter().any(|token| token.contains(request_placeholder)));
    assert!(!executable.contains(open));
    assert!(subcommands.iter().all(|token| !token.contains(open)));
}

#[test]
fn canonical_adapter_command_matrix_covers_every_declared_state_and_field() {
    let template = canonical_template();
    let system = yaml_field(host_bridge_system(&template), "host_tool_bridge")
        .expect("host bridge system should contain host_tool_bridge");
    let contract = yaml_field(system, "adapter_command_contract")
        .expect("template should declare adapter_command_contract");
    let states = yaml_string_list(yaml_field(contract, "adapter_command_states").unwrap())
        .expect("adapter command states must be a string sequence");
    let required_fields = yaml_string_list(yaml_field(contract, "matrix_required_fields").unwrap())
        .expect("matrix required fields must be a string sequence");
    let matrix = yaml_field(contract, "admissible_route_command_combinations")
        .and_then(YamlValue::as_sequence)
        .expect("adapter command matrix must be a sequence");
    assert!(
        !matrix.is_empty(),
        "adapter command matrix must not be empty"
    );

    let mut observed_states = Vec::new();
    let mut observed_ids = Vec::new();
    for row in matrix {
        for field in &required_fields {
            assert!(
                yaml_field(row, field).is_some(),
                "matrix row must declare {field}"
            );
        }
        let id = yaml_string(yaml_field(row, "id").unwrap()).expect("matrix id must be non-empty");
        assert!(
            observed_ids.iter().all(|seen| *seen != id),
            "matrix ids must be unique"
        );
        observed_ids.push(id);

        let state = yaml_string(yaml_field(row, "adapter_command_state").unwrap())
            .expect("matrix adapter command state must be non-empty");
        assert!(states.iter().any(|declared| *declared == state));
        if !observed_states.iter().any(|seen| *seen == state) {
            observed_states.push(state);
        }
    }
    for state in states {
        assert!(
            observed_states.iter().any(|observed| *observed == state),
            "declared adapter command state {state} must have a matrix row"
        );
    }
}

#[test]
fn project_adapter_command_override_is_template_declared_and_route_admissible() {
    let template = canonical_template();
    let project = project_config();
    let schema = command_schema();
    let template_system = host_bridge_system(&template);
    let project_system = host_bridge_system(&project);
    let template_bridge = yaml_field(template_system, "host_tool_bridge")
        .expect("template host bridge system should contain host_tool_bridge");
    let project_bridge = yaml_field(project_system, "host_tool_bridge")
        .expect("project host bridge system should contain host_tool_bridge");
    let template_contract = yaml_field(template_bridge, "adapter_command_contract")
        .expect("template should declare adapter_command_contract");
    let project_command = yaml_field(project_bridge, "adapter_command")
        .expect("project should select the declared adapter_command map");
    let template_bridge_map = template_bridge
        .as_mapping()
        .expect("template host_tool_bridge must be a mapping");
    let project_bridge_map = project_bridge
        .as_mapping()
        .expect("project host_tool_bridge must be a mapping");
    for key in project_bridge_map.keys() {
        assert!(
            template_bridge_map.contains_key(key),
            "project host bridge field must be declared by the master template"
        );
    }
    let project_map = project_command
        .as_mapping()
        .expect("project adapter_command must be a mapping");
    let schema_properties = schema["properties"]
        .as_object()
        .expect("schema properties must be an object");
    let supported_fields =
        yaml_string_list(yaml_field(template_contract, "supported_fields").unwrap())
            .expect("template supported_fields must be a string sequence");
    for required in schema["required"].as_array().unwrap() {
        let key = required.as_str().unwrap();
        assert!(yaml_field(project_command, key).is_some());
    }
    for key in project_map.keys() {
        let key = key.as_str().unwrap();
        assert!(schema_properties.contains_key(key));
        assert!(supported_fields.iter().any(|declared| *declared == key));
    }

    let executable = yaml_string(yaml_field(project_command, "executable").unwrap())
        .expect("project adapter executable must be non-empty");
    assert!(!executable.contains('\0'));
    assert!(!executable.contains('\r'));
    assert!(!executable.contains('\n'));
    let subcommands = yaml_string_list(yaml_field(project_command, "subcommands").unwrap())
        .expect("project subcommands must be a string sequence");
    let args = yaml_string_list(yaml_field(project_command, "args").unwrap())
        .expect("project args must be a string sequence");
    for token in subcommands.iter().chain(args.iter()) {
        assert!(!token.is_empty());
        assert!(!token.contains('\0'));
        assert!(!token.contains('\r'));
        assert!(!token.contains('\n'));
    }
    let open = yaml_string(yaml_field(template_contract, "placeholder_open").unwrap()).unwrap();
    let close = yaml_string(yaml_field(template_contract, "placeholder_close").unwrap()).unwrap();
    let request_placeholder =
        yaml_string(yaml_field(template_contract, "request_placeholder").unwrap()).unwrap();
    let mut occurrences = 0;
    for token in subcommands
        .iter()
        .chain(args.iter())
        .chain(std::iter::once(&executable))
    {
        let (count, terminated) = placeholder_occurrences(token, open, close);
        assert!(terminated);
        occurrences += count;
    }
    assert_eq!(occurrences, 1);
    assert!(args.iter().any(|token| token.contains(request_placeholder)));
    assert!(!executable.contains(open));
    assert!(subcommands.iter().all(|token| !token.contains(open)));

    let project_transport = yaml_string(yaml_field(project_system, "dispatch_transport").unwrap())
        .expect("project system dispatch transport must be declared");
    let project_boundary = yaml_string(yaml_field(project_system, "execution_boundary").unwrap())
        .expect("project system execution boundary must be declared");
    let matrix = yaml_field(template_contract, "admissible_route_command_combinations")
        .and_then(YamlValue::as_sequence)
        .unwrap();
    assert!(matrix.iter().any(|row| {
        yaml_string(yaml_field(row, "dispatch_transport").unwrap()) == Some(project_transport)
            && yaml_string(yaml_field(row, "execution_boundary").unwrap()) == Some(project_boundary)
            && yaml_field(row, "fail_closed").and_then(YamlValue::as_bool) == Some(false)
    }));
}
