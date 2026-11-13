/* SPDX-License-Identifier: GPL-3.0-only */
/* Linux (x86_64) and macOS (x86_64/arm64, experimental) via raw syscalls. */
#include "omni.h"
#if defined(__APPLE__)
#define U "_"
enum { S_READ = 3, S_WRITE = 4, S_OPEN = 5, S_CLOSE = 6, S_IOCTL = 54, S_EXIT = 1 };
#define F_WR 0x601
#define T_GET 0x40487413UL
#define T_SET 0x80487414UL
#define T_WIN 0x40087468UL
#define TIO_LEN 72
#define LFLAG_OFF 24
#else
#define U ""
enum { S_READ = 0, S_WRITE = 1, S_OPEN = 2, S_CLOSE = 3, S_IOCTL = 16, S_EXIT = 231 };
#define F_WR 0x241
#define T_GET 0x5401UL
#define T_SET 0x5402UL
#define T_WIN 0x5413UL
#define TIO_LEN 36
#define LFLAG_OFF 12
#endif

static long sc(long n, long a, long b, long c) {
#if defined(__linux__) && defined(__x86_64__)
  long r;
  __asm__ volatile("syscall" : "=a"(r) : "a"(n), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
  return r;
#elif defined(__APPLE__) && defined(__x86_64__)
  long r = 0x2000000 + n;
  __asm__ volatile("syscall\n jnc 1f\n neg %%rax\n1:" : "+a"(r) : "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory", "cc");
  return r;
#elif defined(__APPLE__) && defined(__aarch64__)
  register long x0 __asm__("x0") = a, x1 __asm__("x1") = b, x2 __asm__("x2") = c, x16 __asm__("x16") = n;
  __asm__ volatile("svc #0x80\n b.cc 1f\n neg x0, x0\n1:" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x16) : "memory", "cc");
  return x0;
#else
#error "Unsupported platform"
#endif
}

void p_write(const void *b, int n) {
  const char *p = b;
  while (n > 0) { long w = sc(S_WRITE, 1, (long)p, n); if (w <= 0) return; p += w; n -= (int)w; }
}
int p_getc(void) { u8 c; return sc(S_READ, 0, (long)&c, 1) == 1 ? c : -1; }
void p_exit(int c) { sc(S_EXIT, c, 0, 0); for (;;) {} }
int p_read_file(const char *path, u8 *buf, int cap) {
  long fd = sc(S_OPEN, (long)path, 0, 0), n = 0, r;
  if (fd < 0) return -1;
  while (n < cap && (r = sc(S_READ, fd, (long)buf + n, cap - n)) > 0) n += r;
  sc(S_CLOSE, fd, 0, 0);
  return (int)n;
}
int p_write_file(const char *path, const u8 *buf, int n) {
  long fd = sc(S_OPEN, (long)path, F_WR, 0644);
  if (fd < 0) return -1;
  while (n > 0) { long w = sc(S_WRITE, fd, (long)buf, n); if (w <= 0) { sc(S_CLOSE, fd, 0, 0); return -1; } buf += w; n -= (int)w; }
  sc(S_CLOSE, fd, 0, 0);
  return 0;
}
static u8 saved[80]; static int have;
void p_raw(int on) {
  if (on) {
    u8 t[80];
    if (sc(S_IOCTL, 0, T_GET, (long)saved) < 0) return;
    have = 1; memcpy(t, saved, sizeof t);
#if defined(__APPLE__)
    memset(t, 0, 16); memset(t + 24, 0, 8);   /* clear iflag, oflag, lflag; keep cflag */
    t[32 + 16] = 1; t[32 + 17] = 0;           /* VMIN=1, VTIME=0 */
#else
    memset(t, 0, 8); memset(t + 12, 0, 4);    /* clear iflag, oflag, lflag; keep cflag */
    t[17 + 6] = 1; t[17 + 5] = 0;
#endif
    sc(S_IOCTL, 0, T_SET, (long)t);
  } else if (have) sc(S_IOCTL, 0, T_SET, (long)saved);
}
void p_size(int *r, int *c) {
  unsigned short w[4] = {0, 0, 0, 0};
  if (sc(S_IOCTL, 1, T_WIN, (long)w) < 0 || !w[0] || !w[1]) { *r = 24; *c = 80; return; }
  *r = w[0]; *c = w[1];
}
__attribute__((used)) void start_c(long *sp) { p_exit(app_main((int)sp[0], (char **)(sp + 1))); }
#if defined(__x86_64__)
__asm__(".globl _start\n_start:\n xor %ebp,%ebp\n mov %rsp,%rdi\n and $-16,%rsp\n call " U "start_c\n");
#else
__asm__(".globl _start\n.p2align 2\n_start:\n mov x0, sp\n and sp, x0, #-16\n bl " U "start_c\n");
#endif
