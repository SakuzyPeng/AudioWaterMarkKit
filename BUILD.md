# 编译指南

本文档说明如何编译 AWMKit 及其依赖项。

## 环境要求

### macOS
- Xcode Command Line Tools: `xcode-select --install`
- Homebrew 包管理器
- 必要工具：
  ```bash
  brew install pkg-config libsndfile mpg123 fftw libgcrypt autoconf automake libtool
  ```

### Linux (Ubuntu/Debian)
```bash
sudo apt install build-essential pkg-config libsndfile1-dev libmpg123-dev \
                 libfftw3-dev libgcrypt-dev libtool autoconf automake
```

## 1. 编译 audiowmark （必需）

audiowmark 是音频水印的核心引擎，需要提前编译。

### macOS 编译步骤

**第一步：编译 libzita-resampler**

```bash
cd /tmp
git clone https://github.com/digital-stage/zita-resampler.git libzita-resampler
cd libzita-resampler
mkdir -p build && cd build
cmake ..
make -j$(sysctl -n hw.ncpu)

# 安装到系统路径
cp libzita-resampler.a /usr/local/lib/
mkdir -p /usr/local/include/zita-resampler
cp ../source/zita-resampler/*.h /usr/local/include/zita-resampler/
```

**第二步：编译 audiowmark**

```bash
cd /tmp
git clone https://github.com/swesterfeld/audiowmark.git
cd audiowmark

./autogen.sh
./configure
make -j$(sysctl -n hw.ncpu)

# 验证
./src/audiowmark --version
# audiowmark 0.6.5
```

编译好的二进制位置：`/tmp/audiowmark/src/audiowmark`

### Linux 编译步骤

大多数 Linux 发行版提供了预编译的 libzita-resampler：

```bash
# Ubuntu/Debian
sudo apt install libzita-resampler-dev

# CentOS/RHEL
sudo yum install zita-resampler-devel

# 编译 audiowmark
cd /tmp
git clone https://github.com/swesterfeld/audiowmark.git
cd audiowmark

./autogen.sh
./configure
make -j$(nproc)
make install
```

编译好的二进制通常在 `/usr/local/bin/audiowmark`

## 2. 编译 AWMKit

### bundled 前置资源

自包含构建依赖仓库内的 bundled 资源：

```bash
bundled/audiowmark-macos-arm64.zip
```

若缺失该文件，`--features bundled` 的本地构建无法产出自包含二进制。

### 基础库编译

```bash
cd /path/to/awmkit

# 基础库（默认含 multichannel）
cargo build --release

# 库 + FFI（macOS 原生 App 推荐，bundled 优先）
cargo build --features ffi,bundled --release

# 库 + 多声道支持
cargo build --features ffi,multichannel --release

# 自包含 CLI（bundled 优先，回退 --audiowmark/PATH）
cargo build --bin awmkit --features full-cli --release
```

> 说明：`src-tauri` 已不纳入默认构建工作流（workspace 已移除）。
> 当前默认消息协议为 `v2`：4-byte 时间字段为 `27-bit UTC 分钟 + 5-bit key_slot`，单活密钥阶段 `key_slot=0`。

编译输出：
- `target/release/libawmkit.a` - 静态库
- `target/release/libawmkit.dylib` - 动态库 (macOS)
- `target/release/libawmkit.so` - 动态库 (Linux)

### Swift 绑定编译

```bash
cd /path/to/awmkit/bindings/swift

# 开发版本
swift build

# 发布版本
swift build -c release

# 运行测试
swift test
```

### Swift CLI 编译

```bash
cd /path/to/awmkit/cli-swift

# 开发版本 (仅需 Rust 库)
./build.sh

# 验证
.build/debug/awm --help

# 发布版本 (包括 audiowmark)
./dist.sh /tmp/audiowmark/src/audiowmark
```

生成的分发包：`cli-swift/dist/awm-cli-1.0.0-macos.tar.gz`

## 3. 特性说明

### multichannel feature

支持 5.1、7.1 等多声道音频处理。依赖：
- `hound` - WAV 格式支持
- `claxon` - FLAC 格式支持

启用此特性后，编译脚本会自动添加 multichannel 相关 FFI 函数。

### 构建脚本自动化

修改 `cli-swift/build.sh` 和 `dist.sh` 中的编译命令：

**build.sh**:
```bash
# 第 13 行，改为
cargo build --features ffi,multichannel --release
```

**dist.sh**:
```bash
# 第 33 行，改为
cargo build --features ffi,multichannel --release
```

## 4. 常见问题

### 编译错误：symbol(s) not found for architecture arm64

**原因**：依赖库架构不匹配（x86_64 vs arm64）

**解决**：重新编译 libzita-resampler（见第 1 步第一步）

```bash
# 检查现有库
file /usr/local/lib/libzita-resampler.a

# 清理旧版本
rm /usr/local/lib/libzita-resampler.a
rm -r /usr/local/include/zita-resampler
```

### 编译错误：cannot find -lzita-resampler

**原因**：未安装 libzita-resampler 开发文件

**解决**：
```bash
# macOS: 按第 1 步编译安装
# Linux: sudo apt install libzita-resampler-dev
```

### Swift 编译失败：Undefined symbols for architecture arm64

**原因**：Rust FFI 库未启用 multichannel feature

**解决**：
```bash
# 确保 build.sh 包含 multichannel feature
cargo build --features ffi,multichannel --release
```

### audiowmark 报错：Unknown encoder 'libgsm'

**原因**：GSM 编码器不可用

**解决**：不使用 GSM 编码的转换（项目已优化过）

## 5. 验证编译

### 验证 Rust 库

```bash
# 检查库文件
ls -lh target/release/libawmkit.*

# 运行测试
cargo test --features ffi,multichannel

# 检查 FFI 符号
nm target/release/libawmkit.dylib | grep awm_audio_embed
```

### 验证 Swift CLI

```bash
cd cli-swift/dist/awm-cli

# 检查状态
./awm status

# 检查可执行文件
ls -lh bin/
# -rwxr-xr-x  awm          (Swift CLI)
# -rwxr-xr-x  audiowmark   (音频引擎)

# 检查依赖库
otool -L bin/awm | grep awmkit
```

### 验证分发包

```bash
# 解压分发包
tar -xzf awm-cli-1.0.0-macos.tar.gz

# 运行测试
cd awm-cli
./awm status

# 输出应包含
# [OK] 密钥: 已配置 (32 字节)
# [OK] audiowmark: 可用 (bundled)
```

## 6. 编译时间参考

| 模块 | 时间 | 说明 |
|------|------|------|
| libzita-resampler | ~30s | cmake + make |
| audiowmark | ~2m | configure + make |
| Rust 库 (首次) | ~10s | 无依赖更新 |
| Rust 库 (增量) | ~2s | 仅重编译改动 |
| Swift CLI | ~10s | 首次构建 |
| 分发打包 | ~5s | 复制依赖库 |
| **总计** | **~3m** | 完整新编译 |

## 7. 跨平台编译

### macOS → Linux (交叉编译)

不建议交叉编译。建议在目标平台直接编译。

### macOS arm64 → x86_64

```bash
# 需要安装 x86_64 工具链
rustup target add x86_64-apple-darwin
cargo build --target x86_64-apple-darwin --release
```

> 注意：audiowmark 的分发包已包含兼容的二进制，无需交叉编译

## 参考资源

- [audiowmark GitHub](https://github.com/swesterfeld/audiowmark)
- [libzita-resampler GitHub](https://github.com/digital-stage/zita-resampler)
- [AWMKit README](./README.md)
- [实验数据](./EXPERIMENTS.md)
