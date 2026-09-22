#!/usr/bin/env bash
# ArcadeEdit - Run GUI Mode (Rendered Desktop)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"
echo "✦ Starting ArcadeEdit (Rendered Desktop)..."
exec cargo run -p arcade-desktop -- "$@"

