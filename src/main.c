/* SPDX-License-Identifier: GPL-3.0-only */
/* OmniDoc 1.0-alpha-1 - terminal word processor / markup editor.
 * Freestanding: no libc, no malloc. All memory is static. */
#include "omni.h"
#define VER "1.0-alpha-1"
#define CAP (1 << 22)  /* max document size: 4 MiB */
#define BIG (1 << 24)  /* scratch buffers: 16 MiB (zero-page backed .bss) */

static u8 B[CAP], X[BIG], Z[BIG], O[1 << 18];
static int N, cur, topl, want, dirty, on, nops, pused, xl, xo;
static char fname[512], msg[96];
static u8 pool[1 << 20];
typedef struct { int pos, len, del, off; } Op;
static Op ops[4096];

static int slen(const char *s) { int n = 0; while (s[n]) n++; return n; }
static int streq(const char *a, const char *b) { while (*a && *a == *b) { a++; b++; } return *a == *b; }
static void setmsg(const char *s) { int i = 0; while (s[i] && i < 95) { msg[i] = s[i]; i++; } msg[i] = 0; }

/* ---------- format table ---------- */
enum { RN, RT, RD, RO, RH, RM, RX }; /* none, text, docx, odf, hwpx, omd, xml-text */
typedef struct { const char *ext, *name; int rd, wr; } Fm; /* wr: 0 none,1 text,2 omd,3 docx */
static const Fm FM[] = {
  {"omd","OmniDoc",RM,2},{"txt","Plain Text",RT,1},{"md","Markdown",RT,1},{"html","HTML",RT,1},
  {"xhtml","XHTML",RT,1},{"xml","XML",RT,1},{"tex","LaTeX",RT,1},{"rst","reStructuredText",RT,1},
  {"org","Org mode",RT,1},{"typ","Typst",RT,1},{"wiki","MediaWiki",RT,1},{"adoc","AsciiDoc",RT,1},
  {"sgml","SGML",RT,1},{"gml","GML",RT,1},{"docx","Word OOXML",RD,3},{"docm","Word OOXML macro",RD,0},
  {"odt","OpenDocument",RO,0},{"sxw","OpenOffice 1.x",RO,0},{"hwpx","Hancom OWPML",RH,0},
  {"hml","Hancom Markup",RX,0},{"abw","AbiWord",RX,0},
  {"rtf","RTF",RN,0},{"doc","Word 97-2003",RN,0},{"pdf","PDF",RN,0},{"hwp","Hancom HWP",RN,0},
  {"gul","JungUm GUL",RN,0},{"jung","JungUm",RN,0},{"pages","Apple Pages",RN,0},{"wpd","WordPerfect",RN,0},
  {"wps","MS Works/WPS",RN,0},{"lwp","Lotus Word Pro",RN,0},{"wri","Windows Write",RN,0},
  {"cwk","ClarisWorks",RN,0},{"uof","UOF",RN,0},
};
static const Fm *fm_of(const char *p) {
  const char *d = 0;
  for (; *p; p++) { if (*p == '.') d = p + 1; else if (*p == '/' || *p == '\\') d = 0; }
  if (!d) return 0;
  for (int i = 0; i < (int)(sizeof FM / sizeof *FM); i++) {
    const char *a = d, *b = FM[i].ext;
    while (*a && *b && (*a | 32) == *b) { a++; b++; }
    if (!*a && !*b) return &FM[i];
  }
  return 0;
}

/* ---------- buffer + undo ---------- */
static void rins(int p, const u8 *s, int n) { memmove(B + p + n, B + p, (usz)(N - p)); memcpy(B + p, s, (usz)n); N += n; }
static void rdel(int p, int n) { memmove(B + p, B + p + n, (usz)(N - p - n)); N -= n; }
static void ins(int p, const u8 *s, int n) {
  if (N + n > CAP) { setmsg("Document too large"); return; }
  if (nops && !ops[nops - 1].del && ops[nops - 1].pos + ops[nops - 1].len == p && n < 5 && s[0] != '\n' && B[p - 1] != '\n')
    ops[nops - 1].len += n;
  else { if (nops == 4096) nops = pused = 0; ops[nops].pos = p; ops[nops].len = n; ops[nops].del = 0; ops[nops++].off = pused; }
  rins(p, s, n); cur = p + n; dirty = 1;
}
static void del(int p, int n) {
  if (n <= 0 || p < 0 || p + n > N) return;
  if (nops == 4096 || pused + n > (int)sizeof pool) nops = pused = 0;
  memcpy(pool + pused, B + p, (usz)n);
  ops[nops].pos = p; ops[nops].len = n; ops[nops].del = 1; ops[nops++].off = pused;
  pused += n; rdel(p, n); cur = p; dirty = 1;
}
static void undo(void) {
  if (!nops) { setmsg("Nothing to undo"); return; }
  Op o = ops[--nops];
  if (o.del) { rins(o.pos, pool + o.off, o.len); cur = o.pos + o.len; } else { rdel(o.pos, o.len); cur = o.pos; }
  pused = o.off; dirty = 1;
}

/* ---------- UTF-8 ---------- */
static u32 dec(const u8 *s, int avail, int *l) {
  u8 c = s[0]; *l = 1;
  if (c < 0x80) return c;
  if (c >= 0xC2 && c < 0xE0 && avail > 1) { *l = 2; return (u32)(c & 31) << 6 | (s[1] & 63); }
  if (c >= 0xE0 && c < 0xF0 && avail > 2) { *l = 3; return (u32)(c & 15) << 12 | (u32)(s[1] & 63) << 6 | (s[2] & 63); }
  if (c >= 0xF0 && c < 0xF8 && avail > 3) { *l = 4; return (u32)(c & 7) << 18 | (u32)(s[1] & 63) << 12 | (u32)(s[2] & 63) << 6 | (s[3] & 63); }
  return c;
}
static int wid(u32 c) {
  if ((c >= 0x1100 && c <= 0x115F) || (c >= 0x2E80 && c <= 0xA4CF) || (c >= 0xAC00 && c <= 0xD7A3) ||
      (c >= 0xF900 && c <= 0xFAFF) || (c >= 0xFE30 && c <= 0xFE6F) || (c >= 0xFF00 && c <= 0xFF60) ||
      (c >= 0xFFE0 && c <= 0xFFE6) || (c >= 0x20000 && c <= 0x3FFFD)) return 2;
  return (c >= 0x300 && c <= 0x36F) ? 0 : 1;
}
static int nxt(int p) { if (p < N) { do p++; while (p < N && (B[p] & 0xC0) == 0x80); } return p; }
static int prv(int p) { if (p > 0) { do p--; while (p > 0 && (B[p] & 0xC0) == 0x80); } return p; }
static int cw(int p) { int l; return wid(dec(B + p, N - p, &l)); }
static int ls(int p) { while (p > 0 && B[p - 1] != '\n') p--; return p; }
static int le(int p) { while (p < N && B[p] != '\n') p++; return p; }
static int colof(int p) { int w = 0; for (int i = ls(p); i < p; i = nxt(i)) w += cw(i); return w; }
static int adv(int p, int want_) { int w = 0, e = le(p); while (p < e) { int c = cw(p); if (w + c > want_) break; w += c; p = nxt(p); } return p; }
static int vend(int p, int C) { /* start of next visual (soft-wrapped) row */
  int w = 0;
  while (p < N) {
    if (B[p] == '\n') return p + 1;
    int c = cw(p); if (w + c > C && w > 0) return p;
    w += c; p = nxt(p);
  }
  return p;
}

/* ---------- import (text extraction) ---------- */
static void bput(u8 c) { if (N < CAP - 8) B[N++] = c; }
static void bcp(u32 c) {
  if (c < 0x80) bput((u8)c);
  else if (c < 0x800) { bput((u8)(0xC0 | c >> 6)); bput((u8)(0x80 | (c & 63))); }
  else if (c < 0x10000) { bput((u8)(0xE0 | c >> 12)); bput((u8)(0x80 | (c >> 6 & 63))); bput((u8)(0x80 | (c & 63))); }
  else { bput((u8)(0xF0 | c >> 18)); bput((u8)(0x80 | (c >> 12 & 63))); bput((u8)(0x80 | (c >> 6 & 63))); bput((u8)(0x80 | (c & 63))); }
}
static int is_name(const u8 *a, int n, const char *w) { int k = slen(w); if (n != k) return 0; for (int i = 0; i < n; i++) if ((a[i] | 32) != w[i]) return 0; return 1; }
static void xml_text(const u8 *s, int n) {
  for (int i = 0; i < n;) {
    if (s[i] == '<') {
      int j = i + 1, cl = 0;
      if (j < n && s[j] == '/') { cl = 1; j++; }
      int a = j;
      while (j < n && s[j] != '>' && s[j] != ' ' && s[j] != '/' && s[j] != '\n' && s[j] != '\t' && s[j] != '\r') j++;
      for (int k = a; k < j; k++) if (s[k] == ':') a = k + 1;
      int len = j - a;
      if (cl && (is_name(s + a, len, "p") || is_name(s + a, len, "h"))) bput('\n');
      else if (!cl && (is_name(s + a, len, "br") || is_name(s + a, len, "line-break"))) bput('\n');
      else if (!cl && is_name(s + a, len, "tab")) bput(' ');
      while (i < n && s[i] != '>') i++;
      i++;
    } else if (s[i] == '&') {
      int j = i + 1; while (j < n && j < i + 10 && s[j] != ';') j++;
      if (j < n && s[j] == ';') {
        const u8 *e = s + i + 1; int l = j - i - 1; u32 v = 0;
        if (is_name(e, l, "amp")) v = '&'; else if (is_name(e, l, "lt")) v = '<'; else if (is_name(e, l, "gt")) v = '>';
        else if (is_name(e, l, "quot")) v = '"'; else if (is_name(e, l, "apos")) v = '\'';
        else if (l > 1 && e[0] == '#') {
          int hx = (e[1] | 32) == 'x';
          for (int k = hx ? 2 : 1; k < l; k++) { int d = e[k] <= '9' ? e[k] - '0' : (e[k] | 32) - 'a' + 10; v = v * (hx ? 16 : 10) + (u32)d; }
        }
        if (v) { bcp(v); i = j + 1; continue; }
      }
      bput('&'); i++;
    } else { if (s[i] != '\n' && s[i] != '\r') bput(s[i]); i++; }
  }
}
static int load(const char *path) {
  const Fm *f = fm_of(path);
  if (!f || f->rd == RN) { setmsg("No reader for this format yet (see docs/project/roadmap.md)"); return -1; }
  int n = p_read_file(path, X, BIG);
  if (n < 0) { setmsg("Cannot read file"); return -1; }
  N = 0; cur = topl = want = dirty = nops = pused = 0;
  int z = 0;
  switch (f->rd) {
    case RT: { int o = (n >= 3 && X[0] == 0xEF && X[1] == 0xBB && X[2] == 0xBF) ? 3 : 0; if (n - o > CAP) { setmsg("File too large"); return -1; } memcpy(B, X + o, (usz)(n - o)); N = n - o; break; }
    case RM: z = zget(X, n, "content.md", Z, BIG); if (z >= 0) { memcpy(B, Z, (usz)z); N = z; } break;
    case RD: z = zget(X, n, "word/document.xml", Z, BIG); if (z >= 0) xml_text(Z, z); break;
    case RO: z = zget(X, n, "content.xml", Z, BIG); if (z >= 0) xml_text(Z, z); break;
    case RH: {
      char nm[32] = "Contents/section";
      for (int i = 0; i < 100; i++) {
        int k = 16;
        if (i >= 10) nm[k++] = (char)('0' + i / 10);
        nm[k++] = (char)('0' + i % 10); memcpy(nm + k, ".xml", 5);
        z = zget(X, n, nm, Z, BIG);
        if (z == -2) { z = i ? 0 : -2; break; }
        if (z < 0) break;
        xml_text(Z, z);
      }
      break;
    }
    default: xml_text(X, n); break;
  }
  if (z < 0) { N = 0; setmsg("Corrupt or unsupported container"); return -1; }
  return 0;
}

/* ---------- export ---------- */
static void xb(u8 c) { if (xl < BIG - 8) X[xl++] = c; else xo = 1; }
static void xa(const char *s) { while (*s) xb((u8)*s++); }
static int save_docx(void) {
  static const char ct[] = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
  static const char rl[] = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
  xl = xo = 0;
  xa("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>");
  int o = 0;
  for (int i = 0; i < N; i++) {
    u8 c = B[i];
    if (!o) { xa("<w:p><w:r><w:t xml:space=\"preserve\">"); o = 1; }
    if (c == '\n') { xa("</w:t></w:r></w:p>"); o = 0; }
    else if (c == '&') xa("&amp;"); else if (c == '<') xa("&lt;"); else if (c == '>') xa("&gt;");
    else if (c >= 32) xb(c); else if (c == '\t') xb(' ');
  }
  if (o) xa("</w:t></w:r></w:p>");
  xa("</w:body></w:document>");
  if (xo) return -1;
  int dl = xl;
  zw_begin(Z, BIG);
  if (zw_add("[Content_Types].xml", (const u8 *)ct, (int)sizeof ct - 1) || zw_add("_rels/.rels", (const u8 *)rl, (int)sizeof rl - 1) ||
      zw_add("word/document.xml", X, dl)) return -1;
  return zw_end();
}
static int save(void) {
  const Fm *f = fm_of(fname);
  if (!f || !f->wr) { /* read-only format: fall back to OMD */
    int d = -1, l = slen(fname);
    for (int i = 0; i < l; i++) { if (fname[i] == '.') d = i; else if (fname[i] == '/' || fname[i] == '\\') d = -1; }
    if (d < 0) d = l;
    if (d + 5 >= (int)sizeof fname) return -1;
    memcpy(fname + d, ".omd", 5); f = fm_of(fname);
    setmsg("Format is read-only: saved as OMD instead");
  }
  int r = -1;
  if (f->wr == 1) r = p_write_file(fname, B, N);
  else if (f->wr == 2) {
    static const char mt[] = "application/vnd.omnioffice.omd+zip";
    static const char mf[] = "{\"omd\":1,\"generator\":\"OmniDoc " VER "\"}";
    zw_begin(Z, BIG);
    if (!zw_add("mimetype", (const u8 *)mt, (int)sizeof mt - 1) && !zw_add("manifest.json", (const u8 *)mf, (int)sizeof mf - 1) && !zw_add("content.md", B, N)) {
      int n = zw_end(); if (n > 0) r = p_write_file(fname, Z, n);
    }
  } else { int n = save_docx(); if (n > 0) r = p_write_file(fname, Z, n); }
  if (r) { setmsg("Save failed"); return -1; }
  dirty = 0; if (!msg[0]) setmsg("Saved");
  return 0;
}

/* ---------- screen ---------- */
static void os(const char *s) { while (*s && on < (int)sizeof O - 16) O[on++] = (u8)*s++; }
static void oc(u8 c) { if (on < (int)sizeof O - 16) O[on++] = c; }
static void oi(int v) { char t[12]; int n = 0; if (!v) t[n++] = '0'; while (v) { t[n++] = (char)('0' + v % 10); v /= 10; } while (n) oc((u8)t[--n]); }
static void at(int r, int c) { os("\x1b["); oi(r); oc(';'); oi(c); oc('H'); }
static int ostr(const char *s, int room) { /* output UTF-8 string clipped to room columns; returns columns used */
  int w = 0, l;
  while (*s) { u32 c = dec((const u8 *)s, slen(s), &l); int k = wid(c); if (w + k > room) break; for (int i = 0; i < l; i++) oc((u8)s[i]); s += l; w += k; }
  return w;
}
#define CONT(p, q) (cur >= (p) && (cur < (q) || ((q) == N && cur == N && (N == 0 || B[N - 1] != '\n' || (p) == N))))
static void draw(void) {
  int R, C; p_size(&R, &C);
  int rows = R - 2; if (rows < 1 || C < 20) return;
  if (cur < topl) topl = ls(cur);
  for (;;) {
    int p = topl, f = 0;
    for (int r = 0; r < rows; r++) { int q = vend(p, C); if (CONT(p, q)) { f = 1; break; } p = q; }
    if (f || topl >= N) break;
    topl = vend(topl, C);
  }
  on = 0; os("\x1b[?25l"); at(1, 1); os("\x1b[7m");
  int w = ostr(" OmniDoc " VER " | File  Home  Insert  View | ", C);
  w += ostr(fname, C - w); const Fm *f = fm_of(fname);
  if (f) { w += ostr(" [", C - w); w += ostr(f->name, C - w); w += ostr("]", C - w); }
  if (dirty) w += ostr(" *", C - w);
  while (w++ < C) oc(' ');
  os("\x1b[0m");
  int p = topl, crow = 0, ccol = 0, found = 0;
  for (int r = 0; r < rows; r++) {
    at(r + 2, 1);
    if (p <= N) {
      int q = vend(p, C);
      if (!found && CONT(p, q)) { found = 1; crow = r; ccol = 0; for (int i = p; i < cur; i = nxt(i)) ccol += cw(i); }
      for (int i = p; i < q; i++) { u8 c = B[i]; if (c == '\n') continue; oc((c < 32 || c == 127) ? ' ' : c); }
      p = (q == p) ? N + 1 : q;
    }
    os("\x1b[K");
  }
  int ln = 1, wc = 0, in = 0;
  for (int i = 0; i < N; i++) { if (B[i] == '\n' && i < cur) ln++; int sp = B[i] == ' ' || B[i] == '\n' || B[i] == '\t' || B[i] == '\r'; if (!sp && !in) wc++; in = !sp; }
  at(R, 1); os("\x1b[7m");
  if (msg[0]) { w = ostr(" ", C); w += ostr(msg, C - w); }
  else {
    w = ostr(" Ln ", C); oi(ln); w += 3; /* approximate; padded below */
    w += ostr(", Col ", C - w); oi(ccol + 1); w += 3; w += ostr(" | Words ", C - w); oi(wc); w += 4;
    w += ostr(" | ^S Save ^Q Quit ^Z Undo ^F Find", C - w);
  }
  while (w++ < C) oc(' ');
  os("\x1b[0m"); at(crow + 2, (ccol < C ? ccol : C - 1) + 1); os("\x1b[?25h");
  p_write(O, on);
}

/* ---------- input ---------- */
enum { K_UP = 1000, K_DN, K_RT, K_LT, K_HOME, K_END, K_DEL, K_PGUP, K_PGDN };
static int key(void) {
  int c = p_getc();
  if (c != 27) return c;
  int a = p_getc();
  if (a == 'O') { int b = p_getc(); return b == 'H' ? K_HOME : b == 'F' ? K_END : 0; }
  if (a != '[') return 0;
  int n = 0, b;
  while ((b = p_getc()) >= 0 && b < 0x40) if (b >= '0' && b <= '9') n = n * 10 + (b - '0');
  switch (b) {
    case 'A': return K_UP; case 'B': return K_DN; case 'C': return K_RT; case 'D': return K_LT;
    case 'H': return K_HOME; case 'F': return K_END;
    case '~': return n == 1 || n == 7 ? K_HOME : n == 4 || n == 8 ? K_END : n == 3 ? K_DEL : n == 5 ? K_PGUP : n == 6 ? K_PGDN : 0;
  }
  return 0;
}
static void up_(void) { int s = ls(cur); if (s > 0) cur = adv(ls(s - 1), want); }
static void dn_(void) { int e = le(cur); if (e < N) cur = adv(e + 1, want); }
static void find(void) {
  u8 q[64]; int n = 0, R, C; p_size(&R, &C);
  for (;;) {
    on = 0; at(R, 1); os("\x1b[7m Find: "); for (int i = 0; i < n; i++) oc(q[i]); os("\x1b[K\x1b[0m"); p_write(O, on);
    int k = key();
    if (k == '\r' || k == '\n') break;
    if (k == 7 || k == 3) return;
    if ((k == 127 || k == 8) && n) { do n--; while (n && (q[n] & 0xC0) == 0x80); }
    else if (k >= 32 && k < 256 && k != 127 && n < 63) q[n++] = (u8)k;
  }
  if (!n) return;
  for (int s = 1; s <= N; s++) {
    int i = (cur + s) % (N ? N : 1);
    if (i + n <= N && !memcmp(B + i, q, (usz)n)) { cur = i; want = colof(cur); return; }
  }
  setmsg("Not found");
}

int app_main(int argc, char **argv) {
  if (argc >= 2 && streq(argv[1], "--version")) { p_write("OmniDoc " VER "\n", 8 + slen(VER) + 1); return 0; }
  if (argc == 4 && streq(argv[1], "--convert")) { /* non-interactive: omnidoc --convert in out */
    if (load(argv[2])) return 1;
    int l = slen(argv[3]); if (l >= (int)sizeof fname) return 3;
    memcpy(fname, argv[3], (usz)l + 1); msg[0] = 0;
    return save() ? 2 : 0;
  }
  if (argc >= 2) { int l = slen(argv[1]); if (l >= (int)sizeof fname) return 3; memcpy(fname, argv[1], (usz)l + 1); if (load(fname)) { if (p_read_file(fname, X, 1) >= 0) { /* exists but unreadable here */ } } }
  else memcpy(fname, "untitled.omd", 13);
  p_raw(1); p_write("\x1b[?1049h", 8);
  int armed = 0;
  for (;;) {
    draw();
    int k = key(), R, C; p_size(&R, &C);
    if (k < 0) break;
    if (k != 17) armed = 0;
    if (k != 0) msg[0] = 0;
    if (k == 17) { if (dirty && !armed) { setmsg("Unsaved changes - press ^Q again to quit"); armed = 1; continue; } break; }
    else if (k == 19) { msg[0] = 0; save(); }
    else if (k == 26) undo();
    else if (k == 6) find();
    else if (k == K_LT) { cur = prv(cur); want = colof(cur); }
    else if (k == K_RT) { cur = nxt(cur); want = colof(cur); }
    else if (k == K_UP) up_(); else if (k == K_DN) dn_();
    else if (k == K_PGUP) for (int i = 0; i < R - 3; i++) up_();
    else if (k == K_PGDN) for (int i = 0; i < R - 3; i++) dn_();
    else if (k == K_HOME || k == 1) { cur = ls(cur); want = 0; }
    else if (k == K_END || k == 5) { cur = le(cur); want = colof(cur); }
    else if (k == K_DEL) { if (cur < N) del(cur, nxt(cur) - cur); }
    else if (k == 127 || k == 8) { if (cur > 0) del(prv(cur), cur - prv(cur)); want = colof(cur); }
    else if (k == '\r' || k == '\n') { u8 c = '\n'; ins(cur, &c, 1); want = 0; }
    else if (k == '\t') { ins(cur, (const u8 *)"    ", 4); want = colof(cur); }
    else if (k >= 32 && k < 127) { u8 c = (u8)k; ins(cur, &c, 1); want = colof(cur); }
    else if (k >= 0xC2 && k < 0xF8) {
      u8 t[4]; int n = k >= 0xF0 ? 3 : k >= 0xE0 ? 2 : 1; t[0] = (u8)k;
      for (int i = 1; i <= n; i++) { int c = p_getc(); if (c < 0) break; t[i] = (u8)c; }
      ins(cur, t, n + 1); want = colof(cur);
    }
  }
  p_write("\x1b[?1049l", 8); p_raw(0);
  return 0;
}
