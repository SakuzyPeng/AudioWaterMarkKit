#!/bin/bash
# Build zita-resampler for Windows (Cygwin)

set -ex

WORKDIR=${1:-/tmp}
cd "$WORKDIR"

# 克隆源码
if [ ! -d "zita-resampler" ]; then
    git clone --depth 1 https://github.com/digital-stage/zita-resampler.git
fi

cd zita-resampler

# 清理旧构建
rm -rf build && mkdir -p build && cd build

# CMake 配置（SHARED 库）
cmake .. \
    -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_COMPILER=gcc \
    -DCMAKE_CXX_COMPILER=g++

# 编译
make -j$(nproc)

# 安装到系统路径
echo "Installing to Cygwin system paths..."
cp libzita-resampler.dll /usr/bin/
cp libzita-resampler.dll.a /usr/lib/
mkdir -p /usr/include/zita-resampler
cp ../source/zita-resampler/*.h /usr/include/zita-resampler/

# 验证
echo "✓ zita-resampler installation verified:"
ls -lh /usr/bin/libzita-resampler.dll
pkg-config --modversion zita-resampler || echo "(pkg-config not found, but library installed)"

echo "✓ zita-resampler build complete"
