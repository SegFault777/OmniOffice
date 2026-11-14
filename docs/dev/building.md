# Building

Requires a recent stable Rust (tested with 1.99). The first UI build downloads prebuilt Skia binaries.

    cargo test --workspace
    cargo build --release --workspace     # target/release/{omnioffice,omnidoc,omnicel,omnishow}

Linux: `libfontconfig1-dev libfreetype6-dev libgtk-3-dev libxkbcommon-dev` (+ a CJK font such as fonts-noto-cjk).
The launcher starts the other binaries from its own directory, so keep them together.
CI: `.github/workflows/build.yml` runs the tests, then builds and uploads all binaries on Linux, Windows and macOS.
