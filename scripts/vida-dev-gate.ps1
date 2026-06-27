param(
    [ValidateSet("script-check", "quick", "scoped-format", "focused-nextest", "package-nextest", "workspace-nextest", "doc-test", "build-debug", "runtime-smoke", "coverage", "release-package", "release-install", "release-install-status", "target-dir-policy", "proof-scheduler", "invoke-timed-argv-smoke")]
    [string]$Mode = "quick",
    [string]$Package = "vida",
    [string]$TestFilter = "",
    [string[]]$ProofCommand = @(),
    [string]$ProofCommandJson = "",
    [string[]]$FormatFile = @(),
    [string[]]$AllowDirtyFile = @(),
    [string]$ReleaseVersion = "",
    [string]$ReleaseBinDir = "",
    [string]$ReleaseSuffix = "",
    [string]$CoverageOutputPath = ".vida/tmp/operator-output.lcov",
    [string]$CrapOutputPath = ".vida/tmp/workspace-crap.json",
    [double]$CoverageThreshold = 80.0,
    [int]$Jobs = 0,
    [switch]$SkipBuild,
    [switch]$Windows,
    [switch]$CoverageIgnoreRunFail,
    [switch]$Json,
    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
$Records = New-Object System.Collections.Generic.List[object]
$OriginalCargoTargetDir = $env:CARGO_TARGET_DIR

function Test-IsWindowsHost {
    return [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)
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

function Resolve-SystemDrive {
    if (-not [string]::IsNullOrWhiteSpace($env:SystemDrive)) {
        return $env:SystemDrive.TrimEnd('\')
    }
    foreach ($candidate in @($env:SystemRoot, $env:windir, $env:USERPROFILE, $HOME, "C:\Windows")) {
        if ([string]::IsNullOrWhiteSpace($candidate)) {
            continue
        }
        try {
            $root = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($candidate))
            if (-not [string]::IsNullOrWhiteSpace($root)) {
                return $root.TrimEnd('\')
            }
        } catch {
            continue
        }
    }
    return "C:"
}

function Initialize-WindowsHostEnvironment {
    if (-not (Test-IsWindowsHost)) {
        return
    }

    $windowsRoot = $env:SystemRoot
    if ([string]::IsNullOrWhiteSpace($windowsRoot)) {
        $windowsRoot = $env:windir
    }
    if ([string]::IsNullOrWhiteSpace($windowsRoot)) {
        $windowsRoot = "C:\Windows"
    }
    $systemDrive = Resolve-SystemDrive
    Set-EnvIfMissing "SystemDrive" $systemDrive
    Set-EnvIfMissing "SystemRoot" $windowsRoot
    Set-EnvIfMissing "windir" $windowsRoot
    Set-EnvIfMissing "ComSpec" (Join-Path $windowsRoot "System32\cmd.exe")
    Set-EnvIfMissing "ProgramData" (Join-Path $systemDrive "ProgramData")
    Set-EnvIfMissing "ProgramFiles" "C:\Program Files"
    Set-EnvIfMissing "ProgramFiles(x86)" "C:\Program Files (x86)"

    $userName = $env:USERNAME
    if ([string]::IsNullOrWhiteSpace($userName)) {
        $userName = [System.Environment]::UserName
    }
    $userProfile = $env:USERPROFILE
    if ([string]::IsNullOrWhiteSpace($userProfile) -and
        -not [string]::IsNullOrWhiteSpace($env:HOMEDRIVE) -and
        -not [string]::IsNullOrWhiteSpace($env:HOMEPATH)) {
        $userProfile = Join-Path $env:HOMEDRIVE $env:HOMEPATH
    }
    if ([string]::IsNullOrWhiteSpace($userProfile) -and -not [string]::IsNullOrWhiteSpace($userName)) {
        $userProfile = Join-Path "C:\Users" $userName
    }
    Set-EnvIfMissing "USERPROFILE" $userProfile
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) {
        Set-EnvIfMissing "HOMEDRIVE" ([System.IO.Path]::GetPathRoot($userProfile).TrimEnd('\'))
        Set-EnvIfMissing "HOMEPATH" $userProfile.Substring(([System.IO.Path]::GetPathRoot($userProfile)).Length - 1)
        Set-EnvIfMissing "LOCALAPPDATA" (Join-Path $userProfile "AppData\Local")
        Set-EnvIfMissing "APPDATA" (Join-Path $userProfile "AppData\Roaming")
        Set-EnvIfMissing "TEMP" (Join-Path $userProfile "AppData\Local\Temp")
        Set-EnvIfMissing "TMP" $env:TEMP
        if (-not [string]::IsNullOrWhiteSpace($env:TEMP)) {
            New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
        }
    }

    $pathEntries = @(
        (Join-Path $windowsRoot "System32"),
        $windowsRoot,
        (Join-Path $windowsRoot "System32\Wbem"),
        (Join-Path $windowsRoot "System32\WindowsPowerShell\v1.0"),
        (Join-Path $env:ProgramFiles "PowerShell\7"),
        "C:\Program Files\Git\cmd",
        "C:\Program Files\Git\bin",
        (Join-Path $userProfile ".cargo\bin"),
        (Join-Path $env:LOCALAPPDATA "vida-stack\current\bin")
    )
    Add-PathEntries $pathEntries
}

function Import-VisualStudioBuildEnvironment {
    if (-not (Test-IsWindowsHost)) {
        return
    }
    if ((Get-Command "cl.exe" -ErrorAction SilentlyContinue) -and -not [string]::IsNullOrWhiteSpace($env:VCINSTALLDIR)) {
        return
    }

    $vcvarsCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat")
    )
    $vcvarsPath = $vcvarsCandidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
    if (-not $vcvarsPath) {
        return
    }

    $cmdPath = if ([string]::IsNullOrWhiteSpace($env:ComSpec)) { "C:\Windows\System32\cmd.exe" } else { $env:ComSpec }
    & $cmdPath /d /s /c "`"$vcvarsPath`" >nul && set" | ForEach-Object {
        if ($_ -match '^(.*?)=(.*)$') {
            Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2]
        }
    }
    Initialize-WindowsHostEnvironment
}

function Test-ModeNeedsWindowsBuildEnvironment {
    param([string]$ModeName)

    return $ModeName -in @(
        "quick",
        "focused-nextest",
        "package-nextest",
        "workspace-nextest",
        "doc-test",
        "build-debug",
        "runtime-smoke",
        "coverage",
        "release-install",
        "release-install-status"
    )
}

function Test-ModeNeedsBuildConcurrencyGuard {
    param([string]$ModeName)

    return $ModeName -in @(
        "quick",
        "focused-nextest",
        "package-nextest",
        "workspace-nextest",
        "doc-test",
        "build-debug",
        "runtime-smoke",
        "coverage",
        "release-package",
        "release-install"
    )
}

function Resolve-InstalledVidaPath {
    if (Test-IsWindowsHost) {
        if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
            $installedVidaPath = Join-Path $env:LOCALAPPDATA "vida-stack\current\bin\vida.exe"
            if (Test-Path -LiteralPath $installedVidaPath) {
                return $installedVidaPath
            }
        }
    } else {
        $homePath = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::UserProfile)
        if ([string]::IsNullOrWhiteSpace($homePath)) {
            $homePath = $env:HOME
        }
        if (-not [string]::IsNullOrWhiteSpace($homePath)) {
            $installedVidaPath = Join-Path $homePath ".local/share/vida-stack/current/bin/vida"
            if (Test-Path -LiteralPath $installedVidaPath) {
                return $installedVidaPath
            }
        }
    }

    return Resolve-CommandPath "vida"
}

Initialize-WindowsHostEnvironment

function Show-Help {
    @"
Usage:
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode <mode> [-Json] [-Jobs <n>] [-Package <crate>] [-TestFilter <filter>]
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-package -SkipBuild -Windows -ReleaseBinDir <dir> [-ReleaseVersion vX.Y.Z] [-Json]

Modes:
  script-check      No-Cargo proof for diffs, runtime boundaries, and script syntax.
  quick             Debug source proof: git diff check, cargo fmt, cargo check.
  scoped-format     Format only explicit -FormatFile Rust files and fail on out-of-scope dirty files.
  focused-nextest   Focused package test proof; defaults to -Package vida and requires -TestFilter.
  package-nextest   Full vida package test proof with the default nextest profile.
  workspace-nextest Workspace nextest proof with the CI profile.
  doc-test          Workspace Rust doc tests.
  build-debug       Debug build of supported runtime entrypoints.
  runtime-smoke     Build debug vida and run status from the effective target dir.
  coverage          Default test coverage gate: cargo llvm-cov LCOV, cargo-crap JSON, vida quality gate.
  release-package   Build release archives with native PowerShell scripts/build-release.ps1.
  release-install   Installed launcher proof through vida release install.
  release-install-status
                    Non-build summary of latest release-install artifacts, progress, and installed status.
  target-dir-policy Print the effective Cargo target directory policy.
  proof-scheduler   Run proof command snippets: non-Cargo in parallel, Cargo-like sequentially.

Notes:
  Cargo modes set CARGO_TARGET_DIR unless the caller already provided it.
  proof-scheduler accepts -ProofCommandJson '["cmd1","cmd2","cargo --version"]' from any host shell.
  proof-scheduler also accepts -ProofCommand array values when invoked from an already-running PowerShell script.
  coverage runs cargo llvm-cov nextest, writes LCOV to -CoverageOutputPath (default .vida/tmp/operator-output.lcov), and writes CRAP JSON to -CrapOutputPath (default .vida/tmp/workspace-crap.json).
  coverage fails on test failure unless -CoverageIgnoreRunFail is set, then still generates LCOV when cargo-llvm-cov can report it.
  release-package accepts explicit -SkipBuild, -Windows, -ReleaseBinDir, -ReleaseVersion, and -ReleaseSuffix flags for packaging already-built release binaries.
  release-package also honors VIDA_RELEASE_SKIP_BUILD=1, VIDA_RELEASE_BIN_DIR=<dir>, and VIDA_RELEASE_SUFFIX=<suffix> for compatibility.
  JSON mode records operation timing and log artifact paths under .vida/data/state/command-timing.
  scoped-format requires one or more -FormatFile values; add -AllowDirtyFile for intentionally dirty non-Rust task artifacts.
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

function ConvertTo-WindowsProcessArgument {
    param([AllowNull()][string]$Argument)

    if ($null -eq $Argument -or $Argument.Length -eq 0) {
        return '""'
    }
    if ($Argument -notmatch '[\s"]') {
        return $Argument
    }

    $builder = New-Object System.Text.StringBuilder
    [void]$builder.Append('"')
    $backslashCount = 0
    foreach ($char in $Argument.ToCharArray()) {
        if ($char -eq '\') {
            $backslashCount += 1
            continue
        }
        if ($char -eq '"') {
            [void]$builder.Append(('\' * (($backslashCount * 2) + 1)))
            [void]$builder.Append('"')
            $backslashCount = 0
            continue
        }
        if ($backslashCount -gt 0) {
            [void]$builder.Append(('\' * $backslashCount))
            $backslashCount = 0
        }
        [void]$builder.Append($char)
    }
    if ($backslashCount -gt 0) {
        [void]$builder.Append(('\' * ($backslashCount * 2)))
    }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function Join-WindowsProcessArguments {
    param([string[]]$Arguments)

    return (($Arguments | ForEach-Object { ConvertTo-WindowsProcessArgument $_ }) -join " ")
}

function Test-CommandCanInvokeCargo {
    param([string[]]$Command)

    if ($Command.Count -eq 0) {
        return $false
    }
    $joined = ($Command -join " ").Trim().ToLowerInvariant()
    $exe = [System.IO.Path]::GetFileNameWithoutExtension($Command[0]).ToLowerInvariant()
    return $exe -eq "cargo" -or
        $exe -eq "cargo-nextest" -or
        $joined -match '(^|[\s;&|])cargo(\.exe)?\s' -or
        $joined -match '(^|[\s;&|])cargo-nextest(\.exe)?\s' -or
        $joined.Contains("vida release install")
}

function New-CargoTimingContract {
    param(
        [string[]]$Command,
        [int64]$DurationMs
    )

    if (-not (Test-CommandCanInvokeCargo -Command $Command)) {
        return $null
    }

    [pscustomobject]@{
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_lock_wait_ms = $null
        compile_ms = $null
        wait_classification = $(if ($DurationMs -gt 5000) { "cargo_wait_unclassified_without_cargo_phase_data" } else { "not_over_budget" })
    }
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
    $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    Assert-NoReparsePointInPath -Root $RootDir -Path $logDir -OriginalPath $logDir
    $safeId = $OperationId -replace '[^A-Za-z0-9_.-]', '-'
    $stdoutPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.out.txt" -f $safeId, $started)
    $stderrPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.err.txt" -f $safeId, $started)
    $artifactRefs = @($stdoutPath, $stderrPath)
    if ($Mode -eq "release-install") {
        $latestArtifactPath = Join-Path $logDir "latest-release-install-artifacts.json"
        [pscustomobject]@{
            operation_id = $OperationId
            command_or_surface = ($Command -join " ")
            stdout_path = $stdoutPath
            stderr_path = $stderrPath
            started_at = $started.ToString("o")
        } | ConvertTo-Json -Depth 5 | Write-SafeStateFile -Path $latestArtifactPath
        [Console]::Error.WriteLine(("[progress] {0} artifacts ready before wait" -f $OperationId))
        [Console]::Error.WriteLine(("stdout: {0}" -f $stdoutPath))
        [Console]::Error.WriteLine(("stderr: {0}" -f $stderrPath))
        [Console]::Error.WriteLine(("latest: {0}" -f $latestArtifactPath))
    }
    try {
        $process = Start-Process `
            -FilePath $exe `
            -ArgumentList (Join-WindowsProcessArguments $args) `
            -WorkingDirectory $RootDir `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath `
            -NoNewWindow `
            -Wait `
            -PassThru
        $exitCode = $process.ExitCode
        if ($null -eq $exitCode) {
            $exitCode = 0
        }
        if (-not $Json -and $exitCode -eq 0) {
            if ((Test-Path -LiteralPath $stdoutPath) -and (Get-Item -LiteralPath $stdoutPath).Length -gt 0) {
                Get-Content -LiteralPath $stdoutPath -Encoding UTF8 | ForEach-Object { Write-Output $_ }
            }
            if ((Test-Path -LiteralPath $stderrPath) -and (Get-Item -LiteralPath $stderrPath).Length -gt 0) {
                Get-Content -LiteralPath $stderrPath -Encoding UTF8 | ForEach-Object { [Console]::Error.WriteLine($_) }
            }
        }
        if (-not $Json -and $exitCode -ne 0) {
            [Console]::Error.WriteLine(("[fail] {0} exited with code {1}" -f $OperationId, $exitCode))
            [Console]::Error.WriteLine(("stdout: {0}" -f $stdoutPath))
            [Console]::Error.WriteLine(("stderr: {0}" -f $stderrPath))
            foreach ($entry in @(@("stderr", $stderrPath), @("stdout", $stdoutPath))) {
                $label = $entry[0]
                $path = $entry[1]
                if ((Test-Path -LiteralPath $path) -and (Get-Item -LiteralPath $path).Length -gt 0) {
                    [Console]::Error.WriteLine(("--- {0} tail ---" -f $label))
                    Get-Content -LiteralPath $path -Encoding UTF8 -Tail 40 | ForEach-Object { [Console]::Error.WriteLine($_) }
                }
            }
        }
    } catch {
        $exitCode = 1
        throw
    } finally {
        $sw.Stop()
        $record = [pscustomobject]@{
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
        }
        $cargoTiming = New-CargoTimingContract -Command $Command -DurationMs ([int64]$sw.ElapsedMilliseconds)
        if ($null -ne $cargoTiming) {
            $record | Add-Member -NotePropertyName "cargo" -NotePropertyValue $cargoTiming
        }
        $Records.Add($record)
    }
    if ($exitCode -ne 0) {
        exit $exitCode
    }
}

function Start-ScheduledProofProcess {
    param(
        [string]$OperationId,
        [string[]]$Command,
        [string]$SchedulerGroup,
        [int]$SchedulerOrder
    )

    $started = Get-Date
    $exe = $Command[0]
    $args = @()
    if ($Command.Length -gt 1) {
        $args = $Command[1..($Command.Length - 1)]
    }
    $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    Assert-NoReparsePointInPath -Root $RootDir -Path $logDir -OriginalPath $logDir
    $safeId = $OperationId -replace '[^A-Za-z0-9_.-]', '-'
    $stdoutPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.out.txt" -f $safeId, $started)
    $stderrPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.err.txt" -f $safeId, $started)
    $process = Start-Process `
        -FilePath $exe `
        -ArgumentList (Join-WindowsProcessArguments $args) `
        -WorkingDirectory $RootDir `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -NoNewWindow `
        -PassThru
    return [pscustomobject]@{
        operation_id = $OperationId
        command = $Command
        process = $process
        started_at_value = $started
        stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        artifact_refs = @($stdoutPath, $stderrPath)
        scheduler_group = $SchedulerGroup
        scheduler_order = $SchedulerOrder
    }
}

function Stop-ScheduledProofProcesses {
    param(
        [System.Collections.IEnumerable]$Handles,
        [object]$FailedHandle
    )

    if ($null -eq $Handles) {
        return
    }
    foreach ($handle in $Handles) {
        if ($null -eq $handle -or [object]::ReferenceEquals($handle, $FailedHandle)) {
            continue
        }
        $process = $handle.process
        if ($null -eq $process -or $process.HasExited) {
            continue
        }
        try {
            $process.Kill()
        } catch {
            Write-Warning ("Failed to stop scheduled proof process {0}: {1}" -f $handle.operation_id, $_.Exception.Message)
        }
        try {
            $process.WaitForExit()
        } catch {
            Write-Warning ("Failed to wait for scheduled proof process {0}: {1}" -f $handle.operation_id, $_.Exception.Message)
        }
    }
}

function Complete-ScheduledProofProcess {
    param(
        [object]$Handle,
        [System.Collections.IEnumerable]$SiblingHandles = $null
    )

    $Handle.process.WaitForExit()
    $Handle.stopwatch.Stop()
    $exitCode = $Handle.process.ExitCode
    if ($null -eq $exitCode) {
        $exitCode = 0
    }
    $durationMs = [int64]$Handle.stopwatch.ElapsedMilliseconds
    $record = [pscustomobject]@{
        operation_id = $Handle.operation_id
        command_or_surface = ($Handle.command -join " ")
        cwd_or_context = $RootDir
        started_at = $Handle.started_at_value.ToString("o")
        duration_ms = $durationMs
        exit_status = $(if ($exitCode -eq 0) { "pass" } else { "fail" })
        classification = $(if ($durationMs -le 2000) { "fast" } elseif ($durationMs -le 5000) { "watch" } else { "long_gate_expected" })
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_refs = $Handle.artifact_refs
        scheduler_group = $Handle.scheduler_group
        scheduler_order = $Handle.scheduler_order
    }
    $cargoTiming = New-CargoTimingContract -Command $Handle.command -DurationMs $durationMs
    if ($null -ne $cargoTiming) {
        $record | Add-Member -NotePropertyName "cargo" -NotePropertyValue $cargoTiming
    }
    $Records.Add($record)
    if ($exitCode -ne 0) {
        Stop-ScheduledProofProcesses -Handles $SiblingHandles -FailedHandle $Handle
        exit $exitCode
    }
}

function Resolve-ProofSchedulerCommands {
    if ($ProofCommand.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($ProofCommandJson)) {
        Write-Error "Use either -ProofCommand or -ProofCommandJson, not both."
        exit 2
    }
    if (-not [string]::IsNullOrWhiteSpace($ProofCommandJson)) {
        try {
            $parsed = $ProofCommandJson | ConvertFrom-Json
        } catch {
            Write-Error "-ProofCommandJson must be a JSON array of command strings."
            exit 2
        }
        $commands = @($parsed)
        if ($commands.Count -eq 0) {
            Write-Error "-ProofCommandJson must contain at least one command string."
            exit 2
        }
        foreach ($command in $commands) {
            if (-not ($command -is [string]) -or [string]::IsNullOrWhiteSpace($command)) {
                Write-Error "-ProofCommandJson must be a JSON array of non-empty command strings."
                exit 2
            }
        }
        return [string[]]$commands
    }
    return [string[]]$ProofCommand
}

function Invoke-ProofScheduler {
    param([string[]]$Commands)

    if ($Commands.Count -eq 0) {
        Write-Error "-Mode proof-scheduler requires at least one -ProofCommand <command>."
        exit 2
    }

    $cargoCommands = New-Object System.Collections.Generic.List[string]
    $nonCargoCommands = New-Object System.Collections.Generic.List[string]
    foreach ($commandText in $Commands) {
        $command = @($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $commandText)
        if (Test-CommandCanInvokeCargo -Command $command) {
            $cargoCommands.Add($commandText)
        } else {
            $nonCargoCommands.Add($commandText)
        }
    }

    $parallelHandles = New-Object System.Collections.Generic.List[object]
    $order = 0
    foreach ($commandText in $nonCargoCommands) {
        $order += 1
        $parallelHandles.Add((Start-ScheduledProofProcess -OperationId "proof-scheduler-noncargo:$order" -Command @($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $commandText) -SchedulerGroup "non_cargo_parallel" -SchedulerOrder $order))
    }
    foreach ($handle in $parallelHandles) {
        Complete-ScheduledProofProcess -Handle $handle -SiblingHandles $parallelHandles
    }

    $order = 0
    foreach ($commandText in $cargoCommands) {
        $order += 1
        $handle = Start-ScheduledProofProcess -OperationId "proof-scheduler-cargo:$order" -Command @($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $commandText) -SchedulerGroup "cargo_sequential" -SchedulerOrder $order
        Complete-ScheduledProofProcess -Handle $handle
    }
}

function Test-TransientInstalledVidaStatusFailure {
    param([string[]]$ArtifactPaths)

    foreach ($path in $ArtifactPaths) {
        if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }
        $content = Get-Content -LiteralPath $path -Encoding UTF8 -Raw -ErrorAction SilentlyContinue
        if ($content -match "state_store_read_lock_contention|state store read lock contention|database is locked") {
            return $true
        }
    }
    return $false
}

function Invoke-InstalledVidaStatusWithRetry {
    param(
        [string[]]$Command,
        [int]$MaxAttempts = 6,
        [switch]$ReturnRecordOnFailure
    )

    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        $operationId = "installed-vida-status"
        $started = Get-Date
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $exitCode = 0
        $exe = $Command[0]
        $args = @()
        if ($Command.Length -gt 1) {
            $args = $Command[1..($Command.Length - 1)]
        }
        $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
        New-Item -ItemType Directory -Force -Path $logDir | Out-Null
        Assert-NoReparsePointInPath -Root $RootDir -Path $logDir -OriginalPath $logDir
        $safeId = "$operationId-attempt-$attempt"
        $stdoutPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.out.txt" -f $safeId, $started)
        $stderrPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.err.txt" -f $safeId, $started)
        $artifactRefs = @($stdoutPath, $stderrPath)
        try {
            $process = Start-Process `
                -FilePath $exe `
                -ArgumentList (Join-WindowsProcessArguments $args) `
                -WorkingDirectory $RootDir `
                -RedirectStandardOutput $stdoutPath `
                -RedirectStandardError $stderrPath `
                -NoNewWindow `
                -Wait `
                -PassThru
            $exitCode = $process.ExitCode
            if ($null -eq $exitCode) {
                $exitCode = 0
            }
        } catch {
            $exitCode = 1
            throw
        } finally {
            $sw.Stop()
        }

        $retryable = $exitCode -ne 0 -and (Test-TransientInstalledVidaStatusFailure -ArtifactPaths $artifactRefs)
        $exitStatus = if ($exitCode -eq 0) {
            "pass"
        } elseif ($retryable -and $attempt -lt $MaxAttempts) {
            "blocked"
        } else {
            "fail"
        }
        $Records.Add([pscustomobject]@{
            operation_id = $operationId
            command_or_surface = ($Command -join " ")
            cwd_or_context = $RootDir
            started_at = $started.ToString("o")
            duration_ms = [int64]$sw.ElapsedMilliseconds
            exit_status = $exitStatus
            classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
            target_dir_policy = $CargoTargetDirState.target_dir_policy
            effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
            artifact_refs = $artifactRefs
            attempt = $attempt
            max_attempts = $MaxAttempts
            retryable = $retryable
        })

        if ($exitCode -eq 0) {
            if (-not $Json) {
                if ((Test-Path -LiteralPath $stdoutPath) -and (Get-Item -LiteralPath $stdoutPath).Length -gt 0) {
                    Get-Content -LiteralPath $stdoutPath -Encoding UTF8 | ForEach-Object { Write-Output $_ }
                }
                if ((Test-Path -LiteralPath $stderrPath) -and (Get-Item -LiteralPath $stderrPath).Length -gt 0) {
                    Get-Content -LiteralPath $stderrPath -Encoding UTF8 | ForEach-Object { [Console]::Error.WriteLine($_) }
                }
            }
            return
        }

        if ($retryable -and $attempt -lt $MaxAttempts) {
            if (-not $Json) {
                [Console]::Error.WriteLine(("[retry] installed-vida-status hit transient state-store lock contention; attempt {0}/{1}" -f $attempt, $MaxAttempts))
            }
            Start-Sleep -Milliseconds (250 * $attempt)
            continue
        }

        if (-not $Json) {
            [Console]::Error.WriteLine(("[fail] installed-vida-status exited with code {0}" -f $exitCode))
            [Console]::Error.WriteLine(("stdout: {0}" -f $stdoutPath))
            [Console]::Error.WriteLine(("stderr: {0}" -f $stderrPath))
            foreach ($entry in @(@("stderr", $stderrPath), @("stdout", $stdoutPath))) {
                $label = $entry[0]
                $path = $entry[1]
                if ((Test-Path -LiteralPath $path) -and (Get-Item -LiteralPath $path).Length -gt 0) {
                    [Console]::Error.WriteLine(("--- {0} tail ---" -f $label))
                    Get-Content -LiteralPath $path -Encoding UTF8 -Tail 40 | ForEach-Object { [Console]::Error.WriteLine($_) }
                }
            }
        }
        if ($ReturnRecordOnFailure) {
            return
        }
        exit $exitCode
    }
}

function Read-DevGateJsonFile {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }

    try {
        $content = Get-Content -LiteralPath $Path -Encoding UTF8 -Raw
        if ([string]::IsNullOrWhiteSpace($content)) {
            return $null
        }
        return $content | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            parse_status = "failed"
            path = $Path
            error = $_.Exception.Message
        }
    }
}

function Get-ReleaseInstallStatusSummaryExitStatus {
    param(
        [AllowNull()][object]$ProgressSummary,
        [AllowNull()][object]$InstalledStatusRecord
    )

    $releaseStatus = if ($null -ne $ProgressSummary -and $ProgressSummary.PSObject.Properties.Name -contains "status") {
        [string]$ProgressSummary.status
    } else {
        "missing"
    }
    $installedStatus = if ($null -ne $InstalledStatusRecord -and $InstalledStatusRecord.PSObject.Properties.Name -contains "exit_status") {
        [string]$InstalledStatusRecord.exit_status
    } else {
        "missing"
    }

    if ($releaseStatus -eq "pass" -and $installedStatus -eq "pass") {
        return "pass"
    }
    if ($releaseStatus -eq "missing") {
        return "blocked"
    }
    return "fail"
}

function Assert-ReleaseInstallStatusSummaryExitStatus {
    param(
        [string]$CaseName,
        [AllowNull()][object]$ProgressSummary,
        [AllowNull()][object]$InstalledStatusRecord,
        [string]$Expected
    )

    $actual = Get-ReleaseInstallStatusSummaryExitStatus -ProgressSummary $ProgressSummary -InstalledStatusRecord $InstalledStatusRecord
    if ($actual -ne $Expected) {
        throw "release-install-status summary smoke failed for ${CaseName}: expected $Expected, got $actual."
    }
    Add-SkippedRecord "release-install-status-summary:$CaseName" "contract smoke expected $Expected"
    $Records[$Records.Count - 1].exit_status = "pass"
}

function Invoke-ReleaseInstallStatusSummary {
    $started = Get-Date
    $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
    $latestArtifactPath = Join-Path $logDir "latest-release-install-artifacts.json"
    $progressLatestPath = Join-Path $RootDir ".vida\data\state\release-install-progress\latest.json"
    $artifactSummary = Read-DevGateJsonFile -Path $latestArtifactPath
    $progressSummary = Read-DevGateJsonFile -Path $progressLatestPath
    $artifactRefs = @($latestArtifactPath, $progressLatestPath)
    $previousVidaStateDir = $env:VIDA_STATE_DIR
    $statusStateDir = Resolve-PrimaryWorktreeStateDir
    try {
        if (-not [string]::IsNullOrWhiteSpace($statusStateDir)) {
            $env:VIDA_STATE_DIR = $statusStateDir
        }
        Invoke-InstalledVidaStatusWithRetry -Command @((Resolve-InstalledVidaPath), "status", "--json") -ReturnRecordOnFailure
    } finally {
        if ($null -eq $previousVidaStateDir) {
            Remove-Item Env:VIDA_STATE_DIR -ErrorAction SilentlyContinue
        } else {
            $env:VIDA_STATE_DIR = $previousVidaStateDir
        }
    }

    $installedStatusRecord = $Records[$Records.Count - 1]
    $exitStatus = Get-ReleaseInstallStatusSummaryExitStatus -ProgressSummary $progressSummary -InstalledStatusRecord $installedStatusRecord

    $Records.Add([pscustomobject]@{
        operation_id = "release-install-status-summary"
        command_or_surface = "scripts/vida-dev-gate.ps1 -Mode release-install-status"
        cwd_or_context = $RootDir
        started_at = $started.ToString("o")
        duration_ms = [int64]((Get-Date) - $started).TotalMilliseconds
        exit_status = $exitStatus
        classification = "fast"
        target_dir_policy = $CargoTargetDirState.target_dir_policy
        effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
        artifact_refs = $artifactRefs
        release_install_latest_artifacts = $artifactSummary
        release_install_progress_latest = $progressSummary
        installed_status_record = $installedStatusRecord
    })

    if ($exitStatus -ne "pass") {
        exit 2
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

    $readmes = & git -C $RootDir ls-files --cached --others --exclude-standard -- 'README.md' ':(glob)**/README.md'
    if ($LASTEXITCODE -ne 0) {
        [void]$violations.Add("git ls-files failed while checking README.md placement invariant.")
    } else {
        foreach ($readme in $readmes) {
            $relative = ([string]$readme).Replace('\', '/').Trim()
            if ($relative.Length -gt 0 -and $relative -ne "README.md") {
                [void]$violations.Add(("{0}: nested README.md is not allowed; use index.md or a semantic document name." -f $relative))
            }
        }
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
    $command.Add("cargo")
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

if ($Help) {
    Show-Help
    exit 0
}

function Get-PathComparison {
    if ($IsWindows -or ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT)) {
        return [System.StringComparison]::OrdinalIgnoreCase
    }
    return [System.StringComparison]::Ordinal
}

function Get-RelativePathCompat {
    param(
        [string]$Root,
        [string]$Path
    )

    $method = [System.IO.Path].GetMethod("GetRelativePath", [type[]]@([string], [string]))
    if ($null -ne $method) {
        return [System.IO.Path]::GetRelativePath($Root, $Path).Replace('\', '/')
    }

    $fullRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if ($fullPath.Equals($fullRoot, (Get-PathComparison))) {
        return "."
    }
    $rootWithSeparator = $fullRoot + [System.IO.Path]::DirectorySeparatorChar
    $rootUri = New-Object System.Uri($rootWithSeparator)
    $pathUri = New-Object System.Uri($fullPath)
    if ($rootUri.Scheme -ne $pathUri.Scheme) {
        return $fullPath.Replace('\', '/')
    }
    return [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($pathUri).ToString()).Replace('\', '/')
}

function Test-PathInsideRoot {
    param(
        [string]$Root,
        [string]$Path,
        [System.StringComparison]$Comparison
    )

    return $Path.Equals($Root, $Comparison) -or $Path.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, $Comparison)
}

function Test-CargoTargetDirSafeForProcessCleanup {
    param(
        [string]$TargetRoot
    )

    $comparison = Get-PathComparison
    $normalizedTargetRoot = [System.IO.Path]::GetFullPath($TargetRoot).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $targetPathRoot = [System.IO.Path]::GetPathRoot($normalizedTargetRoot).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )

    if ($normalizedTargetRoot.Equals($targetPathRoot, $comparison)) {
        return $false
    }

    if ($CargoTargetDirState.target_dir_policy -ne "caller_provided") {
        return $true
    }

    $normalizedRoot = [System.IO.Path]::GetFullPath($RootDir).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )

    return (Test-PathInsideRoot -Root $normalizedRoot -Path $normalizedTargetRoot -Comparison $comparison) -and
        -not $normalizedTargetRoot.Equals($normalizedRoot, $comparison)
}

function Invoke-StaleCargoTargetProcessCleanup {
    if (-not (Test-IsWindowsHost)) {
        return
    }

    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $exitStatus = "pass"
    $targetRoot = [System.IO.Path]::GetFullPath($CargoTargetDirState.effective_cargo_target_dir).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $stopped = New-Object System.Collections.Generic.List[string]
    $failed = New-Object System.Collections.Generic.List[string]

    try {
        if (-not (Test-Path -LiteralPath $targetRoot)) {
            return
        }

        if (-not (Test-CargoTargetDirSafeForProcessCleanup -TargetRoot $targetRoot)) {
            if (-not $Json) {
                Write-Output ("[cleanup] skipped stale target process cleanup for unsafe Cargo target dir: {0}" -f $CargoTargetDirState.effective_cargo_target_dir)
            }
            return
        }

        $cleanup = Invoke-VidaBuildTargetProcessCleanup -RootDir $RootDir -TargetRoot $targetRoot -ExcludeProcessId $PID
        foreach ($process in $cleanup.StoppedProcesses) {
            [void]$stopped.Add($process)
        }
        foreach ($process in $cleanup.FailedProcesses) {
            [void]$failed.Add($process)
        }

        if ($stopped.Count -gt 0 -and -not $Json) {
            Write-Output ("[cleanup] stopped stale target process(es): {0}" -f ($stopped.ToArray() -join ", "))
        }
        if ($failed.Count -gt 0) {
            $exitStatus = "fail"
            [Console]::Error.WriteLine(("stale target process cleanup failed for: {0}" -f ($failed.ToArray() -join ", ")))
            exit 1
        }
    } finally {
        $sw.Stop()
        $Records.Add([pscustomobject]@{
            operation_id = "stale-target-process-cleanup"
            command_or_surface = "stop executable processes under effective Cargo target dir"
            cwd_or_context = $RootDir
            started_at = $started.ToString("o")
            duration_ms = [int64]$sw.ElapsedMilliseconds
            exit_status = $exitStatus
            classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
            target_dir_policy = $CargoTargetDirState.target_dir_policy
            effective_cargo_target_dir = $CargoTargetDirState.effective_cargo_target_dir
            artifact_refs = @()
            stopped_processes = $stopped.ToArray()
        })
    }
}

function Resolve-ExistingPathTarget {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }

    $item = Get-Item -LiteralPath $Path -Force
    if ($item -is [System.IO.DirectoryInfo]) {
        return $item.FullName
    }

    $target = $item
    if ($item.PSObject.Methods.Name -contains "ResolveLinkTarget") {
        $resolved = $item.ResolveLinkTarget($true)
        if ($null -ne $resolved) {
            $target = $resolved
        }
    }

    return $target.FullName
}

function Assert-NoReparsePointInPath {
    param(
        [string]$Root,
        [string]$Path,
        [string]$OriginalPath
    )

    $relativePath = Get-RelativePathCompat -Root $Root -Path $Path
    $segments = @($relativePath -split "[/\\]+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and $_ -ne "." })
    $current = $Root
    foreach ($segment in $segments) {
        $current = Join-Path $current $segment
        if (-not (Test-Path -LiteralPath $current)) {
            return
        }
        $item = Get-Item -LiteralPath $current -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Path uses a symlink or reparse point inside repository root: $OriginalPath"
        }
    }
}

function Write-SafeStateFile {
    param(
        [string]$Path,
        [Parameter(ValueFromPipeline = $true)]
        [string]$Value
    )

    process {
        $parent = Split-Path -Parent $Path
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
        Assert-NoReparsePointInPath -Root $RootDir -Path $parent -OriginalPath $parent
        Assert-NoReparsePointInPath -Root $RootDir -Path $Path -OriginalPath $Path
        $leaf = Split-Path -Leaf $Path
        $tempPath = Join-Path $parent (".{0}.{1}.tmp" -f $leaf, [System.Guid]::NewGuid().ToString("N"))
        try {
            Set-Content -LiteralPath $tempPath -Encoding UTF8 -Value $Value
            Assert-NoReparsePointInPath -Root $RootDir -Path $tempPath -OriginalPath $tempPath
            Assert-NoReparsePointInPath -Root $RootDir -Path $Path -OriginalPath $Path
            [System.IO.File]::Move($tempPath, $Path, $true)
        }
        finally {
            if (Test-Path -LiteralPath $tempPath) {
                Remove-Item -LiteralPath $tempPath -Force
            }
        }
    }
}

function Resolve-RepoOutputFilePath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "Output path cannot be empty."
    }

    $comparison = Get-PathComparison
    $fullRoot = [System.IO.Path]::GetFullPath($RootDir).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $candidate = $Path
    if (-not [System.IO.Path]::IsPathRooted($candidate)) {
        $candidate = Join-Path $RootDir $candidate
    }
    $fullPath = [System.IO.Path]::GetFullPath($candidate)
    if (-not (Test-PathInsideRoot -Root $fullRoot -Path $fullPath -Comparison $comparison)) {
        throw "Output path is outside repository root: $Path"
    }

    $parent = Split-Path -Parent $fullPath
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Assert-NoReparsePointInPath -Root $fullRoot -Path $parent -OriginalPath $parent
    Assert-NoReparsePointInPath -Root $fullRoot -Path $fullPath -OriginalPath $Path
    return $fullPath
}

function ConvertTo-RepoRelativePath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    $comparison = Get-PathComparison
    $fullRoot = [System.IO.Path]::GetFullPath($RootDir).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $candidate = $Path
    if (-not [System.IO.Path]::IsPathRooted($candidate)) {
        $candidate = Join-Path $RootDir $candidate
    }
    $fullPath = [System.IO.Path]::GetFullPath($candidate)
    if (-not (Test-PathInsideRoot -Root $fullRoot -Path $fullPath -Comparison $comparison)) {
        throw "Path is outside repository root: $Path"
    }
    Assert-NoReparsePointInPath -Root $fullRoot -Path $fullPath -OriginalPath $Path
    $resolvedPath = Resolve-ExistingPathTarget $fullPath
    $fullResolvedPath = [System.IO.Path]::GetFullPath($resolvedPath)
    if (-not (Test-PathInsideRoot -Root $fullRoot -Path $fullResolvedPath -Comparison $comparison)) {
        throw "Path resolves outside repository root: $Path"
    }
    return Get-RelativePathCompat -Root $fullRoot -Path $fullPath
}

function Get-GitPorcelainPaths {
    $paths = New-Object System.Collections.Generic.SortedSet[string]
    $lines = & $GitPath status --porcelain --untracked-files=all
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.Length -lt 4) {
            continue
        }
        $path = $line.Substring(3).Trim()
        if ($path.Contains(" -> ")) {
            $path = ($path -split " -> ")[-1].Trim()
        }
        $path = $path.Trim('"').Replace('\', '/')
        [void]$paths.Add($path)
    }
    return [string[]]@($paths)
}

function Get-ChangedRustSourceFiles {
    $paths = New-Object System.Collections.Generic.SortedSet[string]
    foreach ($path in (Get-GitPorcelainPaths)) {
        if ([string]::IsNullOrWhiteSpace($path) -or
            -not $path.EndsWith(".rs", [System.StringComparison]::OrdinalIgnoreCase) -or
            $path.StartsWith(".vida/cargo-target/", [System.StringComparison]::OrdinalIgnoreCase)) {
            continue
        }
        $fullPath = Join-Path $RootDir $path
        if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
            [void]$paths.Add($path)
        }
    }
    return [string[]]@($paths)
}

function Invoke-ChangedRustfmtCheck {
    [string[]]$changedRustFiles = @(Get-ChangedRustSourceFiles)
    if ($changedRustFiles.Count -eq 0) {
        Add-SkippedRecord "cargo-fmt-check" "no changed Rust source files"
        return
    }

    Invoke-Timed "cargo-fmt-check" (@("rustfmt", "--edition", "2024", "--check", "--") + $changedRustFiles)
}

function Assert-ScopedDirtyFiles {
    param(
        [string[]]$AllowedPaths,
        [string]$Phase
    )

    $allowed = New-Object System.Collections.Generic.HashSet[string]([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($path in $AllowedPaths) {
        [void]$allowed.Add($path)
    }
    $dirty = Get-GitPorcelainPaths
    $outOfScope = @($dirty | Where-Object { -not $allowed.Contains($_) })
    if ($outOfScope.Count -gt 0) {
        [Console]::Error.WriteLine(("scoped-format {0} found out-of-scope dirty file(s): {1}" -f $Phase, ($outOfScope -join ", ")))
        exit 2
    }
}

function Invoke-ScopedFormat {
    if ($FormatFile.Count -eq 0) {
        [Console]::Error.WriteLine("-Mode scoped-format requires at least one -FormatFile <path>.")
        exit 2
    }

    $formatFiles = @($FormatFile | ForEach-Object { ConvertTo-RepoRelativePath $_ })
    $allowFiles = @($AllowDirtyFile | ForEach-Object { ConvertTo-RepoRelativePath $_ })
    $allowed = @($formatFiles + $allowFiles | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    Assert-ScopedDirtyFiles -AllowedPaths $allowed -Phase "precheck"

    foreach ($file in $formatFiles) {
        if (-not $file.EndsWith(".rs", [System.StringComparison]::OrdinalIgnoreCase)) {
            [Console]::Error.WriteLine("scoped-format only formats Rust source files: $file")
            exit 2
        }
        Invoke-Timed "rustfmt-scoped:$file" @("rustfmt", "--edition", "2024", $file)
    }

    Assert-ScopedDirtyFiles -AllowedPaths $allowed -Phase "postcheck"
}

function Invoke-CoverageGate {
    $coveragePath = Resolve-RepoOutputFilePath $CoverageOutputPath
    $crapPath = Resolve-RepoOutputFilePath $CrapOutputPath
    $thresholdText = $CoverageThreshold.ToString([System.Globalization.CultureInfo]::InvariantCulture)

    $llvmCovCommand = New-Object System.Collections.Generic.List[string]
    $llvmCovCommand.Add("cargo")
    $llvmCovCommand.Add("llvm-cov")
    $llvmCovCommand.Add("nextest")
    $llvmCovCommand.Add("--workspace")
    $llvmCovCommand.Add("--lcov")
    $llvmCovCommand.Add("--output-path")
    $llvmCovCommand.Add($coveragePath)
    if ($CoverageIgnoreRunFail) {
        $llvmCovCommand.Add("--ignore-run-fail")
    }
    Invoke-Timed "cargo-llvm-cov-nextest-workspace-lcov" $llvmCovCommand.ToArray()

    $crapCommand = New-Object System.Collections.Generic.List[string]
    $crapCommand.Add("cargo")
    $crapCommand.Add("crap")
    $crapCommand.Add("--workspace")
    $crapCommand.Add("--lcov")
    $crapCommand.Add($coveragePath)
    $crapCommand.Add("--format")
    $crapCommand.Add("json")
    $crapCommand.Add("--output")
    $crapCommand.Add($crapPath)
    if ($Jobs -gt 0) {
        $crapCommand.Add("--jobs")
        $crapCommand.Add([string]$Jobs)
    }
    Invoke-Timed "cargo-crap-workspace-json" $crapCommand.ToArray()

    Invoke-Timed "vida-quality-gate-coverage" @(
        (Resolve-InstalledVidaPath),
        "quality",
        "gate",
        "--prepush",
        "--coverage-file",
        $coveragePath,
        "--coverage-threshold",
        $thresholdText,
        "--advise",
        "--json"
    )
}

$BuildGuard = $null
if (Test-ModeNeedsBuildConcurrencyGuard $Mode) {
    . (Join-Path $PSScriptRoot "build-concurrency-guard.ps1")
    $BuildGuard = Enter-VidaBuildConcurrencyGuard -RootDir $RootDir -Scope "build"
    Invoke-StaleCargoTargetProcessCleanup
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
    } elseif ($Mode -eq "proof-scheduler") {
        Invoke-ProofScheduler -Commands (Resolve-ProofSchedulerCommands)
    } elseif ($Mode -eq "invoke-timed-argv-smoke") {
        $probeDir = Join-Path $RootDir ".vida\data\state\command-timing"
        New-Item -ItemType Directory -Force -Path $probeDir | Out-Null
        $probePath = Join-Path $probeDir "invoke-timed-argv-probe.ps1"
        Set-Content -LiteralPath $probePath -Encoding UTF8 -Value @'
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ProbeArgs
)
$ProbeArgs | ConvertTo-Json -Compress
'@
        $expectedArgs = @("alpha beta", "-dash-like", 'quote"value', "semi;colon")
        Invoke-Timed "invoke-timed-argv-smoke" (@($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $probePath) + $expectedArgs)
        $latestRecord = $Records[$Records.Count - 1]
        $stdoutPath = $latestRecord.artifact_refs[0]
        $convertedArgs = Get-Content -LiteralPath $stdoutPath -Encoding UTF8 -Raw | ConvertFrom-Json
        if ($convertedArgs -is [System.Array]) {
            $actualArgs = [string[]]$convertedArgs
        } else {
            $actualArgs = @($convertedArgs)
        }
        if ($actualArgs.Count -ne $expectedArgs.Count) {
            throw "Invoke-Timed argv smoke failed: expected $($expectedArgs.Count) args, got $($actualArgs.Count)."
        }
        for ($i = 0; $i -lt $expectedArgs.Count; $i++) {
            if ($actualArgs[$i] -ne $expectedArgs[$i]) {
                throw "Invoke-Timed argv smoke failed at index ${i}: expected `$($expectedArgs[$i])`, got `$($actualArgs[$i])`."
            }
        }
        $retryProbePath = Join-Path $probeDir "installed-status-retry-probe.ps1"
        $retryMarkerPath = Join-Path $probeDir "installed-status-retry-marker.txt"
        Remove-Item -LiteralPath $retryMarkerPath -Force -ErrorAction SilentlyContinue
        Set-Content -LiteralPath $retryProbePath -Encoding UTF8 -Value @'
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ProbeArgs
)
$marker = Join-Path $PSScriptRoot "installed-status-retry-marker.txt"
if (-not (Test-Path -LiteralPath $marker)) {
    Set-Content -LiteralPath $marker -Value "seen" -Encoding ascii
    [Console]::Error.WriteLine("state_store_read_lock_contention")
    exit 1
}
Write-Output '{"status":"pass"}'
exit 0
'@
        Invoke-InstalledVidaStatusWithRetry -Command @($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $retryProbePath) -MaxAttempts 3
        $statusRecords = @($Records | Where-Object { $_.operation_id -eq "installed-vida-status" })
        if ($statusRecords.Count -lt 2) {
            throw "installed-vida-status retry smoke failed: expected at least two attempt records."
        }
        if (-not (@($statusRecords | Where-Object { $_.retryable }).Count -ge 1)) {
            throw "installed-vida-status retry smoke failed: expected one retryable attempt."
        }
        if ($statusRecords[-1].exit_status -ne "pass") {
            throw "installed-vida-status retry smoke failed: final attempt status was $($statusRecords[-1].exit_status)."
        }
        Assert-ReleaseInstallStatusSummaryExitStatus `
            -CaseName "pass" `
            -ProgressSummary ([pscustomobject]@{ status = "pass" }) `
            -InstalledStatusRecord ([pscustomobject]@{ exit_status = "pass" }) `
            -Expected "pass"
        Assert-ReleaseInstallStatusSummaryExitStatus `
            -CaseName "failed-progress" `
            -ProgressSummary ([pscustomobject]@{ status = "fail" }) `
            -InstalledStatusRecord ([pscustomobject]@{ exit_status = "pass" }) `
            -Expected "fail"
        Assert-ReleaseInstallStatusSummaryExitStatus `
            -CaseName "failed-installed-status" `
            -ProgressSummary ([pscustomobject]@{ status = "pass" }) `
            -InstalledStatusRecord ([pscustomobject]@{ exit_status = "fail" }) `
            -Expected "fail"
        Assert-ReleaseInstallStatusSummaryExitStatus `
            -CaseName "missing-progress" `
            -ProgressSummary $null `
            -InstalledStatusRecord ([pscustomobject]@{ exit_status = "pass" }) `
            -Expected "blocked"
        $statusFailProbePath = Join-Path $probeDir "installed-status-fail-probe.ps1"
        Set-Content -LiteralPath $statusFailProbePath -Encoding UTF8 -Value @'
[Console]::Error.WriteLine("installed status hard failure")
exit 3
'@
        Invoke-InstalledVidaStatusWithRetry -Command @($PwshPath, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $statusFailProbePath) -MaxAttempts 1 -ReturnRecordOnFailure
        $failedStatusRecord = $Records[$Records.Count - 1]
        if ($failedStatusRecord.operation_id -ne "installed-vida-status" -or $failedStatusRecord.exit_status -ne "fail") {
            throw "installed-vida-status non-exiting failure smoke failed: expected final fail record."
        }
        Assert-ReleaseInstallStatusSummaryExitStatus `
            -CaseName "returned-installed-status-failure" `
            -ProgressSummary ([pscustomobject]@{ status = "pass" }) `
            -InstalledStatusRecord $failedStatusRecord `
            -Expected "fail"
        Invoke-Timed "cargo-timing-contract-smoke" @("cargo", "--version")
        $cargoRecord = $Records[$Records.Count - 1]
        if ($null -eq $cargoRecord.cargo) {
            throw "cargo timing contract smoke failed: cargo object was missing."
        }
        if ($cargoRecord.cargo.target_dir_policy -ne $CargoTargetDirState.target_dir_policy) {
            throw "cargo timing contract smoke failed: target_dir_policy mismatch."
        }
        if ($cargoRecord.cargo.effective_cargo_target_dir -ne $CargoTargetDirState.effective_cargo_target_dir) {
            throw "cargo timing contract smoke failed: effective_cargo_target_dir mismatch."
        }
        if ($cargoRecord.cargo.wait_classification -ne "not_over_budget") {
            throw "cargo timing contract smoke failed: unexpected wait classification."
        }
        Invoke-ProofScheduler -Commands @(
            "Start-Sleep -Milliseconds 250; 'noncargo-a'",
            "Start-Sleep -Milliseconds 250; 'noncargo-b'",
            "cargo --version",
            "cargo --version"
        )
        $scheduled = @($Records | Where-Object { $_.operation_id -like "proof-scheduler-*" })
        $parallelRecords = @($scheduled | Where-Object { $_.scheduler_group -eq "non_cargo_parallel" })
        $cargoRecords = @($scheduled | Where-Object { $_.scheduler_group -eq "cargo_sequential" })
        if ($parallelRecords.Count -ne 2 -or $cargoRecords.Count -ne 2) {
            throw "proof scheduler smoke failed: expected 2 non-Cargo and 2 Cargo records."
        }
        if ($null -ne $parallelRecords[0].cargo -or $null -ne $parallelRecords[1].cargo) {
            throw "proof scheduler smoke failed: non-Cargo records received cargo timing."
        }
        if ($null -eq $cargoRecords[0].cargo -or $null -eq $cargoRecords[1].cargo) {
            throw "proof scheduler smoke failed: Cargo records missed cargo timing."
        }
        if ([datetime]$parallelRecords[1].started_at -ge ([datetime]$parallelRecords[0].started_at).AddMilliseconds($parallelRecords[0].duration_ms)) {
            throw "proof scheduler smoke failed: non-Cargo commands did not overlap."
        }
        if ([datetime]$cargoRecords[1].started_at -lt ([datetime]$cargoRecords[0].started_at).AddMilliseconds($cargoRecords[0].duration_ms)) {
            throw "proof scheduler smoke failed: Cargo commands overlapped."
        }
        Remove-Item -LiteralPath $retryProbePath, $retryMarkerPath, $statusFailProbePath -Force -ErrorAction SilentlyContinue
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
        Invoke-ChangedRustfmtCheck
        Invoke-Timed "cargo-check-vida" @("cargo", "check", "--locked", "-p", "vida")
    } elseif ($Mode -eq "scoped-format") {
        Invoke-ScopedFormat
    } elseif ($Mode -eq "focused-nextest") {
        if ($TestFilter.Trim().Length -eq 0) {
            Write-Error "-Mode focused-nextest requires -TestFilter <filter>."
            exit 2
        }
        $trimmedPackage = $Package.Trim()
        if ($trimmedPackage.Length -eq 0) {
            Write-Error "-Mode focused-nextest requires non-empty -Package <crate>."
            exit 2
        }
        $trimmedTestFilter = $TestFilter.Trim()
        if ($trimmedTestFilter.Length -gt 0) {
            if (-not $Json) {
                Write-Output ("focused-nextest package: {0}" -f $trimmedPackage)
                Write-Output ("focused-nextest filter: {0}" -f $trimmedTestFilter)
            }
            Invoke-Timed "nextest-focused:$trimmedPackage" (New-NextestCommand -NextestArgs @("-p", $trimmedPackage, "--profile", "default", $trimmedTestFilter))
        }
    } elseif ($Mode -eq "package-nextest") {
        Invoke-Timed "nextest-package-vida" (New-NextestCommand -NextestArgs @("-p", "vida", "--profile", "default"))
    } elseif ($Mode -eq "workspace-nextest") {
        Invoke-Timed "nextest-workspace" (New-NextestCommand -NextestArgs @("--workspace", "--profile", "ci"))
    } elseif ($Mode -eq "doc-test") {
        Invoke-Timed "cargo-doc-tests" @("cargo", "test", "--workspace", "--doc", "--locked")
    } elseif ($Mode -eq "build-debug") {
        Invoke-Timed "cargo-build-debug-entrypoints" @("cargo", "build", "--locked", "-p", "vida", "-p", "taskflow-cli", "-p", "docflow-cli", "-p", "vida-pi-agent")
    } elseif ($Mode -eq "runtime-smoke") {
        Invoke-Timed "cargo-build-debug" @("cargo", "build", "--locked", "-p", "vida")
        Invoke-Timed "debug-vida-status" @($DebugVidaPath, "status", "--json")
    } elseif ($Mode -eq "coverage") {
        Invoke-CoverageGate
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
        Invoke-Timed "cargo-build-release-vida" @("cargo", "build", "--locked", "-p", "vida", "--release")
        Invoke-Timed "vida-release-install" @($ReleaseVidaPath, "release", "install", "--skip-build", "--source-binary", $ReleaseVidaPath, "--json")
        $previousVidaStateDir = $env:VIDA_STATE_DIR
        $statusStateDir = Resolve-PrimaryWorktreeStateDir
        try {
            if (-not [string]::IsNullOrWhiteSpace($statusStateDir)) {
                $env:VIDA_STATE_DIR = $statusStateDir
            }
            Invoke-InstalledVidaStatusWithRetry -Command @((Resolve-InstalledVidaPath), "status", "--json")
        } finally {
            if ($null -eq $previousVidaStateDir) {
                Remove-Item Env:VIDA_STATE_DIR -ErrorAction SilentlyContinue
            } else {
                $env:VIDA_STATE_DIR = $previousVidaStateDir
            }
        }
    } elseif ($Mode -eq "release-install-status") {
        Invoke-ReleaseInstallStatusSummary
    }
} finally {
    Pop-Location
    if ($null -ne (Get-Command Exit-VidaBuildConcurrencyGuard -ErrorAction SilentlyContinue)) {
        Exit-VidaBuildConcurrencyGuard -Guard $BuildGuard
    }
    if ($Json) {
        $Records | ConvertTo-Json -Depth 6
    } else {
        foreach ($record in $Records) {
            Write-Host ("[{0}] {1} {2}ms" -f $record.exit_status, $record.operation_id, $record.duration_ms)
        }
    }
    $env:CARGO_TARGET_DIR = $OriginalCargoTargetDir
}
