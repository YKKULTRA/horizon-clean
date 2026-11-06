#!/usr/bin/env bash
# Build only in this checkout; no package installation or user-data migration.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"
state="${HORIZON_BUILD_STATE:-$(dirname "$PWD")/build-state}"
mkdir -p "$state/tmp"
export TMPDIR="$state/tmp" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"
export HORIZON_TELEMETRY_URL=http://127.0.0.1:9
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1 HORIZON_NO_AUDIO=1
# Horizon's own public OAuth app ID, extracted from the checksum-verified official 0.2.33 binary.
# This is configuration, not a credential; authentication goes directly to Microsoft.
export HORIZON_MSA_CLIENT_ID="${HORIZON_MSA_CLIENT_ID:-331e08f8-172d-43b9-835a-3108c2fcdbf3}"
export HORIZON_LAUNCHER_VERSION=0.2.0-clean HORIZON_LAUNCHER_TAG=launcher@0.2.0-clean
export CARGO_PROFILE_DEV_DEBUG=0
exec > >(tee "$state/build.log") 2>&1
trap 'rc=$?; printf "%s\n" "$rc" > "$state/exit-code"' EXIT
# Optional: a command that prints something while the build should wait (e.g. a game is running).
if [[ -n "${HORIZON_BUILD_WAIT_CMD:-}" ]]; then
    while [[ -n "$(bash -c "$HORIZON_BUILD_WAIT_CMD")" ]]; do
        printf 'Waiting before compiling (HORIZON_BUILD_WAIT_CMD).\n'
        sleep 30
    done
fi
printf '\n=== privacy transport regression tests ===\n'
cargo test --locked -p horizon-telemetry
printf '\n=== launcher build and tests ===\n'
(cd launcher && cargo test --locked --release --lib --bins --tests)
printf '\n=== privacy and launcher lints ===\n'
cargo clippy --locked -p horizon-telemetry --all-targets -- -D warnings
(cd launcher && cargo clippy --locked --release --lib --bins --tests -- -D warnings)
printf '\n=== game build ===\n'
cargo build --locked --profile fast -p horizon-client --bin horizon-client --bin horizon-ctl -p horizon-prepare --bin horizon-prepare -p horizon-server --bin horizon-server
printf '\n=== executable smoke checks ===\n'
launcher/target/release/horizon-launcher --version
target/fast/horizon-client --help > "$state/client-help.txt"
target/fast/horizon-ctl --help > "$state/ctl-help.txt"
target/fast/horizon-prepare --version
target/fast/horizon-server --version
printf '\nBUILD AND TESTS PASSED\n'
