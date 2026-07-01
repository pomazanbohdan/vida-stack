[CmdletBinding()]
param(
    [switch]$Apply,
    [switch]$SkipLocked
)

$ErrorActionPreference = "Stop"

$ProjectRoot = "C:\project\vida-stack"
$LiveState = Join-Path $ProjectRoot ".vida\data\state"

function Add-Candidate {
    param(
        [System.Collections.Generic.List[string]]$Candidates,
        [string]$Path
    )
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return
    }
    if (-not $Candidates.Contains($Path)) {
        [void]$Candidates.Add($Path)
    }
}

function Get-DirectoryChildrenByName {
    param(
        [string]$Base,
        [scriptblock]$Predicate
    )
    if (-not (Test-Path -LiteralPath $Base)) {
        return @()
    }
    Get-ChildItem -LiteralPath $Base -Force -Directory -ErrorAction SilentlyContinue |
        Where-Object $Predicate |
        ForEach-Object { $_.FullName }
}

function Test-AllowedPath {
    param([string]$ResolvedPath)

    $exact = @(
        "C:\project\vida-stack\target",
        "C:\project\vida-stack\.vida\tmp",
        "C:\project\vida-stack\.vida\build-temp",
        "C:\project\vida-stack\dist",
        "C:\project\vida-stack\benches\ldrk-qualification\target",
        "C:\project\vida-stack\spikes\vida-runtime-restate\target",
        "C:\project\vida-stack\spikes\local-durable-runtime\target",
        "C:\project\vida-stack\tests\model\target",
        "C:\project\vida-stack-test-temp",
        "C:\project\vida-stack\.vida\worktrees",
        "C:\project\vida-stack\.vida\cache",
        "C:\t",
        "C:\tc",
        "C:\vt",
        "C:\manifest",
        "C:\sstables",
        "C:\wal",
        "C:\vlog",
        "C:\c",
        "C:\Dumps\SystemSettings",
        "C:\WRPEDC6.tmp",
        "C:\temp\vida_cl_probe.c",
        "C:\temp\vida_cl_probe.obj",
        "C:\temp\probe_root_tmp.c",
        "C:\temp\diff_vida.txt"
    )

    if ($exact -contains $ResolvedPath) {
        return $true
    }

    $prefixes = @(
        "C:\project\vida-stack\.vida\cargo-target",
        "C:\project\vida-stack\.vida\release-target-",
        "C:\project\vida-stack\.vida\data\state.backup-",
        "C:\project\vida-stack\.vida\data\state.archive.",
        "C:\project\vida-stack\.vida\data\state.archived-",
        "C:\project\vida-stack\.vida\data\state-backups",
        "C:\project\vida-stack\tmp\",
        "C:\project\vida-stack\tmp-task-notes",
        "C:\project\vida-stack-vh",
        "C:\tmp\vida-",
        "C:\tmp\runtime-path-policy-rooted-",
        "C:\vida-tgt-",
        "C:\vida-tmp-",
        "C:\temp\vida-"
    )

    foreach ($prefix in $prefixes) {
        if ($ResolvedPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $true
        }
    }

    return $false
}

function Test-ProtectedPath {
    param([string]$ResolvedPath)

    $protected = @(
        "C:\_temp\ComfyUI",
        "C:\temp\gemma4-vllm",
        "C:\project\vida_mobile",
        "C:\Users",
        "C:\Windows",
        "C:\Program Files",
        "C:\Program Files (x86)",
        "C:\ProgramData"
    )

    foreach ($path in $protected) {
        if ($ResolvedPath -eq $path -or $ResolvedPath.StartsWith($path + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
            return $true
        }
    }

    if (Test-Path -LiteralPath $LiveState) {
        $resolvedLive = (Resolve-Path -LiteralPath $LiveState).Path
        if ($ResolvedPath -eq $resolvedLive -or $ResolvedPath.StartsWith($resolvedLive + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
            return $true
        }
    }

    return $false
}

function Get-PathSize {
    param([string]$Path)

    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if (-not $item.PSIsContainer) {
        return [pscustomobject]@{ Files = 1; Bytes = [int64]$item.Length }
    }

    $files = 0
    $bytes = [int64]0
    Get-ChildItem -LiteralPath $Path -Force -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
        $files += 1
        $bytes += [int64]$_.Length
    }
    [pscustomobject]@{ Files = $files; Bytes = $bytes }
}

$candidates = [System.Collections.Generic.List[string]]::new()

foreach ($path in @(
    "C:\project\vida-stack\target",
    "C:\project\vida-stack\.vida\tmp",
    "C:\project\vida-stack\.vida\build-temp",
    "C:\project\vida-stack\dist",
    "C:\project\vida-stack\benches\ldrk-qualification\target",
    "C:\project\vida-stack\spikes\vida-runtime-restate\target",
    "C:\project\vida-stack\spikes\local-durable-runtime\target",
    "C:\project\vida-stack\tests\model\target",
    "C:\project\vida-stack-test-temp",
    "C:\project\vida-stack\.vida\worktrees",
    "C:\project\vida-stack\.vida\cache",
    "C:\t",
    "C:\tc",
    "C:\vt",
    "C:\manifest",
    "C:\sstables",
    "C:\wal",
    "C:\vlog",
    "C:\c",
    "C:\Dumps\SystemSettings",
    "C:\WRPEDC6.tmp",
    "C:\temp\vida_cl_probe.c",
    "C:\temp\vida_cl_probe.obj",
    "C:\temp\probe_root_tmp.c",
    "C:\temp\diff_vida.txt"
)) {
    Add-Candidate $candidates $path
}

foreach ($path in Get-DirectoryChildrenByName "C:\project\vida-stack\.vida" { $_.Name -like "cargo-target*" -or $_.Name -like "release-target-*" }) {
    Add-Candidate $candidates $path
}
foreach ($path in Get-DirectoryChildrenByName "C:\project\vida-stack\.vida\data" { $_.Name -like "state.backup-*" -or $_.Name -like "state.archive.*" -or $_.Name -like "state.archived-*" -or $_.Name -eq "state-backups" }) {
    Add-Candidate $candidates $path
}
foreach ($path in Get-DirectoryChildrenByName "C:\project\vida-stack\tmp" { $true }) {
    Add-Candidate $candidates $path
}
foreach ($path in Get-DirectoryChildrenByName "C:\project" { $_.Name -like "vida-stack-vh*" }) {
    Add-Candidate $candidates $path
}
foreach ($path in Get-DirectoryChildrenByName "C:\tmp" { $_.Name -like "vida-*" -or $_.Name -like "runtime-path-policy-rooted-*" }) {
    Add-Candidate $candidates $path
}
foreach ($path in Get-DirectoryChildrenByName "C:\" { $_.Name -like "vida-tgt-*" -or $_.Name -like "vida-tmp-*" }) {
    Add-Candidate $candidates $path
}
if (Test-Path -LiteralPath "C:\temp") {
    Get-ChildItem -LiteralPath "C:\temp" -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like "vida-*" -or $_.Name -in @("vida_cl_probe.c", "vida_cl_probe.obj", "probe_root_tmp.c", "diff_vida.txt") } |
        ForEach-Object { Add-Candidate $candidates $_.FullName }
}

$results = @()

foreach ($candidate in $candidates) {
    if (-not (Test-Path -LiteralPath $candidate)) {
        continue
    }

    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    $allowed = Test-AllowedPath $resolved
    $protected = Test-ProtectedPath $resolved

    if (-not $allowed -or $protected) {
        $results += [pscustomobject]@{
            Path = $resolved
            Status = "refused"
            Files = 0
            MiB = 0
        }
        continue
    }

    $size = Get-PathSize $resolved
    $status = "planned"

    if ($Apply) {
        try {
            Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
            $status = "removed"
        } catch {
            if ($SkipLocked) {
                $status = "partial_or_locked"
            } else {
                throw
            }
        }
    }

    $results += [pscustomobject]@{
        Path = $resolved
        Status = $status
        Files = $size.Files
        MiB = [math]::Round($size.Bytes / 1MB, 2)
    }
}

$summary = [pscustomobject]@{
    mode = $(if ($Apply) { "apply" } else { "dry_run" })
    count = $results.Count
    files = ($results | Measure-Object -Property Files -Sum).Sum
    mib = [math]::Round((($results | Measure-Object -Property MiB -Sum).Sum), 2)
    by_status = ($results | Group-Object Status | ForEach-Object { [pscustomobject]@{ status = $_.Name; count = $_.Count } })
    results = $results
}

$summary | ConvertTo-Json -Depth 5
