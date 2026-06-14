param(
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Find-Ripgrep {
    if ($env:RG -and (Test-Path -LiteralPath $env:RG)) {
        return $env:RG
    }

    $command = Get-Command rg -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $userRoots = @(
        $env:USERPROFILE,
        $HOME,
        [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile),
        "C:\Users\pomaz"
    ) | Where-Object { $_ } | Select-Object -Unique

    $candidates = foreach ($userRoot in $userRoots) {
        Join-Path $userRoot ".bun\install\global\node_modules\@vscode\ripgrep-win32-x64\bin\rg.exe"
        Join-Path $userRoot ".bun\install\global\node_modules\@openai\codex-win32-x64\vendor\x86_64-pc-windows-msvc\codex-path\rg.exe"
    }

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return $candidate
        }
    }

    return $null
}

function Get-ExistingPathArgs {
    param([string[]]$Patterns)

    $paths = New-Object System.Collections.Generic.List[string]
    foreach ($pattern in $Patterns) {
        if ($pattern.Contains("*")) {
            foreach ($match in (Get-ChildItem -Path $pattern -Directory -ErrorAction SilentlyContinue)) {
                $paths.Add($match.FullName)
            }
        } elseif (Test-Path -LiteralPath $pattern) {
            $paths.Add((Resolve-Path -LiteralPath $pattern).Path)
        }
    }
    return $paths.ToArray()
}

function Invoke-RgCheck {
    param(
        [string]$Name,
        [string]$Pattern,
        [string[]]$Paths,
        [string[]]$Globs = @()
    )

    $args = @("--color", "never", "--line-number", $Pattern)
    foreach ($path in $Paths) {
        $args += $path
    }
    foreach ($glob in $Globs) {
        $args += @("-g", $glob)
    }

    $output = @(& $script:Rg @args 2>&1 | ForEach-Object { $_.ToString() })
    $exitCode = $LASTEXITCODE
    $displayOutput = @($output | Select-Object -First 80)
    if ($output.Count -gt $displayOutput.Count) {
        $displayOutput += "... omitted $($output.Count - $displayOutput.Count) additional matches"
    }
    if ($exitCode -eq 1) {
        return [pscustomobject]@{
            name = $Name
            status = "pass"
            matches = @()
        }
    }
    if ($exitCode -ne 0) {
        return [pscustomobject]@{
            name = $Name
            status = "error"
            matches = $displayOutput
        }
    }

    return [pscustomobject]@{
        name = $Name
        status = "blocked"
        matches = $displayOutput
    }
}

$script:Rg = Find-Ripgrep
if (-not $script:Rg) {
    throw "ripgrep executable not found. Install rg, put it on PATH, or set RG to the executable path."
}

$runtimeAuthorityPaths = Get-ExistingPathArgs @(
    "crates/vida/src",
    "crates/taskflow-*",
    "crates/docflow-*"
)
$vidaPaths = Get-ExistingPathArgs @("crates/vida/src")
$blockerCodePaths = Get-ExistingPathArgs @("crates/vida/src", "crates/taskflow-*")

$checks = @(
    (Invoke-RgCheck `
        -Name "no direct read_to_string in runtime authority modules" `
        -Pattern "std::fs::read_to_string|read_to_string\(" `
        -Paths $runtimeAuthorityPaths `
        -Globs @("!**/tests/**", "!crates/runtime-path-policy/**", "!**/generated/**", "!**/adapters/**")),
    (Invoke-RgCheck `
        -Name "no mutable request authority terms in shell" `
        -Pattern "request_paths_authoritative|modern_pending_host_bridge_request|reconciled_blocked_status" `
        -Paths $vidaPaths `
        -Globs @("!**/tests/**", "!**/generated/**", "!**/adapters/**")),
    (Invoke-RgCheck `
        -Name "no broad runtime_dispatch_state export" `
        -Pattern "pub\(crate\) use runtime_dispatch_state::\*" `
        -Paths $vidaPaths `
        -Globs @("!**/tests/**", "!**/generated/**", "!**/adapters/**")),
    (Invoke-RgCheck `
        -Name "no direct string blocker codes outside contract/tests" `
        -Pattern '"host_bridge_|"implementation_artifact_|"stale_|"blocked_dispatch' `
        -Paths $blockerCodePaths `
        -Globs @("!**/tests/**", "!crates/taskflow-contracts/**", "!crates/vida/src/release1_contracts.rs", "!**/generated/**", "!**/adapters/**"))
)

$blocked = @($checks | Where-Object { $_.status -ne "pass" })
$status = if ($blocked.Count -eq 0) { "pass" } else { "blocked" }
$result = [pscustomobject]@{
    surface = "scripts/check-runtime-boundaries.ps1"
    status = $status
    checks = $checks
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
} else {
    Write-Host "runtime boundary checks: $status"
    foreach ($check in $checks) {
        Write-Host "- $($check.name): $($check.status)"
        foreach ($match in $check.matches) {
            Write-Host "  $match"
        }
    }
}

if ($status -ne "pass") {
    exit 1
}
