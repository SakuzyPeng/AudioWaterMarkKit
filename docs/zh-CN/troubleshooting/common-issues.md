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
