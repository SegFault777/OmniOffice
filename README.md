# OmniOffice / OmniDoc

**OmniDoc 0.2.0** is the word processor and markup editor of the OmniOffice suite.
Written in Rust. OS-native windows, clipboard, file dialogs and IME input; all drawing is done with Skia.
Native document format: **OMD** (`.omd`, see `docs/dev/omd-format.md`). License: GPL-3.0-only.

## Workspace

- `crates/omnidoc-core`: document model, editing engine (undo/redo, find/replace, styles, outline),
  command registry (palette/ribbon/shortcuts) and format I/O. No GUI code; fully unit-tested.
- `crates/omnidoc-ui`: winit window + Skia renderer + native dialogs (`rfd`) + clipboard (`arboard`).
- `legacy/`: the freestanding C terminal editor from 1.0-alpha-1 (no libc, no malloc); kept for reference.

## Build

    cargo test -p omnidoc-core
    cargo run --release -p omnidoc-ui -- [file]

Linux needs: libfontconfig, libfreetype, GTK3 (for native dialogs). See `docs/dev/building.md`.

## Status

See `docs/user/supported-formats.md`, `docs/project/roadmap.md` and `docs/dev/ui-design.md`.
