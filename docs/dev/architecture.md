# Architecture

- `src/main.c`: editor core (static gap-less buffer, undo log, UTF-8, soft wrap, VT100 UI),
  format table, importers (XML text extraction) and exporters (OMD, DOCX).
- `src/zip.c`: ZIP reader with a built-in inflate, and a stored-only ZIP writer.
- `src/mem.c`: `memcpy/memmove/memset/memcmp` (required by the compiler in freestanding mode).
- `src/plat_posix.c`: Linux x86_64 and macOS raw syscalls, termios via ioctl, `_start` entry.
- `src/plat_win.c`: Windows x64 via `kernel32` only, VT input/output console modes.
- `src/omni.h`: the platform interface (`p_*`) and the ZIP API.

Rules: no libc, no heap, no dynamic allocation. Buffers: `B` (document, 4 MiB), `X`/`Z`
(16 MiB scratch each, `.bss`). Every code path must check capacities. SPDX header in every file.
The terminal UI uses only ANSI/VT sequences, which keeps the UI code identical on all platforms.
