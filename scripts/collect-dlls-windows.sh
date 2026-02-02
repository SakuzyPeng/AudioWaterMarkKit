#!/bin/bash
# Collect DLL dependencies for Windows audiowmark binary

set -ex

AUDIOWMARK_BIN=${1:-audiowmark.exe}
DEST_DIR=${2:-./audiowmark-dist}

if [ ! -f "$AUDIOWMARK_BIN" ]; then
    echo "Error: $AUDIOWMARK_BIN not found"
    exit 1
fi

echo "Collecting DLL dependencies for: $AUDIOWMARK_BIN"

# 创建目标目录
mkdir -p "$DEST_DIR/bin"

# 复制可执行文件
cp "$AUDIOWMARK_BIN" "$DEST_DIR/bin/"

# 获取依赖列表
echo "Analyzing dependencies with ldd..."
ldd "$AUDIOWMARK_BIN" | tee "$DEST_DIR/dependencies.txt" || true

# 复制所有 DLL 依赖
echo "Copying DLL dependencies..."
DLL_COUNT=0

# 方法 1: 从 ldd 输出提取 DLL 路径
ldd "$AUDIOWMARK_BIN" 2>/dev/null | grep -o '/[^ ]*\.dll' | sort -u | while read dll; do
    if [ -f "$dll" ]; then
        echo "Copying: $dll"
        cp "$dll" "$DEST_DIR/bin/"
        DLL_COUNT=$((DLL_COUNT + 1))
    fi
done

# 方法 2: 复制常见的 Cygwin 依赖库（以防万一）
echo "Copying standard Cygwin libraries..."
for dll in \
    /usr/bin/cygwin1.dll \
    /usr/bin/cyggcc_s-seh-1.dll \
    /usr/bin/cygstdc++-6.dll \
    /usr/bin/cygfftw3f-3.dll \
    /usr/bin/cygsndfile-1.dll \
    /usr/bin/cygmpg123-0.dll \
    /usr/bin/cyggcrypt-20.dll \
    /usr/bin/cyggpg-error-0.dll \
    /usr/bin/cygz.dll \
    /usr/bin/cygFLAC.so \
    /usr/bin/cygogg.so \
    /usr/bin/cygvorbis.so \
    /usr/bin/cygvorbisenc.so \
    /usr/bin/cygopus.so \
    /usr/bin/cygmp3lame.so \
    /usr/bin/libzita-resampler.dll
do
    if [ -f "$dll" ]; then
        cp "$dll" "$DEST_DIR/bin/" 2>/dev/null || true
    fi
done

# 显示收集的 DLL
echo ""
echo "Collected DLLs:"
ls -lh "$DEST_DIR/bin/"/*.dll 2>/dev/null | awk '{print $9, "(" $5 ")"}'
echo ""
echo "Total DLL count: $(ls -1 "$DEST_DIR/bin/"*.dll 2>/dev/null | wc -l)"

# 创建运行脚本
cat > "$DEST_DIR/bin/run.sh" << 'EOF'
#!/bin/bash
# Set library path to find DLLs in current directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export PATH="$SCRIPT_DIR:$PATH"
./audiowmark.exe "$@"
EOF
chmod +x "$DEST_DIR/bin/run.sh"

echo "✓ DLL collection complete"
echo "✓ Distribution directory: $DEST_DIR"
echo "✓ To run: cd $DEST_DIR/bin && ./audiowmark.exe --version"
