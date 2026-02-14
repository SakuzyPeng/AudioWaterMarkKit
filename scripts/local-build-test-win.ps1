param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
  [string]$RuntimeAsset = "",
  [string]$RuntimeRelease = "",
  [string]$RuntimeRepo = "SakuzyPeng/AudioWaterMarkKit",
  [string]$DevAsset = "ffmpeg-n8.0-latest-win64-gpl-shared-8.0.zip",
  [string]$DevRelease = "latest",
  [string]$DevRepo = "BtbN/FFmpeg-Builds",
  [string]$TargetDir = "",
  [bool]$CleanRebuildDirs = $true,
  [bool]$DeepClean = $false,
  [switch]$SkipTests
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
  Write-Error $Message
  exit 1
}

function Resolve-AssetPath([string]$RepoRootPath, [string]$AssetSpec) {
  if ([System.IO.Path]::IsPathRooted($AssetSpec)) {
    return $AssetSpec
  }
  return (Join-Path $RepoRootPath $AssetSpec)
}

function Ensure-ReleaseAsset(
  [string]$AssetPath,
  [string]$Release,
  [string]$Repo
) {
  if (Test-Path $AssetPath) {
    return
  }

  if (!(Get-Command gh -ErrorAction SilentlyContinue)) {
    Fail "Asset missing and gh CLI not found: $AssetPath"
  }
  if ([string]::IsNullOrWhiteSpace($Release) -or [string]::IsNullOrWhiteSpace($Repo)) {
    Fail "Asset missing and release/repo not configured: $AssetPath"
  }

  $assetName = [System.IO.Path]::GetFileName($AssetPath)
  $assetDir = [System.IO.Path]::GetDirectoryName($AssetPath)
  if ([string]::IsNullOrWhiteSpace($assetDir)) {
    $assetDir = (Get-Location).Path
  }
  New-Item -ItemType Directory -Force -Path $assetDir | Out-Null

  Write-Host "[INFO] Downloading asset: $assetName ($Repo@$Release)"
  gh release download $Release --repo $Repo -p $assetName --clobber --dir $assetDir
  if ($LASTEXITCODE -ne 0) {
    Fail "Failed to download asset: $assetName"
  }
}

Set-Location $RepoRoot

$manifestPath = Join-Path $RepoRoot "tools\ffmpeg\manifest.json"
if (([string]::IsNullOrWhiteSpace($RuntimeAsset) -or [string]::IsNullOrWhiteSpace($RuntimeRelease)) -and (Test-Path $manifestPath)) {
  $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($RuntimeAsset)) {
    $RuntimeAsset = "$($manifest.assets.windows_x86_64.name)".Trim()
  }
  if ([string]::IsNullOrWhiteSpace($RuntimeRelease)) {
    $manifestVersion = "$($manifest.version)".Trim()
    if (-not [string]::IsNullOrWhiteSpace($manifestVersion)) {
      $RuntimeRelease = "ffmpeg-runtime-$manifestVersion"
    }
  }
}

if ([string]::IsNullOrWhiteSpace($RuntimeAsset)) {
  $RuntimeAsset = "ffmpeg-win64-gpl-slim-runtime-8.0.2.zip"
}
if ([string]::IsNullOrWhiteSpace($RuntimeRelease)) {
  $RuntimeRelease = "ffmpeg-runtime-8.0.2"
}
if ([string]::IsNullOrWhiteSpace($TargetDir)) {
  $TargetDir = Join-Path $RepoRoot "target-local"
}

$runtimeAssetPath = Resolve-AssetPath $RepoRoot $RuntimeAsset
$devAssetPath = Resolve-AssetPath $RepoRoot $DevAsset
$runtimeOutDir = Join-Path $RepoRoot "ffmpeg-dist"
$devOutDir = Join-Path $RepoRoot "ffmpeg-dev"
$targetDirAbs = [System.IO.Path]::GetFullPath($TargetDir)

if ($CleanRebuildDirs) {
  Write-Host "[INFO] Cleaning rebuild dirs (repo-local)"
  foreach ($p in @($runtimeOutDir, $devOutDir, $targetDirAbs)) {
    Remove-Item -Recurse -Force $p -ErrorAction SilentlyContinue
  }
}

if ($DeepClean) {
  Write-Host "[INFO] Deep clean enabled"
  foreach ($p in @(
      (Join-Path $RepoRoot "target"),
      (Join-Path $RepoRoot "dist"),
      (Join-Path $RepoRoot "ffmpeg-prebuilt")
    )) {
    Remove-Item -Recurse -Force $p -ErrorAction SilentlyContinue
  }
}

Ensure-ReleaseAsset -AssetPath $runtimeAssetPath -Release $RuntimeRelease -Repo $RuntimeRepo
Ensure-ReleaseAsset -AssetPath $devAssetPath -Release $DevRelease -Repo $DevRepo

& (Join-Path $RepoRoot "scripts\bootstrap_ffmpeg_windows.ps1") `
  -Asset $runtimeAssetPath `
  -OutDir $runtimeOutDir `
  -InstallDev:$false `
  -GithubEnv ""

Remove-Item -Recurse -Force $devOutDir -ErrorAction SilentlyContinue
Expand-Archive -Path $devAssetPath -DestinationPath $devOutDir -Force
$devRoot = Get-ChildItem -Path $devOutDir -Directory | Select-Object -First 1
if ($null -eq $devRoot) {
  Fail "Invalid ffmpeg dev zip: missing package root directory."
}
if (!(Test-Path (Join-Path $devRoot.FullName "include\libavutil\avutil.h"))) {
  Fail "Invalid ffmpeg dev zip: missing include/libavutil/avutil.h."
}
if (!(Test-Path (Join-Path $devRoot.FullName "lib\avutil.lib"))) {
  Fail "Invalid ffmpeg dev zip: missing lib/avutil.lib."
}
$devBin = Join-Path $devRoot.FullName "bin"
if (!(Test-Path $devBin)) {
  Fail "Invalid ffmpeg dev zip: missing runtime bin directory."
}

$runtimeLibDir = Join-Path $runtimeOutDir "lib"
$coreDllPattern = '^(avcodec|avformat|avutil|avfilter|swresample)-\d+\.dll$'
$extraDlls = Get-ChildItem -Path (Join-Path $devBin "*.dll") | Where-Object { $_.Name -notmatch $coreDllPattern }
foreach ($dll in $extraDlls) {
  Copy-Item $dll.FullName $runtimeLibDir -Force
}
Write-Host ("[INFO] Injected dependency DLLs from dev package: {0}" -f $extraDlls.Count)

$repoDrive = (Get-Item $RepoRoot).PSDrive
$freeGb = [math]::Round($repoDrive.Free / 1GB, 2)
Write-Host ("[INFO] Drive {0} free space: {1} GB" -f $repoDrive.Name, $freeGb)
if ($repoDrive.Free -lt 3GB) {
  Fail "Insufficient free space (< 3GB). Clean repo artifacts or enable -DeepClean true."
}
if ($repoDrive.Free -lt 6GB) {
  Write-Warning "Low disk space (< 6GB). Build may fail on large updates."
}

New-Item -ItemType Directory -Force -Path $targetDirAbs | Out-Null
$env:PATH = "$runtimeLibDir;$devBin;$env:PATH"
$env:FFMPEG_DIR = $devRoot.FullName
$env:PKG_CONFIG_PATH = "$($devRoot.FullName)\lib\pkgconfig"
$env:CARGO_TARGET_DIR = $targetDirAbs

Write-Host "[INFO] CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR"
Write-Host "[INFO] FFMPEG_DIR=$env:FFMPEG_DIR"
Write-Host "[INFO] Building awmkit-core (full-cli, release)..."
cargo build --bin awmkit-core --features full-cli --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
  Fail "cargo build failed with exit code $LASTEXITCODE"
}

if (-not $SkipTests) {
  Write-Host "[INFO] Running tests (--features app)..."
  cargo test --features app
  if ($LASTEXITCODE -ne 0) {
    Fail "cargo test failed with exit code $LASTEXITCODE"
  }
}

Write-Host "[INFO] Done."
