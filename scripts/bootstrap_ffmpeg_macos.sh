#!/usr/bin/env bash
set -euo pipefail

# Bootstrap FFmpeg runtime + build-time dev environment on macOS.
#
# Usage:
#   scripts/bootstrap_ffmpeg_macos.sh \
#     --repo SakuzyPeng/AudioWaterMarkKit \
#     --release ffmpeg-runtime-8.0.2 \
#     --asset ffmpeg-macos-arm64-gpl-slim-runtime-8.0.2.zip \
#     --out ffmpeg-dist \
#     --install-dev true

REPO=""
RELEASE=""
ASSET=""
OUT_DIR="ffmpeg-dist"
INSTALL_DEV="true"
GITHUB_ENV_FILE="${GITHUB_ENV:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO="$2"
      shift 2
      ;;
    --release)
      RELEASE="$2"
      shift 2
      ;;
    --asset)
      ASSET="$2"
      shift 2
      ;;
    --out)
      OUT_DIR="$2"
      shift 2
      ;;
    --install-dev)
      INSTALL_DEV="$2"
      shift 2
      ;;
    --github-env)
      GITHUB_ENV_FILE="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$ASSET" ]]; then
  echo "--asset is required" >&2
  exit 2
fi

if [[ ! -f "$ASSET" ]]; then
  if [[ -z "$REPO" || -z "$RELEASE" ]]; then
    echo "Asset not found locally, and --repo/--release not provided: $ASSET" >&2
    exit 1
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh CLI not found, cannot download $ASSET" >&2
    exit 1
  fi
  gh release download "$RELEASE" --repo "$REPO" -p "$ASSET" --clobber
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
unzip -q "$ASSET" -d "$OUT_DIR"

ROOT="$(find "$OUT_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$ROOT" ]]; then
  echo "Invalid ffmpeg package: missing root directory in $ASSET" >&2
  exit 1
fi

if [[ -d "$ROOT/lib" ]]; then
  cp -a "$ROOT/lib" "$OUT_DIR/"
elif [[ -d "$ROOT/bin" ]]; then
  mkdir -p "$OUT_DIR/lib"
  cp -a "$ROOT/bin/"*.dylib "$OUT_DIR/lib/"
else
  echo "Invalid ffmpeg package: missing lib/bin directory in $ASSET" >&2
  exit 1
fi

test -d "$OUT_DIR/lib"
ls "$OUT_DIR/lib"/libavcodec*.dylib >/dev/null
ls "$OUT_DIR/lib"/libavformat*.dylib >/dev/null
ls "$OUT_DIR/lib"/libavutil*.dylib >/dev/null
ls "$OUT_DIR/lib"/libavfilter*.dylib >/dev/null
ls "$OUT_DIR/lib"/libswresample*.dylib >/dev/null

ASSET_NAME="$(basename "$ASSET")"
if [[ -f "tools/ffmpeg/manifest.json" ]]; then
  FF_ASSET="$ASSET" FF_ASSET_NAME="$ASSET_NAME" python3 - <<'PY'
import hashlib
import json
import os
import pathlib
import sys

manifest = json.loads(pathlib.Path("tools/ffmpeg/manifest.json").read_text())
expected = ""
for item in manifest.get("assets", {}).values():
    if item.get("name") == os.environ["FF_ASSET_NAME"]:
        expected = item.get("sha256", "").strip()
        break

if not expected:
    print("ffmpeg asset sha256 check skipped (asset not found in manifest).")
    sys.exit(0)

actual = hashlib.sha256(pathlib.Path(os.environ["FF_ASSET"]).read_bytes()).hexdigest()
if actual.lower() != expected.lower():
    raise SystemExit(f"sha256 mismatch: {actual} != {expected}")

print("ffmpeg asset sha256 check passed.")
PY
fi

if [[ "$INSTALL_DEV" == "true" ]]; then
  brew list pkg-config >/dev/null 2>&1 || brew install pkg-config
  brew list ffmpeg >/dev/null 2>&1 || brew install ffmpeg
fi

RUNTIME_DIR="$(cd "$OUT_DIR" && pwd)"
LIB_DIR="$RUNTIME_DIR/lib"
echo "FFmpeg runtime prepared at: $RUNTIME_DIR"
echo "FFmpeg runtime libs at: $LIB_DIR"

if [[ -n "$GITHUB_ENV_FILE" ]]; then
  {
    echo "FFMPEG_RUNTIME_DIR=$RUNTIME_DIR"
    echo "FFMPEG_RUNTIME_LIB_DIR=$LIB_DIR"
  } >> "$GITHUB_ENV_FILE"
fi

