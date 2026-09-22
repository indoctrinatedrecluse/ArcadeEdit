#!/usr/bin/env pwsh
# ArcadeEdit - Run GUI Mode (Rendered Desktop)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir/.."

Push-Location $ProjectRoot
try {
    Write-Host "✦ Starting ArcadeEdit (Rendered Desktop)..." -ForegroundColor Cyan
    cargo run -p arcade-desktop -- $args
} finally {
    Pop-Location
}

