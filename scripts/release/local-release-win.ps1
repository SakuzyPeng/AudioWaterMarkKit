Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

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
  "swresample-6.dll",
  "swscale-9.dll"
)
foreach ($dll in $requiredFfmpegDlls) {
  if (-not (Test-Path (Join-Path $ffmpegRuntimeDir $dll))) {
    Fail "Missing FFmpeg runtime DLL: $dll under $ffmpegRuntimeDir"
  }
}

if (-not (Get-Command iscc -ErrorAction SilentlyContinue)) {
  Fail "Inno Setup compiler (iscc) not found in PATH."
}

Write-Host "[INFO] Building Rust FFI/CLI for Windows..."
cargo build --release --features ffi,app,bundled --target x86_64-pc-windows-msvc
cargo build --release --features full-cli,bundled --bin awmkit --target x86_64-pc-windows-msvc

Write-Host "[INFO] Publishing WinUI app (multi-file)..."
dotnet publish "$RepoRoot\winui-app\AWMKit\AWMKit.csproj" `
  -c Release `
  -r win-x64 `
  -p:Platform=x64 `
  -p:SelfContained=true `
  -p:PublishSingleFile=false `
  -p:PublishTrimmed=false

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
Copy-Item "$ffmpegRuntimeDir\*.dll" (Join-Path $appStage "lib\ffmpeg\") -Force
New-Item -ItemType Directory -Force -Path (Join-Path $appStage "bundled") | Out-Null
Copy-Item $bundledZip (Join-Path $appStage "bundled\") -Force
New-Item -ItemType Directory -Force -Path (Join-Path $appStage "cli") | Out-Null
Copy-Item "$RepoRoot\target\x86_64-pc-windows-msvc\release\awmkit.exe" (Join-Path $appStage "cli\awmkit.exe") -Force

Write-Host "[INFO] Smoke test: CLI version"
& (Join-Path $appStage "cli\awmkit.exe") --version

New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
Write-Host "[INFO] Building Inno installer..."
iscc `
  "/DAppVersion=$version" `
  "/DAppPackageVersion=$packageVersion" `
  "/DAppCommit=$buildCommit" `
  "/DAppSourceDir=$appStage" `
  "/DOutputDir=$distRoot" `
  "$RepoRoot\packaging\windows\inno.iss" | Out-Host

$installer = Join-Path $distRoot "AWMKit-win-x64-ui-installer-$packageVersion-$buildCommit.exe"
if (-not (Test-Path $installer)) {
  $latest = Get-ChildItem -Path $distRoot -Filter "AWMKit-win-x64-ui-installer-*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if ($null -eq $latest) {
    Fail "Installer not produced."
  }
  $installer = $latest.FullName
}

Write-Host "[INFO] Windows installer ready: $installer"
