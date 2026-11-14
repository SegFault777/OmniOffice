# Roadmap

## 202646a (done)
Rust + Skia OmniDoc: styles, navigation pane, command palette, find/replace, redo, RTF/ODT/DOCX/OMD.

## 202646b (this build)
- OmniCel (`.omc`, `.csv`), OmniShow (`.oms`, `.md` outline), OmniOffice launcher.
- DOC (Word 97-2003) import via a new OLE2 reader.
- 12 OmniDoc features, see `docs/user/features-202646b.md`.

## 202646c (0.3.1)
Google Material Symbols icons: ribbon, context menu, command palette, status bar, navigation pane, OmniShow/OmniCel toolbars, launcher.

## Next
- Compile and test on Windows and macOS (CI builds them; only Linux was run during development).
- OmniDoc: HWP v5 import (the OLE2 reader is ready), HWPX export, PDF import, pagination, images, tables, spell check.
- OmniCel/OmniShow: more formats (XLSX, PPTX, HCELL/SHOW) only after OmniDoc's format list is complete, as planned.
- OmniCel: copy/paste, undo, charts, cell formatting. OmniShow: shapes, images, themes, transitions, PPTX export.
- Accessibility (screen reader) bridge, mouse selection polish, rendering cache for large documents.
