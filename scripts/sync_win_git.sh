#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/sync_win_git.sh [--host win-pc] [--branch master] [--build none|rust|winui|all]

Default:
  --host win-pc
  --branch master
  --build all

Behavior:
  1) Validate remote workspace is clean (dirty => fail fast)
  2) git pull --ff-only
  3) Optional build commands on win-pc
EOF
}

HOST="win-pc"
BRANCH="master"
BUILD_MODE="all"
REMOTE_REPO="/d/awmkit/AudioWaterMarkKit"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      HOST="$2"
      shift 2
      ;;
    --branch)
      BRANCH="$2"
      shift 2
      ;;
    --build)
      BUILD_MODE="$2"
      shift 2
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

case "$BUILD_MODE" in
  none|rust|winui|all) ;;
  *)
    echo "Invalid --build value: $BUILD_MODE" >&2
    exit 1
    ;;
esac

echo "[sync] host=$HOST branch=$BRANCH build=$BUILD_MODE"

echo "[sync] checking remote workspace cleanliness..."
DIRTY_OUTPUT="$(ssh "$HOST" "cd $REMOTE_REPO && git status --short" || true)"
if [[ -n "${DIRTY_OUTPUT}" ]]; then
  echo "[error] remote workspace is dirty. resolve manually before sync:" >&2
  echo "$DIRTY_OUTPUT" >&2
  exit 2
fi

echo "[sync] pulling latest from origin/$BRANCH ..."
ssh "$HOST" "cd $REMOTE_REPO && git pull --ff-only origin $BRANCH"

if [[ "$BUILD_MODE" == "none" ]]; then
  echo "[sync] completed (no build)."
  exit 0
fi

if [[ "$BUILD_MODE" == "rust" || "$BUILD_MODE" == "all" ]]; then
  echo "[build] rust ffi ..."
  ssh "$HOST" "cd $REMOTE_REPO && cargo build --lib --features ffi,app,bundled,multichannel --release --target x86_64-pc-windows-msvc"
fi

if [[ "$BUILD_MODE" == "winui" || "$BUILD_MODE" == "all" ]]; then
  echo "[build] winui debug ..."
  ssh "$HOST" "taskkill //F //IM AWMKit.exe >NUL 2>&1 || exit 0"
  ssh "$HOST" "cd $REMOTE_REPO && dotnet build winui-app/AWMKit/AWMKit.csproj -c Debug -p:Platform=x64"
fi

echo "[sync] done."

