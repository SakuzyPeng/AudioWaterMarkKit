param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
  Write-Error $Message
  exit 1
}

Set-Location $RepoRoot

$dirty = git status --porcelain --untracked-files=no
if (-not [string]::IsNullOrWhiteSpace($dirty)) {
  Fail "Repository has tracked modifications. Commit/stash tracked changes before local release."
}

$shortSha = (git rev-parse --short HEAD).Trim()
$buildCommit = $shortSha
if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_RUN_ID)) {
  $runAttempt = if ([string]::IsNullOrWhiteSpace($env:GITHUB_RUN_ATTEMPT)) { "1" } else { $env:GITHUB_RUN_ATTEMPT.Trim() }
  $buildCommit = "$shortSha-r$($env:GITHUB_RUN_ID.Trim())a$runAttempt"
}
$headTag = (git tag --points-at HEAD | Select-Object -First 1)
$headTag = if ($null -eq $headTag) { "" } else { $headTag.Trim() }
$baseVersionMatch = Select-String -Path (Join-Path $RepoRoot "Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if ($null -eq $baseVersionMatch) {
  Fail "Unable to parse version from Cargo.toml."
}
$baseVersion = $baseVersionMatch.Matches[0].Groups[1].Value
$version = if ([string]::IsNullOrWhiteSpace($headTag)) { "$baseVersion+$shortSha" } else { $headTag }
$packageVersion = ($version -replace '[\+/]', '_' -replace '[^0-9A-Za-z._-]', '-')

$bundledZip = Join-Path $RepoRoot "bundled\audiowmark-windows-x86_64.zip"
if (-not (Test-Path $bundledZip)) {
  Fail "Missing bundled zip: $bundledZip"
}

$ffmpegRuntimeDir = Join-Path $RepoRoot "ffmpeg-prebuilt\bin"
if (-not (Test-Path (Join-Path $ffmpegRuntimeDir "avcodec-62.dll"))) {
  if (-not $env:FFMPEG_WINDOWS_RUNTIME_ZIP) {
    $candidate = Get-ChildItem -Path $RepoRoot -Filter "ffmpeg-win64-gpl-slim-runtime-*.zip" | Sort-Object Name | Select-Object -Last 1
    if ($null -eq $candidate) {
      Fail "Missing FFmpeg runtime zip. Set FFMPEG_WINDOWS_RUNTIME_ZIP or place ffmpeg-win64-gpl-slim-runtime-*.zip in repo root."
    }
    $env:FFMPEG_WINDOWS_RUNTIME_ZIP = $candidate.FullName
  }

  if (-not (Test-Path $env:FFMPEG_WINDOWS_RUNTIME_ZIP)) {
    Fail "FFmpeg runtime zip not found: $($env:FFMPEG_WINDOWS_RUNTIME_ZIP)"
  }

  $tmpExtract = Join-Path $RepoRoot ".tmp-release\windows\ffmpeg-unpack"
  Remove-Item $tmpExtract -Recurse -Force -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $tmpExtract | Out-Null
  Expand-Archive -Path $env:FFMPEG_WINDOWS_RUNTIME_ZIP -DestinationPath $tmpExtract -Force
  $root = Get-ChildItem -Path $tmpExtract -Directory | Select-Object -First 1
  if ($null -eq $root) {
    Fail "Invalid FFmpeg zip: missing root directory."
  }
  New-Item -ItemType Directory -Force -Path $ffmpegRuntimeDir | Out-Null
  if (Test-Path (Join-Path $root.FullName "bin")) {
    Copy-Item (Join-Path $root.FullName "bin\*.dll") $ffmpegRuntimeDir -Force
  } elseif (Test-Path (Join-Path $root.FullName "lib")) {
    Copy-Item (Join-Path $root.FullName "lib\*.dll") $ffmpegRuntimeDir -Force
  } else {
    Fail "Invalid FFmpeg zip: missing bin/lib directory."
  }
}

$requiredFfmpegDlls = @(
  "avcodec-62.dll",
  "avformat-62.dll",
  "avutil-60.dll",
  "avfilter-11.dll",
  "swresample-6.dll"
)
foreach ($dll in $requiredFfmpegDlls) {
  if (-not (Test-Path (Join-Path $ffmpegRuntimeDir $dll))) {
    Fail "Missing FFmpeg runtime DLL: $dll under $ffmpegRuntimeDir"
  }
}

$ffmpegBundleDirs = @($ffmpegRuntimeDir)
$ciPreparedFfmpegDir = Join-Path $RepoRoot "ffmpeg-dist\lib"
if (Test-Path $ciPreparedFfmpegDir) {
  $ffmpegBundleDirs += $ciPreparedFfmpegDir
}

if (-not (Get-Command iscc -ErrorAction SilentlyContinue)) {
  Fail "Inno Setup compiler (iscc) not found in PATH."
}

Write-Host "[INFO] Building Rust FFI/core CLI for Windows..."
cargo build --release --features ffi,app,bundled --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
  Fail "cargo build (ffi/app/bundled) failed with exit code $LASTEXITCODE"
}
cargo build --release --features full-cli,bundled --bin awmkit-core --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
  Fail "cargo build (full-cli core) failed with exit code $LASTEXITCODE"
}

$rustFfiDll = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\release\awmkit.dll"
if (-not (Test-Path $rustFfiDll)) {
  Fail "Missing Rust FFI DLL after build: $rustFfiDll"
}

# Ensure csproj-included native DLL exists before dotnet publish item copy phase.
$projectNativeDll = Join-Path $RepoRoot "winui-app\AWMKit\awmkit_native.dll"
Copy-Item $rustFfiDll $projectNativeDll -Force

Write-Host "[INFO] Publishing WinUI app (multi-file)..."
dotnet publish "$RepoRoot\winui-app\AWMKit\AWMKit.csproj" `
  -c Release `
  -r win-x64 `
  -p:Platform=x64 `
  -p:SelfContained=true `
  -p:PublishSingleFile=false `
  -p:PublishTrimmed=false
if ($LASTEXITCODE -ne 0) {
  Fail "dotnet publish failed with exit code $LASTEXITCODE"
}

$publishDir = Get-ChildItem -Path "$RepoRoot\winui-app\AWMKit\bin" -Directory -Recurse |
  Where-Object { $_.Name -eq "publish" } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if ($null -eq $publishDir) {
  Fail "Publish directory not found."
}

$stagingRoot = Join-Path $RepoRoot ".tmp-release\windows\staging"
$appStage = Join-Path $stagingRoot "app"
$distRoot = Join-Path $RepoRoot "dist\local"
Remove-Item $stagingRoot -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $appStage | Out-Null
Copy-Item "$($publishDir.FullName)\*" $appStage -Recurse -Force

$nativeDll = Join-Path $appStage "awmkit_native.dll"
if (-not (Test-Path $nativeDll)) {
  Copy-Item "$RepoRoot\target\x86_64-pc-windows-msvc\release\awmkit.dll" $nativeDll -Force
}

New-Item -ItemType Directory -Force -Path (Join-Path $appStage "lib\ffmpeg") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appStage "cli") | Out-Null
foreach ($dir in $ffmpegBundleDirs) {
  $dlls = Get-ChildItem -Path $dir -Filter "*.dll" -File -ErrorAction SilentlyContinue
  if ($null -eq $dlls -or $dlls.Count -eq 0) {
    continue
  }
  Copy-Item $dlls.FullName (Join-Path $appStage "lib\ffmpeg\") -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $appStage "bundled") | Out-Null
Copy-Item $bundledZip (Join-Path $appStage "bundled\") -Force

$launcherPayloadDir = Join-Path $stagingRoot "launcher-payload"
$launcherPayloadZip = Join-Path $stagingRoot "launcher-payload.zip"
Remove-Item $launcherPayloadDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $launcherPayloadZip -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $launcherPayloadDir | Out-Null
Copy-Item "$RepoRoot\target\x86_64-pc-windows-msvc\release\awmkit-core.exe" (Join-Path $launcherPayloadDir "awmkit-core.exe") -Force
Copy-Item (Join-Path $appStage "lib\ffmpeg\*.dll") $launcherPayloadDir -Force
@'
{"core_binary":"awmkit-core.exe"}
'@ | Set-Content -Path (Join-Path $launcherPayloadDir "manifest.json") -Encoding UTF8
Compress-Archive -Path "$launcherPayloadDir\*" -DestinationPath $launcherPayloadZip -Force

$env:AWMKIT_LAUNCHER_PAYLOAD = $launcherPayloadZip
cargo build --release --features launcher --bin awmkit --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
  Fail "cargo build (launcher) failed with exit code $LASTEXITCODE"
}

Copy-Item "$RepoRoot\target\x86_64-pc-windows-msvc\release\awmkit.exe" (Join-Path $appStage "cli\awmkit.exe") -Force

Write-Host "[INFO] Smoke test: installed CLI mode (clean PATH)"
$runtimeRoot = Join-Path $env:LOCALAPPDATA "awmkit\runtime"
Remove-Item -Recurse -Force $runtimeRoot -ErrorAction SilentlyContinue
$oldPath = $env:PATH
$env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
try {
  & (Join-Path $appStage "cli\awmkit.exe") status --doctor
  if ($LASTEXITCODE -ne 0) {
    Fail "CLI smoke test failed with exit code $LASTEXITCODE"
  }
} finally {
  $env:PATH = $oldPath
}
if (-not (Test-Path $runtimeRoot)) {
  Fail "CLI runtime extraction directory was not created: $runtimeRoot"
}

New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
Write-Host "[INFO] Building Inno installer..."
iscc `
  "/DAppVersion=$version" `
  "/DAppPackageVersion=$packageVersion" `
  "/DAppCommit=$buildCommit" `
  "/DAppSourceDir=$appStage" `
  "/DOutputDir=$distRoot" `
  "$RepoRoot\packaging\windows\inno.iss" | Out-Host
if ($LASTEXITCODE -ne 0) {
  Fail "iscc build failed with exit code $LASTEXITCODE"
}

$installer = Join-Path $distRoot "AWMKit-win-x64-ui-installer-$packageVersion-$buildCommit.exe"
if (-not (Test-Path $installer)) {
  $latest = Get-ChildItem -Path $distRoot -Filter "AWMKit-win-x64-ui-installer-*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if ($null -eq $latest) {
    Fail "Installer not produced."
  }
  $installer = $latest.FullName
}

Write-Host "[INFO] Windows installer ready: $installer"
