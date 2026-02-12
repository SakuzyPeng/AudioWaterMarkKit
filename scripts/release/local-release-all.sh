#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

WIN_HOST="${WIN_HOST:-win-pc}"
WIN_REPO="${WIN_REPO:-D:\\awmkit\\AudioWaterMarkKit}"

cd "${REPO_ROOT}"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "[ERROR] Local repo is dirty. Commit/stash changes before local release."
  exit 1
fi

echo "[INFO] Building macOS local release..."
"${SCRIPT_DIR}/local-release-macos.sh"

echo "[INFO] Validating win-pc worktree clean..."
ssh "${WIN_HOST}" "powershell -NoProfile -Command \"Set-Location '${WIN_REPO}'; \$dirty = git status --porcelain --untracked-files=no; if (-not [string]::IsNullOrWhiteSpace(\$dirty)) { Write-Host \$dirty; exit 1 }\""

echo "[INFO] Building Windows local release on ${WIN_HOST}..."
ssh "${WIN_HOST}" "powershell -NoProfile -ExecutionPolicy Bypass -File '${WIN_REPO}\\scripts\\release\\local-release-win.ps1'"

echo "[INFO] Local release complete."
