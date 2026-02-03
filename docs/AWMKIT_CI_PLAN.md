# AWMKit CI 使用指南

## 概述

**Workflow**: `.github/workflows/build-awmkit.yml`

**支持平台**:
- ✅ macOS ARM64 (Apple Silicon)
- ✅ Windows x86_64
- ⏸️ Linux (暂缓)

**产物**: 完全自包含的 awmkit CLI（内嵌 audiowmark 二进制）

## 触发方式

### 1. 自动触发（推送 tag）

```bash
git tag awmkit-0.1.0
git push origin awmkit-0.1.0
```

- Tag 格式必须为 `awmkit-*`
- 自动触发 CI 构建两个平台
- 自动创建 GitHub Release

### 2. 手动触发（Workflow Dispatch）

在 GitHub Actions 页面：
1. 选择 "Build awmkit CLI" workflow
2. 点击 "Run workflow"
3. 填写参数：
   - **tag**: 发布标签（如 `awmkit-0.1.0`）
   - **prerelease**: 是否为预发布版本

或使用 `gh` CLI：

```bash
gh workflow run build-awmkit.yml \
  -f tag="awmkit-0.1.0" \
  -f prerelease=false
```

## 工作流程

### 核心步骤（每个平台）

1. **下载 audiowmark**：
   - macOS: `audiowmark-macos-arm64-2026-02-03`
   - Windows: `audiowmark-win-2026-02-03`

2. **解压并压缩**：
   - 解压 tar.gz/zip
   - 提取 `audiowmark` 二进制
   - 使用 zstd -19 重新压缩
   - 覆盖 `bundled/*.zst`

3. **编译 awmkit**：
   ```bash
   cargo build --bin awmkit --features full-cli --release --target <target>
   ```

4. **验证**：
   ```bash
   awmkit --version
   ```

5. **打包**：
   - macOS: `awmkit-macos-arm64.tar.gz`
   - Windows: `awmkit-windows-x86_64.zip`

6. **发布**：
   - 创建 GitHub Release
   - 上传二进制包

## 产物说明

### macOS ARM64

```
awmkit-macos-arm64.tar.gz  (~1.2 MB)
├── awmkit-macos-arm64/
    └── awmkit             (可执行文件，包含内嵌 audiowmark)
```

**使用**：
```bash
tar -xzf awmkit-macos-arm64.tar.gz
cd awmkit-macos-arm64
./awmkit --version
./awmkit init
```

### Windows x86_64

```
awmkit-windows-x86_64.zip  (~1.5 MB)
├── awmkit-windows-x86_64/
    └── awmkit.exe         (可执行文件，包含内嵌 audiowmark)
```

**使用**：
```powershell
Expand-Archive awmkit-windows-x86_64.zip
cd awmkit-windows-x86_64
.\awmkit.exe --version
.\awmkit.exe init
```

## 依赖管理

### 上游依赖（audiowmark releases）

CI 依赖以下 audiowmark releases 存在：

| 平台 | Release Tag | Asset 名称 |
|------|-------------|-----------|
| macOS ARM64 | `audiowmark-macos-arm64-2026-02-03` | `audiowmark-macos-arm64.tar.gz` |
| Windows x86_64 | `audiowmark-win-2026-02-03` | `audiowmark-windows-x86_64.zip` |

**更新方法**：

修改 `.github/workflows/build-awmkit.yml` 中的 matrix：

```yaml
- audiowmark_release: audiowmark-macos-arm64-YYYY-MM-DD  # 修改这里
- audiowmark_release: audiowmark-win-YYYY-MM-DD          # 修改这里
```

### Bundled 占位文件

仓库中 `bundled/` 目录包含占位文件：

```
bundled/
├── audiowmark-macos-arm64.zst       (206KB, 真实压缩文件)
├── audiowmark-windows-x86_64.exe.zst (25B, 占位文件)
└── .gitignore
```

**CI 会在构建时覆盖这些文件，不会提交回仓库。**

## 缓存优化

使用 `actions/cache` 缓存：
- `~/.cargo/registry`
- `~/.cargo/git`
- `target/`

**缓存键**：`${{ runner.os }}-cargo-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}`

## 验证 & 诊断

### 本地测试（模拟 CI）

```bash
# 下载 audiowmark
gh release download audiowmark-macos-arm64-2026-02-03 \
  -p audiowmark-macos-arm64.tar.gz

# 解压并准备
tar -xzf audiowmark-macos-arm64.tar.gz
cp audiowmark-dist/bin/audiowmark .
zstd -19 --force audiowmark -o bundled/audiowmark-macos-arm64.zst

# 编译
cargo build --bin awmkit --features full-cli --release

# 验证
./target/release/awmkit --version
./target/release/awmkit status --doctor
```

### Release 验证清单

构建完成后，验证：

- [ ] 下载 `awmkit-macos-arm64.tar.gz`
- [ ] 解压并运行 `./awmkit --version`
- [ ] 运行 `./awmkit status --doctor`（应显示 bundled audiowmark）
- [ ] 测试 `./awmkit init && ./awmkit encode --tag SAKUZY`
- [ ] Windows 同样验证

## 后续扩展

### 添加 macOS x86_64

1. 下载并发布 `audiowmark-macos-x86_64` release
2. 添加 matrix 配置：
   ```yaml
   - os: macos-latest
     target: x86_64-apple-darwin
     audiowmark_release: audiowmark-macos-x86_64-YYYY-MM-DD
     zstd_name: audiowmark-macos-x86_64.zst
   ```
3. 创建占位文件 `bundled/audiowmark-macos-x86_64.zst`

### 添加 Linux

1. 构建 Linux musl 静态链接版 audiowmark
2. 发布 `audiowmark-linux-x86_64-musl` release
3. 添加 matrix 配置
4. 注意：Linux 的 keyring 依赖 Secret Service

## 常见问题

### Q: CI 失败：audiowmark release 不存在

**A**: 检查 audiowmark release 是否已发布，tag 名称是否匹配。

### Q: 编译失败：找不到 bundled/*.zst

**A**: 确保占位文件存在于仓库中，CI 会覆盖它们。

### Q: Windows 产物比 macOS 大

**A**: 正常，Windows 的 zstd 压缩后仍比 macOS 大（~1.5MB vs ~1.2MB）。

### Q: 如何验证自包含？

**A**: 在干净环境运行 `awmkit status --doctor`，应显示 bundled 路径：

```
audiowmark: available
audiowmark path: ~/.awmkit/bin/audiowmark
```

