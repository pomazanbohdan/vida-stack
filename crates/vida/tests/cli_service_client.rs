use std::process::Command;

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn run_json(args: &[&str]) -> serde_json::Value {
    let output = vida()
        .args(args)
        .output()
        .expect("vida command should execute");
    assert!(
        output.status.success(),
        "command {args:?} should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "command {args:?} should emit JSON ({error}): stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn cli_service_first_families_emit_vida_client_operations() {
    let cases = [
        (
            &["service", "status", "--json"][..],
            "service",
            "vida.service.status",
        ),
        (
            &["service", "endpoints", "--json"][..],
            "service",
            "vida.service.endpoint.status",
        ),
        (
            &["service", "capabilities", "--json"][..],
            "service",
            "vida.service.capabilities",
        ),
        (
            &["service", "lifecycle-plan", "--json"][..],
            "service",
            "vida.service.lifecycle.plan",
        ),
        (
            &["service", "lifecycle-status", "--json"][..],
            "service",
            "vida.service.lifecycle.status",
        ),
        (
            &["service", "events", "--json"][..],
            "service",
            "vida.events.since",
        ),
        (
            &["project", "list", "--json"][..],
            "project",
            "vida.project.registry.list",
        ),
        (
            &["project", "resolve", "--project", "vida-stack", "--json"][..],
            "project",
            "vida.project.resolve",
        ),
        (
            &["project", "status", "--project", "vida-stack", "--json"][..],
            "project",
            "vida.project.status",
        ),
        (
            &["wizard", "inspect", "--json"][..],
            "wizard",
            "vida.wizard.schema.get",
        ),
        (
            &["wizard", "draft", "--json"][..],
            "wizard",
            "vida.wizard.session.start",
        ),
        (
            &["wizard", "validate", "--json"][..],
            "wizard",
            "vida.wizard.session.validate",
        ),
        (
            &["wizard", "diff", "--json"][..],
            "wizard",
            "vida.wizard.session.diff",
        ),
        (&["job", "status", "--json"][..], "job", "vida.jobs.get"),
        (
            &["receipt", "get", "--json"][..],
            "receipt",
            "vida.receipts.get",
        ),
    ];

    for (args, family, operation) in cases {
        let payload = run_json(args);
        assert_eq!(payload["family"], family);
        assert_eq!(payload["operation"], operation);
        assert_eq!(payload["status"], "pass");
    }
}

#[test]
fn cli_taskflow_and_docflow_remain_direct_proxy_families() {
    let taskflow = vida()
        .args(["taskflow", "help"])
        .output()
        .expect("taskflow help should execute");
    assert!(
        taskflow.status.success(),
        "taskflow help should remain direct: stderr={}",
        String::from_utf8_lossy(&taskflow.stderr)
    );
    let taskflow_stdout = String::from_utf8_lossy(&taskflow.stdout);
    assert!(
        taskflow_stdout.contains("TaskFlow"),
        "taskflow help should render TaskFlow output, got {taskflow_stdout}"
    );

    let docflow = vida()
        .args(["docflow", "help"])
        .output()
        .expect("docflow help should execute");
    assert!(
        docflow.status.success(),
        "docflow help should remain direct: stderr={}",
        String::from_utf8_lossy(&docflow.stderr)
    );
    let docflow_stdout = String::from_utf8_lossy(&docflow.stdout);
    assert!(
        docflow_stdout.contains("DocFlow") || docflow_stdout.contains("docflow"),
        "docflow help should render DocFlow output, got {docflow_stdout}"
    );
}
