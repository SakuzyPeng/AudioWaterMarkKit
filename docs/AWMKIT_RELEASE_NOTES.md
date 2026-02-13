# AWMKit 0.1.3

## 使用说明

### 下载与运行
**macOS ARM64**:
```bash
tar -xzf awmkit-macos-arm64.tar.gz
./awmkit --version
```
如果 macOS 提示无法打开或来自未识别的开发者：
```bash
xattr -d com.apple.quarantine ./awmkit
```

**macOS ARM64 (GUI)**:
```bash
tar -xzf awmkit-gui-macos-arm64.tar.gz
./awmkit-gui
```
如果 macOS 提示无法打开或来自未识别的开发者：
```bash
xattr -d com.apple.quarantine ./awmkit-gui
```

**Windows x86_64**:
```powershell
Expand-Archive awmkit-windows-x86_64.zip
.\awmkit.exe --version
```

**Windows x86_64 (GUI)**:
```powershell
Expand-Archive awmkit-gui-windows-x86_64.zip
.\awmkit-gui.exe
```

### 初始化密钥
```bash
awmkit init
```

### 嵌入与检测
```bash
awmkit embed --tag TESTA input.wav
awmkit detect input_wm.wav
```

## 简单 Changelog
- **Bundled audiowmark**：内嵌引擎，首次运行自动解压到 `~/.awmkit/bundled/bin/`。
- **CLI 单文件分发**：包内仅包含 `awmkit`/`awmkit.exe` 启动器，首次运行自动解压 runtime 到用户目录。
- **运行时清理命令**：新增 `awmkit cache clean --yes`（仅 runtime）与 `awmkit cache clean --db --yes`（runtime + db/config，密钥仅提醒不阻塞）。
- **Windows DPAPI 回退**：当 Credential Manager 不可用时自动使用 DPAPI。
- **打包结构优化**：发布包内为扁平结构，直接包含可执行文件。
- **Tag 映射工具**：新增 `tag suggest/save/list/remove/clear`（JSON 明文存储，可选保存）。
- **GUI (Slint)**：新增嵌入/检测与初始化界面，支持映射快捷选择与保存提示。
- **国际化 (i18n)**：CLI/GUI 支持 `zh-CN` 与 `en-US`，CLI 支持 `--lang` 与 `LANG/LC_ALL`，GUI 可切换并记忆语言。
