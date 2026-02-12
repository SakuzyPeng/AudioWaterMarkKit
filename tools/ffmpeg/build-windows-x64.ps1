param(
    [string]$FfmpegSrc = "E:\ffmpeg",
    [string]$OutDir = "$PSScriptRoot\..\..\dist\ffmpeg-win64-gpl-slim-runtime",
    [string]$MsysBash = "C:\msys64\usr\bin\bash.exe",
    [string]$VersionTag = "8.0.2"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path $FfmpegSrc)) {
    throw "FFmpeg source directory not found: $FfmpegSrc"
}

if (!(Test-Path $MsysBash)) {
    throw "MSYS bash not found: $MsysBash"
}

$OutDir = [System.IO.Path]::GetFullPath($OutDir)
$Prefix = Join-Path $OutDir "root"
$PackageName = "ffmpeg-win64-gpl-slim-runtime-$VersionTag"
$AssetName = "$PackageName.zip"

if (Test-Path $OutDir) {
    Remove-Item -Recurse -Force $OutDir
}
New-Item -ItemType Directory -Path $OutDir | Out-Null
New-Item -ItemType Directory -Path $Prefix | Out-Null

$UnixSrc = $FfmpegSrc -replace "\\", "/"
$UnixPrefix = $Prefix -replace "\\", "/"

$configure = @"
set -euo pipefail
export MSYSTEM=MINGW64
export CHERE_INVOKING=1
export PATH=/mingw64/bin:/usr/bin:\$PATH
echo "=== Toolchain probe ==="
which gcc || true
which x86_64-w64-mingw32-gcc || true
gcc --version || true
x86_64-w64-mingw32-gcc --version || true
cd '$UnixSrc'
make distclean >/dev/null 2>&1 || true
./configure \
  --prefix='$UnixPrefix' \
  --arch=x86_64 \
  --target-os=mingw32 \
  --cc=x86_64-w64-mingw32-gcc \
  --cxx=x86_64-w64-mingw32-g++ \
  --ar=x86_64-w64-mingw32-ar \
  --ranlib=x86_64-w64-mingw32-ranlib \
  --nm=x86_64-w64-mingw32-nm \
  --enable-shared \
  --disable-static \
  --disable-programs \
  --disable-doc \
  --disable-network \
  --disable-debug \
  --disable-everything \
  --enable-protocol=file,pipe \
  --enable-avcodec \
  --enable-avformat \
  --enable-avutil \
  --enable-avfilter \
  --enable-swresample \
  --enable-filter=aformat \
  --enable-filter=aresample \
  --enable-demuxer=mov,matroska,mpegts,wav,flac,mp3,ogg,aiff \
  --enable-decoder=eac3,ac3,aac,alac,flac,mp3,opus,vorbis,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_f64le \
  --enable-encoder=pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_f64le,flac,aac,alac,libmp3lame,libopus,libvorbis \
  --enable-parser=aac,ac3,flac,mpegaudio,opus,vorbis
make -j\$(nproc)
make install
"@

& $MsysBash -lc $configure

$LibDir = Join-Path $OutDir "lib"
New-Item -ItemType Directory -Path $LibDir | Out-Null
Copy-Item "$Prefix\bin\avcodec-*.dll" $LibDir -Force
Copy-Item "$Prefix\bin\avformat-*.dll" $LibDir -Force
Copy-Item "$Prefix\bin\avutil-*.dll" $LibDir -Force
Copy-Item "$Prefix\bin\avfilter-*.dll" $LibDir -Force
Copy-Item "$Prefix\bin\swresample-*.dll" $LibDir -Force

Copy-Item "$FfmpegSrc\COPYING.GPLv3" (Join-Path $OutDir "LICENSE.GPL.txt") -Force
@"
AWMKit FFmpeg runtime (Windows x64)
Version: $VersionTag
This package contains slim shared libraries required by AWMKit runtime.
Included libraries: avcodec, avformat, avutil, avfilter, swresample.
"@ | Set-Content (Join-Path $OutDir "README.txt") -Encoding UTF8

$PackageRoot = Join-Path $OutDir $PackageName
if (Test-Path $PackageRoot) {
    Remove-Item -Recurse -Force $PackageRoot
}
New-Item -ItemType Directory -Path "$PackageRoot\bin" -Force | Out-Null
Copy-Item "$LibDir\*.dll" "$PackageRoot\bin" -Force
Copy-Item (Join-Path $OutDir "LICENSE.GPL.txt") "$PackageRoot\" -Force
Copy-Item (Join-Path $OutDir "README.txt") "$PackageRoot\" -Force

$zipPath = Join-Path $OutDir $AssetName
Compress-Archive -Path "$PackageRoot\*" -DestinationPath $zipPath -Force
$hash = Get-FileHash -Algorithm SHA256 $zipPath
"$($hash.Hash.ToLower())  $AssetName" | Set-Content (Join-Path $OutDir "$AssetName.sha256") -Encoding UTF8
Get-Item $zipPath
Get-Item (Join-Path $OutDir "$AssetName.sha256")

Write-Host "Built $zipPath"
