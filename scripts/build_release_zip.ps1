param(
    [string]$OutputPath = "dist\SleekManualMaker.zip",
    [string]$HashOutputPath = ""
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
    Write-Host "[BUILD] Building release binary..."
    $cargo = Resolve-CargoPath
    & $cargo build --release --bin SleekManualMaker

    $packageDir = Join-Path $repoRoot "dist\package"
    $assetsDir = Join-Path $packageDir "assets"

    New-Item -ItemType Directory -Force $packageDir | Out-Null
    New-Item -ItemType Directory -Force $assetsDir | Out-Null

    Copy-Item -LiteralPath "target\release\SleekManualMaker.exe" -Destination $packageDir -Force
    Copy-Item -LiteralPath "assets\BIZUDPGothic-Regular.ttf" -Destination $assetsDir -Force

    $resolvedOutputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
    $outputDir = Split-Path -Parent $resolvedOutputPath
    if ($outputDir) {
        New-Item -ItemType Directory -Force $outputDir | Out-Null
    }

    if (Test-Path -LiteralPath $resolvedOutputPath) {
        Remove-Item -LiteralPath $resolvedOutputPath -Force
    }

    Write-Host "[ZIP] Creating $resolvedOutputPath..."
    Compress-Archive -Path (Join-Path $packageDir "*") -DestinationPath $resolvedOutputPath -Force

    $hash = Get-FileHash -LiteralPath $resolvedOutputPath -Algorithm SHA256

    if ($HashOutputPath) {
        $resolvedHashOutputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($HashOutputPath)
        $hashOutputDir = Split-Path -Parent $resolvedHashOutputPath
        if ($hashOutputDir) {
            New-Item -ItemType Directory -Force $hashOutputDir | Out-Null
        }
        Set-Content -LiteralPath $resolvedHashOutputPath -Value "$($hash.Hash)  SleekManualMaker.zip" -Encoding UTF8
    }

    Write-Host "[OK] Created $resolvedOutputPath"
    Write-Host "[SHA256] $($hash.Hash)"
}
finally {
    Pop-Location
}
