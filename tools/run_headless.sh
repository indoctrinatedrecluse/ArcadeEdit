#!/usr/bin/env bash
# ArcadeEdit - Run Headless Mode (CLI / Automation)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"
exec cargo run -p arcade-headless -- "$@"

