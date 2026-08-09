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

function Invoke-IndexRefresh {
    param(
        [string]$RegistryPath,
        [string]$Package = "docflow-markdown",
        [string]$FilesCsv = "crates/docflow-markdown/src/lib.rs"
    )
    $arguments = @("-NoProfile", "-File", $ScriptPath, "-RefreshIndex", "-IncludeWorkingTree", "-Json", "-Packages", $Package, "-Files", $FilesCsv, "-RegistryPath", $RegistryPath)
    $raw = & pwsh @arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Index refresh failed: $($raw -join ' ')" }
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

Add-Case "test_update_hook_binds_placeholder_values_as_arguments" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    Assert-True ($source.Contains(".Replace('{file}', '`$args[0]')")) "file placeholder is not bound through a positional argument"
    Assert-True ($source.Contains(".Replace('{package}', '`$args[1]')")) "package placeholder is not bound through a positional argument"
    Assert-True ($source.Contains('$command, [string]$FileRecord.path, [string]$FileRecord.package')) "placeholder values are not passed as separate process arguments"
    Assert-True (-not $source.Contains(".Replace('{file}', [string]`$FileRecord.path)")) "file placeholder is still interpolated as raw PowerShell source"
    Assert-True (-not $source.Contains(".Replace('{package}', [string]`$FileRecord.package)")) "package placeholder is still interpolated as raw PowerShell source"
}

Add-Case "committed_registry_uses_string_compact_references" {
    $registryPath = Join-Path (Get-Location) ".vida/evidence/mutest-audit/file-registry.json"
    $registry = Get-Content -LiteralPath $registryPath -Raw | ConvertFrom-Json

    foreach ($name in @("needs_tests", "needs_rerun", "needs_rescan")) {
        foreach ($reference in @($registry.$name)) {
            Assert-True ($reference -is [string]) "committed registry $name contains a non-string reference"
        }
    }

    foreach ($row in @($registry.files)) {
        foreach ($defect in @($row.defects)) {
            foreach ($name in @("evidence", "evidence_refs")) {
                if ($null -eq $defect.PSObject.Properties[$name]) { continue }
                foreach ($reference in @($defect.$name)) {
                    Assert-True ($reference -is [string]) "committed defect $name contains a non-string reference for $($row.path)"
                }
            }
        }
    }
}

Add-Case "per_file_loc_and_hash_refresh_contract" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @("Get-FileLineMetrics", "loc_total", "loc_hash", "loc_policy", "RefreshIndex", "content_hash_changed", "mutation_workers_started = `$false")) {
        Assert-True ($source.Contains($needle)) "missing LOC/index-refresh contract: $needle"
    }
    Assert-True ($source.Contains("[System.IO.StreamReader]::new")) "LOC metrics do not stream source files"
    Assert-True (-not $source.Contains("[System.IO.File]::ReadAllLines")) "LOC metrics read entire source files into memory"
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-loc-contract-" + [guid]::NewGuid().ToString("N") + ".json")
    $first = Invoke-IndexRefresh -RegistryPath $registry
    Assert-True ($first.status -eq "index_refreshed") "index refresh did not return index_refreshed"
    Assert-True (-not $first.mutation_workers_started) "index refresh started mutation workers"
    $index = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $row = @($index.files | Where-Object { $_.path -eq "crates/docflow-markdown/src/lib.rs" })[0]
    Assert-True ($null -ne $row) "LOC refresh did not create a file row"
    Assert-True ([int]$row.loc -gt 0) "LOC is missing or zero for a non-empty source file"
    Assert-True ([int]$row.loc_total -ge [int]$row.loc) "physical LOC is smaller than non-empty LOC"
    Assert-True ([string]$row.loc_hash -eq [string]$row.content_hash_sha256) "LOC hash is not tied to content hash"
    Assert-True ($index.loc_policy.loc -eq "non_empty_source_lines") "LOC policy is not persisted"

    $seed = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $seed.files[0].hash = ("0" * 64)
    $seed.files[0].content_hash_sha256 = ("0" * 64)
    $seed.files[0].status = "completed"
    $seed.files[0].needs_rerun = $false
    $seed.files[0].needs_tests = $false
    $seed.files[0].needs_rescan = $false
    $seed.run_id = "seed-run"
    $seed.last_wave_id = "seed-wave"
    $seed.waves = @([ordered]@{ wave_id = "seed-wave"; status = "completed" })
    $seed | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $registry -Encoding UTF8
    $second = Invoke-IndexRefresh -RegistryPath $registry
    Assert-True ($second.status -eq "index_refreshed") "second index refresh did not complete"
    $changed = @((Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json).files | Where-Object { $_.path -eq "crates/docflow-markdown/src/lib.rs" })[0]
    Assert-True ($changed.status -eq "queued") "hash drift did not queue the file"
    Assert-True ($changed.queue_reason -eq "content_hash_changed") "hash drift queue reason is incorrect"
    Assert-True ([bool]$changed.needs_rerun) "hash drift did not set needs_rerun"
    $preserved = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    Assert-True ($preserved.run_id -eq "seed-run") "index refresh dropped top-level run_id"
    Assert-True ($preserved.last_wave_id -eq "seed-wave") "index refresh dropped top-level last_wave_id"
    Assert-True (@($preserved.waves).Count -eq 1) "index refresh dropped wave summaries"
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

Add-Case "thin_index_compacts_duplicate_defects" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @("Get-ThinDefects", "Get-DefectKey", "active_per_file", "dedupe_key", "evidence_refs_max", 'history_path = "defects.jsonl"')) {
        Assert-True ($source.Contains($needle)) "missing thin-index contract: $needle"
    }
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-thin-index-" + [guid]::NewGuid().ToString("N") + ".json")
    $path = "crates/docflow-markdown/src/lib.rs"
    $initial = Invoke-IndexRefresh -RegistryPath $registry -Package "docflow-markdown" -FilesCsv $path
    $seed = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $seedRow = @($seed.files | Where-Object { $_.path -eq $path })[0]
    $wave = "wave-mutest-20260809-000001-aaaa"
    $defect = [ordered]@{
        type = "mutation_compiler_error"; blocker_code = "mutest_driver_target_metadata"; blocker_family = "mutest_tool"
        blocker_reason = "target metadata mismatch"; path = $path; package = "docflow-markdown"; wave_id = $wave
        observed_hash = $seedRow.hash; evidence_refs = @("a", "b", "c", "d", "e"); recommendation = "rerun"
    }
    $duplicate = [ordered]@{}
    foreach ($key in $defect.Keys) { $duplicate[$key] = $defect[$key] }
    $seedRow.status = "blocked"; $seedRow.last_wave_id = $wave; $seedRow.defects = @($defect, $duplicate); $seedRow.needs_rerun = $true
    $seed.last_wave_id = $wave; $seed.waves = @([ordered]@{ wave_id = $wave; status = "blocked" })
    $seed | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $registry -Encoding UTF8
    $result = Invoke-IndexRefresh -RegistryPath $registry -Package "docflow-markdown" -FilesCsv $path
    Assert-True ($result.status -eq "index_refreshed") "thin-index refresh failed"
    $compact = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $row = @($compact.files | Where-Object { $_.path -eq $path })[0]
    Assert-True (@($row.defects).Count -eq 1) "duplicate defect summaries were not compacted"
    Assert-True ([string]$row.defects[0].defect_key -like "mut-*") "deterministic defect key missing"
    Assert-True (@($row.defects[0].evidence_refs).Count -le 4) "evidence references exceeded thin-index limit"
    Assert-True ($compact.index_compaction.mode -eq "active_per_file") "active per-file index policy missing"
    Assert-True ($compact.index_compaction.clear_on_hash_change) "hash-change clearing policy missing"
    Assert-True ($compact.index_compaction.clear_on_success) "success clearing policy missing"
    Assert-True ($compact.summary.active_defects -eq 1) "active defect summary is not synchronized"
    $raw = Get-Content -LiteralPath $registry -Raw
    Assert-True (-not $raw.Contains('"SyncRoot"')) "dictionary adapter properties leaked into the index"
    Assert-True (-not $raw.Contains('"Values"')) "dictionary Values adapter leaked into the index"
    Assert-True ((Get-Item -LiteralPath $registry).Length -lt 20000) "thin index exceeded compact fixture size"
}

Add-Case "active_defect_key_is_stable_across_waves" {
    $path = "crates/docflow-markdown/src/lib.rs"
    $keys = New-Object System.Collections.Generic.List[string]
    foreach ($wave in @("wave-mutest-20260809-010101-a", "wave-mutest-20260809-010102-b")) {
        $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-key-wave-" + [guid]::NewGuid().ToString("N") + ".json")
        [void](Invoke-IndexRefresh -RegistryPath $registry -Package "docflow-markdown" -FilesCsv $path)
        $seed = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
        $row = @($seed.files | Where-Object { $_.path -eq $path })[0]
        $row.status = "blocked"; $row.last_wave_id = $wave; $row.needs_rerun = $true
        $row.defects = @([ordered]@{ type = "survived_mutants"; blocker_code = ""; path = $path; package = "docflow-markdown"; wave_id = $wave; observed_hash = $row.hash; mutation_identity = "line:42|operator:replace"; recommendation = "rerun" })
        $seed | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $registry -Encoding UTF8
        [void](Invoke-IndexRefresh -RegistryPath $registry -Package "docflow-markdown" -FilesCsv $path)
        $compact = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
        $key = [string](@($compact.files | Where-Object { $_.path -eq $path })[0].defects[0].defect_key)
        [void]$keys.Add($key)
    }
    Assert-True ($keys.Count -eq 2 -and $keys[0] -eq $keys[1]) "defect key changed when only wave_id changed"
}

Add-Case "package_partial_scan_preserves_unselected_rows" {
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-package-partial-" + [guid]::NewGuid().ToString("N") + ".json")
    $path = "crates/docflow-markdown/src/lib.rs"
    [void](Invoke-IndexRefresh -RegistryPath $registry -Package "docflow-markdown" -FilesCsv $path)
    $seed = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $extra = [ordered]@{ path = "crates/operator-output/src/diagnostics.rs"; package = "operator-output"; hash = ("1" * 64); content_hash_sha256 = ("1" * 64); status = "blocked"; needs_rerun = $true; defects = @([ordered]@{ type = "mutation_timeout"; blocker_code = "seed"; path = "crates/operator-output/src/diagnostics.rs"; package = "operator-output"; mutation_identity = "file-level" }) }
    $seed.files = @($seed.files) + @([pscustomobject]$extra)
    $seed | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $registry -Encoding UTF8
    $plan = Invoke-Plan -RegistryPath $registry -Package "docflow-markdown"
    Assert-True ([int]$plan.file_scan.deleted_files -eq 0) "package-scoped plan classified an unselected row as deleted"
}

Add-Case "full_snapshot_classifies_absent_rows" {
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-full-snapshot-" + [guid]::NewGuid().ToString("N") + ".json")
    $seed = [ordered]@{ schema_version = 3; files = @([ordered]@{ path = "crates/docflow-markdown/src/removed-for-contract.rs"; package = "docflow-markdown"; hash = ("0" * 64); content_hash_sha256 = ("0" * 64); status = "completed"; needs_rerun = $false; needs_tests = $false; needs_rescan = $false; defects = @() }) }
    $seed | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $registry -Encoding UTF8
    $raw = & pwsh -NoProfile -File $ScriptPath -PlanOnly -IncludeWorkingTree -Json -RegistryPath $registry 2>&1
    Assert-True ($LASTEXITCODE -eq 0) "full snapshot plan failed: $($raw -join ' ')"
    $plan = ($raw -join [Environment]::NewLine | ConvertFrom-Json)
    Assert-True ([int]$plan.file_scan.deleted_files -eq 1) "full snapshot did not classify the absent row"
}

Add-Case "active_backlog_changes_only_selected_rows" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    foreach ($needle in @("Set-CurrentFileDefects", "Clear-CurrentFileDefects", "active_defects", "deleted_from_snapshot", "PartialSelection")) {
        Assert-True ($source.Contains($needle)) "missing active-backlog contract: $needle"
    }
    $registry = Join-Path (Join-Path (Get-Location) ".vida/tmp") ("mutest-active-backlog-" + [guid]::NewGuid().ToString("N") + ".json")
    $firstPath = "crates/operator-output/src/next_actions.rs"
    $secondPath = "crates/operator-output/src/diagnostics.rs"
    [void](Invoke-IndexRefresh -RegistryPath $registry -Package "operator-output" -FilesCsv "$firstPath,$secondPath")
    $seed = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $first = @($seed.files | Where-Object { $_.path -eq $firstPath })[0]
    $second = @($seed.files | Where-Object { $_.path -eq $secondPath })[0]
    $second.status = "blocked"; $second.needs_rerun = $true; $second.last_wave_id = "wave-seed"
    $second.defects = @([ordered]@{ type = "mutation_compiler_error"; blocker_code = "seed"; path = $secondPath; package = "operator-output"; wave_id = "wave-seed"; observed_hash = $second.hash; recommendation = "rerun" })
    $first.hash = ("0" * 64); $first.content_hash_sha256 = ("0" * 64); $first.status = "blocked"; $first.needs_rerun = $true
    $first.defects = @([ordered]@{ type = "mutation_timeout"; blocker_code = "old"; path = $firstPath; package = "operator-output"; wave_id = "wave-old"; observed_hash = ("0" * 64); recommendation = "rerun" })
    $seed | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $registry -Encoding UTF8
    [void](Invoke-IndexRefresh -RegistryPath $registry -Package "operator-output" -FilesCsv $firstPath)
    $result = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $changed = @($result.files | Where-Object { $_.path -eq $firstPath })[0]
    $untouched = @($result.files | Where-Object { $_.path -eq $secondPath })[0]
    Assert-True ($changed.status -eq "queued") "changed row was not queued"
    Assert-True (@($changed.defects).Count -eq 0) "changed row retained stale active defects"
    Assert-True ($untouched.status -eq "blocked") "partial refresh changed an untouched row"
    Assert-True (@($untouched.defects).Count -eq 1) "partial refresh removed an untouched defect"
}

Add-Case "defect_history_is_local_only" {
    [void](& git check-ignore --quiet -- ".vida/evidence/mutest-audit/defects.jsonl")
    Assert-True ($LASTEXITCODE -eq 0) "raw defect history is not ignored by Git"
    $tracked = @(git ls-files -- ".vida/evidence/mutest-audit/defects.jsonl")
    Assert-True ($tracked.Count -eq 0) "raw defect history must not be tracked"
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

Add-Case "default_launcher_does_not_trust_project_executables" {
    $source = Get-Content -LiteralPath $ScriptPath -Raw
    $resolverStart = $source.IndexOf("function Resolve-MutestCargoPath")
    $resolverEnd = $source.IndexOf("function Get-CargoTargetArguments", $resolverStart)
    Assert-True ($resolverStart -ge 0 -and $resolverEnd -gt $resolverStart) "mutest path resolver boundary is missing"
    $resolver = $source.Substring($resolverStart, $resolverEnd - $resolverStart)
    Assert-True (-not $resolver.Contains(".vida\tmp\mutest-rs-pathfix-bin")) "resolver trusts a project-controlled executable"
    Assert-True (-not $resolver.Contains("mutest-rs\target")) "resolver trusts a sibling build executable"
    Assert-True (-not $resolver.Contains('source = "auto"')) "resolver retains automatic direct-executable selection"

    $plan = Invoke-Plan
    Assert-True ([string]::IsNullOrWhiteSpace([string]$plan.config.mutest_cargo_path)) "default plan selected a direct mutest executable"
    Assert-True ($plan.config.mutest_cargo_path_source -eq "cargo-subcommand") "default plan did not use Cargo subcommand resolution"
}

Add-Case "launcher_environment_and_target_contract" {
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
