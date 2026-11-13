/* SPDX-License-Identifier: GPL-3.0-only */
/* OmniDoc: freestanding build, no libc, no malloc. Platform layer interface. */
#ifndef OMNI_H
#define OMNI_H
typedef unsigned char u8;
typedef unsigned int u32;
typedef __SIZE_TYPE__ usz;
void *memcpy(void *, const void *, usz);
void *memmove(void *, const void *, usz);
void *memset(void *, int, usz);
int memcmp(const void *, const void *, usz);
/* platform (plat_posix.c / plat_win.c) */
void p_write(const void *b, int n);
int p_read_file(const char *path, u8 *buf, int cap); /* bytes or -1 */
int p_write_file(const char *path, const u8 *buf, int n); /* 0 or -1 */
int p_getc(void);                                    /* blocking, -1 on EOF */
void p_raw(int on);                                  /* raw terminal mode */
void p_size(int *rows, int *cols);
void p_exit(int code);
int app_main(int argc, char **argv);
/* zip.c */
int zget(const u8 *z, int n, const char *name, u8 *out, int cap); /* -2: missing */
void zw_begin(u8 *buf, int cap);
int zw_add(const char *name, const u8 *d, int n); /* stored entry */
int zw_end(void);                                  /* total length or -1 */
#endif
