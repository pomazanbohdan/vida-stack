[CmdletBinding()]
param(
    [switch]$Apply,
    [switch]$SkipLocked,
    [string[]]$OnlyPath
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
        "C:\project\vida-stack\crates\runtime-path-policy\result.bin",
        "C:\project\vida-stack\~\.cache\pre-commit",
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

function Resolve-OnlyPath {
    param([string]$RequestedPath)

    if ([string]::IsNullOrWhiteSpace($RequestedPath)) {
        throw "OnlyPath rejected: path must not be empty."
    }

    try {
        return (Resolve-Path -LiteralPath $RequestedPath -ErrorAction Stop).Path
    } catch {
        throw "OnlyPath rejected: path does not resolve to an existing path: $RequestedPath"
    }
}


function Get-RegisteredWorktreePaths {
    $paths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    try {
        $lines = & git -C $ProjectRoot worktree list --porcelain 2>$null
        foreach ($line in $lines) {
            if ($line -like "worktree *") {
                $path = $line.Substring("worktree ".Length)
                if (-not [string]::IsNullOrWhiteSpace($path)) {
                    try {
                        $resolved = (Resolve-Path -LiteralPath $path -ErrorAction Stop).Path
                        [void]$paths.Add($resolved)
                    } catch {
                        [void]$paths.Add($path)
                    }
                }
            }
        }
    } catch {
        # Fail closed for worktree-like cleanup if the registry cannot be read.
    }
    return $paths
}

function Test-DirectoryEmpty {
    param([string]$Path)

    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if (-not $item.PSIsContainer) {
        return $false
    }

    $firstChild = Get-ChildItem -LiteralPath $Path -Force -ErrorAction SilentlyContinue | Select-Object -First 1
    return $null -eq $firstChild
}

function Test-StaleCargoLeanCtxCopyTree {
    param([string]$Path)

    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if (-not $item.PSIsContainer) {
        return $false
    }

    $expected = Join-Path $Path "Users\pomaz\.cargo\bin\lean-ctx.exe"
    if (-not (Test-Path -LiteralPath $expected -PathType Leaf)) {
        return $false
    }

    $allowedRoots = @("Users")
    $rootChildren = Get-ChildItem -LiteralPath $Path -Force -ErrorAction Stop
    foreach ($child in $rootChildren) {
        if ($allowedRoots -notcontains $child.Name) {
            return $false
        }
    }

    $allowedPrefixes = @(
        $Path,
        (Join-Path $Path "Users"),
        (Join-Path $Path "Users\pomaz"),
        (Join-Path $Path "Users\pomaz\.cargo"),
        (Join-Path $Path "Users\pomaz\.cargo\bin"),
        $expected
    )

    Get-ChildItem -LiteralPath $Path -Force -Recurse -ErrorAction Stop | ForEach-Object {
        $fullName = $_.FullName
        $allowed = $false
        foreach ($allowedPath in $allowedPrefixes) {
            if ($fullName -eq $allowedPath) {
                $allowed = $true
                break
            }
        }
        if (-not $allowed) {
            throw "Unexpected content under stale C:\c cleanup candidate: $fullName"
        }
    }

    return $true
}

function Test-SafeToDeleteCandidate {
    param(
        [string]$ResolvedPath,
        [System.Collections.Generic.HashSet[string]]$RegisteredWorktrees
    )

    if ($RegisteredWorktrees.Contains($ResolvedPath)) {
        return [pscustomobject]@{ Safe = $false; Reason = "registered_worktree" }
    }

    if ($ResolvedPath -eq "C:\project\vida-stack-test-temp" -or $ResolvedPath.StartsWith("C:\project\vida-stack-vh", [System.StringComparison]::OrdinalIgnoreCase)) {
        if ($RegisteredWorktrees.Count -eq 0) {
            return [pscustomobject]@{ Safe = $false; Reason = "worktree_registry_unavailable" }
        }
    }

    if ($ResolvedPath -eq "C:\project\vida-stack\.vida\worktrees" -or $ResolvedPath -eq "C:\project\vida-stack\.vida\cache") {
        if (-not (Test-DirectoryEmpty $ResolvedPath)) {
            return [pscustomobject]@{ Safe = $false; Reason = "not_empty" }
        }
    }

    if ($ResolvedPath -eq "C:\c") {
        try {
            if (-not (Test-StaleCargoLeanCtxCopyTree $ResolvedPath)) {
                return [pscustomobject]@{ Safe = $false; Reason = "unexpected_c_drive_content" }
            }
        } catch {
            return [pscustomobject]@{ Safe = $false; Reason = "unexpected_c_drive_content" }
        }
    }

    $broadRootDirectories = @("C:\manifest", "C:\sstables", "C:\wal", "C:\vlog")
    if ($broadRootDirectories -contains $ResolvedPath) {
        $item = Get-Item -LiteralPath $ResolvedPath -Force -ErrorAction Stop
        if ($item.PSIsContainer -and -not (Test-DirectoryEmpty $ResolvedPath)) {
            return [pscustomobject]@{ Safe = $false; Reason = "unvalidated_non_empty_root_directory" }
        }
    }

    return [pscustomobject]@{ Safe = $true; Reason = "safe" }
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
    "C:\project\vida-stack\crates\runtime-path-policy\result.bin",
    "C:\project\vida-stack\~\.cache\pre-commit",
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

$registeredWorktrees = Get-RegisteredWorktreePaths
$discoveredCandidates = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath $candidate) {
        [void]$discoveredCandidates.Add((Resolve-Path -LiteralPath $candidate).Path)
    }
}

$selectedCandidates = @($candidates)
if ($null -ne $OnlyPath -and $OnlyPath.Count -gt 0) {
    $selectedCandidates = @()
    foreach ($requestedPath in $OnlyPath) {
        $resolvedRequestedPath = Resolve-OnlyPath $requestedPath
        if (-not $discoveredCandidates.Contains($resolvedRequestedPath)) {
            throw "OnlyPath rejected: path is not a discovered cleanup candidate: $resolvedRequestedPath"
        }
        if (-not (Test-AllowedPath $resolvedRequestedPath)) {
            throw "OnlyPath rejected: path is not allowlisted: $resolvedRequestedPath"
        }
        if (Test-ProtectedPath $resolvedRequestedPath) {
            throw "OnlyPath rejected: path is protected or live runtime state: $resolvedRequestedPath"
        }
        if ($registeredWorktrees.Contains($resolvedRequestedPath)) {
            throw "OnlyPath rejected: path is a registered worktree: $resolvedRequestedPath"
        }
        if (-not ($selectedCandidates -contains $resolvedRequestedPath)) {
            $selectedCandidates += $resolvedRequestedPath
        }
    }
}

$results = @()

foreach ($candidate in $selectedCandidates) {
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
            Reason = $(if (-not $allowed) { "not_allowlisted" } else { "protected" })
        }
        continue
    }

    $safety = Test-SafeToDeleteCandidate $resolved $registeredWorktrees
    if (-not $safety.Safe) {
        $results += [pscustomobject]@{
            Path = $resolved
            Status = "skipped_safety"
            Files = 0
            MiB = 0
            Reason = $safety.Reason
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
        Reason = "safe"
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
