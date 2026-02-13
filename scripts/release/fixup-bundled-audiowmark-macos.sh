#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <bundled-audiowmark-macos-arm64.zip>" >&2
  exit 1
fi

ZIP_PATH="$1"
if [[ ! -f "${ZIP_PATH}" ]]; then
  echo "[ERROR] bundled zip not found: ${ZIP_PATH}" >&2
  exit 1
fi

if ! command -v otool >/dev/null 2>&1 || ! command -v install_name_tool >/dev/null 2>&1; then
  echo "[ERROR] otool/install_name_tool is required on macOS." >&2
  exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/awmkit-bundled-fixup.XXXXXX")"
cleanup() {
  if [[ -d "${TMP_ROOT}" ]]; then
    rm -r "${TMP_ROOT}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

unzip -q "${ZIP_PATH}" -d "${TMP_ROOT}"

BIN_DIR="${TMP_ROOT}/bin"
AUDIOWMARK_BIN="${BIN_DIR}/audiowmark"
if [[ ! -f "${AUDIOWMARK_BIN}" ]]; then
  echo "[ERROR] missing bin/audiowmark in ${ZIP_PATH}" >&2
  exit 1
fi

copy_homebrew_deps() {
  local file="$1"
  while IFS= read -r dep; do
    [[ "${dep}" =~ ^(/opt/homebrew|/usr/local)/ ]] || continue
    local base
    base="$(basename "${dep}")"
    if [[ ! -f "${dep}" ]]; then
      echo "[ERROR] dependency not found: ${dep}" >&2
      exit 1
    fi
    if [[ ! -f "${BIN_DIR}/${base}" ]]; then
      cp "${dep}" "${BIN_DIR}/${base}"
      copied_any=1
    fi
  done < <(otool -L "${file}" | awk 'NR>1 {print $1}')
}

# Copy direct + transitive Homebrew dylibs into bin/.
copied_any=1
while [[ ${copied_any} -eq 1 ]]; do
  copied_any=0
  for f in "${AUDIOWMARK_BIN}" "${BIN_DIR}"/lib*.dylib; do
    [[ -f "${f}" ]] || continue
    copy_homebrew_deps "${f}"
  done
done

for dylib in "${BIN_DIR}"/lib*.dylib; do
  [[ -f "${dylib}" ]] || continue
  base="$(basename "${dylib}")"
  install_name_tool -id "@loader_path/${base}" "${dylib}"
done

for bin in "${AUDIOWMARK_BIN}" "${BIN_DIR}"/lib*.dylib; do
  [[ -f "${bin}" ]] || continue
  while IFS= read -r dep; do
    dep_base="$(basename "${dep}")"
    if [[ -f "${BIN_DIR}/${dep_base}" ]]; then
      target_dep="@loader_path/${dep_base}"
      if [[ "${dep}" != "${target_dep}" ]]; then
        install_name_tool -change "${dep}" "${target_dep}" "${bin}"
      fi
    fi
  done < <(otool -L "${bin}" | awk 'NR>1 {print $1}')
done

if command -v codesign >/dev/null 2>&1; then
  for bin in "${AUDIOWMARK_BIN}" "${BIN_DIR}"/lib*.dylib; do
    [[ -f "${bin}" ]] || continue
    codesign --force --sign - "${bin}" >/dev/null
  done
fi

if otool -L "${AUDIOWMARK_BIN}" | awk 'NR>1 {print $1}' | grep -Eq '^(/opt/homebrew|/usr/local)/'; then
  echo "[ERROR] audiowmark still has absolute Homebrew deps after fixup." >&2
  exit 1
fi

OUT_ZIP="${TMP_ROOT}/fixed.zip"
(cd "${TMP_ROOT}" && zip -q -r "${OUT_ZIP}" bin)
cp "${OUT_ZIP}" "${ZIP_PATH}"

echo "[INFO] fixed bundled audiowmark: ${ZIP_PATH}"
