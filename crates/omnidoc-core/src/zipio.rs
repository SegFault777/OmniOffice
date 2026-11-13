// SPDX-License-Identifier: GPL-3.0-only
//! ZIP reader (stored + deflate via miniz_oxide) and stored-only writer.
fn r16(p: &[u8]) -> usize { p[0] as usize | (p[1] as usize) << 8 }
fn r32(p: &[u8]) -> usize { r16(p) | r16(&p[2..]) << 16 }
/// Ok(None) when the entry does not exist.
pub fn read(z: &[u8], name: &str) -> Result<Option<Vec<u8>>, String> {
    let e = (0..z.len().saturating_sub(21)).rev().find(|&i| r32(&z[i..]) == 0x06054b50).ok_or("not a zip file")?;
    let (cnt, mut p) = (r16(&z[e + 10..]), r32(&z[e + 16..]));
    for _ in 0..cnt {
        if p + 46 > z.len() || r32(&z[p..]) != 0x02014b50 { return Err("corrupt zip directory".into()); }
        let (m, cs, fl, el, cl, lh) = (r16(&z[p + 10..]), r32(&z[p + 20..]), r16(&z[p + 28..]), r16(&z[p + 30..]), r16(&z[p + 32..]), r32(&z[p + 42..]));
        if z.get(p + 46..p + 46 + fl) == Some(name.as_bytes()) {
            let d = lh + 30 + r16(&z[lh + 26..]) + r16(&z[lh + 28..]);
            let raw = z.get(d..d + cs).ok_or("truncated zip")?;
            return match m { 0 => Ok(Some(raw.to_vec())), 8 => miniz_oxide::inflate::decompress_to_vec(raw).map(Some).map_err(|_| "inflate failed".into()), _ => Err("unsupported compression".into()) };
        }
        p += 46 + fl + el + cl;
    }
    Ok(None)
}
pub fn crc32(d: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in d { c ^= b as u32; for _ in 0..8 { c = if c & 1 != 0 { (c >> 1) ^ 0xEDB88320 } else { c >> 1 }; } }
    !c
}
#[derive(Default)]
pub struct Writer { buf: Vec<u8>, cd: Vec<(String, u32, u32, u32)> }
impl Writer {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, name: &str, d: &[u8]) {
        let (crc, off) = (crc32(d), self.buf.len() as u32);
        let b = &mut self.buf;
        b.extend(0x04034b50u32.to_le_bytes()); b.extend([20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]);
        b.extend(crc.to_le_bytes()); b.extend((d.len() as u32).to_le_bytes()); b.extend((d.len() as u32).to_le_bytes());
        b.extend((name.len() as u16).to_le_bytes()); b.extend([0, 0]); b.extend(name.as_bytes()); b.extend(d);
        self.cd.push((name.into(), crc, d.len() as u32, off));
    }
    pub fn finish(mut self) -> Vec<u8> {
        let s = self.buf.len() as u32;
        let cd = self.cd.clone();
        let n = cd.len() as u16;
        for (nm, crc, sz, off) in cd {
            let b = &mut self.buf;
            b.extend(0x02014b50u32.to_le_bytes()); b.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]);
            b.extend(crc.to_le_bytes()); b.extend(sz.to_le_bytes()); b.extend(sz.to_le_bytes());
            b.extend((nm.len() as u16).to_le_bytes()); b.extend([0u8; 12]); b.extend(off.to_le_bytes()); b.extend(nm.as_bytes());
        }
        let cs = self.buf.len() as u32 - s;
        let b = &mut self.buf;
        b.extend(0x06054b50u32.to_le_bytes()); b.extend([0, 0, 0, 0]); b.extend(n.to_le_bytes()); b.extend(n.to_le_bytes());
        b.extend(cs.to_le_bytes()); b.extend(s.to_le_bytes()); b.extend([0, 0]);
        self.buf
    }
}
