<# 
VIDA Windows installer.

Usage:
  pwsh -ExecutionPolicy Bypass -File install.ps1 install
  pwsh -ExecutionPolicy Bypass -File install.ps1 upgrade -Version v0.9.0
  pwsh -ExecutionPolicy Bypass -File install.ps1 doctor
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet("install", "init", "upgrade", "use", "doctor", "help")]
    [string] $Command = "help",

    [string] $Version,
    [string] $Archive,
    [ValidateSet("auto", "windows-x86_64")]
    [string] $Target = "auto",
    [string] $Root,
    [string] $BinDir,
    [string] $Bins,
    [int] $KeepReleases = 3,
    [switch] $Force,
    [switch] $DryRun
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"

$RepoSlug = if ($env:VIDA_INSTALL_REPO) { $env:VIDA_INSTALL_REPO } else { "pomazanbohdan/vida-stack" }
if (-not $Version) { $Version = if ($env:VIDA_VERSION) { $env:VIDA_VERSION } else { "latest" } }
if (-not $Archive) { $Archive = if ($env:VIDA_ARCHIVE_FILE) { $env:VIDA_ARCHIVE_FILE } else { "" } }
if (-not $Root) {
    $Root = if ($env:VIDA_HOME) {
        $env:VIDA_HOME
    } else {
        Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "vida-stack"
    }
}
if (-not $BinDir) {
    $BinDir = if ($env:VIDA_BIN_DIR) {
        $env:VIDA_BIN_DIR
    } else {
        Join-Path $Root "bin"
    }
}
if (-not $Bins) { $Bins = if ($env:VIDA_INSTALL_BINS) { $env:VIDA_INSTALL_BINS } else { "all" } }
if ($env:VIDA_KEEP_RELEASES) { $KeepReleases = [int] $env:VIDA_KEEP_RELEASES }

$Root = [System.IO.Path]::GetFullPath($Root)
$BinDir = [System.IO.Path]::GetFullPath($BinDir)

function Write-Log {
    param([string] $Message)
    Write-Host "[vida-installer] $Message"
}

function Fail {
    param([string] $Message)
    Write-Error "[vida-installer] ERROR: $Message"
    exit 1
}

function Show-Usage {
    @"
VIDA Windows installer

Usage:
  pwsh -ExecutionPolicy Bypass -File install.ps1 <install|init|upgrade|use|doctor|help> [options]

Options:
  -Version TAG      Release tag to install. Defaults to latest.
  -Archive PATH     Local release zip instead of GitHub download.
  -Target TARGET    Release asset target: auto|windows-x86_64.
  -BinDir PATH      Directory for launcher .cmd shims. Defaults to %LOCALAPPDATA%\vida-stack\bin.
  -Bins LIST        Comma-separated launchers to expose: vida,taskflow,docflow,all.
  -Root PATH        Install root. Defaults to %LOCALAPPDATA%\vida-stack.
  -Force            Overwrite an already installed release of the same version.
  -DryRun           Print planned actions without changing files.

Examples:
  irm https://github.com/pomazanbohdan/vida-stack/releases/latest/download/vida-install.ps1 -OutFile vida-install.ps1
  pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install
  pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 upgrade -Version v0.9.0
  pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install -Bins vida,taskflow,docflow
  pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 doctor
"@
}

function Normalize-Bins {
    param([string] $Raw)
    $clean = ($Raw -replace "\s+", "")
    if ($clean -eq "all") {
        return @("vida", "taskflow", "docflow")
    }
    $values = @()
    foreach ($item in ($clean -split ",")) {
        if (-not $item) { continue }
        if ($item -notin @("vida", "taskflow", "docflow")) {
            Fail "Unsupported -Bins entry: $item. Allowed: vida,taskflow,docflow,all"
        }
        $values += $item
    }
    if ($values.Count -eq 0) { Fail "-Bins must include at least one launcher" }
    return $values
}

$SelectedBins = Normalize-Bins $Bins

function Test-BinSelected {
    param([string] $Name)
    return $SelectedBins -contains $Name
}

function Resolve-TargetSuffix {
    if ($Target -eq "auto" -or $Target -eq "windows-x86_64") {
        return "-windows-x86_64"
    }
    Fail "Unsupported install target: $Target. Allowed: auto|windows-x86_64"
}

function Get-ArchiveBase {
    param([string] $Tag)
    return "vida-stack-$Tag$(Resolve-TargetSuffix)"
}

function Resolve-Version {
    if ($Version -ne "latest") { return $Version }
    if ($Archive) {
        $name = [System.IO.Path]::GetFileName($Archive)
        if ($name -match "^vida-stack-(.+?)(-windows-x86_64)?\.zip$") {
            return $Matches[1]
        }
        Fail "Unable to infer version from local archive name: $name"
    }

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $payload = Invoke-RestMethod -Uri "https://api.github.com/repos/$RepoSlug/releases/latest" -Headers @{ "User-Agent" = "vida-installer" }
    if (-not $payload.tag_name) { Fail "Missing tag_name in latest-release payload" }
    return [string] $payload.tag_name
}

function Download-File {
    param([string] $Url, [string] $Destination)
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing -Headers @{ "User-Agent" = "vida-installer" }
}

function Verify-ArchiveChecksum {
    param([string] $ArchivePath, [string] $ChecksumPath)
    if (-not (Test-Path -LiteralPath $ChecksumPath)) { return }
    if ($DryRun) {
        Write-Log "Would verify checksum for $([System.IO.Path]::GetFileName($ArchivePath))"
        return
    }

    $archiveName = [System.IO.Path]::GetFileName($ArchivePath)
    $line = Get-Content -LiteralPath $ChecksumPath | Where-Object { $_ -match "\s+$([regex]::Escape($archiveName))$" } | Select-Object -First 1
    if (-not $line) { Fail "Checksum file does not contain entry for $archiveName" }
    $expected = ($line -split "\s+")[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $ArchivePath).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Fail "Checksum mismatch for $archiveName. Expected $expected, got $actual"
    }
}

function Copy-ProjectFile {
    param([string] $SourcePath, [string] $TargetPath, [string] $Label)
    if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) { Fail "Missing project bootstrap source: $SourcePath" }
    if ($DryRun) {
        Write-Log "Would copy $Label into $TargetPath"
        return
    }
    if ((Test-Path -LiteralPath $TargetPath) -and -not $Force) {
        Write-Log "Keeping existing ${Label}: $TargetPath"
        return
    }
    New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($TargetPath)) | Out-Null
    Copy-Item -LiteralPath $SourcePath -Destination $TargetPath -Force
}

function Copy-ProjectTree {
    param([string] $SourcePath, [string] $TargetPath, [string] $Label)
    if (-not (Test-Path -LiteralPath $SourcePath -PathType Container)) { Fail "Missing project bootstrap tree: $SourcePath" }
    if ($DryRun) {
        Write-Log "Would copy $Label into $TargetPath"
        return
    }
    if ((Test-Path -LiteralPath $TargetPath) -and -not $Force) {
        Write-Log "Keeping existing ${Label}: $TargetPath"
        return
    }
    Remove-Item -LiteralPath $TargetPath -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($TargetPath)) | Out-Null
    Copy-Item -LiteralPath $SourcePath -Destination $TargetPath -Recurse -Force
}

function Ensure-RuntimeConfigScaffold {
    param([string] $ReleaseRoot)
    $targetConfig = Join-Path $ReleaseRoot "vida.config.yaml"
    if (Test-Path -LiteralPath $targetConfig) { return }
    $template = Join-Path $ReleaseRoot "install\assets\vida.config.yaml.template"
    if (-not (Test-Path -LiteralPath $template)) {
        Fail "Missing runtime config template: $template"
    }
    if ($DryRun) {
        Write-Log "Would scaffold $targetConfig from $template"
        return
    }
    Copy-Item -LiteralPath $template -Destination $targetConfig -Force
}

function Get-RuntimeBinaryPath {
    param([string] $ReleaseRoot, [string] $Name)
    $exe = Join-Path $ReleaseRoot "bin\$Name.exe"
    if (Test-Path -LiteralPath $exe) { return $exe }
    return (Join-Path $ReleaseRoot "bin\$Name")
}

function Write-EnvironmentFiles {
    $envPs1 = Join-Path $Root "env.ps1"
    $envCmd = Join-Path $Root "env.cmd"
    if ($DryRun) {
        Write-Log "Would write $envPs1 and $envCmd"
        return
    }
    New-Item -ItemType Directory -Force -Path $Root | Out-Null
    @"
`$env:VIDA_HOME = if (`$env:VIDA_HOME) { `$env:VIDA_HOME } else { "$Root" }
`$env:VIDA_ROOT = if (`$env:VIDA_ROOT) { `$env:VIDA_ROOT } else { Join-Path `$env:VIDA_HOME "current" }
if ((`$env:PATH -split ';') -notcontains "$BinDir") { `$env:PATH = "$BinDir;`$env:PATH" }
"@ | Set-Content -LiteralPath $envPs1 -Encoding UTF8
    @"
@echo off
set "VIDA_HOME=$Root"
set "VIDA_ROOT=$Root\current"
set "PATH=$BinDir;%PATH%"
"@ | Set-Content -LiteralPath $envCmd -Encoding ASCII
}

function Install-PathHook {
    if ($DryRun) {
        Write-Log "Would add $BinDir to the user PATH"
        return
    }
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $current) { $current = "" }
    $parts = $current -split ";" | Where-Object { $_ }
    if ($parts -notcontains $BinDir) {
        $next = if ($current) { "$current;$BinDir" } else { $BinDir }
        [Environment]::SetEnvironmentVariable("Path", $next, "User")
        Write-Log "Added launcher directory to user PATH: $BinDir"
    }
}

function Write-CmdWrapper {
    param([string] $Launcher)
    if ($DryRun) {
        Write-Log "Would write launcher $BinDir\$Launcher.cmd"
        return
    }
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $path = Join-Path $BinDir "$Launcher.cmd"
    if ($Launcher -eq "vida") {
        @"
@echo off
setlocal
set "VIDA_HOME=$Root"
set "VIDA_ROOT=$Root\current"
set "RUNTIME_BIN=%VIDA_ROOT%\bin\vida.exe"
if not exist "%RUNTIME_BIN%" set "RUNTIME_BIN=%VIDA_ROOT%\bin\vida"
if /I "%~1"=="upgrade" goto manage
if /I "%~1"=="install" goto manage
if /I "%~1"=="use" goto manage
if /I "%~1"=="root" (
  echo %VIDA_ROOT%
  exit /b 0
)
"%RUNTIME_BIN%" %*
exit /b %ERRORLEVEL%
:manage
powershell -NoProfile -ExecutionPolicy Bypass -File "%VIDA_HOME%\installer\install.ps1" %* -Root "%VIDA_HOME%" -BinDir "$BinDir"
exit /b %ERRORLEVEL%
"@ | Set-Content -LiteralPath $path -Encoding ASCII
    } else {
        @"
@echo off
setlocal
set "VIDA_HOME=$Root"
set "VIDA_ROOT=$Root\current"
set "RUNTIME_BIN=%VIDA_ROOT%\bin\$Launcher.exe"
if not exist "%RUNTIME_BIN%" set "RUNTIME_BIN=%VIDA_ROOT%\bin\$Launcher"
"%RUNTIME_BIN%" %*
exit /b %ERRORLEVEL%
"@ | Set-Content -LiteralPath $path -Encoding ASCII
    }
}

function Install-Wrappers {
    foreach ($launcher in @("vida", "taskflow", "docflow")) {
        if (Test-BinSelected $launcher) { Write-CmdWrapper $launcher }
    }
}

function Install-ManagementScript {
    param([string] $Tag)
    $installerDir = Join-Path $Root "installer"
    $target = Join-Path $installerDir "install.ps1"
    if ($DryRun) {
        Write-Log "Would install management script into $target"
        return
    }
    New-Item -ItemType Directory -Force -Path $installerDir | Out-Null
    if ($PSCommandPath -and (Test-Path -LiteralPath $PSCommandPath)) {
        if ([System.IO.Path]::GetFullPath($PSCommandPath) -ne [System.IO.Path]::GetFullPath($target)) {
            Copy-Item -LiteralPath $PSCommandPath -Destination $target -Force
        }
    } elseif (-not $Archive) {
        Download-File "https://github.com/$RepoSlug/releases/download/$Tag/vida-install.ps1" $target
    } else {
        Fail "Unable to install management script from the current invocation while using a local archive."
    }
}

function Set-CurrentRelease {
    param([string] $ReleaseRoot)
    $current = Join-Path $Root "current"
    if ($DryRun) {
        Write-Log "Would point $current -> $ReleaseRoot"
        return
    }
    Remove-Item -LiteralPath $current -Recurse -Force -ErrorAction SilentlyContinue
    try {
        New-Item -ItemType Junction -Path $current -Target $ReleaseRoot | Out-Null
    } catch {
        New-Item -ItemType Directory -Force -Path $current | Out-Null
        Copy-Item -LiteralPath (Join-Path $ReleaseRoot "*") -Destination $current -Recurse -Force
    }
}

function Get-CurrentReleasePath {
    $current = Join-Path $Root "current"
    if (-not (Test-Path -LiteralPath $current)) { return "" }
    try {
        return (Get-Item -LiteralPath $current).Target
    } catch {
        return [System.IO.Path]::GetFullPath($current)
    }
}

function Cleanup-OldReleases {
    $releasesDir = Join-Path $Root "releases"
    if (-not (Test-Path -LiteralPath $releasesDir -PathType Container)) { return }
    $items = @(Get-ChildItem -LiteralPath $releasesDir -Directory | Sort-Object Name)
    while ($items.Count -gt $KeepReleases) {
        Remove-Item -LiteralPath $items[0].FullName -Recurse -Force
        $items = @($items | Select-Object -Skip 1)
    }
}

function Install-Release {
    param([string] $Tag)
    $archiveBase = Get-ArchiveBase $Tag
    $releasesDir = Join-Path $Root "releases"
    $releaseRoot = Join-Path $releasesDir $Tag

    if ((Test-Path -LiteralPath $releaseRoot) -and -not $Force) {
        $active = Get-CurrentReleasePath
        if ($active -and ([System.IO.Path]::GetFullPath($active) -eq [System.IO.Path]::GetFullPath($releaseRoot))) {
            Write-Host ""
            Write-Host "VIDA $Tag is already the active installed version"
            Write-Host "Active release: $(Join-Path $Root "current")"
            Write-Host "Release root: $releaseRoot"
            return
        }
    }

    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vida-install-" + [guid]::NewGuid().ToString("N"))
    $archivePath = Join-Path $tempDir "$archiveBase.zip"
    $checksumPath = Join-Path $tempDir "$archiveBase.sha256"
    $extractDir = Join-Path $tempDir "extract"

    if ($DryRun) {
        Write-Log "Resolved release target: windows-x86_64"
        Write-Log "Resolved archive: $archiveBase.zip"
        Write-Log "Would install release into $releaseRoot"
        Write-Log "Would install launchers into $BinDir"
        return
    }

    New-Item -ItemType Directory -Force -Path $tempDir, $extractDir, $releasesDir | Out-Null
    try {
        if ($Archive) {
            if (-not (Test-Path -LiteralPath $Archive -PathType Leaf)) { Fail "Local archive not found: $Archive" }
            Write-Log "Using local archive $Archive"
            Copy-Item -LiteralPath $Archive -Destination $archivePath -Force
        } else {
            $url = "https://github.com/$RepoSlug/releases/download/$Tag/$archiveBase.zip"
            Write-Log "Downloading $url"
            Download-File $url $archivePath
            $checksumUrl = "https://github.com/$RepoSlug/releases/download/$Tag/$archiveBase.sha256"
            Write-Log "Downloading $checksumUrl"
            Download-File $checksumUrl $checksumPath
        }
        Verify-ArchiveChecksum $archivePath $checksumPath

        Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
        $extractedRoot = Get-ChildItem -LiteralPath $extractDir -Directory | Select-Object -First 1
        if (-not $extractedRoot) { Fail "Unable to resolve extracted release root" }
        if ((Test-Path -LiteralPath $releaseRoot) -and -not $Force) {
            Fail "Release $Tag already exists at $releaseRoot. Re-run with -Force to replace it."
        }
        Remove-Item -LiteralPath $releaseRoot -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item -LiteralPath $extractedRoot.FullName -Destination $releaseRoot

        Ensure-RuntimeConfigScaffold $releaseRoot
        Install-ManagementScript $Tag
        Write-EnvironmentFiles
        Install-PathHook
        Install-Wrappers
        Set-CurrentRelease $releaseRoot
        Cleanup-OldReleases

        Write-Log "Installed VIDA $Tag into $releaseRoot"
        Write-Log "Active release: $(Join-Path $Root "current")"
        $launcherPaths = foreach ($selected in $SelectedBins) { Join-Path $BinDir "$selected.cmd" }
        Write-Log "Launchers: $([string]::Join(', ', $launcherPaths))"
        Write-Host ""
        Write-Host "Try it now:"
        Write-Host "  . `"$Root\env.ps1`""
        if (Test-BinSelected "vida") { Write-Host "  vida doctor" }
        if (Test-BinSelected "taskflow") { Write-Host "  taskflow help" }
        if (Test-BinSelected "docflow") { Write-Host "  docflow help" }
    } finally {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-Doctor {
    $missing = $false
    $current = Join-Path $Root "current"
    if (-not (Test-Path -LiteralPath $current -PathType Container)) {
        Write-Log "Missing active release link: $current"
        $missing = $true
    }
    foreach ($launcher in @("vida", "taskflow", "docflow")) {
        if (Test-BinSelected $launcher) {
            $wrapper = Join-Path $BinDir "$launcher.cmd"
            if (-not (Test-Path -LiteralPath $wrapper -PathType Leaf)) {
                Write-Log "Missing launcher: $wrapper"
                $missing = $true
            }
        }
    }
    foreach ($path in @((Join-Path $Root "env.ps1"), (Join-Path $Root "env.cmd"), (Join-Path $Root "installer\install.ps1"))) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            Write-Log "Missing install surface: $path"
            $missing = $true
        }
    }
    if (Test-Path -LiteralPath $current -PathType Container) {
        foreach ($runtime in @("vida", "taskflow", "docflow")) {
            $runtimePath = Get-RuntimeBinaryPath $current $runtime
            if (-not (Test-Path -LiteralPath $runtimePath -PathType Leaf)) {
                Write-Log "Missing bundled $runtime binary"
                $missing = $true
            }
        }
        foreach ($path in @(".codex\config.toml", "AGENTS.sidecar.md", "vida.config.yaml", "install\assets\vida.config.yaml.template")) {
            $candidate = Join-Path $current $path
            if (-not (Test-Path -LiteralPath $candidate)) {
                Write-Log "Missing packaged surface: $candidate"
                $missing = $true
            }
        }
    }
    if ($missing) { Fail "Doctor found missing installation surfaces." }
    Write-Log "Doctor check passed for $Root"
}

function Use-Release {
    param([string] $Tag)
    if ($Tag -eq "latest") { Fail "use requires -Version <tag>" }
    $releaseRoot = Join-Path (Join-Path $Root "releases") $Tag
    if (-not (Test-Path -LiteralPath $releaseRoot -PathType Container)) {
        Fail "Installed release not found: $releaseRoot"
    }
    Set-CurrentRelease $releaseRoot
    Install-Wrappers
    Write-Log "Switched active VIDA release to $Tag"
}

function Bootstrap-CurrentProject {
    param([string] $ReleaseRoot)
    $projectRoot = if ($env:VIDA_PROJECT_ROOT) { $env:VIDA_PROJECT_ROOT } else { (Get-Location).Path }
    if (-not (Test-Path -LiteralPath $projectRoot -PathType Container)) { Fail "Missing target project directory: $projectRoot" }
    Copy-ProjectFile (Join-Path $ReleaseRoot "AGENTS.md") (Join-Path $projectRoot "AGENTS.md") "framework bootstrap carrier"
    Copy-ProjectFile (Join-Path $ReleaseRoot "AGENTS.sidecar.md") (Join-Path $projectRoot "AGENTS.sidecar.md") "project sidecar scaffold"
    Copy-ProjectTree (Join-Path $ReleaseRoot "vida") (Join-Path $projectRoot "vida") "framework protocol tree"
    Copy-ProjectTree (Join-Path $ReleaseRoot ".codex") (Join-Path $projectRoot ".codex") "project-local Codex configuration"
    Copy-ProjectFile (Join-Path $ReleaseRoot "install\assets\vida.config.yaml.template") (Join-Path $projectRoot "vida.config.yaml") "project runtime config scaffold"
    Write-Host ""
    Write-Host "Current project bootstrap is ready"
    Write-Host "Project root: $projectRoot"
}

switch ($Command) {
    "help" {
        Show-Usage
    }
    "doctor" {
        Invoke-Doctor
    }
    "install" {
        $resolved = Resolve-Version
        Install-Release $resolved
    }
    "upgrade" {
        $resolved = Resolve-Version
        Install-Release $resolved
    }
    "init" {
        $resolved = Resolve-Version
        Install-Release $resolved
        Bootstrap-CurrentProject (Join-Path $Root "current")
    }
    "use" {
        Use-Release $Version
    }
}
