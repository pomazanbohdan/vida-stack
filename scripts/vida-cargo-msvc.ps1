$ErrorActionPreference = "Stop"

$rootDir = Split-Path -Parent $PSScriptRoot
$windowsEnvScript = Join-Path $PSScriptRoot "vida-windows-env.ps1"
if (-not (Test-Path -LiteralPath $windowsEnvScript -PathType Leaf)) {
    throw "[vida-cargo-msvc] Missing Windows environment helper: $windowsEnvScript"
}

. $windowsEnvScript

if ([string]::IsNullOrWhiteSpace($env:VIDA_MSVC_TEMP_DIR)) {
    $env:VIDA_MSVC_TEMP_DIR = Join-Path $rootDir ".vida\build-temp\msvc"
}

Import-VidaMsvcEnvironment

$cargo = Get-Command "cargo" -ErrorAction Stop
$cargoArgs = @($args)
if ($cargoArgs.Count -eq 0) {
    throw "[vida-cargo-msvc] Pass cargo arguments, for example: test -p vida --bin vida status_surface"
}

function Convert-VidaCargoTestArgs {
    param([string[]]$Arguments)

    if ($Arguments.Count -eq 0 -or $Arguments[0] -ne "test" -or $Arguments -contains "--") {
        return $Arguments
    }

    $harnessFlags = @(
        "--nocapture",
        "--test-threads",
        "--exact",
        "--ignored",
        "--include-ignored",
        "--skip",
        "--show-output",
        "--format"
    )
    for ($index = 1; $index -lt $Arguments.Count; $index++) {
        $arg = $Arguments[$index]
        $isHarnessArg = $false
        foreach ($flag in $harnessFlags) {
            if ($arg -eq $flag -or $arg.StartsWith("$flag=")) {
                $isHarnessArg = $true
                break
            }
        }
        if ($isHarnessArg) {
            return @($Arguments[0..($index - 1)] + @("--") + $Arguments[$index..($Arguments.Count - 1)])
        }
    }

    return $Arguments
}

$cargoArgs = @(Convert-VidaCargoTestArgs -Arguments $cargoArgs)
& $cargo.Source @cargoArgs
exit $LASTEXITCODE
