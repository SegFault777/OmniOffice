#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Round-trip self-test. Usage: sh tests/selftest.sh ./omnidoc
set -e
BIN="${1:-./omnidoc}"; T="${TMPDIR:-/tmp}/omni_t$$"; mkdir -p "$T"
printf 'Hello & <world>\n안녕하세요 OmniDoc\n\nlast line\n' > "$T/a.txt"
"$BIN" --convert "$T/a.txt" "$T/b.omd"
"$BIN" --convert "$T/b.omd" "$T/c.txt";  cmp "$T/a.txt" "$T/c.txt"
"$BIN" --convert "$T/a.txt" "$T/d.docx"
"$BIN" --convert "$T/d.docx" "$T/e.txt"; cmp "$T/a.txt" "$T/e.txt"
"$BIN" --convert "$T/a.txt" "$T/f.md";   cmp "$T/a.txt" "$T/f.md"
echo "selftest OK"; rm -rf "$T"
