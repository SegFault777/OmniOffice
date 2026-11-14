# Building

Requires a recent stable Rust (tested with 1.99). The first UI build downloads prebuilt Skia binaries.

    cargo test --workspace
    cargo build --release --workspace     # target/release/{omnioffice,omnidoc,omnicel,omnishow}

Linux: `libfontconfig1-dev libfreetype6-dev libgtk-3-dev libxkbcommon-dev` (+ a CJK font such as fonts-noto-cjk).
The launcher starts the other binaries from its own directory, so keep them together.
CI: `.github/workflows/build.yml` runs the tests, then builds and uploads all binaries on Linux, Windows and macOS.

Icons: `python3 tools/gen_icons.py` re-downloads the Material Symbols used by the UI and regenerates `crates/omni-shell/src/icons_data.rs`.
To add an icon, add it to `ICONS` in the script and map it in `omnidoc_core::cmd::icon` (or draw it with `omni_shell::icon`).

## CI toolchain and cross-checking
CI pins Rust (`.github/workflows/build.yml`, currently 1.99.1) so a new compiler cannot silently break old transitive dependencies.
Bump it deliberately: change the pin, run the checks below, fix what breaks.

Cross-check other platforms without a Mac/Windows machine (needs the target's std, e.g. conda-forge `rust-std-aarch64-apple-darwin`):

    cargo check --workspace --target aarch64-apple-darwin
    cargo check --workspace --target x86_64-pc-windows-msvc

Watch for "future version of Rust" warnings at the end of the output: they name the crate that will break next.
History: 202646c failed on macOS because winit 0.29 pulled `objc2 0.4.1`, which a newer compiler rejects (E0277 in
`ivar_forwarding_impls.rs`); upgrading to winit 0.30 / rfd 0.15 (objc2 0.5/0.6) removed it.
