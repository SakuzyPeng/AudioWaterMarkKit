param(
    [string]$FfmpegSrc = "E:\ffmpeg",
    [string]$OutDir = "$PSScriptRoot\..\..\dist\ffmpeg-windows-x86_64-minimal",
    [string]$MsysBash = "C:\msys64\usr\bin\bash.exe"
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
$AssetName = "ffmpeg-windows-x86_64-minimal.zip"

if (Test-Path $OutDir) {
    Remove-Item -Recurse -Force $OutDir
}
New-Item -ItemType Directory -Path $OutDir | Out-Null
New-Item -ItemType Directory -Path $Prefix | Out-Null

$UnixSrc = $FfmpegSrc -replace "\\", "/"
$UnixPrefix = $Prefix -replace "\\", "/"

$configure = @"
set -euo pipefail
cd '$UnixSrc'
make distclean >/dev/null 2>&1 || true
./configure \
  --prefix='$UnixPrefix' \
  --arch=x86_64 \
  --target-os=mingw32 \
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
  --enable-swresample \
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
Copy-Item "$Prefix\bin\swresample-*.dll" $LibDir -Force
Copy-Item "$Prefix\include" (Join-Path $OutDir "include") -Recurse -Force

$zipPath = Join-Path $OutDir $AssetName
Compress-Archive -Path "$LibDir\*", (Join-Path $OutDir "include\*") -DestinationPath $zipPath -Force
$hash = Get-FileHash -Algorithm SHA256 $zipPath
"$($hash.Hash.ToLower())  $AssetName" | Set-Content (Join-Path $OutDir "$AssetName.sha256") -Encoding UTF8
Get-Item $zipPath
Get-Item (Join-Path $OutDir "$AssetName.sha256")

Write-Host "Built $zipPath"
