$ErrorActionPreference = "Stop"

$rootDir = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "build-concurrency-guard.ps1")
. (Join-Path $PSScriptRoot "vida-process-runner.ps1")

$guard = Enter-VidaBuildConcurrencyGuard -RootDir $rootDir -Scope "process-runner-smoke"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vida-process-runner-smoke-" + [guid]::NewGuid().ToString("N"))
$stdoutPath = Join-Path $tempRoot "stdout.txt"
$stderrPath = Join-Path $tempRoot "stderr.txt"
$descendantScript = Join-Path $tempRoot "descendant.ps1"
$childScript = Join-Path $tempRoot "child.ps1"
$startedAt = [System.Diagnostics.Stopwatch]::StartNew()
$descendantPid = $null
try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    $pwsh = (Get-Command pwsh -ErrorAction Stop).Source
    Set-Content -LiteralPath $descendantScript -Encoding UTF8 -Value "Start-Sleep -Seconds 10; Write-Output descendant"
    Set-Content -LiteralPath $childScript -Encoding UTF8 -Value @"
`$descendant = Start-Process -FilePath '$pwsh' -ArgumentList '-NoLogo','-NoProfile','-File','$descendantScript' -NoNewWindow -PassThru
Write-Output "descendant-pid=`$(`$descendant.Id)"
Write-Output 'runner-child-pass'
[Console]::Error.WriteLine('runner-child-error')
"@
    $exitCode = Invoke-VidaProcess `
        -FilePath $pwsh `
        -ArgumentList @("-NoLogo", "-NoProfile", "-File", $childScript) `
        -WorkingDirectory $rootDir `
        -StdoutPath $stdoutPath `
        -StderrPath $stderrPath
    $startedAt.Stop()
    if ($exitCode -ne 0) {
        throw "process runner smoke child exited with code $exitCode"
    }
    if (-not (Get-Content -LiteralPath $stdoutPath -Raw).Contains("runner-child-pass")) {
        throw "process runner smoke stdout was not captured"
    }
    if (-not (Get-Content -LiteralPath $stderrPath -Raw).Contains("runner-child-error")) {
        throw "process runner smoke stderr was not captured"
    }
    $stdout = Get-Content -LiteralPath $stdoutPath -Raw
    if ($stdout -match 'descendant-pid=(\d+)') {
        $descendantPid = [int]$Matches[1]
    } else {
        throw "process runner smoke did not capture descendant pid"
    }
    if ($startedAt.ElapsedMilliseconds -ge 5000) {
        throw "process runner smoke waited for descendant: $($startedAt.ElapsedMilliseconds)ms"
    }
} finally {
    if ($null -ne $descendantPid) {
        Stop-Process -Id $descendantPid -Force -ErrorAction SilentlyContinue
    }
    Exit-VidaBuildConcurrencyGuard -Guard $guard
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

$lockPath = Join-Path $rootDir ".vida\data\state\script-locks\process-runner-smoke.lock"
if (Test-Path -LiteralPath $lockPath) {
    throw "process runner smoke left the build guard lock behind: $lockPath"
}
Write-Output "process-runner-smoke: pass"
