[CmdletBinding()]
param(
    [string[]]$Packages = @(),
    [string[]]$Files = @(),
    [string]$Nightly = "nightly-2026-07-18",
    [string]$ToolRoot = "",
    [string]$MutestCargoPath = "",
    [string]$MutestNativeLibPath = "",
    [ValidateRange(1, 64)]
    [int]$MaxWorkers = 5,
    [ValidateRange(0.25, 128)]
    [double]$MemoryPerWorkerGb = 2,
    [ValidateRange(1, 20)]
    [int]$Depth = 3,
    [ValidateRange(1, 1000)]
    [int]$BatchSize = 25,
    [ValidateRange(1, 10080)]
    [int]$PackageTimeoutMinutes = 1440,
    [ValidateRange(0.1, 168)]
    [double]$MaxHours = 168,
    [string]$TargetDir = ".vida/tmp/mutest-target",
    [string]$MetadataRoot = ".vida/tmp/mutest-json",
    [string]$EvidenceRoot = ".vida/evidence/mutest-audit",
    [string]$RegistryPath = ".vida/evidence/mutest-audit/file-registry.json",
    [string]$DefectLogPath = "",
    [string]$TestUpdateCommand = "",
    [ValidateRange(90, 100)]
    [double]$Threshold = 90,
    [switch]$PlanOnly,
    [switch]$Json,
    [switch]$Resume,
    [switch]$IncludeWorkingTree,
    [switch]$FullRescan,
    [switch]$RefreshIndex,
    [switch]$AutoUpdateTests,
    [ValidateRange(1, 300)]
    [int]$PollSeconds = 2,
    [Alias("h")]
    [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$script:GitPath = $null
$script:CargoPath = $null
$script:RustupPath = $null
$Files = @($Files | ForEach-Object { ([string]$_) -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
$Packages = @($Packages | ForEach-Object { ([string]$_) -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })

function Write-Usage {
    @"
Continuous mutest-rs audit scheduler.

Default: five requested workers, capped by CPU/free memory. Workers refill immediately
when a package exits; each worker owns an isolated Cargo target and metadata directory.

Examples:
  pwsh -File scripts/vida-mutest-audit.ps1 -PlanOnly -IncludeWorkingTree
  pwsh -File scripts/vida-mutest-audit.ps1 -PlanOnly -IncludeWorkingTree -Json
  pwsh -File scripts/vida-mutest-audit.ps1 -PlanOnly -IncludeWorkingTree -FullRescan -Json
  pwsh -File scripts/vida-mutest-audit.ps1 -IncludeWorkingTree
  pwsh -File scripts/vida-mutest-audit.ps1 -Resume -IncludeWorkingTree

The default run is a Git-snapshot diff scan. A compatible completed file record is
resumed by SHA-256 content hash; -FullRescan explicitly invalidates all file records.
Use -RefreshIndex to update the canonical file index with per-file LOC/hash metrics
without starting mutation workers; this preserves existing wave results and flags
content drift for the next mutation wave.
Files at or below the threshold are recorded as needs_tests; a file is green only when
mutation_score_percent > threshold_percent (default: > 90%). To run a controlled test-update hook,
pass -AutoUpdateTests -TestUpdateCommand with {file} and {package} placeholders.

On Windows the script auto-discovers cargo-mutest.exe and the MSVC windows.lib
directory. Use -MutestCargoPath or -MutestNativeLibPath only to override discovery.
Workers use isolated writable TMP/TEMP directories and select --lib/--bin for
standard production paths automatically.
`-Files` and `-Packages` accept comma-separated values for shell-safe batch waves.
"@
}

function Get-RepoRoot {
    $candidate = Split-Path -Parent $PSScriptRoot
    $rootResult = Invoke-Captured -FilePath (Resolve-GitPath) -ArgumentList @("-C", $candidate, "rev-parse", "--show-toplevel") -WorkingDirectory $candidate
    $root = if ($rootResult.ExitCode -eq 0) { $rootResult.Stdout.Trim() } else { $null }
    if ([string]::IsNullOrWhiteSpace($root)) {
        throw "Not inside a git worktree: $candidate"
    }
    return [System.IO.Path]::GetFullPath($root.Trim())
}

function Resolve-GitPath {
    if (-not [string]::IsNullOrWhiteSpace($script:GitPath) -and (Test-Path -LiteralPath $script:GitPath -PathType Leaf)) {
        return $script:GitPath
    }
    $command = Get-Command git.exe -ErrorAction SilentlyContinue
    if ($null -ne $command -and -not [string]::IsNullOrWhiteSpace($command.Source) -and (Test-Path -LiteralPath $command.Source -PathType Leaf)) {
        $script:GitPath = [System.IO.Path]::GetFullPath($command.Source)
        return $script:GitPath
    }
    $programFiles = [Environment]::GetEnvironmentVariable("ProgramFiles")
    $programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
    $localAppData = [Environment]::GetEnvironmentVariable("LocalAppData")
    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($programFiles)) {
        [void]$candidates.Add((Join-Path $programFiles "Git\cmd\git.exe"))
        [void]$candidates.Add((Join-Path $programFiles "Git\bin\git.exe"))
    }
    if (-not [string]::IsNullOrWhiteSpace($programFilesX86)) {
        [void]$candidates.Add((Join-Path $programFilesX86 "Git\cmd\git.exe"))
        [void]$candidates.Add((Join-Path $programFilesX86 "Git\bin\git.exe"))
    }
    if (-not [string]::IsNullOrWhiteSpace($localAppData)) {
        [void]$candidates.Add((Join-Path $localAppData "Programs\Git\cmd\git.exe"))
        [void]$candidates.Add((Join-Path $localAppData "Programs\Git\bin\git.exe"))
    }
    [void]$candidates.Add("C:\Program Files\Git\cmd\git.exe")
    [void]$candidates.Add("C:\Program Files\Git\bin\git.exe")
    [void]$candidates.Add("C:\Program Files (x86)\Git\cmd\git.exe")
    [void]$candidates.Add("C:\Program Files (x86)\Git\bin\git.exe")
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $script:GitPath = [System.IO.Path]::GetFullPath($candidate)
            return $script:GitPath
        }
    }
    throw "Git executable not found. Install Git or provide git.exe on PATH."
}

function Resolve-CargoPath {
    if (-not [string]::IsNullOrWhiteSpace($script:CargoPath) -and (Test-Path -LiteralPath $script:CargoPath -PathType Leaf)) {
        return $script:CargoPath
    }
    $command = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if ($null -eq $command) { $command = Get-Command cargo -ErrorAction SilentlyContinue }
    if ($null -ne $command) {
        $commandPath = if (-not [string]::IsNullOrWhiteSpace($command.Source)) { $command.Source } else { $command.Path }
        if (-not [string]::IsNullOrWhiteSpace($commandPath) -and (Test-Path -LiteralPath $commandPath -PathType Leaf)) {
            $script:CargoPath = [System.IO.Path]::GetFullPath($commandPath)
            return $script:CargoPath
        }
    }
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) { [void]$candidates.Add((Join-Path $userProfile ".cargo\bin\cargo.exe")) }
    [void]$candidates.Add("C:\Users\$([Environment]::UserName)\.cargo\bin\cargo.exe")
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $script:CargoPath = [System.IO.Path]::GetFullPath($candidate)
            return $script:CargoPath
        }
    }
    throw "cargo executable not found. Install Rust or provide cargo.exe on PATH."
}

function Resolve-RustupPath {
    if (-not [string]::IsNullOrWhiteSpace($script:RustupPath) -and (Test-Path -LiteralPath $script:RustupPath -PathType Leaf)) {
        return $script:RustupPath
    }
    $command = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if ($null -eq $command) { $command = Get-Command rustup -ErrorAction SilentlyContinue }
    if ($null -ne $command) {
        $commandPath = if (-not [string]::IsNullOrWhiteSpace($command.Source)) { $command.Source } else { $command.Path }
        if (-not [string]::IsNullOrWhiteSpace($commandPath) -and (Test-Path -LiteralPath $commandPath -PathType Leaf)) {
            $script:RustupPath = [System.IO.Path]::GetFullPath($commandPath)
            return $script:RustupPath
        }
    }
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) { [void]$candidates.Add((Join-Path $userProfile ".cargo\bin\rustup.exe")) }
    [void]$candidates.Add("C:\Users\$([Environment]::UserName)\.cargo\bin\rustup.exe")
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $script:RustupPath = [System.IO.Path]::GetFullPath($candidate)
            return $script:RustupPath
        }
    }
    throw "rustup executable not found. Install rustup or provide rustup.exe on PATH."
}

function Resolve-MutestCargoPath {
    param([string]$RequestedPath)
    if (-not [string]::IsNullOrWhiteSpace($RequestedPath)) {
        $resolved = if ([System.IO.Path]::IsPathRooted($RequestedPath)) {
            [System.IO.Path]::GetFullPath($RequestedPath)
        } else {
            [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $RequestedPath))
        }
        if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
            throw "MutestCargoPath does not exist: $resolved"
        }
        return [pscustomobject]@{ path = $resolved; source = "explicit" }
    }
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $candidates = @(
        (Join-Path $RepoRoot ".vida\tmp\mutest-rs-pathfix-bin\cargo-mutest.exe"),
        (Join-Path (Split-Path -Parent $RepoRoot) "mutest-rs\target\release\cargo-mutest.exe"),
        (Join-Path (Split-Path -Parent $RepoRoot) "mutest-rs\target\debug\cargo-mutest.exe")
    )
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) { $candidates += Join-Path $userProfile ".cargo\bin\cargo-mutest.exe" }
    $candidates = @($candidates | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return [pscustomobject]@{ path = [System.IO.Path]::GetFullPath($candidate); source = "auto" }
        }
    }
    return [pscustomobject]@{ path = $null; source = "cargo-subcommand" }
}

function Get-CargoTargetArguments {
    param([string]$MutationFilterPath)
    if ([string]::IsNullOrWhiteSpace($MutationFilterPath)) { return @("--all-targets") }
    $normalized = Normalize-RepoPath $MutationFilterPath
    if ($normalized -match '^crates/[^/]+/src/lib\.rs$') { return @("--lib") }
    if ($normalized -match '^crates/[^/]+/src/bin/(.+)\.rs$') {
        $binName = $Matches[1].Replace('/', '-').Replace('\\', '-')
        return @("--bin", $binName)
    }
    return @("--all-targets")
}

function Invoke-Captured {
    param([string]$FilePath, [string[]]$ArgumentList = @(), [string]$WorkingDirectory = $RepoRoot)
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $FilePath
    $psi.WorkingDirectory = $WorkingDirectory
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = (($ArgumentList | ForEach-Object {
        $escaped = ([string]$_).Replace('"', '\"')
        "`"$escaped`""
    }) -join ' ')
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    try {
        [void]$process.Start()
    } catch {
        throw "Cannot start $FilePath $($ArgumentList -join ' '): $($_.Exception.Message)"
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    [pscustomobject]@{ ExitCode = $process.ExitCode; Stdout = $stdout; Stderr = $stderr }
}

function Get-GitValue {
    param([string[]]$Arguments)
    $result = Invoke-Captured -FilePath (Resolve-GitPath) -ArgumentList $Arguments
    if ($result.ExitCode -ne 0) { throw "git $($Arguments -join ' ') failed: $($result.Stderr.Trim())" }
    return $result.Stdout.Trim()
}

function Get-Provenance {
    $lockPath = Join-Path $RepoRoot "Cargo.lock"
    $toolCommit = $null
    if (-not [string]::IsNullOrWhiteSpace($ToolRoot) -and (Test-Path -LiteralPath $ToolRoot)) {
        try { $toolCommit = Get-GitValue -Arguments @("-C", (Resolve-Path -LiteralPath $ToolRoot).Path, "rev-parse", "HEAD") } catch { $toolCommit = $null }
    }
    $rustc = "unavailable"
    $cargo = "unavailable"
    try {
        $rustupPath = Resolve-RustupPath
        $rustcResult = Invoke-Captured -FilePath $rustupPath -ArgumentList @("run", $Nightly, "rustc", "--version")
        if ($rustcResult.ExitCode -eq 0) { $rustc = $rustcResult.Stdout.Trim() }
        $cargoResult = Invoke-Captured -FilePath $rustupPath -ArgumentList @("run", $Nightly, "cargo", "--version")
        if ($cargoResult.ExitCode -eq 0) { $cargo = $cargoResult.Stdout.Trim() }
    } catch { }
    [ordered]@{
        head = Get-GitValue -Arguments @("rev-parse", "HEAD")
        head_tree = Get-GitValue -Arguments @("rev-parse", "HEAD^{tree}")
        index_tree = Get-GitValue -Arguments @("write-tree")
        cargo_lock_sha256 = if (Test-Path -LiteralPath $lockPath) { (Get-FileHash -LiteralPath $lockPath -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        tool_root = if ([string]::IsNullOrWhiteSpace($ToolRoot)) { $null } else { [System.IO.Path]::GetFullPath($ToolRoot) }
        tool_commit = $toolCommit
        nightly = $Nightly
        rustc = $rustc
        cargo = $cargo
    }
}

function Assert-WorkingTreePolicy {
    if ($IncludeWorkingTree) { return }
    $status = Get-GitValue -Arguments @("status", "--porcelain")
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw "Worktree is dirty. Use -IncludeWorkingTree for an explicit immutable working-tree audit."
    }
}

function Get-WorkspacePackages {
    $result = Invoke-Captured -FilePath (Resolve-CargoPath) -ArgumentList @("+$Nightly", "metadata", "--format-version", "1", "--no-deps", "--locked")
    if ($result.ExitCode -ne 0) { throw "cargo metadata failed: $($result.Stderr.Trim())" }
    $metadata = $result.Stdout | ConvertFrom-Json
    return @($metadata.packages | ForEach-Object { $_.name } | Sort-Object -Unique)
}

function Resolve-PackageSet {
    param([string[]]$WorkspacePackages)
    if ($Packages.Count -eq 0) { return @($WorkspacePackages) }
    $known = New-Object 'System.Collections.Generic.HashSet[string]' -ArgumentList ([StringComparer]::Ordinal)
    foreach ($package in $WorkspacePackages) { [void]$known.Add($package) }
    foreach ($package in $Packages) {
        if (-not $known.Contains($package)) { throw "Unknown workspace package: $package" }
    }
    return @($Packages | Sort-Object -Unique)
}

function Get-FreeMemoryGb {
    try {
        $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
        return [double]$os.FreePhysicalMemory / 1MB
    } catch {
        try {
            $available = [System.GC]::GetGCMemoryInfo().TotalAvailableMemoryBytes
            return [double]$available / 1GB
        } catch { return [double]::PositiveInfinity }
    }
}

function Get-ResourcePlan {
    $cpuCap = [Math]::Max(1, [Environment]::ProcessorCount)
    $freeGb = Get-FreeMemoryGb
    $memoryCap = if ([double]::IsPositiveInfinity($freeGb)) { $MaxWorkers } else { [Math]::Max(1, [int][Math]::Floor($freeGb / $MemoryPerWorkerGb)) }
    $effective = [Math]::Max(1, [Math]::Min($MaxWorkers, [Math]::Min($cpuCap, $memoryCap)))
    [ordered]@{
        requested_workers = $MaxWorkers
        effective_workers = $effective
        cpu_cap = $cpuCap
        free_memory_gb = [Math]::Round($freeGb, 3)
        memory_per_worker_gb = $MemoryPerWorkerGb
        memory_cap = $memoryCap
    }
}

function Convert-ToSafeName {
    param([string]$Value)
    $safe = ($Value -replace '[^A-Za-z0-9_.-]', '_').Trim('_')
    if ([string]::IsNullOrWhiteSpace($safe)) { return "package" }
    return $safe
}

function Get-PackageCategory {
    param([string]$Package)
    if ($Package -eq "vida-test-support") { return "test-support" }
    return "production"
}

function Normalize-RepoPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { throw "File path cannot be empty." }
    $candidate = $Path.Replace('/', '\')
    $absolute = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $candidate))
    $rootPrefix = $RepoRoot.TrimEnd('\') + '\'
    if (-not $absolute.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "File path escapes repository root: $Path"
    }
    return $absolute.Substring($RepoRoot.Length + 1).Replace('\', '/')
}

function Test-ProductionRustPath {
    param([string]$Path)
    $normalized = $Path.Replace('\', '/')
    if ($normalized -notmatch '^crates/[^/]+/src/.+\.rs$') { return $false }
    $segments = $normalized.ToLowerInvariant().Split('/')
    if (@($segments | Where-Object { $_ -match 'generated|freezed|mock' }).Count -gt 0) { return $false }
    $leaf = $segments[-1]
    if ($leaf -match '(^|[._-])(generated|freezed|mock|mocks)([._-]|$)') { return $false }
    return $true
}

function Get-ProductionFilesForSnapshot {
    param([string]$Treeish)
    $tracked = @()
    $raw = Get-GitValue -Arguments @("ls-tree", "-r", "--name-only", $Treeish, "--", "crates")
    if (-not [string]::IsNullOrWhiteSpace($raw)) { $tracked = @($raw -split "`r?`n") }
    $working = @()
    if ($IncludeWorkingTree) {
        $working = @(Get-ChildItem -LiteralPath (Join-Path $RepoRoot "crates") -Recurse -File -Filter *.rs -ErrorAction SilentlyContinue |
            ForEach-Object { $_.FullName.Substring($RepoRoot.Length + 1).Replace('\', '/') })
    }
    return @($tracked + $working | ForEach-Object { $_.Trim() } | Where-Object {
        $_ -and (Test-ProductionRustPath $_)
    } | Sort-Object -Unique)
}

function Get-FileContentHash {
    param([string]$RelativePath)
    $absolute = Join-Path $RepoRoot ($RelativePath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) { return $null }
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($absolute)
    try { return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
}

function Get-FileLineMetrics {
    param([string]$RelativePath)
    $absolute = Join-Path $RepoRoot ($RelativePath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) {
        return [ordered]@{ loc = 0; loc_total = 0 }
    }
    $lines = @([System.IO.File]::ReadAllLines($absolute))
    $nonEmpty = @($lines | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    return [ordered]@{ loc = [int]$nonEmpty.Count; loc_total = [int]$lines.Count }
}

function Get-PackageForFile {
    param([string]$RelativePath, [string[]]$KnownPackages)
    $parts = $RelativePath.Replace('\', '/').Split('/')
    if ($parts.Count -lt 2) { return $null }
    $directoryName = $parts[1]
    $normalizedDirectory = $directoryName.Replace('-', '_')
    $match = @($KnownPackages | Where-Object { $_ -eq $directoryName -or $_.Replace('-', '_') -eq $normalizedDirectory })
    if (@($match).Count -gt 0) { return [string]$match[0] }
    return $directoryName
}

function Get-AbsoluteArtifactPath {
    param([string]$Path, [string]$DefaultPath)
    $candidate = if ([string]::IsNullOrWhiteSpace($Path)) { $DefaultPath } else { $Path }
    if ([System.IO.Path]::IsPathRooted($candidate)) { return [System.IO.Path]::GetFullPath($candidate) }
    return [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $candidate))
}

function Find-MutestNativeLibPath {
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    if ([string]::IsNullOrWhiteSpace($userProfile)) { return $null }
    $registryRoot = Join-Path $userProfile ".cargo\registry\src"
    if (-not (Test-Path -LiteralPath $registryRoot -PathType Container)) { return $null }
    $candidates = @(Get-ChildItem -LiteralPath $registryRoot -Recurse -File -Filter "windows.lib" -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Parent -and $_.Directory.Parent.Name -like "windows_x86_64_msvc-*" })
    $preferred = @($candidates | Where-Object { $_.Directory.Parent.Name -eq "windows_x86_64_msvc-0.42.2" } | Select-Object -First 1)
    if (@($preferred).Count -gt 0) { return $preferred[0].Directory.FullName }
    if (@($candidates).Count -gt 0) { return $candidates[0].Directory.FullName }
    return $null
}

function Get-MutestLaunch {
    param([string[]]$Arguments)
    if ([string]::IsNullOrWhiteSpace($MutestCargoPathAbsolute)) {
        $cargoPath = Resolve-CargoPath
        return [ordered]@{ file = $cargoPath; args = @($Arguments); display = "`"$cargoPath`" " + ($Arguments -join ' ') }
    }
    $launchArgs = @("run", $Nightly, $MutestCargoPathAbsolute) + @($Arguments)
    $rustupPath = Resolve-RustupPath
    return [ordered]@{ file = $rustupPath; args = $launchArgs; display = "`"$rustupPath`" " + ($launchArgs -join ' ') }
}

function Get-MutestCommandText {
    param([string[]]$Arguments)
    return (Get-MutestLaunch -Arguments $Arguments).display
}

function Get-MutestEnvironment {
    param([string]$TempRoot = "")
    $environment = @{}
    if (-not [string]::IsNullOrWhiteSpace($MutestNativeLibPathAbsolute)) {
        $existingFlags = [Environment]::GetEnvironmentVariable("RUSTFLAGS", "Process")
        $nativeFlag = "-L native=$MutestNativeLibPathAbsolute"
        $environment["RUSTFLAGS"] = if ([string]::IsNullOrWhiteSpace($existingFlags)) { $nativeFlag } else { "$existingFlags $nativeFlag" }
    }
    if (-not [string]::IsNullOrWhiteSpace($TempRoot)) {
        [void](New-Item -ItemType Directory -Force -Path $TempRoot)
        $environment["TMP"] = $TempRoot
        $environment["TEMP"] = $TempRoot
    }
    return $environment
}

function Get-OptionalProperty {
    param([object]$Object, [string]$Name, [object]$Default = $null)
    if ($null -eq $Object) { return $Default }
    if ($Object -is [System.Collections.IDictionary] -and $Object.Contains($Name)) { return $Object[$Name] }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) { return $Default }
    return $property.Value
}

function Convert-ToOrderedRecord {
    param([object]$Object)
    $record = [ordered]@{}
    if ($null -eq $Object) { return $record }
    if ($Object -is [System.Collections.IDictionary]) {
        foreach ($key in $Object.Keys) { $record[[string]$key] = $Object[$key] }
        return $record
    }
    foreach ($property in $Object.PSObject.Properties) { $record[$property.Name] = $property.Value }
    return $record
}

function Get-WaveSortValue {
    param([object]$Row)
    $wave = [string](Get-OptionalProperty $Row "last_wave_id" "")
    $match = [regex]::Match($wave, 'wave-mutest-(?<stamp>\d{8}-\d{6})')
    if ($match.Success) {
        return [DateTime]::ParseExact($match.Groups["stamp"].Value, "yyyyMMdd-HHmmss", [Globalization.CultureInfo]::InvariantCulture)
    }
    $updated = [string](Get-OptionalProperty $Row "wave_updated_at" "")
    try { return [DateTime]::Parse($updated, [Globalization.CultureInfo]::InvariantCulture) } catch { return [DateTime]::MinValue }
}

function Convert-ToCanonicalFileRow {
    param([object]$Row)
    if ($null -eq $Row -or [string]::IsNullOrWhiteSpace([string](Get-OptionalProperty $Row "path"))) { return $null }
    try { $path = Normalize-RepoPath ([string](Get-OptionalProperty $Row "path")) } catch { return $null }
    $record = [ordered]@{}
    foreach ($property in $Row.PSObject.Properties) { $record[$property.Name] = $property.Value }
    $record.path = $path
    if (-not $record.Contains("last_wave_id")) { $record["last_wave_id"] = $null }
    if (-not $record.Contains("wave_status")) { $record["wave_status"] = "idle" }
    if (-not $record.Contains("wave_updated_at")) { $record["wave_updated_at"] = $null }
    if (-not $record.Contains("wave_count")) { $record["wave_count"] = [int](Get-OptionalProperty $Row "scan_count" 0) }
    if (-not $record.Contains("blocker_code")) { $record["blocker_code"] = $null }
    if (-not $record.Contains("blocker_family")) { $record["blocker_family"] = $null }
    if (-not $record.Contains("blocker_reason")) { $record["blocker_reason"] = $null }
    if (-not $record.Contains("next_action")) { $record["next_action"] = $null }
    if (-not $record.Contains("loc")) { $record["loc"] = $null }
    if (-not $record.Contains("loc_total")) { $record["loc_total"] = $null }
    if (-not $record.Contains("loc_hash")) { $record["loc_hash"] = $null }
    return [pscustomobject]$record
}

function Get-UniqueFileRows {
    param([object[]]$Rows)
    $byPath = @{}
    foreach ($row in @($Rows)) {
        $canonical = Convert-ToCanonicalFileRow -Row $row
        if ($null -eq $canonical) { continue }
        $key = ([string]$canonical.path).ToLowerInvariant()
        if (-not $byPath.ContainsKey($key)) {
            $byPath[$key] = $canonical
            continue
        }
        $existing = $byPath[$key]
        $existingUpdated = [string](Get-OptionalProperty $existing "updated_at" "")
        $candidateUpdated = [string](Get-OptionalProperty $canonical "updated_at" "")
        if ($candidateUpdated -ge $existingUpdated) { $byPath[$key] = $canonical }
    }
    return @($byPath.Values | Sort-Object path)
}

function New-MutationRegistry {
    return [ordered]@{
        schema_version = 3; index_role = "mutation_wave_orchestrator"; registry_revision = 0
        updated_at = $null; repo_root = $RepoRoot; run_id = $null; last_wave_id = $null
        snapshot_mode = $null; snapshot_index_tree = $null; config_hash = $null; threshold_percent = $Threshold
        full_rescan = $false; diff_scan = [ordered]@{ candidates = 0; queued = 0; resumed = 0; deleted = 0; queue_policy = "new_or_hash_changed_or_pending_flags" }
        waves = @(); files = @(); needs_tests = @(); needs_rerun = @(); needs_rescan = @()
    }
}

function Get-IndexRefreshRows {
    param(
        [string[]]$CandidateFiles,
        [string[]]$KnownPackages,
        [object]$Registry,
        [string]$SnapshotTree,
        [switch]$PartialSelection
    )
    $oldByPath = @{}
    $oldRows = @(Get-UniqueFileRows -Rows @($Registry.files))
    foreach ($old in $oldRows) {
        if ($null -ne $old.path) { $oldByPath[([string]$old.path).ToLowerInvariant()] = $old }
    }
    $rows = New-Object System.Collections.Generic.List[object]
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' -ArgumentList ([StringComparer]::OrdinalIgnoreCase)
    foreach ($path in @($CandidateFiles | ForEach-Object { Normalize-RepoPath $_ } | Sort-Object -Unique)) {
        [void]$seen.Add($path)
        $hash = Get-FileContentHash -RelativePath $path
        $metrics = Get-FileLineMetrics -RelativePath $path
        $pathKey = $path.ToLowerInvariant()
        $old = if ($oldByPath.ContainsKey($pathKey)) { $oldByPath[$pathKey] } else { $null }
        if ($null -ne $old) {
            $record = [ordered]@{}
            foreach ($property in $old.PSObject.Properties) { $record[$property.Name] = $property.Value }
            $oldHashValue = Get-OptionalProperty -Object $old -Name "hash"
            if ($null -eq $oldHashValue) { $oldHashValue = Get-OptionalProperty -Object $old -Name "content_hash_sha256" }
            $oldHash = if ($null -ne $oldHashValue) { [string]$oldHashValue } else { $null }
            $record.hash = $hash
            $record.content_hash_sha256 = $hash
            $record.loc = [int]$metrics.loc
            $record.loc_total = [int]$metrics.loc_total
            $record.loc_hash = $hash
            $record.snapshot_index_tree = $SnapshotTree
            if ([string]::IsNullOrWhiteSpace($oldHash) -or $oldHash -ne $hash) {
                $record.status = "queued"
                $record.queue_reason = if ([string]::IsNullOrWhiteSpace($oldHash)) { "missing_content_hash" } else { "content_hash_changed" }
                $record.needs_rerun = $true
                $record.wave_status = "queued"
                $record.wave_updated_at = [DateTime]::UtcNow.ToString("o")
            }
            $record.updated_at = [DateTime]::UtcNow.ToString("o")
            [void]$rows.Add([pscustomobject]$record)
            continue
        }
        [void]$rows.Add([pscustomobject][ordered]@{
            path = $path; package = Get-PackageForFile -RelativePath $path -KnownPackages $KnownPackages
            hash = $hash; content_hash_sha256 = $hash; loc = [int]$metrics.loc; loc_total = [int]$metrics.loc_total; loc_hash = $hash
            status = "queued"; queue_reason = "new_file"; mutation_score = $null; mutation_score_ratio = $null
            killed = 0; survived = 0; timeout = 0; no_coverage = 0; compile_error = 0; defects = @(); recommendations = @()
            needs_tests = $false; needs_rerun = $true; needs_rescan = $false; blocker_code = $null; blocker_family = $null
            blocker_reason = $null; next_action = $null; last_scan_hash = $null; last_scan_run_id = $null
            snapshot_index_tree = $SnapshotTree; config_hash = $null; resume_source = "index_refresh"; scan_count = 0
            last_wave_id = $null; wave_status = "queued"; wave_updated_at = [DateTime]::UtcNow.ToString("o"); wave_count = 0
            test_update_status = "not_requested"; updated_at = [DateTime]::UtcNow.ToString("o")
        })
    }
    foreach ($old in $oldRows) {
        if ($null -ne $old.path -and -not $seen.Contains([string]$old.path)) {
            [void]$rows.Add($old)
        }
    }
    return @(Get-UniqueFileRows -Rows @($rows.ToArray()))
}

function Read-FileRegistry {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return New-MutationRegistry }
    try {
        $value = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $registry = New-MutationRegistry
        foreach ($property in $value.PSObject.Properties) { $registry[$property.Name] = $property.Value }
        $registry.schema_version = 3
        $registry.index_role = "mutation_wave_orchestrator"
        $registry.registry_revision = [int](Get-OptionalProperty $value "registry_revision" 0)
        $registry.files = @(Get-UniqueFileRows -Rows @(Get-OptionalProperty $value "files" @()))
        $registry.waves = @((Get-OptionalProperty $value "waves" @()))
        return $registry
    } catch {
        throw "Cannot read file registry $Path`: $($_.Exception.Message)"
    }
}

function Get-FileStatusCounts {
    param([object[]]$Rows)
    $counts = [ordered]@{ queued = 0; running = 0; completed = 0; blocked = 0; timeout = 0; needs_tests = 0; needs_rerun = 0; needs_rescan = 0; deleted_from_snapshot = 0 }
    foreach ($row in @($Rows)) {
        $status = [string](Get-OptionalProperty $row "status" "queued")
        if ($counts.Contains($status)) { $counts[$status]++ }
        if ($status -ne "needs_tests" -and [bool](Get-OptionalProperty $row "needs_tests" $false)) { $counts.needs_tests++ }
        if ([bool](Get-OptionalProperty $row "needs_rerun" $false)) { $counts.needs_rerun++ }
        if ($status -ne "needs_rescan" -and [bool](Get-OptionalProperty $row "needs_rescan" $false)) { $counts.needs_rescan++ }
    }
    return $counts
}

function Get-InferredRegistryWaves {
    param([object[]]$Rows)
    return @($Rows | Where-Object { -not [string]::IsNullOrWhiteSpace([string](Get-OptionalProperty $_ "last_wave_id" "")) } | Group-Object { [string](Get-OptionalProperty $_ "last_wave_id" "") } | ForEach-Object {
        $groupRows = @($_.Group)
        $first = $groupRows | Select-Object -First 1
        $runId = [string](Get-OptionalProperty $first "last_scan_run_id" "")
        $statuses = @($groupRows | ForEach-Object { [string](Get-OptionalProperty $_ "wave_status" "unknown") } | Sort-Object -Unique)
        [ordered]@{
            wave_id = [string]$_.Name
            run_id = $runId
            status = if ($statuses.Count -eq 1) { $statuses[0] } else { "completed_with_followups" }
            counts = Get-FileStatusCounts -Rows $groupRows
            artifact_root = $null
            inferred_from_rows = $true
        }
    } | Sort-Object wave_id)
}

function Write-CanonicalRegistry {
    param([object]$Registry, [string]$Path)
    $Registry.files = @(Get-UniqueFileRows -Rows @($Registry.files))
    $Registry.schema_version = 3
    $Registry.index_role = "mutation_wave_orchestrator"
    $Registry.registry_revision = [int](Get-OptionalProperty $Registry "registry_revision" 0) + 1
    $Registry.updated_at = [DateTime]::UtcNow.ToString("o")
    $Registry.needs_tests = @($Registry.files | Where-Object { $_.needs_tests } | ForEach-Object { $_.path })
    $Registry.needs_rerun = @($Registry.files | Where-Object { $_.needs_rerun } | ForEach-Object { $_.path })
    $Registry.needs_rescan = @($Registry.files | Where-Object { $_.needs_rescan } | ForEach-Object { $_.path })
    Write-AtomicJson -Path $Path -Value $Registry
}

function Get-FileRegistryPlan {
    param(
        [string[]]$CandidateFiles,
        [string[]]$KnownPackages,
        [object]$Registry,
        [string]$ConfigHash,
        [string]$SnapshotTree,
        [switch]$PartialSelection
    )
    $oldByPath = @{}
    foreach ($old in @(Get-UniqueFileRows -Rows @($Registry.files))) {
        if ($null -ne $old.path) { $oldByPath[([string]$old.path).ToLowerInvariant()] = $old }
    }
    $rows = New-Object System.Collections.Generic.List[object]
    $queue = New-Object System.Collections.Generic.List[object]
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' -ArgumentList ([StringComparer]::OrdinalIgnoreCase)
    $normalizedCandidates = @($CandidateFiles | ForEach-Object { Normalize-RepoPath $_ } | Sort-Object -Unique)
    foreach ($path in $normalizedCandidates) {
        [void]$seen.Add($path)
        $hash = Get-FileContentHash -RelativePath $path
        $metrics = Get-FileLineMetrics -RelativePath $path
        $pathKey = $path.ToLowerInvariant()
        $old = if ($oldByPath.ContainsKey($pathKey)) { $oldByPath[$pathKey] } else { $null }
        $oldHashValue = Get-OptionalProperty -Object $old -Name "hash"
        if ($null -eq $oldHashValue) { $oldHashValue = Get-OptionalProperty -Object $old -Name "content_hash_sha256" }
        $oldHash = if ($null -ne $oldHashValue) { [string]$oldHashValue } else { $null }
        $compatible = (-not $FullRescan) -and $null -ne $old -and $oldHash -eq $hash -and
            [string](Get-OptionalProperty $old "config_hash") -eq $ConfigHash -and [string](Get-OptionalProperty $old "status") -eq "completed" -and
            -not [bool](Get-OptionalProperty $old "needs_tests") -and -not [bool](Get-OptionalProperty $old "needs_rerun") -and -not [bool](Get-OptionalProperty $old "needs_rescan")
        if ($compatible) {
            $record = [ordered]@{}
            foreach ($property in $old.PSObject.Properties) { $record[$property.Name] = $property.Value }
            $record.resume_source = "compatible_registry"
            $record.snapshot_index_tree = $SnapshotTree
            $record.loc = [int]$metrics.loc
            $record.loc_total = [int]$metrics.loc_total
            $record.loc_hash = $hash
            [void]$rows.Add([pscustomobject]$record)
            continue
        }
        $reason = if ($FullRescan) { "full_rescan" } elseif ($null -eq $old) { "new_file" } elseif ($oldHash -ne $hash) { "content_hash_changed" } elseif ([bool](Get-OptionalProperty $old "needs_tests") ) { "needs_tests" } elseif ([bool](Get-OptionalProperty $old "needs_rescan") ) { "needs_rescan" } elseif ([bool](Get-OptionalProperty $old "needs_rerun") ) { "needs_rerun" } else { "incompatible_config" }
        $record = [ordered]@{
            path = $path; package = Get-PackageForFile -RelativePath $path -KnownPackages $KnownPackages
            hash = $hash; content_hash_sha256 = $hash; loc = [int]$metrics.loc; loc_total = [int]$metrics.loc_total; loc_hash = $hash
            status = "queued"; queue_reason = $reason
            mutation_score = $null; mutation_score_ratio = $null; killed = 0; survived = 0; timeout = 0; no_coverage = 0
            compile_error = 0; defects = @(); recommendations = @(); needs_tests = $false; needs_rerun = $true; needs_rescan = $false
            blocker_code = $null; blocker_family = $null; blocker_reason = $null; next_action = $null
            last_scan_hash = Get-OptionalProperty $old "last_scan_hash"; last_scan_run_id = Get-OptionalProperty $old "last_scan_run_id"
            snapshot_index_tree = $SnapshotTree; config_hash = $ConfigHash; resume_source = "queued"; scan_count = if ((Get-OptionalProperty $old "scan_count" 0)) { [int](Get-OptionalProperty $old "scan_count" 0) } else { 0 }
            last_wave_id = Get-OptionalProperty $old "last_wave_id"; wave_status = "queued"; wave_updated_at = [DateTime]::UtcNow.ToString("o"); wave_count = [int](Get-OptionalProperty $old "wave_count" 0)
            test_update_status = if ((Get-OptionalProperty $old "test_update_status")) { Get-OptionalProperty $old "test_update_status" } else { "not_requested" }
            updated_at = [DateTime]::UtcNow.ToString("o")
        }
        [void]$rows.Add([pscustomobject]$record); [void]$queue.Add([pscustomobject]$record)
    }
    $canonicalOldRows = @(Get-UniqueFileRows -Rows @($Registry.files))
    if ($PartialSelection) {
    foreach ($old in $canonicalOldRows) {
        if ($null -ne $old.path -and -not $seen.Contains([string]$old.path)) {
            $untouched = [ordered]@{}
            foreach ($property in $old.PSObject.Properties) { $untouched[$property.Name] = $property.Value }
            [void]$rows.Add([pscustomobject]$untouched)
        }
    }
    } else {
    foreach ($old in $canonicalOldRows) {
        if ($null -ne $old.path -and -not $seen.Contains([string]$old.path)) {
            $deleted = [ordered]@{}
            foreach ($property in $old.PSObject.Properties) { $deleted[$property.Name] = $property.Value }
            $deleted.status = "deleted_from_snapshot"; $deleted.needs_rerun = $false; $deleted.needs_tests = $false; $deleted.needs_rescan = $false
            $deleted.updated_at = [DateTime]::UtcNow.ToString("o")
            [void]$rows.Add([pscustomobject]$deleted)
        }
    }
    }
    $allRows = @($rows.ToArray())
    $queuedRows = @($queue.ToArray())
    [pscustomobject]@{ files = $allRows; queue = $queuedRows; resumed = @($allRows | Where-Object { $_.resume_source -eq "compatible_registry" }); changed = $queuedRows; deleted = @($allRows | Where-Object { $_.status -eq "deleted_from_snapshot" }) }
}

function Get-FileScorePercent {
    param([object]$Stats)
    $denominator = [int]$Stats.killed + [int]$Stats.survived
    if ($denominator -le 0) { return $null }
    return [Math]::Round(100.0 * [double]$Stats.killed / $denominator, 3)
}

function Get-ExecutionBlocker {
    param([string]$Text)
    $value = if ($null -eq $Text) { "" } else { [string]$Text }
    if ($value -match '(?i)LNK1104[^\r\n]*C:\\WINDOWS\\lnk') {
        return [ordered]@{ code = "windows_linker_temp_path"; family = "toolchain"; reason = "MSVC link.exe resolved TMP/TEMP to protected C:\\WINDOWS"; next_action = "use the worker-private TMP/TEMP environment and rerun the file" }
    }
    if ($value -match '(?i)cannot find target in Cargo package metadata') {
        return [ordered]@{ code = "mutest_driver_target_metadata"; family = "mutest_tool"; reason = "mutest-driver target lookup did not match Cargo metadata on Windows"; next_action = "upgrade or rebuild mutest-rs with Windows path normalization, then rerun the file" }
    }
    if ($value -match '(?i)compiler unexpectedly panicked|internal compiler error|rustc.*ICE') {
        return [ordered]@{ code = "rustc_internal_compiler_error"; family = "rust_toolchain"; reason = "nightly rustc panicked while compiling the mutest target"; next_action = "pin or update the nightly/toolchain and rerun the file" }
    }
    if ($value -match '(?i)could not compile|linking with .* failed|error: aborting') {
        return [ordered]@{ code = "rust_compile_failure"; family = "toolchain"; reason = "Cargo compilation failed before mutation evaluation"; next_action = "inspect worker stderr, fix the compile/toolchain blocker, then rerun the file" }
    }
    return [ordered]@{ code = "mutation_no_evidence"; family = "mutation_runner"; reason = "worker produced no evaluable mutation evidence"; next_action = "inspect worker evidence and rerun the file" }
}

function Invoke-TestUpdateHook {
    param([object]$FileRecord, [string]$RunEvidenceRoot, [string]$EventPath)
    if (-not $AutoUpdateTests) { return [ordered]@{ status = "not_requested"; reason = "AutoUpdateTests_not_set" } }
    if ([string]::IsNullOrWhiteSpace($TestUpdateCommand)) { return [ordered]@{ status = "blocked"; reason = "TestUpdateCommand_required" } }
    $safe = Convert-ToSafeName ([string]$FileRecord.path)
    $evidence = Join-Path $RunEvidenceRoot "test-updates\$safe"
    [void](New-Item -ItemType Directory -Force -Path $evidence)
    $command = $TestUpdateCommand.Replace('{file}', [string]$FileRecord.path).Replace('{package}', [string]$FileRecord.package)
    $result = Invoke-Captured -FilePath "pwsh" -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $command)
    $result | Select-Object ExitCode, Stdout, Stderr | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $evidence "result.json") -Encoding UTF8
    $status = if ($result.ExitCode -eq 0) { "completed" } else { "blocked" }
    $payload = [ordered]@{ path = $FileRecord.path; package = $FileRecord.package; status = $status; command = $command; exit_code = $result.ExitCode; evidence = $evidence }
    Write-Event -Path $EventPath -Event "test_update_$status" -Data $payload
    return $payload
}

function Get-MutestCommand {
    param([string]$Package, [string]$WorkerTarget, [string]$WorkerMetadata, [int]$BatchSizeOverride = $BatchSize, [string]$MutationFilterPath = "")
    $targetArguments = Get-CargoTargetArguments -MutationFilterPath $MutationFilterPath
    $arguments = @(
        "run", "--package", $Package
    ) + $targetArguments + @(
        "--locked",
        "--target-dir", $WorkerTarget, "--metadata-out-root-dir", $WorkerMetadata,
        "--depth", [string]$Depth, "--safe", "--mutation-operators", "all", "--parallel-mutants",
        "--mutant-batch-algorithm", "greedy", "--mutant-batch-size", [string]$BatchSizeOverride, "--timings"
    )
    if (-not [string]::IsNullOrWhiteSpace($MutationFilterPath)) {
        $normalizedFilter = Normalize-RepoPath $MutationFilterPath
        if (-not (Test-ProductionRustPath $normalizedFilter)) { throw "Mutation filter is outside production Rust scope: $normalizedFilter" }
        $arguments += @("--filter-mutations", "file:$normalizedFilter")
    }
    if ([string]::IsNullOrWhiteSpace($MutestCargoPathAbsolute)) { return @("+$Nightly", "mutest") + $arguments }
    return $arguments
}

function Get-CommandHash {
    param([object[]]$Commands)
    $json = $Commands | ConvertTo-Json -Depth 20 -Compress
    $bytes = [Text.Encoding]::UTF8.GetBytes($json)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant() } finally { $sha.Dispose() }
}

function Get-IgnoredTests {
    $ignored = New-Object System.Collections.Generic.List[object]
    foreach ($file in (Get-ChildItem -LiteralPath (Join-Path $RepoRoot "crates") -Recurse -File -Filter *.rs -ErrorAction SilentlyContinue)) {
        $matches = Select-String -LiteralPath $file.FullName -Pattern '#\s*\[\s*ignore(?:\s*\(|\s*\])' -AllMatches -ErrorAction SilentlyContinue
        foreach ($match in $matches) {
            [void]$ignored.Add([ordered]@{ path = $file.FullName.Substring($RepoRoot.Length + 1); line = $match.LineNumber; text = $match.Line.Trim() })
        }
    }
    return $ignored.ToArray()
}

function Write-AtomicJson {
    param([string]$Path, [object]$Value)
    $parent = Split-Path -Parent $Path
    [void](New-Item -ItemType Directory -Force -Path $parent)
    $temp = "$Path.$([guid]::NewGuid().ToString('N')).tmp"
    $Value | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $temp -Encoding UTF8
    Move-Item -LiteralPath $temp -Destination $Path -Force
}

function Write-Event {
    param([string]$Path, [string]$Event, [object]$Data)
    $record = [ordered]@{ recorded_at = [DateTime]::UtcNow.ToString("o"); event = $Event; data = $Data }
    Add-Content -LiteralPath $Path -Value ($record | ConvertTo-Json -Depth 30 -Compress) -Encoding UTF8
}

function Get-MutestStats {
    param([string]$MetadataPath)
    $counts = [ordered]@{ generated = 0; evaluated = 0; killed = 0; survived = 0; no_coverage = 0; compile_error = 0; timeout = 0; flaky = 0; equivalent = 0; unknown = 0 }
    $files = @(Get-ChildItem -LiteralPath $MetadataPath -Recurse -File -Filter *.json -ErrorAction SilentlyContinue)
    function Visit-Node {
        param([object]$Node)
        if ($null -eq $Node) { return }
        if ($Node -is [System.Collections.IDictionary]) {
            foreach ($key in $Node.Keys) {
                $value = $Node[$key]
                $name = ([string]$key).ToLowerInvariant().Replace('-', '_')
                if ($counts.Contains($name) -and $value -is [ValueType] -and -not ($value -is [bool])) { $counts[$name] += [int]$value }
                if ($name -eq 'status' -or $name -eq 'outcome' -or $name -eq 'result') {
                    $status = ([string]$value).ToLowerInvariant().Replace('-', '_').Replace(' ', '_')
                    if ($status -match 'killed|survived|no_coverage|compile_error|timeout|flaky|equivalent') {
                        $keyName = if ($status -match 'no_coverage') { 'no_coverage' } elseif ($status -match 'compile_error') { 'compile_error' } else { $status }
                        $counts[$keyName]++
                    }
                }
                Visit-Node $value
            }
            return
        }
        if ($Node -is [System.Collections.IEnumerable] -and -not ($Node -is [string])) {
            foreach ($item in $Node) { Visit-Node $item }
        } elseif ($Node -is [psobject] -and -not ($Node -is [string]) -and -not ($Node -is [ValueType])) {
            foreach ($property in $Node.PSObject.Properties) { Visit-Node ([ordered]@{ $property.Name = $property.Value }) }
        }
    }
    foreach ($file in $files) {
        try {
            $json = (Get-Content -LiteralPath $file.FullName -Raw) | ConvertFrom-Json
            $handled = $false
            if ($file.Name -eq "mutations.json") {
                $mutationStats = Get-OptionalProperty -Object $json -Name "stats"
                $totalMutations = Get-OptionalProperty -Object $mutationStats -Name "total_mutations_count"
                if ($null -ne $totalMutations) {
                    $counts.generated += [int]$totalMutations
                    $handled = $true
                }
            } elseif ($file.Name -eq "evaluation.json") {
                $runs = @(Get-OptionalProperty -Object $json -Name "mutation_runs")
                if (@($runs).Count -gt 0) {
                    $lastRun = $runs[-1]
                    $detection = Get-OptionalProperty -Object $lastRun -Name "all_mutations_detection_stats"
                    if ($null -ne $detection) {
                        $counts.evaluated += [int](Get-OptionalProperty -Object $detection -Name "total_mutations_count" -Default 0)
                        $counts.killed += [int](Get-OptionalProperty -Object $detection -Name "detected_mutations_count" -Default 0)
                        $counts.survived += [int](Get-OptionalProperty -Object $detection -Name "undetected_mutations_count" -Default 0)
                        $counts.timeout += [int](Get-OptionalProperty -Object $detection -Name "timed_out_mutations_count" -Default 0)
                        $counts.compile_error += [int](Get-OptionalProperty -Object $detection -Name "crashed_mutations_count" -Default 0)
                        $handled = $true
                    }
                }
            }
            if (-not $handled) { Visit-Node $json }
        } catch { $counts.unknown++ }
    }
    $denominator = $counts.killed + $counts.survived
    [ordered]@{
        generated = $counts.generated
        evaluated = $counts.evaluated
        killed = $counts.killed
        survived = $counts.survived
        no_coverage = $counts.no_coverage
        compile_error = $counts.compile_error
        timeout = $counts.timeout
        flaky = $counts.flaky
        equivalent = $counts.equivalent
        unknown = $counts.unknown
        metadata_files = @($files).Count
        mutation_score = if ($denominator -gt 0) { [Math]::Round([double]$counts.killed / $denominator, 6) } else { $null }
        score_denominator = $denominator
    }
}

function Start-MutestWorker {
    param([string]$Package, [int]$Slot, [string]$RunTargetRoot, [string]$RunMetadataRoot, [string]$RunEvidenceRoot, [string]$EventPath, [int]$BatchSizeOverride = $BatchSize, [string]$EvidenceNamespace = "packages", [string[]]$FilePaths = @(), [string]$FilePath = "", [string]$WaveId = "")
    $selectedPath = if (-not [string]::IsNullOrWhiteSpace($FilePath)) { Normalize-RepoPath $FilePath } elseif (@($FilePaths).Count -eq 1) { Normalize-RepoPath ([string]$FilePaths[0]) } else { "" }
    $workerNameSource = if ([string]::IsNullOrWhiteSpace($selectedPath)) { $Package } else { $selectedPath }
    $workerKey = Convert-ToSafeName $workerNameSource
    $workerTarget = Join-Path $RunTargetRoot "files\$workerKey"
    $workerMetadata = Join-Path $RunMetadataRoot "files\$workerKey"
    $waveRoot = if ([string]::IsNullOrWhiteSpace($WaveId)) { $EvidenceNamespace } else { Join-Path $EvidenceNamespace (Convert-ToSafeName $WaveId) }
    $workerEvidence = Join-Path $RunEvidenceRoot "$waveRoot\files\$workerKey"
    if (Test-Path -LiteralPath $workerMetadata) { Remove-Item -LiteralPath $workerMetadata -Recurse -Force }
    $workerTemp = Join-Path $workerEvidence "tmp"
    [void](New-Item -ItemType Directory -Force -Path $workerTarget, $workerMetadata, $workerEvidence, $workerTemp)
    $stdout = Join-Path $workerEvidence "stdout.log"
    $stderr = Join-Path $workerEvidence "stderr.log"
    $args = Get-MutestCommand -Package $Package -WorkerTarget $workerTarget -WorkerMetadata $workerMetadata -BatchSizeOverride $BatchSizeOverride -MutationFilterPath $selectedPath
    $launch = Get-MutestLaunch -Arguments $args
    $startParams = @{
        FilePath = $launch.file
        ArgumentList = $launch.args
        WorkingDirectory = $RepoRoot
        RedirectStandardOutput = $stdout
        RedirectStandardError = $stderr
        WindowStyle = "Hidden"
        PassThru = $true
    }
    $launchEnvironment = Get-MutestEnvironment -TempRoot $workerTemp
    if (@($launchEnvironment.Keys).Count -gt 0) { $startParams.Environment = $launchEnvironment }
    $process = Start-Process @startParams
    $worker = [pscustomobject]@{
        package = $Package; path = $selectedPath; files = if ([string]::IsNullOrWhiteSpace($selectedPath)) { @($FilePaths) } else { @($selectedPath) }; wave_id = $WaveId; slot = $Slot; process = $process; started_at = [DateTime]::UtcNow
        deadline = [DateTime]::UtcNow.AddMinutes($PackageTimeoutMinutes); target_dir = $workerTarget
        metadata_dir = $workerMetadata; evidence_dir = $workerEvidence; temp_dir = $workerTemp; stdout = $stdout; stderr = $stderr
        command_args = $args; command = $launch.display; batch_size = $BatchSizeOverride; timed_out = $false
    }
    Write-Event -Path $EventPath -Event "worker_started" -Data ([ordered]@{ package = $Package; path = $selectedPath; wave_id = $WaveId; slot = $Slot; target_dir = $workerTarget; metadata_dir = $workerMetadata; command = $worker.command })
    return $worker
}

function Stop-WorkerTree {
    param([object]$Worker)
    try {
        if (-not $Worker.process.HasExited) {
            try { $Worker.process.Kill($true) } catch { & taskkill.exe /PID $Worker.process.Id /T /F 2>$null | Out-Null }
            $Worker.process.WaitForExit()
        }
    } catch { }
}

function Complete-MutestWorker {
    param([object]$Worker, [string]$RunEvidenceRoot, [string]$EventPath)
    $exitCode = -1
    try { if ($Worker.process.HasExited) { $exitCode = $Worker.process.ExitCode } } catch { }
    $stats = Get-MutestStats -MetadataPath $Worker.metadata_dir
    $stderrText = if (Test-Path -LiteralPath $Worker.stderr) { Get-Content -LiteralPath $Worker.stderr -Raw } else { "" }
    $stdoutText = if (Test-Path -LiteralPath $Worker.stdout) { Get-Content -LiteralPath $Worker.stdout -Raw } else { "" }
    $noEvidence = [int](Get-OptionalProperty $stats "metadata_files" 0) -eq 0 -and [int]$stats.generated -eq 0 -and [int]$stats.evaluated -eq 0
    $toolFailure = ($stderrText + "`n" + $stdoutText) -match '(?i)(compiler unexpectedly|internal compiler error|cannot find target|could not compile|error: aborting|panic)'
    $status = if ($Worker.timed_out) { "timeout" } elseif ($noEvidence -and $toolFailure) { "blocked" } elseif ($exitCode -eq 0 -or [int]$stats.evaluated -gt 0) { "completed" } else { "blocked" }
    $blocker = if ($status -eq "blocked") { Get-ExecutionBlocker -Text ($stderrText + "`n" + $stdoutText) } else { $null }
    $report = [ordered]@{
        package = $Worker.package; path = $Worker.path; wave_id = $Worker.wave_id; category = Get-PackageCategory $Worker.package; status = $status; exit_code = $exitCode; timed_out = [bool]$Worker.timed_out
        started_at = $Worker.started_at.ToString("o"); finished_at = [DateTime]::UtcNow.ToString("o")
        duration_seconds = [Math]::Round(([DateTime]::UtcNow - $Worker.started_at).TotalSeconds, 3)
        target_dir = $Worker.target_dir; metadata_dir = $Worker.metadata_dir; temp_dir = $Worker.temp_dir
        stdout = $Worker.stdout; stderr = $Worker.stderr; command = $Worker.command; stats = $stats
        batch_size = $Worker.batch_size
        blocker_code = if ($null -eq $blocker) { $null } else { $blocker.code }
        blocker_family = if ($null -eq $blocker) { $null } else { $blocker.family }
        blocker_reason = if ($null -eq $blocker) { $null } else { $blocker.reason }
        next_action = if ($null -eq $blocker) { $null } else { $blocker.next_action }
        survivors = if ($stats.survived -gt 0) { @("$($Worker.path):$($stats.survived) survived mutant(s); inspect metadata under $($Worker.metadata_dir)") } else { @() }
    }
    Write-AtomicJson -Path (Join-Path $Worker.evidence_dir "package-report.json") -Value $report
    Write-Event -Path $EventPath -Event "worker_completed" -Data $report
    return $report
}

function Invoke-SynchronousRescan {
    param([string]$Package, [string]$FilePath, [string]$RunTargetRoot, [string]$RunMetadataRoot, [string]$RunEvidenceRoot, [string]$EventPath, [string]$WaveId = "")
    try {
        $worker = Start-MutestWorker -Package $Package -FilePath $FilePath -WaveId $WaveId -Slot 0 -RunTargetRoot $RunTargetRoot -RunMetadataRoot $RunMetadataRoot -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath -EvidenceNamespace "rescans" -FilePaths @($FilePath)
        while (-not $worker.process.HasExited) {
            if ([DateTime]::UtcNow -gt $worker.deadline) { $worker.timed_out = $true; Stop-WorkerTree -Worker $worker; break }
            Start-Sleep -Seconds ([Math]::Max(1, [Math]::Min($PollSeconds, 5)))
        }
        return Complete-MutestWorker -Worker $worker -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath
    } catch {
        return [ordered]@{ package = $Package; status = "blocked"; exit_code = $null; timed_out = $false; stats = [ordered]@{ generated = 0; evaluated = 0; killed = 0; survived = 0; no_coverage = 0; compile_error = 0; timeout = 0; flaky = 0; equivalent = 0; unknown = 0; mutation_score = $null; score_denominator = 0 }; error = $_.Exception.Message }
    }
}

function Write-DefectProtocolPlan {
    param([string]$Path, [string]$RunId, [object[]]$Defects, [string]$EventPath)
    $defectGroups = @($Defects | Group-Object {
        $code = Get-OptionalProperty $_ "blocker_code" $null
        if ([string]::IsNullOrWhiteSpace([string]$code)) { [string](Get-OptionalProperty $_ "type" "unknown") } else { [string]$code }
    } | ForEach-Object {
        $first = @($_.Group) | Select-Object -First 1
        [ordered]@{
            key = $_.Name
            count = $_.Count
            family = Get-OptionalProperty $first "blocker_family" "mutation"
            reason = Get-OptionalProperty $first "blocker_reason" ""
            next_action = Get-OptionalProperty $first "recommendation" "inspect worker evidence and rerun the file"
            paths = @($_.Group | ForEach-Object { [string](Get-OptionalProperty $_ "path" "") } | Sort-Object -Unique)
        }
    })
    $protocol = [ordered]@{
        schema_version = 1; run_id = $RunId; status = if (@($Defects).Count -eq 0) { "no_defects" } else { "queued_for_remediation" }
        protocol = "docs/process/project-error-search-runtime-diagnostics-protocol.md"
        test_protocol = "docs/process/zombie-d-test-writing-protocol.md"
        defect_count = @($Defects).Count
        defects = @($Defects)
        defect_groups = $defectGroups
        next_actions = if (@($Defects).Count -eq 0) { @("No mutation defects require remediation.") } else { @("Classify each defect by owner and family.", "Add or update focused ZOMBIE-D proof for each confirmed defect.", "Run the affected file through the mutation queue again after fixes.") }
        taskflow_state_restored = $false
    }
    Write-AtomicJson -Path $Path -Value $protocol
    Write-Event -Path $EventPath -Event "defect_protocol_queued" -Data ([ordered]@{ path = $Path; defect_count = @($Defects).Count; protocol = $protocol.protocol })
    return $protocol
}

function Test-ProvenanceDrift {
    param([object]$Expected)
    $current = Get-Provenance
    foreach ($name in @("head", "head_tree", "index_tree", "cargo_lock_sha256", "tool_commit", "nightly")) {
        if ([string]$current[$name] -ne [string]$Expected[$name]) { return [pscustomobject]@{ drifted = $true; field = $name; expected = $Expected[$name]; actual = $current[$name] } }
    }
    return [pscustomobject]@{ drifted = $false }
}

function Get-LatestCheckpoint {
    $base = Join-Path $RepoRoot $EvidenceRoot
    if (-not (Test-Path -LiteralPath $base)) { return $null }
    return Get-ChildItem -LiteralPath $base -Recurse -File -Filter checkpoint.json -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
}

if ($Help) { Write-Usage; exit 0 }
$RepoRoot = Get-RepoRoot
if ([string]::IsNullOrWhiteSpace($ToolRoot)) {
    $siblingToolRoot = Join-Path (Split-Path -Parent $RepoRoot) "mutest-rs"
    if (Test-Path -LiteralPath $siblingToolRoot) { $ToolRoot = $siblingToolRoot }
}
$MutestCargoPathAbsolute = $null
$MutestCargoPathSource = if ($RefreshIndex) { "index_refresh" } else { $null }
$MutestNativeLibPathAbsolute = $null
if (-not $RefreshIndex) {
    $mutestPathResolution = Resolve-MutestCargoPath -RequestedPath $MutestCargoPath
    $MutestCargoPathAbsolute = $mutestPathResolution.path
    $MutestCargoPathSource = $mutestPathResolution.source
}
if (-not $RefreshIndex -and -not [string]::IsNullOrWhiteSpace($MutestNativeLibPath)) {
    $nativeCandidate = if ([System.IO.Path]::IsPathRooted($MutestNativeLibPath)) {
        [System.IO.Path]::GetFullPath($MutestNativeLibPath)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $MutestNativeLibPath))
    }
    if (Test-Path -LiteralPath $nativeCandidate -PathType Leaf) { $nativeCandidate = Split-Path -Parent $nativeCandidate }
    if (-not (Test-Path -LiteralPath $nativeCandidate -PathType Container)) {
        throw "MutestNativeLibPath does not exist: $nativeCandidate"
    }
    $MutestNativeLibPathAbsolute = $nativeCandidate
} elseif (-not [string]::IsNullOrWhiteSpace($MutestCargoPathAbsolute) -and [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
    $MutestNativeLibPathAbsolute = Find-MutestNativeLibPath
    if ([string]::IsNullOrWhiteSpace($MutestNativeLibPathAbsolute)) {
        throw "Custom MutestCargoPath on Windows requires MutestNativeLibPath (directory containing windows.lib)."
    }
}
if ($RefreshIndex -and ($PlanOnly -or $Resume -or $FullRescan -or $AutoUpdateTests)) {
    throw "-RefreshIndex cannot be combined with -PlanOnly, -Resume, -FullRescan, or -AutoUpdateTests."
}
Assert-WorkingTreePolicy
$WorkspacePackages = Get-WorkspacePackages
$SelectedPackages = Resolve-PackageSet -WorkspacePackages $WorkspacePackages
$Provenance = Get-Provenance
$ResourcePlan = Get-ResourcePlan
$SnapshotFiles = @(Get-ProductionFilesForSnapshot -Treeish $Provenance.index_tree)
$RequestedFiles = if (@($Files).Count -gt 0) {
    @($Files | ForEach-Object {
        $normalized = Normalize-RepoPath $_
        if (-not (Test-ProductionRustPath $normalized)) { throw "File is outside production Rust scope: $normalized" }
        if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot ($normalized.Replace('/', '\'))) -PathType Leaf)) { throw "Requested file does not exist: $normalized" }
        $normalized
    } | Sort-Object -Unique)
} else { $SnapshotFiles }
$CandidateFiles = @($RequestedFiles | Where-Object {
    $package = Get-PackageForFile -RelativePath $_ -KnownPackages $WorkspacePackages
    if ($SelectedPackages -notcontains $package) { return $false }
    return $true
} | Sort-Object -Unique)
if (@($Files).Count -gt 0 -and @($CandidateFiles).Count -ne @($RequestedFiles).Count) {
    $excluded = @($RequestedFiles | Where-Object { $CandidateFiles -notcontains $_ }) -join ', '
    throw "Requested files are outside selected packages: $excluded"
}
$resumeCheckpoint = if ($Resume) { Get-LatestCheckpoint } else { $null }
if ($Resume -and $null -eq $resumeCheckpoint) { throw "Resume requested but no checkpoint.json exists below $(Join-Path $RepoRoot $EvidenceRoot)." }
$RunId = if ($Resume) { ((Get-Content -LiteralPath $resumeCheckpoint.FullName -Raw) | ConvertFrom-Json).run_id } else { "mutest-$(Get-Date -Format yyyyMMdd-HHmmss)-$([guid]::NewGuid().ToString('N').Substring(0,8))" }
$RunTargetRoot = Join-Path $RepoRoot $TargetDir
$RunMetadataRoot = Join-Path $RepoRoot $MetadataRoot
$RunEvidenceRoot = Join-Path $RepoRoot (Join-Path $EvidenceRoot $RunId)
$WaveId = "wave-$RunId"
$RegistryPathAbsolute = Get-AbsoluteArtifactPath -Path $RegistryPath -DefaultPath (Join-Path $EvidenceRoot "file-registry.json")
$DefectLogPathAbsolute = Get-AbsoluteArtifactPath -Path $DefectLogPath -DefaultPath (Join-Path $EvidenceRoot "defects.jsonl")
$ConfigHash = Get-CommandHash -Commands @([ordered]@{
    command_policy = "one-file-filtered-worker-v2"; threshold = $Threshold; nightly = $Nightly; depth = $Depth; batch_size = $BatchSize
    timeout_minutes = $PackageTimeoutMinutes; include_working_tree = [bool]$IncludeWorkingTree; file_scope = "crates/**/src/**/*.rs"
    mutest_cargo_path = $MutestCargoPathAbsolute; mutest_cargo_path_source = $MutestCargoPathSource; mutest_native_lib_path = $MutestNativeLibPathAbsolute
    target_selector_policy = "auto-lib-bin-v1"; temp_environment_policy = "worker-private-tmp-v1"
})
$Registry = Read-FileRegistry -Path $RegistryPathAbsolute
$PartialSelection = @($Files).Count -gt 0
if ($RefreshIndex) {
    $RefreshId = "index-refresh-$(Get-Date -Format yyyyMMdd-HHmmss)-$([guid]::NewGuid().ToString('N').Substring(0,8))"
    $refreshedRows = Get-IndexRefreshRows -CandidateFiles $CandidateFiles -KnownPackages $WorkspacePackages -Registry $Registry -SnapshotTree $Provenance.index_tree -PartialSelection:$PartialSelection
    $RegistryDocument = Convert-ToOrderedRecord -Object $Registry
    $RegistryDocument.schema_version = 3
    $RegistryDocument.index_role = "mutation_wave_orchestrator"
    $RegistryDocument.snapshot_mode = if ($IncludeWorkingTree) { "working_tree" } else { "index" }
    $RegistryDocument.snapshot_index_tree = $Provenance.index_tree
    $RegistryDocument.loc_policy = [ordered]@{
        loc = "non_empty_source_lines"
        loc_total = "physical_lines"
        loc_hash = "content_hash_sha256"
        excludes_blank_lines = $true
        comments_included = $true
    }
    $latestWaveCandidates = @($refreshedRows | Where-Object { -not [string]::IsNullOrWhiteSpace([string](Get-OptionalProperty $_ "last_wave_id" "")) } | Sort-Object @{ Expression = { Get-WaveSortValue $_ }; Descending = $true } | Select-Object -First 1)
    $latestWaveRow = if ($latestWaveCandidates.Count -gt 0) { $latestWaveCandidates[0] } else { $null }
    $currentWaveId = [string](Get-OptionalProperty $RegistryDocument "last_wave_id" "")
    $currentWaveMatch = [regex]::Match($currentWaveId, 'wave-mutest-(?<stamp>\d{8}-\d{6})')
    $currentWaveTime = if ($currentWaveMatch.Success) { [DateTime]::ParseExact($currentWaveMatch.Groups["stamp"].Value, "yyyyMMdd-HHmmss", [Globalization.CultureInfo]::InvariantCulture) } else { [DateTime]::MinValue }
    if ($null -ne $latestWaveRow -and ([string]::IsNullOrWhiteSpace($currentWaveId) -or (Get-WaveSortValue $latestWaveRow) -gt $currentWaveTime)) {
        $RegistryDocument.last_wave_id = [string](Get-OptionalProperty $latestWaveRow "last_wave_id" $null)
        $RegistryDocument.run_id = [string](Get-OptionalProperty $latestWaveRow "last_scan_run_id" $null)
    }
    $existingWaves = @((Get-OptionalProperty $RegistryDocument "waves" @()))
    if ($existingWaves.Count -eq 0) {
        $inferredWaves = @(Get-InferredRegistryWaves -Rows $refreshedRows)
        if ($inferredWaves.Count -gt 0) { $RegistryDocument.waves = $inferredWaves }
    }
    $RegistryDocument.files = @($refreshedRows)
    $RegistryDocument.index_refresh = [ordered]@{
        refresh_id = $RefreshId
        refreshed_at = [DateTime]::UtcNow.ToString("o")
        candidate_files = @($CandidateFiles).Count
        selected_files = @($refreshedRows | Where-Object { $CandidateFiles -contains $_.path }).Count
        mode = if ($PartialSelection) { "partial" } else { "all_production_files" }
        mutation_workers_started = $false
        hash_policy = "sha256_content_drift_marks_needs_rerun"
        loc_policy = $RegistryDocument.loc_policy
    }
    Write-CanonicalRegistry -Registry $RegistryDocument -Path $RegistryPathAbsolute
    [void](New-Item -ItemType Directory -Force -Path $RunEvidenceRoot)
    $refreshReport = [ordered]@{
        schema_version = 1
        status = "index_refreshed"
        refresh_id = $RefreshId
        registry_path = $RegistryPathAbsolute
        registry_revision = [int](Get-OptionalProperty $RegistryDocument "registry_revision" 0)
        candidate_files = @($CandidateFiles).Count
        indexed_files = @($refreshedRows).Count
        loc_policy = $RegistryDocument.loc_policy
        hash_policy = "sha256_content_drift_marks_needs_rerun"
        mutation_workers_started = $false
    }
    Write-AtomicJson -Path (Join-Path $RunEvidenceRoot "index-refresh.json") -Value $refreshReport
    $refreshReport | ConvertTo-Json -Depth 20
    exit 0
}
$FilePlan = Get-FileRegistryPlan -CandidateFiles $CandidateFiles -KnownPackages $WorkspacePackages -Registry $Registry -ConfigHash $ConfigHash -SnapshotTree $Provenance.index_tree -PartialSelection:$PartialSelection
$QueueFiles = @($FilePlan.queue | Sort-Object path)
$QueuePackages = @($QueueFiles | ForEach-Object { $_.package } | Where-Object { $_ } | Sort-Object -Unique)
$Commands = @($QueueFiles | ForEach-Object {
    $package = [string]$_.package
    $path = [string]$_.path
    $safe = Convert-ToSafeName $path
    $manifestTarget = Join-Path $RunTargetRoot "files\$safe"
    $manifestMetadata = Join-Path $RunMetadataRoot "files\$safe"
    $manifestArgs = Get-MutestCommand -Package $package -WorkerTarget $manifestTarget -WorkerMetadata $manifestMetadata -MutationFilterPath $path
    $fallbackArgs = Get-MutestCommand -Package $package -WorkerTarget $manifestTarget -WorkerMetadata $manifestMetadata -BatchSizeOverride 1 -MutationFilterPath $path
    [ordered]@{ path = $path; package = $package; category = Get-PackageCategory $package; worker_scope = "single_production_file"; mutation_filter = "file:$path"; target_dir = $manifestTarget; metadata_dir = $manifestMetadata; args = @($manifestArgs); command = Get-MutestCommandText -Arguments $manifestArgs; fallback_batch_size = 1; fallback_args = @($fallbackArgs); fallback_command = Get-MutestCommandText -Arguments $fallbackArgs }
})
$CommandHash = Get-CommandHash -Commands $Commands
$RegistryDocument = Convert-ToOrderedRecord -Object $Registry
$RegistryDocument.schema_version = 3; $RegistryDocument.index_role = "mutation_wave_orchestrator"; $RegistryDocument.run_id = $RunId; $RegistryDocument.last_wave_id = $WaveId
$RegistryDocument.snapshot_mode = if ($IncludeWorkingTree) { "working_tree" } else { "index" }; $RegistryDocument.snapshot_index_tree = $Provenance.index_tree
$RegistryDocument.loc_policy = [ordered]@{ loc = "non_empty_source_lines"; loc_total = "physical_lines"; loc_hash = "content_hash_sha256"; excludes_blank_lines = $true; comments_included = $true }
$RegistryDocument.config_hash = $ConfigHash; $RegistryDocument.threshold_percent = $Threshold; $RegistryDocument.full_rescan = [bool]$FullRescan
$RegistryDocument.diff_scan = [ordered]@{ candidates = @($CandidateFiles).Count; queued = @($QueueFiles).Count; resumed = @($FilePlan.resumed).Count; deleted = @($FilePlan.deleted).Count; queue_policy = "one-file-workers-new-or-hash-changed-or-pending-flags" }
$RegistryDocument.files = @($FilePlan.files)
$WaveSummary = [ordered]@{ wave_id = $WaveId; run_id = $RunId; started_at = [DateTime]::UtcNow.ToString("o"); completed_at = $null; status = "planned"; counts = [ordered]@{ candidates = @($CandidateFiles).Count; queued = @($QueueFiles).Count; running = 0; completed = 0; blocked = 0; timeout = 0; needs_tests = 0; needs_rerun = @($QueueFiles).Count; needs_rescan = 0 }; artifact_root = $RunEvidenceRoot; registry_revision = [int](Get-OptionalProperty $Registry "registry_revision" 0) }
$Manifest = [ordered]@{
    schema_version = 1; run_id = $RunId; generated_at = [DateTime]::UtcNow.ToString("o"); repo_root = $RepoRoot
    provenance = $Provenance; packages = $SelectedPackages; queue_packages = $QueuePackages; ignored_tests = @(Get-IgnoredTests); resources = $ResourcePlan
    config = [ordered]@{ nightly = $Nightly; depth = $Depth; batch_size = $BatchSize; package_timeout_minutes = $PackageTimeoutMinutes; max_hours = $MaxHours; target_dir = $TargetDir; metadata_root = $MetadataRoot; evidence_root = $EvidenceRoot; requested_workers = $MaxWorkers; include_working_tree = [bool]$IncludeWorkingTree; threshold_percent = $Threshold; full_rescan = [bool]$FullRescan; registry_path = $RegistryPathAbsolute; defect_log_path = $DefectLogPathAbsolute; auto_update_tests = [bool]$AutoUpdateTests; test_update_command = $TestUpdateCommand; mutest_cargo_path = $MutestCargoPathAbsolute; mutest_cargo_path_source = $MutestCargoPathSource; mutest_native_lib_path = $MutestNativeLibPathAbsolute; target_selector_policy = "auto-lib-bin-v1"; temp_environment_policy = "worker-private-tmp-v1" }
    command_hash = $CommandHash; config_hash = $ConfigHash; commands = $Commands; registry_path = $RegistryPathAbsolute; registry_revision = [int](Get-OptionalProperty $Registry "registry_revision" 0); wave_id = $WaveId; wave = $WaveSummary; index = [ordered]@{ role = "mutation_wave_orchestrator"; path = $RegistryPathAbsolute; row_count = @($FilePlan.files).Count; unique_path_count = @($FilePlan.files | ForEach-Object path | Sort-Object -Unique).Count }
    file_scan = [ordered]@{ mode = if ($FullRescan) { "full_rescan" } else { "diff" }; candidate_files = @($CandidateFiles).Count; queued_files = @($QueueFiles).Count; resumed_files = @($FilePlan.resumed).Count; deleted_files = @($FilePlan.deleted).Count; registry_role = "mutation_wave_orchestrator" }
}

if ($PlanOnly) {
    [void](New-Item -ItemType Directory -Force -Path $RunEvidenceRoot, (Join-Path $RunEvidenceRoot "packages"))
    Write-AtomicJson -Path (Join-Path $RunEvidenceRoot "manifest.json") -Value $Manifest
    $planReport = [ordered]@{
        schema_version = 1
        run_id = $RunId
        status = "planned"
        provenance = $Provenance
        resources = $ResourcePlan
        packages = $SelectedPackages
        queue_packages = $QueuePackages
        file_scan = $Manifest.file_scan
        registry_path = $RegistryPathAbsolute
        defect_log_path = $DefectLogPathAbsolute
        registry_revision = [int](Get-OptionalProperty $Registry "registry_revision" 0)
        wave = $WaveSummary
        file_index = [ordered]@{ path = $RegistryPathAbsolute; row_count = @($FilePlan.files).Count; unique_path_count = @($FilePlan.files | ForEach-Object path | Sort-Object -Unique).Count }
        ignored_tests = @(Get-IgnoredTests)
        command_hash = $CommandHash
        config_hash = $ConfigHash
        commands = $Commands
        limitations = @("plan-only: no mutants evaluated and canonical registry is unchanged", "safe mutation contexts only", "registry dependencies are not mutation targets", "one worker owns one repo-relative production file")
    }
    Write-AtomicJson -Path (Join-Path $RunEvidenceRoot "parallel-report.json") -Value $planReport
    $planMarkdown = @(
        "# mutest-rs audit plan $RunId",
        "",
        "- requested workers: $($ResourcePlan.requested_workers)",
        "- effective workers: $($ResourcePlan.effective_workers)",
        "- packages: $(@($SelectedPackages).Count)",
        "- candidate files: $(@($CandidateFiles).Count)",
        "- queued files: $(@($FilePlan.queue).Count)",
        "- resumed files: $(@($FilePlan.resumed).Count)",
        "- scan mode: $(if ($FullRescan) { 'full_rescan' } else { 'diff' })",
        "- status: planned",
        "",
        "## Command manifest",
        "",
        "Persisted in `manifest.json` and `$RegistryPathAbsolute`; no mutants were evaluated in PlanOnly mode."
    )
    $planMarkdown | Set-Content -LiteralPath (Join-Path $RunEvidenceRoot "parallel-report.md") -Encoding UTF8
    $Manifest | ConvertTo-Json -Depth 40
    exit 0
}

$WaveSummary.status = "running"
$RegistryDocument.waves = @($Registry.waves) + @($WaveSummary)
foreach ($fileRecord in @($QueueFiles)) {
    $fileRecord.last_wave_id = $WaveId
    $fileRecord.wave_status = "queued"
    $fileRecord.wave_updated_at = [DateTime]::UtcNow.ToString("o")
}
Write-CanonicalRegistry -Registry $RegistryDocument -Path $RegistryPathAbsolute
[void](New-Item -ItemType Directory -Force -Path $RunEvidenceRoot, (Join-Path $RunEvidenceRoot "waves"))
$EventPath = Join-Path $RunEvidenceRoot "events.jsonl"
$CheckpointPath = Join-Path $RunEvidenceRoot "checkpoint.json"
$Results = New-Object System.Collections.Generic.List[object]
$FileByPath = @{}
foreach ($fileRecord in @($FilePlan.files)) { $FileByPath[([string]$fileRecord.path).ToLowerInvariant()] = $fileRecord }
$CompletedFiles = New-Object 'System.Collections.Generic.HashSet[string]' -ArgumentList ([StringComparer]::OrdinalIgnoreCase)
$PendingFiles = New-Object 'System.Collections.Generic.Queue[object]'
$RetryFiles = New-Object 'System.Collections.Generic.Queue[object]'
if ($Resume) {
    if (-not (Test-Path -LiteralPath $CheckpointPath)) { throw "Resume requested but checkpoint is missing: $CheckpointPath" }
    $checkpoint = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
    if ([string]$checkpoint.command_hash -ne $CommandHash -or [string]$checkpoint.provenance.head -ne [string]$Provenance.head -or [string]$checkpoint.provenance.index_tree -ne [string]$Provenance.index_tree -or ((@($checkpoint.packages) -join '|') -ne ($SelectedPackages -join '|'))) {
        throw "Resume rejected: commit/tree/index/tool/package or command manifest drift detected."
    }
    foreach ($checkpointResult in @($checkpoint.results)) {
        $checkpointPath = [string](Get-OptionalProperty $checkpointResult "path" "")
        if ($checkpointResult.status -eq "completed" -and -not [string]::IsNullOrWhiteSpace($checkpointPath) -and $FileByPath.ContainsKey($checkpointPath.ToLowerInvariant())) {
            [void]$CompletedFiles.Add($checkpointPath); [void]$Results.Add($checkpointResult)
        }
    }
    foreach ($retryPath in @($checkpoint.retry_files)) {
        $retryKey = ([string]$retryPath).ToLowerInvariant()
        if ($FileByPath.ContainsKey($retryKey)) { $RetryFiles.Enqueue($FileByPath[$retryKey]) }
    }
}
foreach ($fileRecord in $QueueFiles) { if (-not $CompletedFiles.Contains([string]$fileRecord.path)) { $PendingFiles.Enqueue($fileRecord) } }
Write-AtomicJson -Path (Join-Path $RunEvidenceRoot "manifest.json") -Value $Manifest
Write-Event -Path $EventPath -Event "run_started" -Data ([ordered]@{ run_id = $RunId; wave_id = $WaveId; requested_workers = $MaxWorkers; effective_workers = $ResourcePlan.effective_workers; package_count = @($QueuePackages).Count; file_count = @($QueueFiles).Count; command_hash = $CommandHash; full_rescan = [bool]$FullRescan })

$Active = @{}
$nextSlot = 0
$deadline = [DateTime]::UtcNow.AddHours($MaxHours)
try {
while ($RetryFiles.Count -gt 0 -or $PendingFiles.Count -gt 0 -or $Active.Count -gt 0) {
    $drift = Test-ProvenanceDrift -Expected $Provenance
    if ($drift.drifted) {
        foreach ($worker in @($Active.Values)) { $worker.timed_out = $false; Stop-WorkerTree -Worker $worker; [void]$Results.Add((Complete-MutestWorker -Worker $worker -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath)) }
        Write-Event -Path $EventPath -Event "provenance_drift" -Data $drift
        throw "Provenance drift detected ($($drift.field)); active workers stopped and marked for rerun."
    }
    while (($RetryFiles.Count -gt 0 -or $PendingFiles.Count -gt 0) -and $Active.Count -lt [int]$ResourcePlan.effective_workers) {
        $isRetry = $RetryFiles.Count -gt 0
        $fileRecord = if ($isRetry) { $RetryFiles.Dequeue() } else { $PendingFiles.Dequeue() }
        $package = [string]$fileRecord.package
        $workerBatchSize = 1
        $fileRecord.wave_status = "running"; $fileRecord.wave_updated_at = [DateTime]::UtcNow.ToString("o")
        try {
            $worker = Start-MutestWorker -Package $package -FilePath $fileRecord.path -WaveId $WaveId -Slot ($nextSlot % [int]$ResourcePlan.effective_workers) -RunTargetRoot $RunTargetRoot -RunMetadataRoot $RunMetadataRoot -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath -BatchSizeOverride $workerBatchSize
            $nextSlot++
            $Active[$worker.process.Id] = $worker
        } catch {
            $safe = Convert-ToSafeName $fileRecord.path
            $failedEvidence = Join-Path $RunEvidenceRoot "$(Join-Path (Convert-ToSafeName $WaveId) (Join-Path 'files' $safe))"
            [void](New-Item -ItemType Directory -Force -Path $failedEvidence)
            $failed = [ordered]@{ package = $package; path = $fileRecord.path; wave_id = $WaveId; category = Get-PackageCategory $package; status = "blocked"; exit_code = $null; timed_out = $false; batch_size = $workerBatchSize; error = $_.Exception.Message; command = "cargo " + ((Get-MutestCommand -Package $package -WorkerTarget (Join-Path $RunTargetRoot "files\$safe") -WorkerMetadata (Join-Path $RunMetadataRoot "files\$safe") -BatchSizeOverride $workerBatchSize -MutationFilterPath $fileRecord.path) -join ' '); stats = Get-MutestStats -MetadataPath (Join-Path $RunMetadataRoot "files\$safe") }
            Write-AtomicJson -Path (Join-Path $failedEvidence "package-report.json") -Value $failed
            [void]$Results.Add($failed); Write-Event -Path $EventPath -Event "worker_start_failed" -Data $failed
        }
    }
    foreach ($key in @($Active.Keys)) {
        $worker = $Active[$key]
        $expired = [DateTime]::UtcNow -gt $worker.deadline
        if ($expired -or $worker.process.HasExited) {
            if ($expired -and -not $worker.process.HasExited) { $worker.timed_out = $true; Stop-WorkerTree -Worker $worker }
            $report = Complete-MutestWorker -Worker $worker -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath
            $Active.Remove($key)
            $stdoutText = if (Test-Path -LiteralPath $worker.stderr) { Get-Content -LiteralPath $worker.stderr -Raw } else { "" }
            $batchCrash = $report.status -eq "blocked" -and $worker.batch_size -gt 1 -and $stdoutText -match '(?i)(batch[^\r\n]*(crash|panic|fail)|(?:panic|crash)[^\r\n]*mutant|mutant[^\r\n]*batch)'
            if ($batchCrash) {
                [void]$Results.Remove($report); $RetryFiles.Enqueue($FileByPath[([string]$worker.path).ToLowerInvariant()])
                Write-Event -Path $EventPath -Event "batch_retry_scheduled" -Data ([ordered]@{ path = $worker.path; package = $worker.package; from_batch_size = $worker.batch_size; retry_batch_size = 1 })
            } else { [void]$Results.Add($report) }
            $completedRecord = $FileByPath[([string]$worker.path).ToLowerInvariant()]
            $completedRecord.wave_status = if ($report.status -eq "timeout") { "timeout" } elseif ($report.status -eq "completed") { "completed" } else { "blocked" }
            $completedRecord.wave_updated_at = [DateTime]::UtcNow.ToString("o")
            $completedRecord.last_wave_id = $WaveId
            $completedRecord.status = $completedRecord.wave_status
            $completedRecord.needs_rerun = $report.status -ne "completed"
            $RegistryDocument.files = @($FilePlan.files)
            Write-CanonicalRegistry -Registry $RegistryDocument -Path $RegistryPathAbsolute
            Write-Event -Path $EventPath -Event "queue_refill" -Data ([ordered]@{ reason = "worker_terminal_refill"; wave_id = $WaveId; completed_path = $worker.path; pending = $PendingFiles.Count; active = $Active.Count })
            $checkpoint = [ordered]@{ schema_version = 2; run_id = $RunId; wave_id = $WaveId; command_hash = $CommandHash; provenance = $Provenance; packages = $SelectedPackages; results = @($Results.ToArray()); pending_files = @($PendingFiles.ToArray() | ForEach-Object { $_.path }); retry_files = @($RetryFiles.ToArray() | ForEach-Object { $_.path }); active = @($Active.Values | ForEach-Object { $_.path }) }
            Write-AtomicJson -Path $CheckpointPath -Value $checkpoint
        }
    }
    if ([DateTime]::UtcNow -gt $deadline) {
        foreach ($worker in @($Active.Values)) { $worker.timed_out = $true; Stop-WorkerTree -Worker $worker; [void]$Results.Add((Complete-MutestWorker -Worker $worker -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath)) }
        $Active.Clear(); throw "Audit max-hours deadline exceeded ($MaxHours h)."
    }
    if ($Active.Count -gt 0) { Start-Sleep -Seconds $PollSeconds }
}
} finally {
    foreach ($worker in @($Active.Values)) {
        Stop-WorkerTree -Worker $worker
        Write-Event -Path $EventPath -Event "worker_cleanup" -Data ([ordered]@{ package = $worker.package; slot = $worker.slot; reason = "finally" })
    }
    $Active.Clear()
}

$Defects = New-Object System.Collections.Generic.List[object]
$RescanFiles = New-Object System.Collections.Generic.List[object]
foreach ($fileRecord in @($FilePlan.files | Where-Object { $_.resume_source -ne "compatible_registry" -and $_.status -ne "deleted_from_snapshot" })) {
    $fileResult = @($Results | Where-Object { ([string](Get-OptionalProperty $_ "path" "")).ToLowerInvariant() -eq ([string]$fileRecord.path).ToLowerInvariant() } | Select-Object -Last 1)
    if (@($fileResult).Count -eq 0) {
        $fileRecord.status = "blocked"; $fileRecord.wave_status = "blocked"; $fileRecord.needs_rerun = $true; $fileRecord.updated_at = [DateTime]::UtcNow.ToString("o")
        continue
    }
    $fileReport = $fileResult[0]
    $stats = $fileReport.stats
    foreach ($field in @("killed", "survived", "timeout", "no_coverage", "compile_error")) { $fileRecord.$field = [int]$stats.$field }
    $fileRecord.mutation_score_ratio = $stats.mutation_score
    $fileRecord.mutation_score = Get-FileScorePercent -Stats $stats
    $fileRecord.last_scan_hash = $fileRecord.hash
    $fileRecord.last_scan_run_id = $RunId
    $fileRecord.scan_count = [int]$fileRecord.scan_count + 1
    $fileRecord.wave_count = [int](Get-OptionalProperty $fileRecord "wave_count" 0) + 1
    $fileRecord.last_wave_id = $WaveId
    $fileRecord.updated_at = [DateTime]::UtcNow.ToString("o")
    $lowCoverage = ($null -eq $fileRecord.mutation_score) -or ([double]$fileRecord.mutation_score -le $Threshold) -or ([int]$fileRecord.no_coverage -gt 0)
    $stderrText = if ($fileReport.stderr -and (Test-Path -LiteralPath $fileReport.stderr)) { Get-Content -LiteralPath $fileReport.stderr -Raw } else { "" }
    $noEvidence = ([int](Get-OptionalProperty $stats "metadata_files" 0) -eq 0 -and [int]$stats.generated -eq 0 -and [int]$stats.evaluated -eq 0)
    $compilerError = $stderrText -match '(?i)(internal compiler error|rustc.*panic|compiler unexpectedly|could not compile)'
    if ($fileReport.status -ne "completed" -or $noEvidence -or $compilerError) {
        $blocker = Get-ExecutionBlocker -Text $stderrText
        $fileRecord.status = if ($fileReport.timed_out) { "timeout" } else { "blocked" }; $fileRecord.wave_status = $fileRecord.status; $fileRecord.needs_rerun = $true; $fileRecord.needs_tests = $false
        $fileRecord.blocker_code = if ($fileReport.timed_out) { "mutation_timeout" } else { $blocker.code }
        $fileRecord.blocker_family = if ($fileReport.timed_out) { "runtime" } else { $blocker.family }
        $fileRecord.blocker_reason = if ($fileReport.timed_out) { "worker deadline exceeded" } else { $blocker.reason }
        $fileRecord.next_action = if ($fileReport.timed_out) { "inspect process-tree evidence and rerun the file" } else { $blocker.next_action }
        $defectRecord = [ordered]@{ defect_id = [guid]::NewGuid().ToString("N"); type = if ($fileReport.timed_out) { "mutation_timeout" } elseif ($compilerError) { "mutation_compiler_error" } else { "mutation_no_evidence" }; blocker_code = $fileRecord.blocker_code; blocker_family = $fileRecord.blocker_family; blocker_reason = $fileRecord.blocker_reason; path = $fileRecord.path; package = $fileRecord.package; wave_id = $WaveId; evidence = @($fileReport.stderr, $fileReport.stdout); recommendation = $fileRecord.next_action }
        $fileRecord.defects = @($fileRecord.defects) + $defectRecord; [void]$Defects.Add($defectRecord)
    } elseif ($lowCoverage) {
        $fileRecord.status = "needs_tests"; $fileRecord.wave_status = "needs_tests"; $fileRecord.needs_tests = $true; $fileRecord.needs_rerun = $false; $fileRecord.needs_rescan = $false
        $fileRecord.recommendations = @("apply ZOMBIE-D focused test update", "rescan after test update")
        $defectRecord = [ordered]@{ defect_id = [guid]::NewGuid().ToString("N"); type = if ($fileRecord.no_coverage -gt 0) { "no_coverage" } else { "survived_mutants" }; path = $fileRecord.path; package = $fileRecord.package; wave_id = $WaveId; score_percent = $fileRecord.mutation_score; killed = $fileRecord.killed; survived = $fileRecord.survived; no_coverage = $fileRecord.no_coverage; recommendation = "add focused tests, then rescan this file" }
        $fileRecord.defects = @($fileRecord.defects) + $defectRecord; [void]$Defects.Add($defectRecord)
        $update = Invoke-TestUpdateHook -FileRecord $fileRecord -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath
        $fileRecord.test_update_status = $update.status
        if ($update.status -eq "completed") {
            $fileRecord.status = "needs_rescan"; $fileRecord.wave_status = "needs_rescan"; $fileRecord.needs_tests = $false; $fileRecord.needs_rescan = $true; $fileRecord.needs_rerun = $true
            [void]$RescanFiles.Add($fileRecord)
        }
    } else {
        $fileRecord.status = "completed"; $fileRecord.wave_status = "completed"; $fileRecord.needs_tests = $false; $fileRecord.needs_rerun = $false; $fileRecord.needs_rescan = $false; $fileRecord.test_update_status = "not_needed"
    }
    $fileRecord.wave_updated_at = [DateTime]::UtcNow.ToString("o")
}

foreach ($fileRecord in @($RescanFiles.ToArray())) {
    Write-Event -Path $EventPath -Event "rescan_started" -Data ([ordered]@{ package = $fileRecord.package; path = $fileRecord.path; wave_id = $WaveId; reason = "test_update_completed" })
    $rescan = Invoke-SynchronousRescan -Package $fileRecord.package -FilePath $fileRecord.path -WaveId $WaveId -RunTargetRoot $RunTargetRoot -RunMetadataRoot $RunMetadataRoot -RunEvidenceRoot $RunEvidenceRoot -EventPath $EventPath
    $fileRecord.status = [string]$rescan.status
        $fileRecord.wave_status = if ($rescan.status -eq "completed") { "completed" } else { [string]$rescan.status }
        $fileRecord.needs_rescan = $false
        $fileRecord.needs_rerun = $rescan.status -ne "completed"
        $fileRecord.needs_tests = $false
        $fileRecord.rescan_run_id = $RunId
        $score = Get-FileScorePercent -Stats $rescan.stats
        $fileRecord.mutation_score = $score
        $fileRecord.mutation_score_ratio = $rescan.stats.mutation_score
        $fileRecord.killed = [int]$rescan.stats.killed; $fileRecord.survived = [int]$rescan.stats.survived; $fileRecord.no_coverage = [int]$rescan.stats.no_coverage
        if ($rescan.status -eq "completed" -and (($null -eq $score) -or ($score -le $Threshold) -or $fileRecord.no_coverage -gt 0)) {
            $fileRecord.status = "needs_tests"; $fileRecord.needs_tests = $true; $fileRecord.needs_rerun = $false
        }
        $fileRecord.last_wave_id = $WaveId; $fileRecord.wave_updated_at = [DateTime]::UtcNow.ToString("o")
    Write-Event -Path $EventPath -Event "rescan_completed" -Data ([ordered]@{ package = $fileRecord.package; path = $fileRecord.path; wave_id = $WaveId; status = $fileRecord.status; mutation_score = $fileRecord.mutation_score })
}

if (-not [string]::IsNullOrWhiteSpace($DefectLogPathAbsolute)) {
    $defectParent = Split-Path -Parent $DefectLogPathAbsolute
    [void](New-Item -ItemType Directory -Force -Path $defectParent)
    foreach ($defectEntry in @($Defects.ToArray())) { Add-Content -LiteralPath $DefectLogPathAbsolute -Value ($defectEntry | ConvertTo-Json -Depth 30 -Compress) -Encoding UTF8 }
}
$defectProtocolPath = Join-Path $RunEvidenceRoot "defect-remediation.json"
$DefectProtocol = Write-DefectProtocolPlan -Path $defectProtocolPath -RunId $RunId -Defects $Defects.ToArray() -EventPath $EventPath
$RegistryDocument.last_scan_run_id = $RunId
$RegistryDocument.defect_protocol_path = $defectProtocolPath
$RegistryDocument.files = @($FilePlan.files)
$WaveSummary.completed_at = [DateTime]::UtcNow.ToString("o")
$WaveSummary.status = if (@($FilePlan.files | Where-Object { $_.needs_rerun }).Count -gt 0) { "completed_with_followups" } else { "completed" }
$WaveSummary.counts = Get-FileStatusCounts -Rows @($FilePlan.files | Where-Object { [string](Get-OptionalProperty $_ "last_wave_id" "") -eq $WaveId })
$WaveSummary.registry_revision = [int](Get-OptionalProperty $RegistryDocument "registry_revision" 0) + 1
$RegistryDocument.waves = @($RegistryDocument.waves | Where-Object { [string](Get-OptionalProperty $_ "wave_id" "") -ne $WaveId }) + @($WaveSummary)
Write-CanonicalRegistry -Registry $RegistryDocument -Path $RegistryPathAbsolute

$aggregate = [ordered]@{ generated = 0; evaluated = 0; killed = 0; survived = 0; no_coverage = 0; compile_error = 0; timeout = 0; flaky = 0; equivalent = 0; unknown = 0 }
$production = [ordered]@{ generated = 0; evaluated = 0; killed = 0; survived = 0; no_coverage = 0; compile_error = 0; timeout = 0; flaky = 0; equivalent = 0; unknown = 0 }
$testSupport = [ordered]@{ generated = 0; evaluated = 0; killed = 0; survived = 0; no_coverage = 0; compile_error = 0; timeout = 0; flaky = 0; equivalent = 0; unknown = 0 }
foreach ($packageReport in @($Results.ToArray())) { foreach ($field in @($aggregate.Keys)) { $aggregate[$field] += [int]$packageReport.stats.$field } }
foreach ($packageReport in @($Results.ToArray())) {
    $bucket = if ((Get-PackageCategory $packageReport.package) -eq "test-support") { $testSupport } else { $production }
    foreach ($field in @($production.Keys)) { $bucket[$field] += [int]$packageReport.stats.$field }
}
$denom = $aggregate.killed + $aggregate.survived
$aggregate.mutation_score = if ($denom -gt 0) { [Math]::Round([double]$aggregate.killed / $denom, 6) } else { $null }
$aggregate.score_denominator = $denom
foreach ($bucket in @($production, $testSupport)) {
    $bucketDenom = $bucket.killed + $bucket.survived
    $bucket.mutation_score = if ($bucketDenom -gt 0) { [Math]::Round([double]$bucket.killed / $bucketDenom, 6) } else { $null }
    $bucket.score_denominator = $bucketDenom
}
$FinalStatusCounts = Get-FileStatusCounts -Rows @($FilePlan.files)
$DefectCounts = [ordered]@{ survived_mutants = 0; mutation_compiler_error = 0; mutation_timeout = 0; mutation_no_evidence = 0; no_coverage = 0 }
foreach ($defect in @($Defects.ToArray())) {
    $defectType = [string](Get-OptionalProperty $defect "type" "unknown")
    if (-not $DefectCounts.Contains($defectType)) { $DefectCounts[$defectType] = 0 }
    $DefectCounts[$defectType]++
}
$final = [ordered]@{
    schema_version = 3; run_id = $RunId; wave_id = $WaveId; status = $WaveSummary.status; provenance = $Provenance; resources = $ResourcePlan
    workers = @($Results.ToArray()); aggregate = $aggregate; categories = [ordered]@{ production = $production; test_support = $testSupport }; ignored_tests = @(Get-IgnoredTests); command_hash = $CommandHash; config_hash = $ConfigHash; commands = $Commands
    file_scan = $Manifest.file_scan; registry_path = $RegistryPathAbsolute; registry_revision = [int](Get-OptionalProperty $RegistryDocument "registry_revision" 0); wave = $WaveSummary; index = [ordered]@{ role = "mutation_wave_orchestrator"; path = $RegistryPathAbsolute; row_count = @($FilePlan.files).Count; unique_path_count = @($FilePlan.files | ForEach-Object path | Sort-Object -Unique).Count }; defect_log_path = $DefectLogPathAbsolute; defect_protocol = $DefectProtocol
    defects = @($Defects.ToArray()); defect_counts = $DefectCounts
    recommendations = @($FilePlan.files | ForEach-Object { @($_.recommendations) } | Where-Object { $_ } | Sort-Object -Unique)
    limitations = @("safe mutation contexts only", "compile/infra/equivalent cases excluded from mutation score", "registry dependencies are not mutation targets", "file-registry.json is the only authoritative per-file index; worker reports are evidence")
}
$RegistryDocument.summary = [ordered]@{
    run_id = $RunId; wave_id = $WaveId; status = $WaveSummary.status; mutation_score = $aggregate.mutation_score
    generated = $aggregate.generated; evaluated = $aggregate.evaluated; killed = $aggregate.killed; survived = $aggregate.survived
    timeout = $aggregate.timeout; compile_error = $aggregate.compile_error; files_total = @($FilePlan.files).Count
    files_completed = $FinalStatusCounts.completed; files_needs_tests = $FinalStatusCounts.needs_tests; files_blocked = $FinalStatusCounts.blocked
    files_timeout = $FinalStatusCounts.timeout; files_needs_rerun = $FinalStatusCounts.needs_rerun; files_needs_rescan = $FinalStatusCounts.needs_rescan
    defects = @($Defects.ToArray()).Count; defect_counts = $DefectCounts; evidence_root = $RunEvidenceRoot; report_path = (Join-Path $RunEvidenceRoot "parallel-report.json"); defect_protocol_path = $defectProtocolPath
}
Write-CanonicalRegistry -Registry $RegistryDocument -Path $RegistryPathAbsolute
Write-AtomicJson -Path (Join-Path $RunEvidenceRoot "parallel-report.json") -Value $final
$markdown = @("# mutest-rs audit $RunId", "", "- wave: $WaveId", "- requested workers: $($ResourcePlan.requested_workers)", "- effective workers: $($ResourcePlan.effective_workers)", "- packages scanned: $(@($QueuePackages).Count)", "- candidate files: $(@($CandidateFiles).Count)", "- queued files: $(@($QueueFiles).Count)", "- mutation score: $($aggregate.mutation_score)", "- index rows: $(@($FilePlan.files).Count)", "- unique index paths: $(@($FilePlan.files | ForEach-Object path | Sort-Object -Unique).Count)", "- files needing tests: $($FinalStatusCounts.needs_tests)", "- files needing rerun: $($FinalStatusCounts.needs_rerun)", "- defects: $(@($Defects.ToArray()).Count)", "", "## Counts", "", ($aggregate | ConvertTo-Json -Compress), "", "## Queue policy", "", "Workers refill immediately after every terminal file; each worker uses an isolated Cargo target directory and a repo-relative --filter-mutations file:<path>. File registry compatibility is SHA-256 + config-hash based; -FullRescan invalidates compatible records.", "", "## Canonical index", "", "See `$RegistryPathAbsolute`; it is the only authoritative per-file status index and stores wave summaries without duplicate file rows.", "", "## Defect protocol", "", "See `defect-remediation.json` and `defects.jsonl`; TaskFlow state was not restored or reconciled.")
$markdown | Set-Content -LiteralPath (Join-Path $RunEvidenceRoot "parallel-report.md") -Encoding UTF8
Write-Event -Path $EventPath -Event "run_completed" -Data ([ordered]@{ run_id = $RunId; wave_id = $WaveId; status = $final.status; mutation_score = $aggregate.mutation_score; packages = @($QueuePackages).Count; files = @($CandidateFiles).Count; index_rows = @($FilePlan.files).Count; unique_index_paths = @($FilePlan.files | ForEach-Object path | Sort-Object -Unique).Count; needs_tests = $FinalStatusCounts.needs_tests; needs_rerun = $FinalStatusCounts.needs_rerun; defects = @($Defects.ToArray()).Count; defect_protocol = $defectProtocolPath })
$final | ConvertTo-Json -Depth 40
