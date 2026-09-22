#!/usr/bin/env pwsh
# ArcadeEdit - Run Headless Mode (CLI / Automation)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir/.."

Push-Location $ProjectRoot
try {
    cargo run -p arcade-headless -- $args
} finally {
    Pop-Location
}

