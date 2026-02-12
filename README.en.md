# AWMKit

[中文](./README.md)

AWMKit is a cross-platform audio watermark toolkit built around a Rust core, with CLI, native macOS app, WinUI app, and Swift bindings.

## Highlights

- 128-bit watermark message protocol with HMAC verification (`v2` slot semantics)
- End-to-end CLI workflow: keys, tag mapping, embed, detect, evidence management
- Native macOS / Windows apps sharing the same Rust FFI core
- SQLite persistence for mappings, evidence, and app settings
- Current output limitation: embed output is `WAV` only (`FLAC` output is temporarily disabled)
- Safety policy: inputs with existing watermarks are auto-skipped; force-embed has been removed

## Platform Matrix

| Component | macOS (arm64) | Windows (x64) |
| --- | --- | --- |
| `awmkit` CLI | Supported | Supported |
| macOS App | Supported | N/A |
| WinUI App | N/A | Supported |
| Rust FFI (`awmkit`/`awmkit_native`) | Supported | Supported |

## Quick Start

- Documentation index (English): [`docs/en-US/INDEX.md`](./docs/en-US/INDEX.md)
- CLI usage: [`docs/en-US/cli/usage.md`](./docs/en-US/cli/usage.md)
- Build matrix: [`docs/en-US/build/build-matrix.md`](./docs/en-US/build/build-matrix.md)

## Docs Navigation

- CLI: [`docs/en-US/cli/usage.md`](./docs/en-US/cli/usage.md)
- GUI (macOS): [`docs/en-US/gui/macos-app.md`](./docs/en-US/gui/macos-app.md)
- GUI (WinUI): [`docs/en-US/gui/winui-app.md`](./docs/en-US/gui/winui-app.md)
- Build & release: [`docs/en-US/build/build-matrix.md`](./docs/en-US/build/build-matrix.md), [`docs/en-US/build/ci-artifacts.md`](./docs/en-US/build/ci-artifacts.md)
- Protocol & data: [`docs/en-US/spec/message-and-key-slot.md`](./docs/en-US/spec/message-and-key-slot.md), [`docs/en-US/spec/database-schema.md`](./docs/en-US/spec/database-schema.md)
- Troubleshooting: [`docs/en-US/troubleshooting/common-issues.md`](./docs/en-US/troubleshooting/common-issues.md)

## Upstream Engine (audiowmark)

- AWMKit relies on [`audiowmark`](https://github.com/swesterfeld/audiowmark) for audio embed/detect execution.
- Runtime integration uses bundled executables (pack + extract), not static/dynamic library linking.
- Binary source and CI integration details are documented at [`docs/en-US/build/ci-artifacts.md`](./docs/en-US/build/ci-artifacts.md).
- License note: this repository is GPLv3+, and distribution follows upstream `audiowmark` open-source license requirements.
- Attribution: thanks to the `audiowmark` author and contributors for the core watermarking engine.

## License

This project is licensed under [GNU GPLv3 or later](./LICENSE).
