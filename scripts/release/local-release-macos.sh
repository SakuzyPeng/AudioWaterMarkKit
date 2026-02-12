#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TMP_ROOT="${REPO_ROOT}/.tmp-release/macos"
DIST_ROOT="${REPO_ROOT}/dist/local"

cd "${REPO_ROOT}"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "[ERROR] Repository is dirty. Commit/stash changes before release."
  exit 1
fi

SHORT_SHA="$(git rev-parse --short HEAD)"
HEAD_TAG="$(git tag --points-at HEAD | head -n1 || true)"
BASE_VERSION="$(awk -F '\"' '/^version = /{print $2; exit}' Cargo.toml)"
VERSION="${HEAD_TAG:-${BASE_VERSION}+${SHORT_SHA}}"
PACKAGE_VERSION="$(echo "${VERSION}" | tr '+/' '__' | tr -c '[:alnum:]._-' '-')"

MAC_BUNDLED_ZIP="${REPO_ROOT}/bundled/audiowmark-macos-arm64.zip"
if [[ ! -f "${MAC_BUNDLED_ZIP}" ]]; then
  echo "[ERROR] Missing bundled zip: ${MAC_BUNDLED_ZIP}"
  exit 1
fi

if [[ -z "${FFMPEG_MACOS_RUNTIME_ZIP:-}" ]]; then
  FFMPEG_MACOS_RUNTIME_ZIP="$(ls "${REPO_ROOT}"/ffmpeg-macos-arm64-gpl-slim-runtime-*.zip 2>/dev/null | sort | tail -n1 || true)"
fi
if [[ -z "${FFMPEG_MACOS_RUNTIME_ZIP}" || ! -f "${FFMPEG_MACOS_RUNTIME_ZIP}" ]]; then
  echo "[ERROR] Missing FFmpeg macOS runtime zip. Set FFMPEG_MACOS_RUNTIME_ZIP or place ffmpeg-macos-arm64-gpl-slim-runtime-*.zip in repo root."
  exit 1
fi

rm -rf "${TMP_ROOT}"
mkdir -p "${TMP_ROOT}" "${DIST_ROOT}"

echo "[INFO] Building Rust CLI/FFI (macOS arm64)..."
cargo build --release --features full-cli,bundled,ffi,app --target aarch64-apple-darwin

echo "[INFO] Building macOS app..."
DERIVED_DATA="${TMP_ROOT}/DerivedData"
xcodebuild \
  -project "${REPO_ROOT}/macos-app/AWMKit.xcodeproj" \
  -scheme AWMKit \
  -configuration Release \
  -sdk macosx \
  -destination "platform=macOS,arch=arm64" \
  -derivedDataPath "${DERIVED_DATA}" \
  ARCHS=arm64 \
  ONLY_ACTIVE_ARCH=YES \
  CODE_SIGNING_ALLOWED=NO \
  CODE_SIGNING_REQUIRED=NO \
  build

APP_SRC="${DERIVED_DATA}/Build/Products/Release/AWMKit.app"
if [[ ! -d "${APP_SRC}" ]]; then
  echo "[ERROR] Missing built app: ${APP_SRC}"
  exit 1
fi

echo "[INFO] Preparing FFmpeg dylibs..."
FFMPEG_UNPACK="${TMP_ROOT}/ffmpeg-unpack"
FFMPEG_LIB="${TMP_ROOT}/ffmpeg-lib"
mkdir -p "${FFMPEG_UNPACK}" "${FFMPEG_LIB}"
unzip -q "${FFMPEG_MACOS_RUNTIME_ZIP}" -d "${FFMPEG_UNPACK}"
FFMPEG_ROOT="$(find "${FFMPEG_UNPACK}" -mindepth 1 -maxdepth 1 -type d | head -n1 || true)"
if [[ -z "${FFMPEG_ROOT}" ]]; then
  echo "[ERROR] Invalid FFmpeg zip: missing root directory."
  exit 1
fi
if [[ -d "${FFMPEG_ROOT}/lib" ]]; then
  cp -a "${FFMPEG_ROOT}/lib/"*.dylib "${FFMPEG_LIB}/"
elif [[ -d "${FFMPEG_ROOT}/bin" ]]; then
  cp -a "${FFMPEG_ROOT}/bin/"*.dylib "${FFMPEG_LIB}/"
else
  echo "[ERROR] Invalid FFmpeg zip: missing lib/bin directory."
  exit 1
fi

APP_STAGE="${TMP_ROOT}/app-stage/AWMKit.app"
CLI_STAGE="${TMP_ROOT}/cli-stage"
mkdir -p "${TMP_ROOT}/app-stage" "${CLI_STAGE}/lib/ffmpeg" "${CLI_STAGE}/bundled"
cp -a "${APP_SRC}" "${APP_STAGE}"
mkdir -p "${APP_STAGE}/Contents/Frameworks/ffmpeg" "${APP_STAGE}/Contents/Resources/bundled"
cp -a "${FFMPEG_LIB}/"*.dylib "${APP_STAGE}/Contents/Frameworks/ffmpeg/"
cp "${MAC_BUNDLED_ZIP}" "${APP_STAGE}/Contents/Resources/bundled/"

cp "${REPO_ROOT}/target/aarch64-apple-darwin/release/awmkit" "${CLI_STAGE}/awmkit"
chmod +x "${CLI_STAGE}/awmkit"
cp -a "${FFMPEG_LIB}/"*.dylib "${CLI_STAGE}/lib/ffmpeg/"
cp "${MAC_BUNDLED_ZIP}" "${CLI_STAGE}/bundled/"

APP_TARBALL="${DIST_ROOT}/AWMKit-macos-arm64-${PACKAGE_VERSION}-${SHORT_SHA}.tar.gz"
CLI_TARBALL="${DIST_ROOT}/awmkit-cli-macos-arm64-${PACKAGE_VERSION}-${SHORT_SHA}.tar.gz"
tar -C "${TMP_ROOT}/app-stage" -czf "${APP_TARBALL}" "AWMKit.app"
tar -C "${CLI_STAGE}" -czf "${CLI_TARBALL}" .

echo "[INFO] macOS release artifacts:"
echo "  ${APP_TARBALL}"
echo "  ${CLI_TARBALL}"
