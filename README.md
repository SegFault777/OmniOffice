# OmniOffice

**OmniOffice 202646b** is a small cross-platform office suite written in Rust. OS-native windows, clipboard,
file dialogs and IME input; everything is drawn with Skia. License: GPL-3.0-only.

| Application | Purpose | Native format | Binary |
|---|---|---|---|
| OmniOffice | launcher: choose which app to open | - | `omnioffice` |
| OmniDoc | word processor / markup editor (Word, Hancom Hangul) | `.omd` | `omnidoc` |
| OmniCel | spreadsheet (Excel, Hancom Cell) | `.omc` | `omnicel` |
| OmniShow | presentations (PowerPoint, Hancom Show) | `.oms` | `omnishow` |

Versions are named `<year><ISO week><letter>`: `202646a` (= the former 0.2.0) was the first build of week 46 of 2026,
`202646b` is the second. Cargo packages use the placeholder `0.3.0`; the build id lives in `omni-common`.
See `docs/project/versioning.md`.

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
