[CmdletBinding()]
param(
    [string]$ScriptPath = (Join-Path (Split-Path -Parent $PSScriptRoot) "vida-mutest-audit.ps1")
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$cases = New-Object System.Collections.Generic.List[object]
$failures = New-Object System.Collections.Generic.List[object]
$ContractRegistryPath = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-contract-tests-" + $PID + ".json")

function Add-Case {
    param([string]$Name, [scriptblock]$Body)
    $started = [DateTime]::UtcNow
    try {
        & $Body
        $record = [ordered]@{ name = $Name; status = "pass"; duration_ms = [Math]::Round(([DateTime]::UtcNow - $started).TotalMilliseconds, 2) }
        [void]$cases.Add($record)
        $record | ConvertTo-Json -Compress
    } catch {
        $record = [ordered]@{ name = $Name; status = "fail"; error = $_.Exception.Message; duration_ms = [Math]::Round(([DateTime]::UtcNow - $started).TotalMilliseconds, 2) }
        [void]$cases.Add($record); [void]$failures.Add($record)
        $record | ConvertTo-Json -Compress
    }
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Invoke-Plan {
    param(
        [string]$RegistryPath = $ContractRegistryPath,
        [switch]$FullRescan,
        [string]$MutestCargoPath = "",
        [string]$MutestNativeLibPath = "",
        [string]$Package = "vida",
        [string]$FilesCsv = ""
    )
    $arguments = @("-NoProfile", "-File", $ScriptPath, "-PlanOnly", "-IncludeWorkingTree", "-Json", "-Packages", $Package)
    if (-not [string]::IsNullOrWhiteSpace($RegistryPath)) { $arguments += @("-RegistryPath", $RegistryPath) }
    if ($FullRescan) { $arguments += "-FullRescan" }
    if (-not [string]::IsNullOrWhiteSpace($MutestCargoPath)) { $arguments += @("-MutestCargoPath", $MutestCargoPath) }
    if (-not [string]::IsNullOrWhiteSpace($MutestNativeLibPath)) { $arguments += @("-MutestNativeLibPath", $MutestNativeLibPath) }
    if (-not [string]::IsNullOrWhiteSpace($FilesCsv)) { $arguments += @("-Files", $FilesCsv) }
    $raw = & pwsh @arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "PlanOnly failed: $($raw -join ' ')" }
    return ($raw -join "`n" | ConvertFrom-Json)
}

Add-Case "powershell_parser" {
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path -LiteralPath $ScriptPath), [ref]$tokens, [ref]$errors) | Out-Null
    Assert-True ($errors.Count -eq 0) "parser errors: $($errors | ForEach-Object Message -join '; ')"
}

Add-Case "default_requested_workers_is_five" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source -match '\[int\]\$MaxWorkers\s*=\s*5') "MaxWorkers default is not five"
}

Add-Case "explicit_json_switch_is_accepted" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source -match '\[switch\]\$Json') "Json switch is not declared"
    $plan = Invoke-Plan
    Assert-True ($plan.schema_version -eq 1) "explicit Json plan did not return the machine manifest"
}

Add-Case "plan_only_manifest_and_resource_cap" {
    $plan = Invoke-Plan
    Assert-True ($plan.resources.requested_workers -eq 5) "requested worker count is not five"
    Assert-True ($plan.resources.effective_workers -ge 1 -and $plan.resources.effective_workers -le 5) "effective worker cap is invalid"
    Assert-True ($plan.commands[0].args -contains "--target-dir") "target-dir missing from manifest"
    Assert-True ($plan.commands[0].args -contains "--metadata-out-root-dir") "metadata root missing from manifest"
    Assert-True ($plan.commands[0].args -contains "--parallel-mutants") "parallel mutant flag missing"
    Assert-True ($plan.commands[0].args -contains "--filter-mutations") "per-file mutation filter missing"
    Assert-True ([string]$plan.commands[0].path -match '^crates/.+\.rs$') "per-file command path missing"
    $filterIndex = [Array]::IndexOf([object[]]$plan.commands[0].args, "--filter-mutations")
    Assert-True ($filterIndex -ge 0 -and [string]$plan.commands[0].args[$filterIndex + 1] -eq "file:$($plan.commands[0].path)") "per-file mutation filter path mismatch"
    Assert-True ($plan.commands[0].fallback_batch_size -eq 1) "batch-size-one fallback missing"
    $runEvidence = Join-Path (Join-Path (Get-Location) ".vida/evidence/mutest-audit") $plan.run_id
    Assert-True (Test-Path -LiteralPath (Join-Path $runEvidence "manifest.json")) "PlanOnly manifest was not persisted"
    Assert-True (Test-Path -LiteralPath (Join-Path $runEvidence "parallel-report.json")) "PlanOnly report was not persisted"
    Assert-True (Test-Path -LiteralPath (Join-Path $runEvidence "parallel-report.md")) "PlanOnly markdown report was not persisted"
    $report = Get-Content -LiteralPath (Join-Path $runEvidence "parallel-report.json") -Raw | ConvertFrom-Json
    Assert-True ($report.status -eq "planned") "PlanOnly report status is not planned"
}

Add-Case "command_manifest_is_stable" {
    $first = Invoke-Plan; $second = Invoke-Plan
    Assert-True ($first.command_hash -eq $second.command_hash) "command hash drifted between identical plans"
    Assert-True ($first.commands[0].command -eq $second.commands[0].command) "command text drifted between identical plans"
}

Add-Case "continuous_refill_and_report_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @("queue_refill", "worker_terminal_refill", "batch_retry_scheduled", "Start-MutestWorker", "Complete-MutestWorker", "events.jsonl", "checkpoint.json", "parallel-report.json")) {
        Assert-True ($source.Contains($needle)) "missing scheduler/report contract: $needle"
    }
    foreach ($needle in @("generated", "evaluated", "killed", "survived", "no_coverage", "compile_error", "timeout", "flaky")) {
        Assert-True ($source.Contains($needle)) "missing metric: $needle"
    }
}

Add-Case "resume_rejects_drift" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source.Contains("Resume rejected")) "resume drift rejection is missing"
    Assert-True ($source.Contains("index_tree")) "index-tree provenance guard is missing"
    Assert-True ($source.Contains("command_hash")) "command hash guard is missing"
}

Add-Case "controlled_file_diff_registry_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @("[string[]]`$Files", "`$FullRescan", "`$Threshold", "ValidateRange(90, 100)", "-le `$Threshold", "file-registry.json", "needs_tests", "needs_rerun", "needs_rescan", "Invoke-TestUpdateHook", "Invoke-SynchronousRescan", "defect-remediation.json")) {
        Assert-True ($source.Contains($needle)) "missing controlled mutation contract: $needle"
    }
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-contract-" + [guid]::NewGuid().ToString("N") + ".json")
    $first = Invoke-Plan -RegistryPath $registry -FullRescan
    Assert-True ($first.file_scan.mode -eq "full_rescan") "FullRescan did not set full_rescan mode"
    Assert-True ($first.file_scan.candidate_files -ge 1) "diff scanner found no production Rust files"
    Assert-True (-not (Test-Path -LiteralPath $registry)) "PlanOnly mutated the canonical file registry"
    Assert-True ($first.wave.wave_id -like "wave-*") "PlanOnly wave id is missing"
    Assert-True ($first.index.row_count -eq $first.index.unique_path_count) "PlanOnly index preview contains duplicate paths"
    $second = Invoke-Plan -RegistryPath $registry
    Assert-True ($second.file_scan.mode -eq "diff") "default scan is not diff mode"
    Assert-True ($second.file_scan.resumed_files -ge 0) "diff scan resume count missing"
    $third = Invoke-Plan -RegistryPath $registry -FullRescan
    Assert-True ($third.file_scan.queued_files -eq $third.file_scan.candidate_files) "FullRescan did not queue every candidate file"
}

Add-Case "single_index_wave_orchestrator_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @(
        "schema_version = 3", "mutation_wave_orchestrator", "Get-UniqueFileRows", "Write-CanonicalRegistry",
        "last_wave_id", "wave_status", "wave_updated_at", "waves =", "one-file-workers",
        "--filter-mutations", "pending_files", "retry_files", "workers =", "registry_role", "summary =", "files_needs_tests"
    )) {
        Assert-True ($source.Contains($needle)) "missing wave-index contract: $needle"
    }
}

Add-Case "plan_only_preserves_canonical_index" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source.Contains('if ($PlanOnly)')) "PlanOnly branch is missing"
    Assert-True ($source.Contains("canonical registry is unchanged")) "PlanOnly immutability contract is missing"
    Assert-True ($source.Contains('Write-CanonicalRegistry -Registry $RegistryDocument')) "canonical registry write is not explicit"
}

Add-Case "custom_mutest_launcher_and_schema_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @(
        "MutestCargoPath", "MutestNativeLibPath", "Get-MutestLaunch", "Get-MutestEnvironment", "RUSTFLAGS",
        "mutations.json", "evaluation.json", "total_mutations_count", "detected_mutations_count", "undetected_mutations_count",
        "timed_out_mutations_count", "crashed_mutations_count"
    )) {
        Assert-True ($source.Contains($needle)) "missing custom mutest/schema contract: $needle"
    }
    $nativeRoot = (Get-Location).Path
    $plan = Invoke-Plan -MutestCargoPath $ScriptPath -MutestNativeLibPath $nativeRoot
    $expectedCargo = [System.IO.Path]::GetFullPath($ScriptPath)
    $expectedNative = [System.IO.Path]::GetFullPath($nativeRoot)
    Assert-True ($plan.config.mutest_cargo_path -eq $expectedCargo) "custom mutest path was not persisted in config"
    Assert-True ($plan.config.mutest_native_lib_path -eq $expectedNative) "custom native path was not persisted in config"
    Assert-True ($plan.commands[0].args[0] -eq "run") "custom cargo-mutest args lost the run subcommand"
    Assert-True ($plan.commands[0].command -match "run nightly-2026-07-18") "custom launcher did not use the pinned rustup nightly"
    Assert-True ($plan.commands[0].command.Contains($expectedCargo)) "custom launcher path is absent from the command manifest"
}

Add-Case "automatic_launcher_environment_and_target_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @(
        "Resolve-MutestCargoPath", "mutest_cargo_path_source", "Get-CargoTargetArguments",
        "--lib", "--bin", "workerTemp", "Get-MutestEnvironment -TempRoot", '$environment["TMP"]', '$environment["TEMP"]',
        "Get-ExecutionBlocker", "mutest_driver_target_metadata", "windows_linker_temp_path", "defect_groups"
    )) {
        Assert-True ($source.Contains($needle)) "missing automatic execution contract: $needle"
    }
}

Add-Case "csv_selector_contract" {
    $plan = Invoke-Plan -Package "docflow-markdown" -FilesCsv "crates/docflow-markdown/src/lib.rs"
    Assert-True ($plan.file_scan.candidate_files -eq 1) "CSV file selector did not produce one candidate"
    Assert-True ($plan.commands[0].args -contains "--lib") "CSV file selector did not select --lib"
    Assert-True ($plan.commands[0].args -contains "file:crates/docflow-markdown/src/lib.rs") "CSV file selector did not preserve the path"
}

Add-Case "defect_protocol_and_test_update_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source.Contains("docs/process/project-error-search-runtime-diagnostics-protocol.md")) "defect protocol reference missing"
    Assert-True ($source.Contains("docs/process/zombie-d-test-writing-protocol.md")) "test-writing protocol reference missing"
    Assert-True ($source.Contains("defects.jsonl")) "defect log path missing"
    Assert-True ($source.Contains("taskflow_state_restored = `$false")) "TaskFlow restore guard missing"
}

$summary = [ordered]@{ schema_version = 1; status = if ($failures.Count -eq 0) { "pass" } else { "fail" }; total = $cases.Count; passed = $cases.Count - $failures.Count; failed = $failures.Count; cases = $cases.ToArray() }
"SUMMARY " + ($summary | ConvertTo-Json -Depth 20 -Compress)
if ($failures.Count -gt 0) { exit 1 }
