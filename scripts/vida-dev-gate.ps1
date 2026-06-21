param(
    [ValidateSet("script-check", "quick", "focused-nextest", "package-nextest", "workspace-nextest", "vida-bin-shards", "doc-test", "build-debug", "runtime-smoke", "release-package", "release-install", "target-dir-policy", "cargo-env-check")]
    [string]$Mode = "quick",
    [string]$TestFilter = "",
    [string]$ReleaseVersion = "",
    [string]$ReleaseBinDir = "",
    [string]$ReleaseSuffix = "",
    [int]$Jobs = 0,
    [switch]$SkipBuild,
    [switch]$Windows,
    [switch]$Json,
    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
$Records = New-Object System.Collections.Generic.List[object]
$OriginalCargoTargetDir = $env:CARGO_TARGET_DIR
. (Join-Path $PSScriptRoot "vida-windows-env.ps1")

function Test-IsWindowsHost {
    return Test-VidaWindowsHost
}

function Set-EnvIfMissing {
    param(
        [string]$Name,
        [string]$Value
    )

    if ([string]::IsNullOrWhiteSpace((Get-Item -Path "Env:$Name" -ErrorAction SilentlyContinue).Value) -and
        -not [string]::IsNullOrWhiteSpace($Value)) {
        Set-Item -Path "Env:$Name" -Value $Value
    }
}

function Add-PathEntries {
    param([string[]]$Entries)

    $separator = [System.IO.Path]::PathSeparator
    $existing = New-Object System.Collections.Generic.HashSet[string]([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in (($env:Path -split [regex]::Escape([string]$separator)) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })) {
        [void]$existing.Add(([System.IO.Path]::GetFullPath($entry.Trim())))
    }

    $toPrepend = New-Object System.Collections.Generic.List[string]
    foreach ($entry in $Entries) {
        if ([string]::IsNullOrWhiteSpace($entry) -or -not (Test-Path -LiteralPath $entry)) {
            continue
        }
        $fullPath = [System.IO.Path]::GetFullPath($entry)
        if ($existing.Add($fullPath)) {
            $toPrepend.Add($fullPath)
        }
    }

    if ($toPrepend.Count -gt 0) {
        $env:Path = (($toPrepend.ToArray() + @($env:Path)) -join [string]$separator)
    }
}

function Initialize-WindowsHostEnvironment {
    Initialize-VidaWindowsEnvironment -NormalizeBuildTemp
}

function Import-VisualStudioBuildEnvironment {
    Import-VidaMsvcEnvironment
}

function Test-ModeNeedsWindowsBuildEnvironment {
    param([string]$ModeName)

    return $ModeName -in @(
        "quick",
        "focused-nextest",
        "package-nextest",
        "workspace-nextest",
        "vida-bin-shards",
        "doc-test",
        "build-debug",
        "runtime-smoke",
        "release-package",
        "release-install",
        "cargo-env-check"
    )
}

function Resolve-InstalledVidaPath {
    $installedVidaPath = Join-Path $env:LOCALAPPDATA "vida-stack\current\bin\vida.exe"
    if (Test-Path -LiteralPath $installedVidaPath) {
        return $installedVidaPath
    }
    return Resolve-CommandPath "vida"
}

Initialize-WindowsHostEnvironment

function Show-Help {
    @"
Usage:
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode <mode> [-Json] [-Jobs <n>] [-TestFilter <filter>]
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-package -SkipBuild -Windows -ReleaseBinDir <dir> [-ReleaseVersion vX.Y.Z] [-Json]

Modes:
  script-check      No-Cargo proof for diffs, runtime boundaries, and script syntax.
  quick             Debug source proof: git diff check, cargo fmt, cargo check.
  focused-nextest   Focused vida package test proof; requires -TestFilter.
  package-nextest   Full vida package test proof with the default nextest profile.
  workspace-nextest Workspace nextest proof with the CI profile.
  vida-bin-shards   Full vida --bin test proof split by top-level test module.
  doc-test          Workspace Rust doc tests.
  build-debug       Debug build of supported runtime entrypoints.
  runtime-smoke     Build debug vida and run status from the effective target dir.
  release-package   Build release archives with native PowerShell scripts/build-release.ps1.
  release-install   Installed launcher proof through vida release install.
  target-dir-policy Print the effective Cargo target directory policy.
  cargo-env-check   Validate Windows Cargo/MSVC command and temp environment.

Notes:
  Cargo modes set CARGO_TARGET_DIR unless the caller already provided it.
  release-package accepts explicit -SkipBuild, -Windows, -ReleaseBinDir, -ReleaseVersion, and -ReleaseSuffix flags for packaging already-built release binaries.
  release-package also honors VIDA_RELEASE_SKIP_BUILD=1, VIDA_RELEASE_BIN_DIR=<dir>, and VIDA_RELEASE_SUFFIX=<suffix> for compatibility.
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

function Resolve-PrimaryWorktreeStateDir {
    if (-not [string]::IsNullOrWhiteSpace($env:VIDA_STATE_DIR)) {
        return $env:VIDA_STATE_DIR
    }

    try {
        $commonDirOutput = & $GitPath -C $RootDir rev-parse --path-format=absolute --git-common-dir 2>$null
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($commonDirOutput)) {
            return $null
        }

        $commonDir = [System.IO.Path]::GetFullPath(($commonDirOutput | Select-Object -First 1).Trim())
        if ((Split-Path -Leaf $commonDir) -ne ".git") {
            return $null
        }

        $primaryRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $commonDir))
        $normalizedRoot = [System.IO.Path]::GetFullPath($RootDir)
        if ($primaryRoot -eq $normalizedRoot) {
            return $null
        }

        $candidateStateDir = Join-Path $primaryRoot ".vida\data\state"
        if (Test-Path -LiteralPath $candidateStateDir) {
            return $candidateStateDir
        }
    } catch {
        return $null
    }

    return $null
}

$CargoTargetDirState = Resolve-CargoTargetDirPolicy
$env:CARGO_TARGET_DIR = $CargoTargetDirState.effective_cargo_target_dir
$DebugVidaPath = Join-Path $CargoTargetDirState.effective_cargo_target_dir "debug\vida.exe"
$ReleaseVidaPath = Join-Path $CargoTargetDirState.effective_cargo_target_dir "release\vida.exe"
$VidaBinTopLevelShardSplitThreshold = 200
$VidaBinNestedShardSplitThreshold = 50

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
$CargoPath = Resolve-VidaCommandPath "cargo" @((Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")) -Required
$PwshPath = Resolve-CommandPath "pwsh" @(
    "C:\Program Files\PowerShell\7\pwsh.exe",
    "$env:ProgramFiles\PowerShell\7\pwsh.exe",
    (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps\pwsh.exe")
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
        [string[]]$NextestArgs
    )

    $command = New-Object System.Collections.Generic.List[string]
    $command.Add($CargoPath)
    $command.Add("nextest")
    $command.Add("run")
    $command.Add("--locked")
    foreach ($arg in $NextestArgs) {
        $command.Add($arg)
    }
    if ($Jobs -gt 0) {
        $command.Add("-j")
        $command.Add([string]$Jobs)
    }
    return $command.ToArray()
}

function Get-VidaBinShardSplitThreshold {
    param([int]$Depth)

    if ($Depth -le 1) {
        return $VidaBinTopLevelShardSplitThreshold
    }
    return $VidaBinNestedShardSplitThreshold
}

function Add-VidaBinShardFilters {
    param(
        [string[]]$TestPaths,
        [int]$Depth,
        [System.Collections.Generic.List[string]]$Filters
    )

    $groups = @{}
    foreach ($testPath in $TestPaths) {
        [string[]]$segments = $testPath -split '::'
        if ($segments.Count -lt $Depth) {
            continue
        }
        $prefix = ($segments[0..($Depth - 1)] -join '::')
        if (-not $groups.ContainsKey($prefix)) {
            $groups[$prefix] = New-Object System.Collections.Generic.List[string]
        }
        $groups[$prefix].Add($testPath)
    }

    foreach ($prefix in ($groups.Keys | Sort-Object)) {
        [string[]]$groupPaths = @($groups[$prefix])
        $hasChildren = $false
        foreach ($testPath in $groupPaths) {
            if (($testPath -split '::').Count -gt $Depth) {
                $hasChildren = $true
                break
            }
        }

        if ($groupPaths.Count -gt (Get-VidaBinShardSplitThreshold -Depth $Depth) -and $hasChildren) {
            Add-VidaBinShardFilters -TestPaths $groupPaths -Depth ($Depth + 1) -Filters $Filters
        } elseif ($hasChildren) {
            $Filters.Add(("{0}::" -f $prefix))
        } else {
            $Filters.Add($prefix)
        }
    }
}

function Get-VidaBinTestShardFilters {
    $listOutput = & $CargoPath test --locked -p vida --bin vida -- --list
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $testPaths = New-Object System.Collections.Generic.List[string]
    foreach ($line in $listOutput) {
        if ($line -match '^([A-Za-z0-9_]+::.+): test$') {
            $testPaths.Add($Matches[1])
        }
    }

    $filters = New-Object System.Collections.Generic.List[string]
    Add-VidaBinShardFilters -TestPaths ([string[]]@($testPaths)) -Depth 1 -Filters $filters
    return [string[]]@($filters)
}

function Invoke-VidaBinTestShards {
    [string[]]$filters = @(Get-VidaBinTestShardFilters)
    if ($filters.Count -eq 0) {
        Write-Error "No vida --bin test shards were discovered."
        exit 2
    }

    foreach ($filter in $filters) {
        $operationName = $filter -replace ':+$', ''
        $operationId = "cargo-test-vida-bin-$($operationName -replace '[^A-Za-z0-9_.-]', '-')"
        Invoke-Timed $operationId @(
            $CargoPath,
            "test",
            "--locked",
            "-p",
            "vida",
            "--bin",
            "vida",
            $filter,
            "--",
            "--test-threads=1"
        )
    }
}

if ($Help) {
    Show-Help
    exit 0
}

Push-Location $RootDir
try {
    if (Test-ModeNeedsWindowsBuildEnvironment $Mode) {
        Import-VisualStudioBuildEnvironment
    }

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
    } elseif ($Mode -eq "cargo-env-check") {
        $Records.Add([pscustomobject]@{
            operation_id = "cargo-env-check"
            command_or_surface = "scripts/vida-dev-gate.ps1 -Mode cargo-env-check"
            cwd_or_context = $RootDir
            started_at = (Get-Date).ToString("o")
            duration_ms = 0
            exit_status = "pass"
            classification = "fast"
            target_dir_policy = $CargoTargetDirState.target_dir_policy
            effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
            cargo_path = $CargoPath
            pwsh_path = $PwshPath
            git_path = $GitPath
            cl_path = (Resolve-VidaCommandPath "cl.exe")
            link_path = (Resolve-VidaCommandPath "link.exe")
            temp = $env:TEMP
            tmp = $env:TMP
            vcinstalldir = $env:VCINSTALLDIR
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
        Invoke-Timed "powershell-build-release-parse" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-Command",
            '$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path "scripts/build-release.ps1"), [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }'
        )
        Invoke-Timed "powershell-release-package-check-parse" @(
            $PwshPath,
            "-NoLogo",
            "-NoProfile",
            "-Command",
            '$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path "scripts/check-release-package.ps1"), [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }'
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
        Invoke-Timed "cargo-fmt-check" @($CargoPath, "fmt", "-p", "vida", "--", "--check")
        Invoke-Timed "cargo-check-vida" @($CargoPath, "check", "--locked", "-p", "vida")
    } elseif ($Mode -eq "focused-nextest") {
        if ($TestFilter.Trim().Length -eq 0) {
            Write-Error "-Mode focused-nextest requires -TestFilter <filter>."
            exit 2
        }
        if ($TestFilter.Trim().Length -gt 0) {
            Invoke-Timed "nextest-focused" (New-NextestCommand -NextestArgs @("-p", "vida", "--profile", "default", $TestFilter))
        }
    } elseif ($Mode -eq "package-nextest") {
        Invoke-Timed "nextest-package-vida" (New-NextestCommand -NextestArgs @("-p", "vida", "--profile", "default"))
    } elseif ($Mode -eq "workspace-nextest") {
        Invoke-Timed "nextest-workspace" (New-NextestCommand -NextestArgs @("--workspace", "--profile", "ci"))
    } elseif ($Mode -eq "vida-bin-shards") {
        Invoke-VidaBinTestShards
    } elseif ($Mode -eq "doc-test") {
        Invoke-Timed "cargo-doc-tests" @($CargoPath, "test", "--workspace", "--doc", "--locked")
    } elseif ($Mode -eq "build-debug") {
        Invoke-Timed "cargo-build-debug-entrypoints" @($CargoPath, "build", "--locked", "-p", "vida", "-p", "taskflow-cli", "-p", "docflow-cli", "-p", "vida-pi-agent")
    } elseif ($Mode -eq "runtime-smoke") {
        Invoke-Timed "cargo-build-debug" @($CargoPath, "build", "--locked", "-p", "vida")
        Invoke-Timed "debug-vida-status" @($DebugVidaPath, "status", "--json")
    } elseif ($Mode -eq "release-package") {
        $releaseCommand = New-Object System.Collections.Generic.List[string]
        $releaseCommand.Add($PwshPath)
        $releaseCommand.Add("-NoLogo")
        $releaseCommand.Add("-NoProfile")
        $releaseCommand.Add("-ExecutionPolicy")
        $releaseCommand.Add("Bypass")
        $releaseCommand.Add("-File")
        $releaseCommand.Add("scripts/build-release.ps1")
        if ($SkipBuild -or ($env:VIDA_RELEASE_SKIP_BUILD -match '^(1|true|TRUE|yes|YES)$')) {
            $releaseCommand.Add("-SkipBuild")
        }
        if ($Windows) {
            $releaseCommand.Add("-Windows")
        }
        if ($ReleaseVersion.Trim().Length -gt 0) {
            $releaseCommand.Add("-Version")
            $releaseCommand.Add($ReleaseVersion)
        }
        if ($ReleaseBinDir.Trim().Length -gt 0) {
            $releaseCommand.Add("-ReleaseBinDir")
            $releaseCommand.Add($ReleaseBinDir)
        }
        if ($ReleaseSuffix.Trim().Length -gt 0) {
            $releaseCommand.Add("-ReleaseSuffix")
            $releaseCommand.Add($ReleaseSuffix)
        }
        if ($Json) {
            $releaseCommand.Add("-Json")
        }
        Invoke-Timed "release-package" $releaseCommand.ToArray()
    } elseif ($Mode -eq "release-install") {
        Invoke-Timed "cargo-build-release-vida" @($CargoPath, "build", "--locked", "-p", "vida", "--release")
        Invoke-Timed "vida-release-install" @($ReleaseVidaPath, "release", "install", "--skip-build", "--source-binary", $ReleaseVidaPath, "--json")
        $previousVidaStateDir = $env:VIDA_STATE_DIR
        $statusStateDir = Resolve-PrimaryWorktreeStateDir
        try {
            if (-not [string]::IsNullOrWhiteSpace($statusStateDir)) {
                $env:VIDA_STATE_DIR = $statusStateDir
            }
            Invoke-Timed "installed-vida-status" @((Resolve-InstalledVidaPath), "status", "--json")
        } finally {
            if ($null -eq $previousVidaStateDir) {
                Remove-Item Env:VIDA_STATE_DIR -ErrorAction SilentlyContinue
            } else {
                $env:VIDA_STATE_DIR = $previousVidaStateDir
            }
        }
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
