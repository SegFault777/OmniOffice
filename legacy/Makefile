# SPDX-License-Identifier: GPL-3.0-only
SRC = src/main.c src/zip.c src/mem.c
CFLAGS = -std=gnu11 -Os -Wall -Wextra -ffreestanding -fno-builtin -fno-stack-protector \
         -fno-asynchronous-unwind-tables -fno-tree-loop-distribute-patterns -fno-pic -fno-pie
.PHONY: linux windows macos clean
linux:
	$(CC) $(CFLAGS) -nostdlib -static -no-pie -o omnidoc $(SRC) src/plat_posix.c
windows:
	gcc $(CFLAGS) -nostdlib -mconsole -Wl,-e,mainCRTStartup -o omnidoc.exe $(SRC) src/plat_win.c -lkernel32
macos:
	clang $(CFLAGS) -nostdlib -static -Wl,-e,_start -o omnidoc $(SRC) src/plat_posix.c
clean:
	rm -f omnidoc omnidoc.exe
