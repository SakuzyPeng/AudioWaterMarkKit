# AWMKit GUI 使用说明

本指南面向 AWMKit GUI（`awmkit-gui`），覆盖安装、功能入口、语言与重置说明。

## 安装与启动

### macOS (ARM64)

1. 解压：
```bash
tar -xzf awmkit-gui-macos-arm64.tar.gz
```

2. 启动：
```bash
open "AWMKit GUI.app"
```

如果系统提示来自未识别开发者，可执行：
```bash
xattr -d com.apple.quarantine "AWMKit GUI.app"
```

### Windows (x86_64)

```powershell
Expand-Archive awmkit-gui-windows-x86_64.zip
.\awmkit-gui.exe
```

## 主要功能入口

- **嵌入**：选择音频文件、填写 Tag、调整强度并执行嵌入。
- **检测**：批量检测文件，输出结果日志。
- **状态 / 初始化**：
  - 显示密钥与 audiowmark 状态
  - 一键初始化密钥
  - 语言切换
  - 危险操作（清理/重置）
- **标签管理**：查看与维护用户名 ↔ Tag 映射。

## 语言切换

在 **状态 / 初始化** 页的语言下拉框中选择语言，自动保存到配置文件：
- macOS/Linux: `~/.awmkit/config.toml`
- Windows: `%LOCALAPPDATA%\\awmkit\\config.toml`

## 缓存、配置与数据路径

- audiowmark 缓存：
  - macOS/Linux: `~/.awmkit/bundled/`
  - Windows: `%LOCALAPPDATA%\\awmkit\\bundled\\`
- Tag 映射：
  - macOS/Linux: `~/.awmkit/awmkit.db`（表：`tag_mappings`）
  - Windows: `%LOCALAPPDATA%\\awmkit\\awmkit.db`（表：`tag_mappings`）
- 配置文件：
  - macOS/Linux: `~/.awmkit/config.toml`
  - Windows: `%LOCALAPPDATA%\\awmkit\\config.toml`

## 危险操作（清理/重置）

在 **状态 / 初始化** 页底部的“危险操作”中：

1) **清理缓存/配置**  
删除缓存二进制与配置文件，不删除密钥。

2) **完全重置**  
删除密钥、Tag 映射、配置与缓存，且不可恢复。

为避免误触，操作时需输入确认短语：
- 清理缓存：`RESET`
- 完全重置：`AWMKIT`

## 常见问题

**macOS 提示无法打开或来源不明**  
执行：
```bash
xattr -d com.apple.quarantine "AWMKit GUI.app"
```

**Windows SmartScreen 阻止**  
选择“仍要运行”。如为企业环境，需管理员策略允许。
