# Building

Requirements: GCC or Clang, `make`. Nothing else (no libc headers are used).

- Linux: `make linux` (x86_64 only).
- Windows: MinGW-w64 gcc, `make windows` (or see `.github/workflows/build.yml`).
- macOS: `make macos`. Experimental: Apple does not support binaries without libSystem, and
  this target has not been verified on real hardware.

Flags that matter: `-ffreestanding -fno-builtin -fno-tree-loop-distribute-patterns -nostdlib`.
Run `sh tests/selftest.sh ./omnidoc` after every change.
