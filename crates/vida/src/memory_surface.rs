use std::process::ExitCode;

use crate::{print_surface_header, print_surface_line, state_store, MemoryArgs, StateStore};

pub(crate) async fn run_memory(args: MemoryArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;

    match StateStore::open_existing(state_dir).await {
        Ok(store) => match store.ensure_memory_governance_guard().await {
            Ok(()) => match store.active_instruction_root().await {
                Ok(root_artifact_id) => match store
                    .inspect_effective_instruction_bundle(&root_artifact_id)
                    .await
                {
                    Ok(bundle) => {
                        print_surface_header(render, "vida memory");
                        print_surface_line(
                            render,
                            "effective instruction bundle root",
                            &bundle.root_artifact_id,
                        );
                        print_surface_line(
                            render,
                            "mandatory chain",
                            &bundle.mandatory_chain_order.join(" -> "),
                        );
                        print_surface_line(
                            render,
                            "source version tuple",
                            &bundle.source_version_tuple.join(", "),
                        );
                        print_surface_line(render, "receipt", &bundle.receipt_id);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to resolve effective instruction bundle: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to read active instruction root: {error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("Failed to enforce memory governance guard: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}
