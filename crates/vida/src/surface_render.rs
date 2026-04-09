use crate::RenderMode;
use serde::Serialize;

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

pub(crate) fn print_root_help() {
    println!("VIDA Binary Foundation");
    println!();
    println!("Usage:");
    println!("  vida <command>");
    println!("  vida taskflow <args...>");
    println!("  vida docflow <args...>");
    println!();
    println!("Root commands:");
    println!("  init      bootstrap framework carriers into the current project");
    println!(
        "  boot      initialize authoritative state and instruction/framework-memory surfaces"
    );
    println!("  orchestrator-init  render the compiled startup view for the orchestrator lane");
    println!(
        "  agent-init         render the bounded startup view or packet activation view for a worker/agent lane; this surface does not itself execute the packet"
    );
    println!("  protocol  resolve and render framework protocol/guide surfaces");
    println!(
        "  project-activator  inspect project activation posture and bounded onboarding next steps"
    );
    println!("  agent-feedback  record host-agent feedback and refresh local strategy state");
    println!("  task      task import/list/show/ready over the authoritative state store");
    println!("  memory    inspect the effective instruction bundle");
    println!("  status    inspect backend, state spine, and latest receipts");
    println!("  doctor    run bounded runtime integrity checks");
    println!("  consume   thin root alias to the TaskFlow consume family");
    println!("  lane      inspect or mutate canonical lane/takeover operator state");
    println!("  approval  reserved root operator surface; currently fail-closed");
    println!("  recovery  thin root alias to the TaskFlow recovery family");
    println!("  taskflow  delegate to the TaskFlow runtime family");
    println!("  docflow   delegate to the DocFlow runtime family");
    println!();
    println!("Notes:");
    println!("  - root commands stay fail-closed");
    println!("  - runtime-family help paths are `vida taskflow help` and `vida docflow help`");
    println!(
        "  - TaskFlow remains execution authority; DocFlow remains documentation/readiness surface"
    );
}
