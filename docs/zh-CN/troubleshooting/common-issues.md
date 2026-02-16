# 常见问题

[English](../../en-US/troubleshooting/common-issues.md)

## 1. `audiowmark` 找不到

症状：`awmkit status --doctor` 显示引擎不可用。  
排查顺序：

1. 确认 bundled 包存在（如 `bundled/audiowmark-macos-arm64.zip` / `bundled/audiowmark-windows-x86_64.zip`）。
2. 确认缓存可写（`~/.awmkit/bundled` 或 `%LOCALAPPDATA%\\awmkit\\bundled`）。
3. 临时指定回退路径：`--audiowmark <PATH>`。

## 2. WinUI 报 `EntryPointNotFound`

常见原因：`awmkit_native.dll` 与当前源码版本不匹配。  
处理：

1. 重新构建 Rust FFI：`cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc`
2. 覆盖复制：`awmkit.dll -> winui-app/AWMKit/awmkit_native.dll`
3. 重启应用后重试。

## 3. 首次运行无法嵌入/检测

原因：尚未配置密钥。  
处理：先在密钥页生成密钥，或 CLI 执行 `awmkit init`。

## 4. 检测结果 `invalid_hmac`

说明：检测到候选消息，但当前可用密钥无法通过 HMAC 校验。  
常见场景：

- 使用了错误槽位或错误密钥
- 密钥被替换后，旧样本仍在验证

建议结合 `detect --json` 查看：`decode_slot_hint`、`decode_slot_used`、`slot_status`。

## 5. 数据库状态异常（红色/不可用）

排查：

1. 检查数据库文件路径是否可访问。
2. 确认文件没有被其他进程长期锁定。
3. 先备份再执行恢复（必要时重建 `~/.awmkit/awmkit.db` / `%LOCALAPPDATA%\\awmkit\\awmkit.db`）。

## 6. macOS 提示“应用已损坏，无法打开”

说明：未签名/未公证的本地构建或预发布包，可能被 Gatekeeper 拦截并显示“已损坏”提示。  
处理：

1. 先确认安装包来源可信。
2. 清除隔离属性后再启动：
   - `xattr -dr com.apple.quarantine /path/to/AWMKit.app`
3. 若仍被拦截，可在“系统设置 -> 隐私与安全性”中允许该应用后重试。

## 7. 管道 I/O 兼容问题（`stdin/stdout`）

说明：当前默认优先使用 `audiowmark` 管道 I/O（`-` 作为输入/输出）。`detect` 对非 WAV 输入会走“FFmpeg 解码 -> WAV pipe -> audiowmark”的真流式链路；若本地环境不兼容，运行时会自动回退到文件 I/O。  
在新版中，FFI（Swift/ObjC/.NET）主链已对 Unix `SIGPIPE` 做了防护，管道写入失败会转为普通错误并进入回退，而不是导致宿主进程直接闪退。
回退日志已按文件粒度输出（包含操作类型与输入路径），便于定位具体样本。

手动强制关闭管道模式（便于排查）：

- macOS/Linux：`AWMKIT_DISABLE_PIPE_IO=1 awmkit ...`
- Windows PowerShell：`$env:AWMKIT_DISABLE_PIPE_IO=1; awmkit ...`

建议：

1. 先执行 `awmkit status --doctor` 确认当前 `audiowmark` 来源和版本。
2. 若出现异常且关闭 pipe 后恢复，优先升级/替换 `audiowmark` 二进制。
3. `AWMKIT_DISABLE_PIPE_IO=1` 只改变 I/O 通道策略，不改变水印业务语义。
