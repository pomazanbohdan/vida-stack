use std::path::PathBuf;

pub(crate) fn repo_runtime_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .expect("repo root should exist two levels above crates/vida")
}

pub(crate) fn block_on_state_store<T>(
    future: impl std::future::Future<Output = Result<T, crate::StateStoreError>>,
) -> Result<T, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future))
            .map_err(|error| error.to_string()),
        Err(_) => tokio::runtime::Runtime::new()
            .map_err(|error| format!("Failed to initialize Tokio runtime: {error}"))?
            .block_on(future)
            .map_err(|error| error.to_string()),
    }
}

pub(crate) fn print_json_pretty(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("json payload should render")
    );
}

#[cfg(test)]
mod tests {
    use crate::test_cli_support::cli;
    use crate::Cli;
    use clap::CommandFactory;
    use std::process::ExitCode;

    #[test]
    fn unknown_root_command_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(crate::run(cli(&["unknown"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn clap_help_lists_protocol() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();
        assert!(
            help.contains("protocol"),
            "protocol should be present in help"
        );
    }

    #[test]
    fn clap_help_lists_init_before_boot() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();
        let init_index = help.find("init").expect("init should be present in help");
        let boot_index = help.find("boot").expect("boot should be present in help");
        assert!(
            init_index < boot_index,
            "init should appear before boot in help"
        );
    }
}
