use crate::RenderMode;
use serde::Serialize;

const BOOTSTRAP_COMMAND_ENTRIES: [(&str, &str); 4] = [
    ("init", "bootstrap framework carriers into the current project"),
    (
        "boot",
        "initialize authoritative state and instruction/framework-memory surfaces",
    ),
    (
        "orchestrator-init",
        "render the compiled startup view for the orchestrator lane",
    ),
    (
        "agent-init",
        "render the bounded startup view or packet activation view for a worker/agent lane; this surface does not itself execute the packet",
    ),
];
const ACTIVATION_COMMAND_ENTRIES: [(&str, &str); 1] = [(
    "project-activator",
    "inspect project activation posture and bounded onboarding next steps",
)];
const PROTOCOL_COMMAND_ENTRIES: [(&str, &str); 1] = [(
    "protocol",
    "resolve and render framework protocol/guide surfaces",
)];
const RUNTIME_STATUS_COMMAND_ENTRIES: [(&str, &str); 2] = [
    (
        "status",
        "inspect backend, state spine, and latest receipts",
    ),
    ("doctor", "run bounded runtime integrity checks"),
];
const TASK_RUNTIME_COMMAND_ENTRIES: [(&str, &str); 3] = [
    (
        "task",
        "task import/list/show/ready over the authoritative state store",
    ),
    ("consume", "thin root alias to the TaskFlow consume family"),
    ("taskflow", "delegate to the TaskFlow runtime family"),
];
const DOCUMENTATION_COMMAND_ENTRIES: [(&str, &str); 1] =
    [("docflow", "delegate to the DocFlow runtime family")];
const LANE_CONTROL_COMMAND_ENTRIES: [(&str, &str); 3] = [
    (
        "lane",
        "inspect or mutate canonical lane/takeover operator state",
    ),
    (
        "approval",
        "inspect approval wait/complete state over run-graph approval law",
    ),
    (
        "recovery",
        "thin root alias to the TaskFlow recovery family",
    ),
];
const SUPPORT_COMMAND_ENTRIES: [(&str, &str); 2] = [
    (
        "agent-feedback",
        "record host-agent feedback and refresh local strategy state",
    ),
    ("memory", "inspect the effective instruction bundle"),
];

pub(crate) fn print_surface_json<T: Serialize + ?Sized>(
    value: &T,
    as_json: bool,
    context: &str,
) -> bool {
    if !as_json {
        return false;
    }

    println!("{}", serde_json::to_string_pretty(value).expect(context));
    true
}

pub(crate) fn print_surface_header(render: RenderMode, title: &str) {
    match render {
        RenderMode::Plain => println!("{title}"),
        RenderMode::Color => println!("\x1b[1;36m{title}\x1b[0m"),
        RenderMode::ColorEmoji => println!("\x1b[1;36m📘 {title}\x1b[0m"),
    }
}

pub(crate) fn print_surface_line(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: {value}"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: {value}"),
        RenderMode::ColorEmoji => println!("🔹 \x1b[1;34m{label}\x1b[0m: {value}"),
    }
}

pub(crate) fn print_surface_ok(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: pass ({value})"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: \x1b[1;32mpass\x1b[0m ({value})"),
        RenderMode::ColorEmoji => {
            println!("✅ \x1b[1;34m{label}\x1b[0m: \x1b[1;32mpass\x1b[0m ({value})")
        }
    }
}

fn command_family_scope_and_availability(
    surface: &str,
    family_id: &str,
) -> (&'static str, &'static str) {
    if surface == "vida agent-init" {
        match family_id {
            "runtime_status" | "lane_control" | "project_activation" => {
                ("root_only", "view_only_reference")
            }
            "task_runtime" => ("shared", "view_only_reference"),
            _ => ("shared", "callable"),
        }
    } else {
        match family_id {
            "runtime_status" | "lane_control" => ("orchestrator_preferred", "callable"),
            _ => ("shared", "callable"),
        }
    }
}

fn operator_command_family(
    surface: &str,
    family_id: &str,
    label: &str,
    notes: &str,
    entries: &[(&str, &str)],
) -> serde_json::Value {
    let (lane_scope, availability) = command_family_scope_and_availability(surface, family_id);
    serde_json::json!({
        "family_id": family_id,
        "label": label,
        "lane_scope": lane_scope,
        "availability": availability,
        "notes": notes,
        "commands": entries.iter().map(|(command, _)| *command).collect::<Vec<_>>(),
        "entries": entries
            .iter()
            .map(|(command, summary)| serde_json::json!({
                "command": command,
                "summary": summary,
            }))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn operator_command_map(surface: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": "v1",
        "source_surface": "vida --help",
        "projection_surface": surface,
        "families": [
            operator_command_family(
                surface,
                "bootstrap",
                "Bootstrap",
                "bootstrap and startup entry surfaces",
                &BOOTSTRAP_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "project_activation",
                "Project Activation",
                "project activation and onboarding posture",
                &ACTIVATION_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "protocol_discovery",
                "Protocol Discovery",
                "protocol and guide lookup surfaces",
                &PROTOCOL_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "runtime_status",
                "Runtime Status",
                "bounded runtime inspection and integrity checks",
                &RUNTIME_STATUS_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "task_runtime",
                "Task Runtime",
                "task authority and taskflow runtime entry surfaces",
                &TASK_RUNTIME_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "documentation",
                "Documentation",
                "documentation and readiness runtime family",
                &DOCUMENTATION_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "lane_control",
                "Lane Control",
                "lane, approval, and recovery operator surfaces",
                &LANE_CONTROL_COMMAND_ENTRIES,
            ),
            operator_command_family(
                surface,
                "support",
                "Support",
                "feedback and memory support surfaces",
                &SUPPORT_COMMAND_ENTRIES,
            ),
        ],
    })
}

pub(crate) fn print_compact_command_families(render: RenderMode, surface: &str) {
    let families = operator_command_map(surface)["families"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for family in families {
        let label = family["label"].as_str().unwrap_or("unknown");
        let lane_scope = family["lane_scope"].as_str().unwrap_or("unknown");
        let availability = family["availability"].as_str().unwrap_or("unknown");
        let commands = family["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        print_surface_line(
            render,
            "command family",
            &format!("{label} [{lane_scope}/{availability}]: {commands}"),
        );
    }
}

pub(crate) fn print_root_help() {
    println!("VIDA Binary Foundation");
    println!();
    println!("Usage:");
    println!("  vida <command>");
    println!("  vida taskflow <args...>");
    println!("  vida docflow <args...>");
    println!();
    println!("Root commands:");
    for family in operator_command_map("vida --help")["families"]
        .as_array()
        .into_iter()
        .flatten()
    {
        for entry in family["entries"].as_array().into_iter().flatten() {
            let command = entry["command"].as_str().unwrap_or("unknown");
            let summary = entry["summary"].as_str().unwrap_or("");
            println!("  {command:<18} {summary}");
        }
    }
    println!();
    println!("Notes:");
    println!("  - root commands stay fail-closed");
    println!("  - runtime-family help paths are `vida taskflow help` and `vida docflow help`");
    println!(
        "  - TaskFlow remains execution authority; DocFlow remains documentation/readiness surface"
    );
}
