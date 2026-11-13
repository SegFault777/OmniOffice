/* SPDX-License-Identifier: GPL-3.0-only */
/* Minimal ZIP reader (stored + deflate, own inflate) and stored-only writer. */
#include "omni.h"

typedef struct { const u8 *in; int len, cnt; u32 bb; int bc; u8 *out; int cap, n, err; } S;
typedef struct { short count[16], symbol[288]; } Hf;

static int bits(S *s, int n) {
  u32 v = s->bb;
  while (s->bc < n) {
    if (s->cnt >= s->len) { s->err = 1; return 0; }
    v |= (u32)s->in[s->cnt++] << s->bc; s->bc += 8;
  }
  s->bb = v >> n; s->bc -= n;
  return (int)(v & ((1u << n) - 1));
}
static int decode(S *s, const Hf *h) {
  int code = 0, first = 0, index = 0;
  for (int len = 1; len <= 15; len++) {
    code |= bits(s, 1);
    int c = h->count[len];
    if (code - c < first) return h->symbol[index + (code - first)];
    index += c; first += c; first <<= 1; code <<= 1;
    if (s->err) return -1;
  }
  return -1;
}
static void construct(Hf *h, const short *len, int n) {
  short offs[16];
  for (int i = 0; i < 16; i++) h->count[i] = 0;
  for (int i = 0; i < n; i++) h->count[len[i]]++;
  offs[1] = 0;
  for (int l = 1; l < 15; l++) offs[l + 1] = offs[l] + h->count[l];
  for (int i = 0; i < n; i++) if (len[i]) h->symbol[offs[len[i]]++] = (short)i;
}
static const short LB[] = {3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258};
static const short LE[] = {0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0};
static const short DB[] = {1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577};
static const short DE[] = {0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13};

static int codes(S *s, const Hf *lc, const Hf *dc) {
  int sym;
  do {
    sym = decode(s, lc);
    if (sym < 0) return -1;
    if (sym < 256) { if (s->n >= s->cap) return -1; s->out[s->n++] = (u8)sym; }
    else if (sym > 256) {
      sym -= 257;
      if (sym >= 29) return -1;
      int len = LB[sym] + bits(s, LE[sym]);
      sym = decode(s, dc);
      if (sym < 0 || sym >= 30) return -1;
      int dist = DB[sym] + bits(s, DE[sym]);
      if (s->err || dist > s->n || s->n + len > s->cap) return -1;
      for (; len; len--) { s->out[s->n] = s->out[s->n - dist]; s->n++; }
    }
  } while (sym != 256);
  return 0;
}
static int inflate(const u8 *in, int len, u8 *out, int cap) {
  S s = {in, len, 0, 0, 0, out, cap, 0, 0};
  static Hf fl, fd; static int fixed;
  int last;
  do {
    last = bits(&s, 1);
    int type = bits(&s, 2);
    if (s.err) return -1;
    if (type == 0) {
      s.bb = 0; s.bc = 0;
      if (s.cnt + 4 > len) return -1;
      int n = in[s.cnt] | in[s.cnt + 1] << 8; s.cnt += 4;
      if (s.cnt + n > len || s.n + n > cap) return -1;
      memcpy(out + s.n, in + s.cnt, n); s.cnt += n; s.n += n;
    } else if (type == 1) {
      if (!fixed) {
        short l[288]; int i = 0;
        for (; i < 144; i++) l[i] = 8;
        for (; i < 256; i++) l[i] = 9;
        for (; i < 280; i++) l[i] = 7;
        for (; i < 288; i++) l[i] = 8;
        construct(&fl, l, 288);
        for (i = 0; i < 30; i++) l[i] = 5;
        construct(&fd, l, 30); fixed = 1;
      }
      if (codes(&s, &fl, &fd)) return -1;
    } else if (type == 2) {
      static const u8 ord[19] = {16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15};
      short l[320]; Hf lc, dc, cc; int i;
      int nl = bits(&s, 5) + 257, nd = bits(&s, 5) + 1, nc = bits(&s, 4) + 4;
      if (nl > 286 || nd > 30) return -1;
      for (i = 0; i < 19; i++) l[i] = 0;
      for (i = 0; i < nc; i++) l[ord[i]] = (short)bits(&s, 3);
      construct(&cc, l, 19);
      for (i = 0; i < nl + nd;) {
        int sym = decode(&s, &cc), rep, v = 0;
        if (sym < 0) return -1;
        if (sym < 16) { l[i++] = (short)sym; continue; }
        if (sym == 16) { if (!i) return -1; v = l[i - 1]; rep = 3 + bits(&s, 2); }
        else if (sym == 17) rep = 3 + bits(&s, 3);
        else rep = 11 + bits(&s, 7);
        if (i + rep > nl + nd) return -1;
        while (rep--) l[i++] = (short)v;
      }
      construct(&lc, l, nl); construct(&dc, l + nl, nd);
      if (codes(&s, &lc, &dc)) return -1;
    } else return -1;
  } while (!last);
  return s.n;
}
static u32 r16(const u8 *p) { return p[0] | p[1] << 8; }
static u32 r32(const u8 *p) { return r16(p) | r16(p + 2) << 16; }

int zget(const u8 *z, int n, const char *nm, u8 *out, int cap) {
  int e = n - 22, nl = 0;
  while (nm[nl]) nl++;
  while (e >= 0 && r32(z + e) != 0x06054b50u) e--;
  if (e < 0) return -1;
  int cnt = (int)r16(z + e + 10); u32 p = r32(z + e + 16);
  for (int i = 0; i < cnt; i++) {
    if (p + 46 > (u32)n || r32(z + p) != 0x02014b50u) return -1;
    int m = (int)r16(z + p + 10), fl = (int)r16(z + p + 28), el = (int)r16(z + p + 30), cl = (int)r16(z + p + 32);
    u32 cs = r32(z + p + 20), lh = r32(z + p + 42);
    if (fl == nl && !memcmp(z + p + 46, nm, (usz)nl)) {
      if (lh + 30 > (u32)n) return -1;
      u32 d = lh + 30 + r16(z + lh + 26) + r16(z + lh + 28);
      if (d + cs > (u32)n) return -1;
      if (m == 0) { if ((int)cs > cap) return -1; memcpy(out, z + d, cs); return (int)cs; }
      if (m == 8) return inflate(z + d, (int)cs, out, cap);
      return -1;
    }
    p += 46 + (u32)fl + (u32)el + (u32)cl;
  }
  return -2;
}

/* ---- writer ---- */
typedef struct { char nm[40]; int nlen; u32 crc, sz, off; } Ze;
static Ze zes[8]; static int nz, zl, zc; static u8 *zb;
static u32 crc32(const u8 *p, int n) {
  u32 c = ~0u;
  while (n--) { c ^= *p++; for (int k = 0; k < 8; k++) c = (c & 1) ? (c >> 1) ^ 0xEDB88320u : c >> 1; }
  return ~c;
}
static void p16(int v) { if (zl + 2 <= zc) { zb[zl] = (u8)v; zb[zl + 1] = (u8)(v >> 8); } zl += 2; }
static void p32(u32 v) { p16((int)(v & 0xFFFF)); p16((int)(v >> 16)); }
void zw_begin(u8 *buf, int cap) { zb = buf; zc = cap; zl = 0; nz = 0; }
int zw_add(const char *nm, const u8 *d, int n) {
  if (nz >= 8 || zl + n + 128 > zc) return -1;
  Ze *e = &zes[nz++]; int k = 0;
  while (nm[k] && k < 39) { e->nm[k] = nm[k]; k++; }
  e->nlen = k; e->crc = crc32(d, n); e->sz = (u32)n; e->off = (u32)zl;
  p32(0x04034b50u); p16(20); p16(0); p16(0); p16(0); p16(0x21);
  p32(e->crc); p32(e->sz); p32(e->sz); p16(k); p16(0);
  memcpy(zb + zl, nm, (usz)k); zl += k; memcpy(zb + zl, d, (usz)n); zl += n;
  return 0;
}
int zw_end(void) {
  int s = zl;
  for (int i = 0; i < nz; i++) {
    Ze *e = &zes[i];
    p32(0x02014b50u); p16(20); p16(20); p16(0); p16(0); p16(0); p16(0x21);
    p32(e->crc); p32(e->sz); p32(e->sz); p16(e->nlen); p16(0); p16(0); p16(0); p16(0); p32(0); p32(e->off);
    if (zl + e->nlen > zc) return -1;
    memcpy(zb + zl, e->nm, (usz)e->nlen); zl += e->nlen;
  }
  int cs = zl - s;
  p32(0x06054b50u); p16(0); p16(0); p16(nz); p16(nz); p32((u32)cs); p32((u32)s); p16(0);
  return zl <= zc ? zl : -1;
}
