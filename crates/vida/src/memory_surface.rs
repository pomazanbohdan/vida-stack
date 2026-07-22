use std::process::ExitCode;
use std::time::Duration;

use crate::{print_surface_header, print_surface_line, state_store, MemoryArgs, StateStore};

const MEMORY_SURFACE_LOCK_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn run_memory(args: MemoryArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;

    match tokio::time::timeout(
        MEMORY_SURFACE_LOCK_TIMEOUT,
        StateStore::open_existing_read_only(state_dir),
    )
    .await
    {
        Ok(Ok(store)) => match tokio::time::timeout(
            MEMORY_SURFACE_LOCK_TIMEOUT,
            store.ensure_memory_governance_guard(),
        )
        .await
        {
            Ok(guard_result) => match guard_result {
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
            Err(_) => {
                eprintln!(
                    "Failed to enforce memory governance guard: timed out while waiting for authoritative datastore lock"
                );
                ExitCode::from(1)
            }
        },
        Ok(Err(error)) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Failed to open authoritative state store: memory governance guard timed out while waiting for authoritative datastore lock"
            );
            ExitCode::from(1)
        }
    }
}
