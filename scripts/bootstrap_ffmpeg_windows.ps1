param(
    [Parameter(Mandatory = $true)][string]$Asset,
    [string]$Repo = "",
    [string]$Release = "",
    [string]$OutDir = "ffmpeg-dist",
    [bool]$InstallDev = $true,
    [string]$GithubEnv = $env:GITHUB_ENV
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (!(Test-Path $Asset)) {
    if ([string]::IsNullOrWhiteSpace($Repo) -or [string]::IsNullOrWhiteSpace($Release)) {
        throw "Asset not found locally and --repo/--release not provided: $Asset"
    }
    if (!(Get-Command gh -ErrorAction SilentlyContinue)) {
        throw "gh CLI not found, cannot download $Asset"
    }
    gh release download $Release --repo $Repo -p $Asset --clobber
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to download ffmpeg asset: $Asset"
    }
}

Remove-Item -Recurse -Force $OutDir -ErrorAction SilentlyContinue
Expand-Archive -Path $Asset -DestinationPath $OutDir -Force

$root = Get-ChildItem -Path $OutDir -Directory | Select-Object -First 1
if ($null -eq $root) {
    throw "Invalid ffmpeg zip: missing package root directory."
}

$runtimeLibDir = Join-Path $OutDir "lib"
if (Test-Path (Join-Path $root.FullName "lib")) {
    Copy-Item (Join-Path $root.FullName "lib") $OutDir -Recurse -Force
} elseif (Test-Path (Join-Path $root.FullName "bin")) {
    New-Item -ItemType Directory -Force -Path $runtimeLibDir | Out-Null
    Copy-Item (Join-Path $root.FullName "bin\\*.dll") $runtimeLibDir -Force
} else {
    throw "Invalid ffmpeg zip: missing lib/bin directory."
}

if (!(Test-Path (Join-Path $runtimeLibDir "avcodec-62.dll"))) { throw "Missing avcodec-62.dll in ffmpeg package." }
if (!(Test-Path (Join-Path $runtimeLibDir "avformat-62.dll"))) { throw "Missing avformat-62.dll in ffmpeg package." }
if (!(Test-Path (Join-Path $runtimeLibDir "avutil-60.dll"))) { throw "Missing avutil-60.dll in ffmpeg package." }
if (!(Test-Path (Join-Path $runtimeLibDir "avfilter-11.dll"))) { throw "Missing avfilter-11.dll in ffmpeg package." }
if (!(Test-Path (Join-Path $runtimeLibDir "swresample-6.dll"))) { throw "Missing swresample-6.dll in ffmpeg package." }

$manifestPath = "tools/ffmpeg/manifest.json"
if (Test-Path $manifestPath) {
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    $assetName = [System.IO.Path]::GetFileName($Asset)
    $expected = $null
    foreach ($entry in $manifest.assets.PSObject.Properties) {
        if ($entry.Value.name -eq $assetName) {
            $expected = "$($entry.Value.sha256)".Trim()
            break
        }
    }
    if (![string]::IsNullOrWhiteSpace($expected)) {
        $actual = (Get-FileHash -Algorithm SHA256 $Asset).Hash.ToLowerInvariant()
        if ($actual -ne $expected.ToLowerInvariant()) {
            throw "sha256 mismatch: $actual != $expected"
        }
        Write-Host "ffmpeg asset sha256 check passed."
    } else {
        Write-Host "ffmpeg asset sha256 check skipped (asset not found in manifest)."
    }
}

if ($InstallDev) {
    $vcpkgExe = "C:\vcpkg\vcpkg.exe"
    if (!(Test-Path $vcpkgExe)) {
        throw "vcpkg executable not found: $vcpkgExe"
    }
    $headerPath = "C:\vcpkg\installed\x64-windows\include\libavutil\avutil.h"
    if (!(Test-Path $headerPath)) {
        & $vcpkgExe install ffmpeg[avcodec,avformat,avfilter,swresample]:x64-windows --recurse
        if ($LASTEXITCODE -ne 0) {
            throw "vcpkg ffmpeg install failed with exit code $LASTEXITCODE"
        }
    }
}

$runtimeDirAbs = (Resolve-Path $OutDir).Path
$runtimeLibDirAbs = (Resolve-Path $runtimeLibDir).Path
Write-Host "FFmpeg runtime prepared at: $runtimeDirAbs"
Write-Host "FFmpeg runtime libs at: $runtimeLibDirAbs"

if (![string]::IsNullOrWhiteSpace($GithubEnv)) {
    "FFMPEG_RUNTIME_DIR=$runtimeDirAbs" | Out-File -FilePath $GithubEnv -Encoding utf8 -Append
    "FFMPEG_RUNTIME_LIB_DIR=$runtimeLibDirAbs" | Out-File -FilePath $GithubEnv -Encoding utf8 -Append
    if ($InstallDev) {
        "VCPKG_ROOT=C:\vcpkg" | Out-File -FilePath $GithubEnv -Encoding utf8 -Append
    }
}

