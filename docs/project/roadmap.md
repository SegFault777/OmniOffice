# Roadmap

## 202646a (done)
Rust + Skia OmniDoc: styles, navigation pane, command palette, find/replace, redo, RTF/ODT/DOCX/OMD.

## 202646b (this build)
- OmniCel (`.omc`, `.csv`), OmniShow (`.oms`, `.md` outline), OmniOffice launcher.
- DOC (Word 97-2003) import via a new OLE2 reader.
- 12 OmniDoc features, see `docs/user/features-202646b.md`.

## 202646c (0.3.1)
Google Material Symbols icons: ribbon, context menu, command palette, status bar, navigation pane, OmniShow/OmniCel toolbars, launcher.

## 202646d (0.3.2)
macOS CI fixed (`objc2 0.4.1` no longer compiles with newer Rust): winit 0.29 -> 0.30, rfd 0.14 -> 0.15, CI toolchain pinned to 1.99.1.
macOS and Windows targets are now cross-checked from Linux before release.

## 202646e (0.4.0)
- OmniShow: PowerPoint-style ribbon (File, Home, Insert, Design, Transitions, Slide Show, View), customizable quick access toolbar,
  right-click ribbon menu, command search, status bar with notes/views/zoom, slide sorter, layouts, themes, shapes,
  transitions (fade, push, wipe) with slideshow animation, undo/redo. OMS v2 (`<!-- directive -->` lines; v1 files still load).
- OmniDoc DOCX: styles.xml/numbering.xml aware (localized heading names, style inheritance, bullets/numbers), tables, text boxes,
  tracked changes, hidden text, AlternateContent fallbacks; writer adds page setup, East-Asian fonts, document properties.
- OmniDoc DOC: cp1252 text, multi-piece documents, tables, fields, simple (non-complex) files. HWP 5.x import (compressed and plain).
- OmniCel: undo/redo, range selection, cut/copy/paste (tab-separated, works with other spreadsheets), icon toolbar.
- OmniDoc: page guides and "Page n of m".

## Still not done
HWPX export, PDF import, real pagination (page breaks that split paragraphs), images, tables as a model element, spell check,
OmniCel charts and cell formatting, OmniShow images and PPTX export, accessibility bridge, Windows/macOS runtime testing.

## Next
- Compile and test on Windows and macOS (CI builds them; only Linux was run during development).
- OmniDoc: HWP v5 import (the OLE2 reader is ready), HWPX export, PDF import, pagination, images, tables, spell check.
- OmniCel/OmniShow: more formats (XLSX, PPTX, HCELL/SHOW) only after OmniDoc's format list is complete, as planned.
- OmniCel: copy/paste, undo, charts, cell formatting. OmniShow: shapes, images, themes, transitions, PPTX export.
- Accessibility (screen reader) bridge, mouse selection polish, rendering cache for large documents.
