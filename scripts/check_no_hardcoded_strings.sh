#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STRICT="${I18N_GUARD_STRICT:-0}"

TARGETS=(
  "$ROOT/macos-app/AWMKit/Sources"
  "$ROOT/winui-app/AWMKit"
)

EXCLUDE=(
  --glob '!**/bin/**'
  --glob '!**/obj/**'
  --glob '!**/publish/**'
  --glob '!**/Strings/**'
  --glob '!**/*.resw'
  --glob '!**/*.xcstrings'
)

# Very simple heuristic: detect direct CJK literals in code/xaml.
RESULT="$(rg -n "[一-龥]" "${TARGETS[@]}" "${EXCLUDE[@]}" || true)"

if [[ -z "$RESULT" ]]; then
  echo "[i18n-guard] no hardcoded CJK literals found in checked sources"
  exit 0
fi

echo "[i18n-guard] detected hardcoded CJK literals:"
echo "$RESULT"

echo
if [[ "$STRICT" == "1" ]]; then
  echo "[i18n-guard] strict mode enabled: failing build"
  exit 1
fi

echo "[i18n-guard] warning only (set I18N_GUARD_STRICT=1 to fail)"
