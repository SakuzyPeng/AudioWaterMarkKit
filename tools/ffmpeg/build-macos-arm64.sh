#!/usr/bin/env bash
set -euo pipefail

# Build slim FFmpeg runtime package for macOS arm64.
# Usage:
#   ./tools/ffmpeg/build-macos-arm64.sh /Users/Sakuzy/code/ffmpeg

FFMPEG_SRC="${1:-/Users/Sakuzy/code/ffmpeg}"
OUT_DIR="${2:-$PWD/dist/ffmpeg-macos-arm64-gpl-slim-runtime}"
PREFIX="${OUT_DIR}/root"
JOBS="${JOBS:-$(sysctl -n hw.ncpu)}"
VERSION_TAG="${VERSION_TAG:-8.0.2}"
PKG_DIR_NAME="ffmpeg-macos-arm64-gpl-slim-runtime-${VERSION_TAG}"
ASSET_NAME="${PKG_DIR_NAME}.zip"

if [[ ! -d "${FFMPEG_SRC}" ]]; then
  echo "FFmpeg source directory not found: ${FFMPEG_SRC}" >&2
  exit 1
fi

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}" "${PREFIX}"

pushd "${FFMPEG_SRC}" >/dev/null
make distclean >/dev/null 2>&1 || true

./configure \
  --prefix="${PREFIX}" \
  --arch=arm64 \
  --target-os=darwin \
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
  --enable-parser=aac,ac3,flac,mpegaudio,opus,vorbis \
  --extra-cflags="-O2 -fPIC" \
  --extra-ldflags="-Wl,-rpath,@loader_path"

make -j"${JOBS}"
make install
popd >/dev/null

mkdir -p "${OUT_DIR}/lib"
cp -a "${PREFIX}/lib/"libavcodec*.dylib "${OUT_DIR}/lib/"
cp -a "${PREFIX}/lib/"libavformat*.dylib "${OUT_DIR}/lib/"
cp -a "${PREFIX}/lib/"libavutil*.dylib "${OUT_DIR}/lib/"
cp -a "${PREFIX}/lib/"libavfilter*.dylib "${OUT_DIR}/lib/"
cp -a "${PREFIX}/lib/"libswresample*.dylib "${OUT_DIR}/lib/"
cp "${FFMPEG_SRC}/COPYING.GPLv2" "${OUT_DIR}/LICENSE.GPLv2.txt"
cat > "${OUT_DIR}/README.txt" <<EOF
AWMKit FFmpeg runtime (macOS arm64)
Version: ${VERSION_TAG}
This package contains slim shared libraries required by AWMKit runtime.
Included libraries: avcodec, avformat, avutil, avfilter, swresample.
EOF

pushd "${OUT_DIR}" >/dev/null
rm -rf "${PKG_DIR_NAME}"
mkdir -p "${PKG_DIR_NAME}"
cp -a lib "${PKG_DIR_NAME}/"
cp -a LICENSE.GPLv2.txt README.txt "${PKG_DIR_NAME}/"
zip -r "${ASSET_NAME}" "${PKG_DIR_NAME}"
shasum -a 256 "${ASSET_NAME}" > "${ASSET_NAME}.sha256"
ls -lh "${ASSET_NAME}" "${ASSET_NAME}.sha256"
popd >/dev/null

echo "Built ${OUT_DIR}/${ASSET_NAME}"
