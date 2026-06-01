param(
    [ValidateSet("script-check", "quick", "focused-nextest", "workspace-nextest", "runtime-smoke", "release-install", "target-dir-policy")]
    [string]$Mode = "quick",
    [string]$TestFilter = "",
    [int]$Jobs = 0,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
$Records = New-Object System.Collections.Generic.List[object]
$OriginalCargoTargetDir = $env:CARGO_TARGET_DIR

function Resolve-CargoTargetDirPolicy {
    $normalizedRoot = [System.IO.Path]::GetFullPath($RootDir)
    if (-not [string]::IsNullOrWhiteSpace($OriginalCargoTargetDir)) {
        return [pscustomobject]@{
            target_dir_policy = "caller_provided"
            effective_cargo_target_dir = [System.IO.Path]::GetFullPath($OriginalCargoTargetDir)
        }
    }

    $worktreeMarker = "{0}.vida{0}worktrees{0}" -f [System.IO.Path]::DirectorySeparatorChar
    $markerIndex = $normalizedRoot.IndexOf($worktreeMarker, [System.StringComparison]::OrdinalIgnoreCase)
    if ($markerIndex -ge 0) {
        $ownerRoot = $normalizedRoot.Substring(0, $markerIndex)
        return [pscustomobject]@{
            target_dir_policy = "repo_local_worktree_shared"
            effective_cargo_target_dir = Join-Path $ownerRoot ".vida\cargo-target"
        }
    }

    return [pscustomobject]@{
        target_dir_policy = "repo_local_default"
        effective_cargo_target_dir = Join-Path $normalizedRoot ".vida\cargo-target"
    }
}

$CargoTargetDirState = Resolve-CargoTargetDirPolicy
$env:CARGO_TARGET_DIR = $CargoTargetDirState.effective_cargo_target_dir
$DebugVidaPath = Join-Path $CargoTargetDirState.effective_cargo_target_dir "debug\vida.exe"

function Invoke-Timed {
    param(
        [string]$OperationId,
        [string[]]$Command
    )

    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $exitCode = 0
    $artifactRefs = @()
    $exe = $Command[0]
    $args = @()
    if ($Command.Length -gt 1) {
        $args = $Command[1..($Command.Length - 1)]
    }
    try {
        if ($Json) {
            $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
            New-Item -ItemType Directory -Force -Path $logDir | Out-Null
            $safeId = $OperationId -replace '[^A-Za-z0-9_.-]', '-'
            $logPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.log" -f $safeId, $started)
            $previousErrorActionPreference = $ErrorActionPreference
            $ErrorActionPreference = "Continue"
            try {
                & $exe @args *> $logPath
            } finally {
                $ErrorActionPreference = $previousErrorActionPreference
            }
            $artifactRefs = @($logPath)
        } else {
            & $exe @args
        }
        $exitCode = $LASTEXITCODE
        if ($null -eq $exitCode) {
            $exitCode = 0
        }
    } catch {
        $exitCode = 1
        throw
    } finally {
        $sw.Stop()
        $Records.Add([pscustomobject]@{
            operation_id = $OperationId
            command_or_surface = ($Command -join " ")
            cwd_or_context = $RootDir
            started_at = $started.ToString("o")
            duration_ms = [int64]$sw.ElapsedMilliseconds
            exit_status = $(if ($exitCode -eq 0) { "pass" } else { "fail" })
            classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
            target_dir_policy = $CargoTargetDirState.target_dir_policy
            effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
            artifact_refs = $artifactRefs
        })
    }
    if ($exitCode -ne 0) {
        exit $exitCode
    }
}

function New-NextestCommand {
    param(
        [string[]]$Args
    )

    $command = New-Object System.Collections.Generic.List[string]
    $command.Add("cargo")
    $command.Add("nextest")
    $command.Add("run")
    $command.Add("--locked")
    foreach ($arg in $Args) {
        $command.Add($arg)
    }
    if ($Jobs -gt 0) {
        $command.Add("-j")
        $command.Add([string]$Jobs)
    }
    return $command.ToArray()
}

Push-Location $RootDir
try {
    if ($Mode -eq "target-dir-policy") {
        $Records.Add([pscustomobject]@{
            operation_id = "target-dir-policy"
            command_or_surface = "scripts/vida-dev-gate.ps1 -Mode target-dir-policy"
            cwd_or_context = $RootDir
            started_at = (Get-Date).ToString("o")
            duration_ms = 0
            exit_status = "pass"
            classification = "fast"
            target_dir_policy = $CargoTargetDirState.target_dir_policy
            effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
            artifact_refs = @()
        })
    } elseif ($Mode -eq "script-check") {
        Invoke-Timed "git-diff-check" @("git", "diff", "--check")
        Invoke-Timed "powershell-dev-gate-parse" @(
            "pwsh",
            "-NoLogo",
            "-NoProfile",
            "-Command",
            '$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path "scripts/vida-dev-gate.ps1"), [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }'
        )
    } elseif ($Mode -eq "quick") {
        Invoke-Timed "cargo-fmt-check" @("cargo", "fmt", "-p", "vida", "--", "--check")
        Invoke-Timed "cargo-check-vida" @("cargo", "check", "--locked", "-p", "vida")
    } elseif ($Mode -eq "focused-nextest") {
        if ($TestFilter.Trim().Length -eq 0) {
            Write-Error "-Mode focused-nextest requires -TestFilter <filter>."
            exit 2
        }
        if ($TestFilter.Trim().Length -gt 0) {
            Invoke-Timed "nextest-focused" (New-NextestCommand @("-p", "vida", "--profile", "default", $TestFilter))
        }
    } elseif ($Mode -eq "workspace-nextest") {
        Invoke-Timed "nextest-workspace" (New-NextestCommand @("--workspace", "--profile", "ci"))
    } elseif ($Mode -eq "runtime-smoke") {
        Invoke-Timed "cargo-build-debug" @("cargo", "build", "-p", "vida")
        Invoke-Timed "debug-vida-status" @($DebugVidaPath, "status", "--json")
    } elseif ($Mode -eq "release-install") {
        Invoke-Timed "vida-release-install" @("vida", "release", "install", "--json")
        Invoke-Timed "installed-vida-status" @("vida", "status", "--json")
    }
} finally {
    Pop-Location
    if ($Json) {
        $Records | ConvertTo-Json -Depth 6
    } else {
        foreach ($record in $Records) {
            Write-Host ("[{0}] {1} {2}ms" -f $record.exit_status, $record.operation_id, $record.duration_ms)
        }
    }
    $env:CARGO_TARGET_DIR = $OriginalCargoTargetDir
}
