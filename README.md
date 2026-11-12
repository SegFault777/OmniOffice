# OmniOffice / OmniDoc

**OmniDoc 1.0-alpha-1** is the word processor and markup editor of the OmniOffice suite.
It is written in freestanding C: **no libc, no malloc, no third-party dependencies**.
All memory is static; the platform layer talks directly to the OS (raw syscalls on Linux/macOS,
`kernel32` on Windows).

Native document format: **OMD** (`.omd`, a stored ZIP, see `docs/dev/omd-format.md`).

License: GPL-3.0-only (see `LICENSE`).

## Build

    make linux      # x86_64 Linux, static binary
    make windows    # x86_64, MinGW-w64 gcc, links kernel32 only
    make macos      # experimental (x86_64/arm64), untested

    sh tests/selftest.sh ./omnidoc

## Use

    omnidoc [file]                       # interactive terminal editor
    omnidoc --convert input output       # headless conversion

Keys: `^S` save, `^Q` quit, `^Z` undo, `^F` find, arrows/Home/End/PgUp/PgDn.

## Status (alpha-1)

Read+write: OMD, TXT, MD, HTML, XHTML, XML, LaTeX, RST, Org, Typst, MediaWiki, AsciiDoc, SGML, GML
(edited as source text) and DOCX (plain text only). Read-only text extraction: DOCM, ODT, SXW, HWPX, HML, ABW.
Planned: RTF, DOC, HWP, PDF, PAGES, WPD, WPS, LWP, WRI, CWK, UOF, GUL, JUNG.
See `docs/user/supported-formats.md` and `docs/project/roadmap.md`.

Documentation: `docs/user/`, `docs/dev/`, `docs/project/`.
