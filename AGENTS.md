# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Rust core: `tag.rs`, `message.rs`, `audio.rs`, `multichannel.rs`, `ffi.rs`, `error.rs`, `charset.rs`, and `lib.rs`.
- `src/bin/awmkit/` contains the current Rust CLI entry and command modules.
- `include/awmkit.h` is the C header for FFI consumers.
- `bindings/swift/` is the Swift Package; tests live in `bindings/swift/Tests/`.
- `vendor/` includes the `audiowmark` binary used for audio embed/detect (see README).

## Build, Test, and Development Commands
```
cargo build --release
cargo build --features cli --release
cargo build --features ffi --release
cargo build --features multichannel --release
cargo test
cargo test --features multichannel
cargo clippy --all-features
```
```
cd bindings/swift && swift build
cd bindings/swift && swift test
```
`audiowmark` must be available on PATH, or use the packaged binary under `vendor/`.

## Coding Style & Naming Conventions
- Rust 2021 edition; run `cargo fmt` and `cargo clippy --all-features` before submitting.
- Clippy denies `unwrap`, `expect`, `panic`, and `todo`; handle errors explicitly and use `?`.
- Naming: modules/functions `snake_case`, types `CamelCase`, constants `SCREAMING_SNAKE_CASE`.
- Keep FFI changes in sync with `include/awmkit.h`.

## Testing Guidelines
- Rust unit tests live next to modules under `#[cfg(test)]`; run with `cargo test` and enable feature-specific tests as needed.
- Swift tests live in `bindings/swift/Tests`; run `swift test`.
- Prefer deterministic fixtures; gate audio-IO tests behind features if they require `audiowmark`.

## Commit & Pull Request Guidelines
- Follow Conventional Commits seen in history: `feat:`, `fix:`, `docs:`, `perf:`, `test:`, `api:`; optional scope (e.g., `feat(cli): add key export`).
- PRs should include a short summary, test commands + results, and any behavior changes or new flags. Link related issues when applicable.
- Communication: prefer Chinese for issues, PR descriptions, and review discussions unless English is specifically requested.

## Security & Configuration Tips
- Keys are caller-managed; Swift CLI uses macOS Keychain.
- Do not commit secrets or production keys; use test keys in examples.
