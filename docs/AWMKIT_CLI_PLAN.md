# AWMKit CLI 计划（MVP → 生产）

## 范围与目标
- 先完成 **awmkit**（Rust 独立 CLI）本地可用版本。
- CI/发布等后续再做，当前仅关注 CLI 本体设计与实现落地。

## 已确认的关键决策（AAAB）
1) **key import/export 文件格式：二进制**（安全、兼容现有实现）
2) **embed 输出：默认 `_wm` 后缀**（避免覆盖）
3) **decode：必须提供 key 校验 HMAC**（严格模式）
4) **支持批量输入 + 进度条**（一次到位）

## CLI 命令结构（Rust 风格）
```
awmkit
├── init
├── tag
│  ├── suggest <username>
│  ├── save <username> [--tag <TAG>]
│  ├── list [--json]
│  ├── remove <username>
│  └── clear
├── key
│  ├── show
│  ├── import <file>
│  ├── export <file>
│  └── rotate
├── encode --tag <TAG> [--version N] [--timestamp M]
├── decode --hex <HEX>
├── embed --tag <TAG> [--strength N] [--output <PATH>] <inputs...>
├── detect [--json] <inputs...>
└── status [--doctor]
```

全局标志：
- `--verbose` / `--quiet`
- `--audiowmark <PATH>`（覆盖默认查找）

## 数据与安全
- **密钥长度固定 32 bytes**
- keyring 保存的内容为 **hex 编码** 的 key（减少 API 限制风险）
- `key show` 输出 fingerprint（SHA256），不输出内容

## 功能细节（MVP）

### init
- 生成随机 32 bytes key
- 写入系统 keyring
- 若已存在则提示使用 `key rotate` 或 `key import`

### key
- show：输出配置状态、长度、fingerprint
- import/export：二进制文件
- rotate：生成新 key 覆盖旧 key

### encode/decode
- encode：输入 tag（1-7 自动补校验位 / 8 直接校验）
- decode：必须有 key

### embed
- 默认输出文件名：`<stem>_wm.<ext>`
- `--output` 仅支持单文件
- 支持通配符与批量（glob 扩展）
- 失败不会中断全部处理，但最终返回 non-zero

### detect
- 默认输出人类可读
- `--json` 输出机器可读结构（每个文件一条）

### status
- 默认显示 key 状态、audiowmark 是否可用
- `--doctor` 显示版本、二进制路径、可用性

## 模块与文件结构（当前实现路径）
```
src/bin/awmkit/
├── main.rs
├── commands/
│  ├── init.rs
│  ├── key.rs
│  ├── encode.rs
│  ├── decode.rs
│  ├── embed.rs
│  ├── detect.rs
│  └── status.rs
├── keystore.rs
├── output.rs
└── util.rs
```

## 风险与后续改进
- keyring 在 Linux 依赖 Secret Service，后续需要明确依赖提示
- audiowmark 路径查找可引入本地缓存/配置
- 可添加 `detect --no-verify` 选项作为低权限场景
- 可扩展 `status --doctor` 输出依赖库信息

## 下一步（本地验证）
- `cargo build --features full-cli --bin awmkit`
- 本地跑通：`init` → `encode` → `embed` → `detect`
### tag
- suggest：根据用户名生成推荐 Tag（不落盘）
- save/list/remove/clear：可选明文 JSON 存储（便于团队约定）
