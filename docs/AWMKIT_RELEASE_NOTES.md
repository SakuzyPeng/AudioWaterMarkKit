# AWMKit 0.1.2

## 使用说明

### 下载与运行
**macOS ARM64**:
```bash
tar -xzf awmkit-macos-arm64.tar.gz
cd awmkit-macos-arm64
./awmkit --version
```
如果 macOS 提示无法打开或来自未识别的开发者：
```bash
xattr -d com.apple.quarantine ./awmkit
```

**Windows x86_64**:
```powershell
Expand-Archive awmkit-windows-x86_64.zip
cd awmkit-windows-x86_64
.\awmkit.exe --version
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
- **Windows DPAPI 回退**：当 Credential Manager 不可用时自动使用 DPAPI。
- **打包结构优化**：发布包内为扁平结构，直接包含可执行文件。
