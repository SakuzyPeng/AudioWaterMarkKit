#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/sync_win_rsync_emergency.sh --emergency [--host win-pc]

Purpose:
  Emergency sync via rsync/cwRsync-style transport.
  Not for release readiness. Always return to Git-first workflow afterward.
EOF
}

HOST="win-pc"
REMOTE_REPO="/d/awmkit/AudioWaterMarkKit"
EMERGENCY="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      HOST="$2"
      shift 2
      ;;
    --emergency)
      EMERGENCY="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ "$EMERGENCY" != "true" ]]; then
  echo "[error] pass --emergency to confirm risk acceptance." >&2
  exit 2
fi

if ! command -v rsync >/dev/null 2>&1; then
  echo "[error] rsync is required on mac side. install rsync/cwRsync equivalent first." >&2
  exit 3
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[warn] Emergency sync mode enabled. This is not a release-safe workflow."
echo "[sync] pushing workspace to $HOST:$REMOTE_REPO"

rsync -az --delete \
  --exclude ".git/" \
  --exclude "target/" \
  --exclude "winui-app/**/bin/" \
  --exclude "winui-app/**/obj/" \
  --exclude "winui-app/**/publish/" \
  --exclude ".DS_Store" \
  "$ROOT_DIR/" "$HOST:$REMOTE_REPO/"

echo "[done] Emergency sync complete."
echo "[next] Return to Git-first workflow before release:"
echo "       1) Commit+push on mac"
echo "       2) scripts/sync_win_git.sh --host $HOST --build all"

