# Roadmap

## 0.2.0 (current)
Done and unit-tested in core: redo, find/replace, styles (title, headings, quote, code, alignment), outline,
command palette registry, RTF read/write, ODT read/write, DOCX styles, OMD v2, clipboard commands.
Written but **not yet compiled or run** (needs a current Rust toolchain): the Skia UI crate.

## Not done yet
- HWP v5 import (OLE2 compound file + zlib records), DOC import (OLE2 Word binary), HWPX export.
- PDF text import, PAGES/WPD/WPS/LWP/WRI/CWK/UOF, GUL/JUNG (no public specification found).
- Pagination, images, tables, spell check, a screen-reader (accessibility) bridge, mouse double/triple click.

## 0.3
Compile/test the UI on all three systems, pagination, images, tables, HWP/DOC import.
