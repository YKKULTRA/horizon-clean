#!/usr/bin/env bash
set -euo pipefail
base="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
# This portable build has its own accounts, settings and saves. Never import Prism credentials.
export HORIZON_HOME="$base/userdata/game"
export HORIZON_LAUNCHER_HOME="$base/userdata/launcher"
export HORIZON_LAUNCHER_DESKTOP="$base/userdata/desktop"
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1 HORIZON_TELEMETRY_URL=off HORIZON_NO_HEARTBEAT=1
mkdir -p "$HORIZON_HOME" "$HORIZON_LAUNCHER_HOME/versions"
ln -sfn "$base/game-build" "$HORIZON_LAUNCHER_HOME/versions/clean-4aa6854"
if [[ ! -e "$HORIZON_LAUNCHER_HOME/settings.json" ]]; then
    printf '%s\n' '{"selected":"clean-4aa6854","install_new_versions":false,"desktop_entry":false}' > "$HORIZON_LAUNCHER_HOME/settings.json"
fi
# Follow this PC's graphics isolation when available; do not alter system configuration.
if command -v nvidia-run >/dev/null 2>&1; then
    exec nvidia-run "$base/horizon-launcher" "$@"
fi
exec "$base/horizon-launcher" "$@"
