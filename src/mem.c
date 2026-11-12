/* SPDX-License-Identifier: GPL-3.0-only */
#include "omni.h"
/* Compile with -fno-builtin -fno-tree-loop-distribute-patterns. */
void *memcpy(void *d, const void *s, usz n) { u8 *a = d; const u8 *b = s; while (n--) *a++ = *b++; return d; }
void *memmove(void *d, const void *s, usz n) {
  u8 *a = d; const u8 *b = s;
  if (a < b) { while (n--) *a++ = *b++; } else { a += n; b += n; while (n--) *--a = *--b; }
  return d;
}
void *memset(void *d, int c, usz n) { u8 *a = d; while (n--) *a++ = (u8)c; return d; }
int memcmp(const void *x, const void *y, usz n) {
  const u8 *a = x, *b = y;
  for (; n; n--, a++, b++) if (*a != *b) return *a - *b;
  return 0;
}
