param(
    [string]$PackageDir = "dist\debug"
)

$ErrorActionPreference = "Stop"

function Resolve-CargoPath {
    if ($env:CARGO -and (Test-Path -LiteralPath $env:CARGO)) {
        return $env:CARGO
    }

    $userCargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $userCargo) {
        return $userCargo
    }

    return "cargo"
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $repoRoot
try {
    Write-Host "[BUILD] Building debug binary..."
    $cargo = Resolve-CargoPath
    & $cargo build --bin SleekManualMaker

    $resolvedPackageDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($PackageDir)
    $assetsDir = Join-Path $resolvedPackageDir "assets"

    New-Item -ItemType Directory -Force $resolvedPackageDir | Out-Null
    New-Item -ItemType Directory -Force $assetsDir | Out-Null

    Copy-Item -LiteralPath "target\debug\SleekManualMaker.exe" -Destination $resolvedPackageDir -Force
    Copy-Item -LiteralPath "assets\BIZUDPGothic-Regular.ttf" -Destination $assetsDir -Force

    $startScript = @"
@echo off
chcp 65001 > nul
cd /d "%~dp0"
echo [DEBUG MODE] Starting SleekManualMaker...
start "" "SleekManualMaker.exe"
"@
    Set-Content -LiteralPath (Join-Path $resolvedPackageDir "start_debug.bat") -Value $startScript -Encoding ASCII

    Write-Host "[OK] Created $resolvedPackageDir"
}
finally {
    Pop-Location
}
