/* SPDX-License-Identifier: GPL-3.0-only */
/* Windows x64: kernel32 only (no C runtime), VT sequences in the console. */
#include "omni.h"
typedef void *H;
#define DI __attribute__((dllimport))
DI H GetStdHandle(u32);
DI int GetConsoleMode(H, u32 *);
DI int SetConsoleMode(H, u32);
DI int SetConsoleCP(u32);
DI int SetConsoleOutputCP(u32);
DI int ReadFile(H, void *, u32, u32 *, void *);
DI int WriteFile(H, const void *, u32, u32 *, void *);
DI H CreateFileA(const char *, u32, u32, void *, u32, u32, H);
DI int CloseHandle(H);
DI int GetConsoleScreenBufferInfo(H, short *);
DI void ExitProcess(u32);
DI char *GetCommandLineA(void);

static H hi, ho; static u32 im, om; static int up;
static void init(void) {
  if (up) return;
  up = 1; hi = GetStdHandle((u32)-10); ho = GetStdHandle((u32)-11);
  GetConsoleMode(hi, &im); GetConsoleMode(ho, &om);
  SetConsoleCP(65001); SetConsoleOutputCP(65001);
  SetConsoleMode(ho, om | 5); /* processed output + VT processing */
}
void p_write(const void *b, int n) {
  const char *p = b; u32 w; init();
  while (n > 0) { if (!WriteFile(ho, p, (u32)n, &w, 0) || !w) return; p += w; n -= (int)w; }
}
int p_getc(void) { u8 c; u32 r = 0; init(); return (ReadFile(hi, &c, 1, &r, 0) && r == 1) ? c : -1; }
void p_raw(int on) { init(); SetConsoleMode(hi, on ? 0x200 : im); if (!on) SetConsoleMode(ho, om); }
void p_size(int *r, int *c) {
  short s[11]; init();
  if (!GetConsoleScreenBufferInfo(ho, s)) { *r = 24; *c = 80; return; }
  *c = s[7] - s[5] + 1; *r = s[8] - s[6] + 1;
}
void p_exit(int c) { ExitProcess((u32)c); for (;;) {} }
int p_read_file(const char *path, u8 *buf, int cap) {
  H f = CreateFileA(path, 0x80000000u, 1, 0, 3, 0x80, 0); u32 r; int n = 0;
  if (f == (H)-1) return -1;
  while (n < cap && ReadFile(f, buf + n, (u32)(cap - n), &r, 0) && r) n += (int)r;
  CloseHandle(f); return n;
}
int p_write_file(const char *path, const u8 *buf, int n) {
  H f = CreateFileA(path, 0x40000000u, 0, 0, 2, 0x80, 0); u32 w;
  if (f == (H)-1) return -1;
  while (n > 0) { if (!WriteFile(f, buf, (u32)n, &w, 0) || !w) { CloseHandle(f); return -1; } buf += w; n -= (int)w; }
  CloseHandle(f); return 0;
}
static char *av[16];
void mainCRTStartup(void) {
  char *c = GetCommandLineA(); int n = 0;
  while (n < 15 && *c) {
    while (*c == ' ') c++;
    if (!*c) break;
    if (*c == '"') { c++; av[n++] = c; while (*c && *c != '"') c++; }
    else { av[n++] = c; while (*c && *c != ' ') c++; }
    if (*c) *c++ = 0;
  }
  ExitProcess((u32)app_main(n, av));
}
