# AWMKit

[中文](./README.md)

AWMKit is a cross-platform audio watermark toolkit built around a Rust core, with CLI, native macOS app, WinUI app, and Swift bindings.

## Highlights

- 128-bit watermark message protocol with HMAC verification (`v2` slot semantics)
- End-to-end CLI workflow: keys, tag mapping, embed, detect, evidence management
- Native macOS / Windows apps sharing the same Rust FFI core
- SQLite persistence for mappings, evidence, and app settings
- Input support: `wav` / `flac` / `mp3` / `ogg` / `opus` / `m4a` / `alac` / `mp4` / `mkv` / `mka` / `ts` / `m2ts` / `m2t`
- Current output limitation: embed output is `WAV` only (non-`wav` output paths fail fast)
- `audiowmark` runtime I/O: `stdin/stdout` pipe is enabled by default; for non-WAV detect input, AWMKit uses true streaming (`FFmpeg decode -> WAV pipe -> audiowmark`); set `AWMKIT_DISABLE_PIPE_IO=1` to force file I/O
- Default multichannel routing (`smart`): `FC` uses mono embed (dual-mono wrapper), `LFE` is skipped by default, and other channels follow pair routing; unknown/custom layouts fall back to sequential pairing with a final mono step for odd channel counts (with warnings)
- Multichannel route execution: RouteStep processing uses internal Rayon parallelism with deterministic merge by step index (public parameters and result schema are unchanged)
- ADM/BWF master embed: `embed` auto-detects ADM/BWF metadata in `RIFF/RF64/BW64` and applies metadata-preserving data replacement; `detect` now supports ADM/BWF inputs through the unified detect pipeline
- Safety policy: inputs with existing watermarks are auto-skipped, with a batch summary warning at the end

## Platform Matrix

| Component | macOS (arm64) | Windows (x64) |
| --- | --- | --- |
| `awmkit` CLI | Supported | Supported |
| macOS App | Supported | N/A |
| WinUI App | N/A | Supported |
| Rust FFI (`awmkit`/`awmkit_native`) | Supported | Supported |

## Quick Start

- Documentation index (English): [`docs/en-US/INDEX.md`](./docs/en-US/INDEX.md)
- UI first-run guide: [`docs/en-US/gui/first-run.md`](./docs/en-US/gui/first-run.md)
- CLI usage: [`docs/en-US/cli/usage.md`](./docs/en-US/cli/usage.md)
- Build matrix: [`docs/en-US/build/build-matrix.md`](./docs/en-US/build/build-matrix.md)

## Local Release (Current Primary Flow)

- macOS (App + CLI): `./scripts/release/local-release-macos.sh`
- Windows (Inno installer, run on `win-pc`): `powershell -File scripts/release/local-release-win.ps1`
- One-shot dual-platform release (from mac, invokes `win-pc`): `./scripts/release/local-release-all.sh`
- Note: local release is the primary path for now; CI workflows are reference/backup.

## Docs Navigation

- CLI: [`docs/en-US/cli/usage.md`](./docs/en-US/cli/usage.md)
- UI first-run: [`docs/en-US/gui/first-run.md`](./docs/en-US/gui/first-run.md)
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
