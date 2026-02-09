#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_HEADER="$ROOT_DIR/include/awmkit.h"
SWIFT_HEADER="$ROOT_DIR/bindings/swift/Sources/CAWMKit/include/awmkit.h"

usage() {
  cat <<'EOF'
Usage:
  scripts/sync_ffi_headers.sh --sync   # copy include/awmkit.h to Swift vendored header
  scripts/sync_ffi_headers.sh --check  # verify both headers are identical
EOF
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

case "$1" in
  --sync)
    cp "$SOURCE_HEADER" "$SWIFT_HEADER"
    echo "[OK] synced: $SWIFT_HEADER"
    ;;
  --check)
    if cmp -s "$SOURCE_HEADER" "$SWIFT_HEADER"; then
      echo "[OK] headers are in sync"
      exit 0
    fi

    echo "[ERROR] headers differ:"
    diff -u "$SOURCE_HEADER" "$SWIFT_HEADER" || true
    echo
    echo "Run: scripts/sync_ffi_headers.sh --sync"
    exit 1
    ;;
  *)
    usage
    exit 1
    ;;
esac
