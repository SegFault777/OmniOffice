# OmniOffice

**OmniOffice 202646d** (crate version 0.3.2) is a small cross-platform office suite written in Rust. OS-native windows, clipboard,
file dialogs and IME input; everything is drawn with Skia. License: GPL-3.0-only.

| Application | Purpose | Native format | Binary |
|---|---|---|---|
| OmniOffice | launcher: choose which app to open | - | `omnioffice` |
| OmniDoc | word processor / markup editor (Word, Hancom Hangul) | `.omd` | `omnidoc` |
| OmniCel | spreadsheet (Excel, Hancom Cell) | `.omc` | `omnicel` |
| OmniShow | presentations (PowerPoint, Hancom Show) | `.oms` | `omnishow` |

Versions are named `<year><ISO week><letter>` and each maps to a crate version (202646a = 0.2.0, 202646b = 0.3.0, 202646c = 0.3.1, 202646d = 0.3.2).
See `docs/project/versioning.md`. Icons: Google Material Symbols (Apache-2.0), see `THIRD_PARTY.md`.

## Workspace

- `omni-common`: ZIP, XML, OLE2/CFB reader, config/recent files, build id.
- `omnidoc-core`, `omnicel-core`, `omnishow-core`: application logic, unit-tested, no GUI code.
- `omni-shell`: winit + softbuffer + Skia window shell used by OmniCel, OmniShow and the launcher.
- `omnidoc-ui`, `omnicel-ui`, `omnishow-ui`, `omnioffice-ui`: the four applications.
- `legacy/`: the freestanding C terminal editor from 1.0-alpha-1.

## Build

    cargo test --workspace
    cargo run --release -p omnioffice-ui          # launcher (build all with: cargo build --release --workspace)
    cargo run --release -p omnidoc-ui -- file.docx

Linux needs `libfontconfig1-dev libfreetype6-dev libgtk-3-dev libxkbcommon-dev`. See `docs/dev/building.md`.
Docs: `docs/user/`, `docs/dev/`, `docs/project/`.
