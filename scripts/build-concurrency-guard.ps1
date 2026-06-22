function Enter-VidaBuildConcurrencyGuard {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootDir,
        [string]$Scope = "build",
        [int]$TimeoutSeconds = -1
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($RootDir)
    $lockDir = Join-Path $resolvedRoot ".vida\data\state\script-locks"
    $lockPath = Join-Path $lockDir "$Scope.lock"

    $currentToken = $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN
    $inheritedToken = $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN
    $hasMatchingInheritedToken = -not [string]::IsNullOrWhiteSpace($currentToken) -and
        -not [string]::IsNullOrWhiteSpace($inheritedToken) -and
        $currentToken -eq $inheritedToken

    if ($env:VIDA_BUILD_SCRIPT_LOCK_HELD -eq "1" -and
        $env:VIDA_BUILD_SCRIPT_LOCK_PATH -eq $lockPath -and
        ($env:VIDA_BUILD_SCRIPT_LOCK_PID -eq [string]$PID -or $hasMatchingInheritedToken)) {
        return [pscustomobject]@{
            Reentrant = $true
            LockPath = $lockPath
        }
    }

    if ($TimeoutSeconds -lt 0) {
        if ([string]::IsNullOrWhiteSpace($env:VIDA_BUILD_LOCK_TIMEOUT_SECONDS)) {
            $TimeoutSeconds = 0
        } else {
            $TimeoutSeconds = [int]$env:VIDA_BUILD_LOCK_TIMEOUT_SECONDS
        }
    }

    New-Item -ItemType Directory -Force -Path $lockDir | Out-Null
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)

    while ($true) {
        try {
            $stream = [System.IO.File]::Open(
                $lockPath,
                [System.IO.FileMode]::OpenOrCreate,
                [System.IO.FileAccess]::ReadWrite,
                [System.IO.FileShare]::None
            )
            $stream.SetLength(0)
            $body = @(
                "pid=$PID",
                "scope=$Scope",
                "root=$resolvedRoot",
                "started_at=$((Get-Date).ToString("o"))",
                "command=$([System.Environment]::CommandLine)"
            ) -join [Environment]::NewLine
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($body)
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush()

            $previousHeld = $env:VIDA_BUILD_SCRIPT_LOCK_HELD
            $previousPath = $env:VIDA_BUILD_SCRIPT_LOCK_PATH
            $previousScope = $env:VIDA_BUILD_SCRIPT_LOCK_SCOPE
            $previousPid = $env:VIDA_BUILD_SCRIPT_LOCK_PID
            $previousToken = $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN
            $previousInheritedToken = $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN
            $lockToken = [guid]::NewGuid().ToString("N")

            $env:VIDA_BUILD_SCRIPT_LOCK_HELD = "1"
            $env:VIDA_BUILD_SCRIPT_LOCK_PATH = $lockPath
            $env:VIDA_BUILD_SCRIPT_LOCK_SCOPE = $Scope
            $env:VIDA_BUILD_SCRIPT_LOCK_PID = [string]$PID
            $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN = $lockToken
            $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN = $lockToken

            return [pscustomobject]@{
                Reentrant = $false
                Stream = $stream
                LockPath = $lockPath
                PreviousHeld = $previousHeld
                PreviousPath = $previousPath
                PreviousScope = $previousScope
                PreviousPid = $previousPid
                PreviousToken = $previousToken
                PreviousInheritedToken = $previousInheritedToken
            }
        } catch [System.IO.IOException] {
            if ($TimeoutSeconds -le 0 -or (Get-Date) -ge $deadline) {
                throw "Build/test/install guard blocked another run. Lock: $lockPath. Wait for the active script to finish or stop the owning process; set VIDA_BUILD_LOCK_TIMEOUT_SECONDS to wait instead of failing fast."
            }
            Start-Sleep -Seconds 1
        }
    }
}

function Exit-VidaBuildConcurrencyGuard {
    param([object]$Guard)

    if ($null -eq $Guard -or $Guard.Reentrant) {
        return
    }

    if ($Guard.Stream) {
        $Guard.Stream.Dispose()
    }
    Remove-Item -LiteralPath $Guard.LockPath -Force -ErrorAction SilentlyContinue

    foreach ($entry in @(
            @("VIDA_BUILD_SCRIPT_LOCK_HELD", $Guard.PreviousHeld),
            @("VIDA_BUILD_SCRIPT_LOCK_PATH", $Guard.PreviousPath),
            @("VIDA_BUILD_SCRIPT_LOCK_SCOPE", $Guard.PreviousScope),
            @("VIDA_BUILD_SCRIPT_LOCK_PID", $Guard.PreviousPid),
            @("VIDA_BUILD_SCRIPT_LOCK_TOKEN", $Guard.PreviousToken),
            @("VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN", $Guard.PreviousInheritedToken)
        )) {
        if ($null -eq $entry[1]) {
            Remove-Item "Env:$($entry[0])" -ErrorAction SilentlyContinue
        } else {
            Set-Item "Env:$($entry[0])" -Value $entry[1]
        }
    }
}
