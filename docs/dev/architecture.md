# Architecture (0.2)

`omnidoc-core` owns all document logic and has one dependency (`miniz_oxide`, for inflate).

- `model.rs`: `Doc > Para(style, align) > Run(text, bold, italic, underline)`.
- `edit.rs`: `Editor` with cursor/selection, typing coalescing, snapshot undo/redo, formatting,
  find/replace (case-insensitive option, formatting-preserving), outline for the navigation pane.
- `cmd.rs`: one registry of commands (id, title, tab, group, shortcut). The ribbon, shortcuts and the
  command palette are all generated from it; `cmd::exec` runs editing commands, the UI handles file/view ones.
- `formats.rs`, `xml.rs`, `zipio.rs`: readers/writers, a small XML tokenizer, ZIP read/write.

`omnidoc-ui` renders one frame per event on a CPU Skia surface and presents it through `softbuffer`
(no GL/Vulkan setup, identical on Windows/Linux/macOS). Text layout uses Skia `textlayout` (system font
fallback, so Hangul/CJK work). Korean IME composition is displayed inline via winit `Ime` events.
