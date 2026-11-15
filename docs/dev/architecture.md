# Architecture (202646b)

Cores contain all logic and are unit-tested; UI crates only draw and forward input.

- `omni-common`: `zipio` (read stored/deflate, write stored), `xml` tokenizer, `cfb` (OLE2 reader), `config`.
- `omnidoc-core`: `model`, `edit` (Editor: selection, undo/redo, formatting, painter, case, sort, autocorrect,
  completion, regex find/replace, stats, outline), `cmd` (command registry + context menu), `formats`.
- `omnicel-core`: sparse sheet, recursive-descent formula evaluator with cycle detection, CSV, OMC.
- `omnishow-core`: deck/slide model, markdown (de)serialization, outline import, OMS.
- `omni-shell`: `App` trait + event loop (winit, softbuffer, CPU Skia surface) and text helpers with CJK font fallback.
- `omnidoc-ui` has its own event loop (IME pre-edit, autosave timer); the other three use `omni-shell`.

202646e: `omnidoc-core` gained `docx` (styles-aware reader/writer) and `legacy` (DOC via piece table, HWP 5 via record stream); `omni-common::cfb` is shared.
`omni-shell::ribbon` is a reusable ribbon widget (tabs, groups, icon buttons) used by OmniShow; `App` has `mouse_up` and `animating` hooks.
OmniShow keeps undo history as deck snapshots; slideshow transitions are drawn from the previous and current slide.
