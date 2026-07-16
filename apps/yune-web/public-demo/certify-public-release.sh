#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# shellcheck disable=SC1091
. "$SCRIPT_DIR/public-release-toolchain.sh"

cd "$YUNE_WEB_REPO_ROOT"
yune_web_activate_release_toolchain

if [ ! -f "$SCRIPT_DIR/dist/build-info.json" ]; then
	echo "Missing source-identified public artifact at $SCRIPT_DIR/dist." >&2
	exit 1
fi

export PLAYWRIGHT_BROWSERS_PATH=${PLAYWRIGHT_BROWSERS_PATH:-"$YUNE_WEB_REPO_ROOT/.cache/ms-playwright"}
npm --prefix apps/yune-web/e2e ci
node apps/yune-web/e2e/node_modules/@playwright/test/cli.js install --with-deps chromium

# These are the binding WEB03-11 values. Diagnostic overrides are deliberately
# overwritten so this entrypoint can produce only release-grade evidence.
export YUNE_WEB_LATENCY_CPU_THROTTLE=4
export YUNE_WEB_LATENCY_KEY_INTERVAL_MS=250
export YUNE_WEB_LATENCY_P95_MS=750
export YUNE_WEB_LATENCY_MAX_MS=1000
npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public
