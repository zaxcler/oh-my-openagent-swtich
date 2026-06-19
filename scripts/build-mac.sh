#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
bun run tauri build --bundles app,dmg
