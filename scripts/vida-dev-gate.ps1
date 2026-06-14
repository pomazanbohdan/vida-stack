param(
    [ValidateSet("script-check", "quick", "focused-nextest", "package-nextest", "workspace-nextest", "doc-test", "build-debug", "runtime-smoke", "release-package", "release-install", "target-dir-policy")]
    [string]$Mode = "quick",
    [string]$TestFilter = "",
    [int]$Jobs = 0,
    [switch]$Json,
    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
$Records = New-Object System.Collections.Generic.List[object]
$OriginalCargoTargetDir = $env:CARGO_TARGET_DIR

function Show-Help {
    @"
Usage:
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode <mode> [-Json] [-Jobs <n>] [-TestFilter <filter>]

Modes:
  script-check      No-Cargo proof for diffs, runtime boundaries, and script syntax.
  quick             Debug source proof: git diff check, cargo fmt, cargo check.
  focused-nextest   Focused vida package test proof; requires -TestFilter.
  package-nextest   Full vida package test proof with the default nextest profile.
  workspace-nextest Workspace nextest proof with the CI profile.
  doc-test          Workspace Rust doc tests.
  build-debug       Debug build of supported runtime entrypoints.
  runtime-smoke     Build debug vida and run status from the effective target dir.
  release-package   Build release archives with scripts/build-release.sh.
  release-install   Installed launcher proof through vida release install.
  target-dir-policy Print the effective Cargo target directory policy.

Notes:
  Cargo modes set CARGO_TARGET_DIR unless the caller already provided it.
  release-package honors VIDA_RELEASE_SKIP_BUILD=1 and VIDA_RELEASE_BIN_DIR=<dir> for packaging already-built release binaries.
  JSON mode records operation timing and log artifact paths under .vida/data/state/command-timing.
"@
}

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
$ReleaseVidaPath = Join-Path $CargoTargetDirState.effective_cargo_target_dir "release\vida.exe"

function Resolve-CommandPath {
    param(
        [string]$Name,
        [string[]]$Candidates = @()
    )

    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return $candidate
        }
    }

    return $Name
}

$GitPath = Resolve-CommandPath "git" @("C:\Program Files\Git\cmd\git.exe")
$PwshPath = Resolve-CommandPath "pwsh" @(
    "C:\Program Files\PowerShell\7\pwsh.exe",
    "$env:ProgramFiles\PowerShell\7\pwsh.exe"
)
$BashPath = Resolve-CommandPath "bash" @(
    "C:\Program Files\Git\bin\bash.exe",
    "$env:ProgramFiles\Git\bin\bash.exe"
)

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

function Add-SkippedRecord {
    param(
        [string]$OperationId,
        [string]$Reason
    )

    $Records.Add([pscustomobject]@{
        operation_id = $OperationId
        command_or_surface = $Reason
        cwd_or_context = $RootDir
        started_at = (Get-Date).ToString("o")
        duration_ms = 0
        exit_status = "skipped"
        classification = "fast"
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_refs = @()
    })
}

function Invoke-DiffWhitespaceCheck {
    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $artifactRefs = @()
    $violations = New-Object System.Collections.Generic.List[string]

    foreach ($diffMode in @(@(), @("--cached"))) {
        $currentFile = ""
        $newLine = 0
        $diffOutput = & $GitPath diff @diffMode --unified=0 --no-ext-diff --
        foreach ($line in $diffOutput) {
            if ($line.StartsWith("+++ b/")) {
                $currentFile = $line.Substring(6)
                continue
            }
            if ($line -match '^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@') {
                $newLine = [int]$Matches[1] - 1
                continue
            }
            if ($line.StartsWith("+") -and -not $line.StartsWith("+++ ")) {
                $newLine++
                $content = $line.Substring(1) -replace "`r$", ""
                if ($content -match '[ \t]+$') {
                    [void]$violations.Add(("{0}:{1}: trailing whitespace." -f $currentFile, $newLine))
                }
                if ($content -match '^ +\t') {
                    [void]$violations.Add(("{0}:{1}: space before tab in indent." -f $currentFile, $newLine))
                }
            }
        }
    }

    $sw.Stop()
    if ($Json -and $violations.Count -gt 0) {
        $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
        New-Item -ItemType Directory -Force -Path $logDir | Out-Null
        $logPath = Join-Path $logDir ("git-diff-check-{0:yyyyMMddHHmmssfff}.log" -f $started)
        $violations | Set-Content -Path $logPath -Encoding utf8
        $artifactRefs = @($logPath)
    } elseif (-not $Json -and $violations.Count -gt 0) {
        $violations | Write-Error
    }

    $Records.Add([pscustomobject]@{
        operation_id = "git-diff-check"
        command_or_surface = "repo-local diff whitespace check"
        cwd_or_context = $RootDir
        started_at = $started.ToString("o")
        duration_ms = [int64]$sw.ElapsedMilliseconds
        exit_status = $(if ($violations.Count -eq 0) { "pass" } else { "fail" })
        classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_refs = $artifactRefs
    })

    if ($violations.Count -gt 0) {
        exit 2
    }
}

function Invoke-RootReadmeOnlyCheck {
    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $artifactRefs = @()
    $violations = New-Object System.Collections.Generic.List[string]

    $readmes = Get-ChildItem -LiteralPath $RootDir -Filter "README.md" -Recurse -Force -File |
        Where-Object {
            $relative = [System.IO.Path]::GetRelativePath($RootDir, $_.FullName).Replace("\", "/")
            $relative -ne "README.md" -and
                -not $relative.StartsWith(".git/") -and
                -not $relative.StartsWith("target/") -and
                -not $relative.StartsWith("vendor/") -and
                -not $relative.StartsWith(".vida/cargo-target/")
        }
    foreach ($readme in $readmes) {
        [void]$violations.Add(("{0}: nested README.md is not allowed; use index.md or a semantic document name." -f [System.IO.Path]::GetRelativePath($RootDir, $readme.FullName).Replace("\", "/")))
    }

    $sw.Stop()
    if ($Json -and $violations.Count -gt 0) {
        $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
        New-Item -ItemType Directory -Force -Path $logDir | Out-Null
        $logPath = Join-Path $logDir ("root-readme-only-check-{0:yyyyMMddHHmmssfff}.log" -f $started)
        $violations | Set-Content -Path $logPath -Encoding utf8
        $artifactRefs = @($logPath)
    } elseif (-not $Json -and $violations.Count -gt 0) {
        $violations | Write-Error
    }

    $Records.Add([pscustomobject]@{
        operation_id = "root-readme-only-check"
        command_or_surface = "repo-local README.md placement invariant"
        cwd_or_context = $RootDir
        started_at = $started.ToString("o")
        duration_ms = [int64]$sw.ElapsedMilliseconds
        exit_status = $(if ($violations.Count -eq 0) { "pass" } else { "fail" })
        classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_refs = $artifactRefs
    })

    if ($violations.Count -gt 0) {
        exit 2
    }
}

function Test-CommandExists {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-ChangedBashScripts {
    $paths = New-Object System.Collections.Generic.SortedSet[string]
    foreach ($diffMode in @(@(), @("--cached"))) {
        $changed = & $GitPath diff @diffMode --name-only -- "scripts/*.sh" "install/*.sh"
        foreach ($path in $changed) {
            if (-not [string]::IsNullOrWhiteSpace($path)) {
                [void]$paths.Add($path)
            }
        }
    }
    return [string[]]@($paths)
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

if ($Help) {
    Show-Help
    exit 0
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
        Invoke-DiffWhitespaceCheck
        Invoke-RootReadmeOnlyCheck
        Invoke-Timed "powershell-dev-gate-parse" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-Command",
            '$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path "scripts/vida-dev-gate.ps1"), [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }'
        )
        Invoke-Timed "powershell-evaluation-log-linter-parse" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-Command",
            '$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path "scripts/check-agent-evaluation-log.ps1"), [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }'
        )
        Invoke-Timed "agent-evaluation-log-fixture-lint" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "scripts/check-agent-evaluation-log.ps1",
            "-Path",
            "tests/fixtures/agent-evaluation-log/pass.md"
        )
        Invoke-Timed "runtime-boundary-lint" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "scripts/check-runtime-boundaries.ps1",
            "-Json"
        )
        [string[]]$changedBashScripts = @(Get-ChangedBashScripts)
        if ($changedBashScripts.Count -eq 0) {
            Add-SkippedRecord "bash-script-parse" "no changed Bash scripts"
        } elseif (Test-CommandExists $BashPath) {
            if ($changedBashScripts.Count -eq 1) {
                Invoke-Timed "bash-script-parse" @($BashPath, "-n", $changedBashScripts[0])
            } else {
                Invoke-Timed "bash-script-parse" (@($BashPath, "-n", "-c", 'for f in "$@"; do source "$f"; done', "_") + $changedBashScripts)
            }
        } else {
            Add-SkippedRecord "bash-script-parse" "bash not found; skipped Bash script syntax checks"
        }
    } elseif ($Mode -eq "quick") {
        Invoke-Timed "git-diff-check" @($GitPath, "diff", "--check")
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
    } elseif ($Mode -eq "package-nextest") {
        Invoke-Timed "nextest-package-vida" (New-NextestCommand @("-p", "vida", "--profile", "default"))
    } elseif ($Mode -eq "workspace-nextest") {
        Invoke-Timed "nextest-workspace" (New-NextestCommand @("--workspace", "--profile", "ci"))
    } elseif ($Mode -eq "doc-test") {
        Invoke-Timed "cargo-doc-tests" @("cargo", "test", "--workspace", "--doc", "--locked")
    } elseif ($Mode -eq "build-debug") {
        Invoke-Timed "cargo-build-debug-entrypoints" @("cargo", "build", "--locked", "-p", "vida", "-p", "taskflow-cli", "-p", "docflow-cli", "-p", "vida-pi-agent")
    } elseif ($Mode -eq "runtime-smoke") {
        Invoke-Timed "cargo-build-debug" @("cargo", "build", "--locked", "-p", "vida")
        Invoke-Timed "debug-vida-status" @($DebugVidaPath, "status", "--json")
    } elseif ($Mode -eq "release-package") {
        if (-not (Test-CommandExists $BashPath)) {
            Write-Error "-Mode release-package requires bash."
            exit 2
        }
        Invoke-Timed "release-package" @($BashPath, "scripts/build-release.sh")
    } elseif ($Mode -eq "release-install") {
        Invoke-Timed "cargo-build-release-vida" @("cargo", "build", "--locked", "-p", "vida", "--release")
        Invoke-Timed "vida-release-install" @($ReleaseVidaPath, "release", "install", "--skip-build", "--source-binary", $ReleaseVidaPath, "--json")
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
