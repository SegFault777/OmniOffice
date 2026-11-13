# Building

Requirements: a recent stable Rust toolchain (the Skia and winit dependency tree needs a current
cargo; Cargo 1.75 cannot resolve it). Core only: any Rust that supports edition 2021.

    cargo test -p omnidoc-core          # fast, no system libraries
    cargo build --release -p omnidoc-ui # downloads prebuilt Skia binaries on first build

Linux: `libfontconfig1-dev libfreetype6-dev libgtk-3-dev libxkbcommon-dev`. Windows/macOS: nothing extra.
CI: `.github/workflows/build.yml` runs core tests, then builds the UI on all three systems.
