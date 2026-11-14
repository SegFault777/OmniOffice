// SPDX-License-Identifier: GPL-3.0-only
//! Read-only OLE2 / Compound File Binary reader (used by .doc, later .hwp).
const END: u32 = 0xFFFF_FFFE;
fn u16at(d: &[u8], o: usize) -> usize { d.get(o..o + 2).map_or(0, |x| x[0] as usize | (x[1] as usize) << 8) }
fn u32at(d: &[u8], o: usize) -> u32 { d.get(o..o + 4).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])) }
pub struct Cfb<'a> { d: &'a [u8], ss: usize, fat: Vec<u32>, mfat: Vec<u32>, mini: Vec<u8>, dir: Vec<(String, u32, usize)>, cutoff: usize }
impl<'a> Cfb<'a> {
    fn chain(&self, start: u32, fat: &[u32]) -> Vec<u32> { let (mut v, mut s) = (vec![], start); while s < 0xFFFF_FFF0 && v.len() < fat.len() + 1 { v.push(s); s = fat.get(s as usize).copied().unwrap_or(END); } v }
    fn sector(&self, id: u32) -> &'a [u8] { let o = (id as usize + 1) * self.ss; self.d.get(o..(o + self.ss).min(self.d.len())).unwrap_or(&[]) }
    pub fn open(d: &'a [u8]) -> Result<Self, String> {
        if d.len() < 512 || d[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] { return Err("not an OLE2 compound file".into()); }
        let (shift, mshift) = (u16at(d, 0x1E), u16at(d, 0x20)); if !(7..=16).contains(&shift) { return Err("bad sector size".into()); }
        let mut c = Cfb { d, ss: 1 << shift, fat: vec![], mfat: vec![], mini: vec![], dir: vec![], cutoff: u32at(d, 0x38) as usize };
        let mut fat_secs: Vec<u32> = (0..109).map(|i| u32at(d, 0x4C + i * 4)).filter(|&s| s < 0xFFFF_FFF0).collect();
        let mut dif = u32at(d, 0x44);
        for _ in 0..u32at(d, 0x48) { if dif >= 0xFFFF_FFF0 { break; } let s = c.sector(dif); for i in 0..(c.ss / 4 - 1) { let v = u32at(s, i * 4); if v < 0xFFFF_FFF0 { fat_secs.push(v); } } dif = u32at(s, c.ss - 4); }
        for s in fat_secs { let b = c.sector(s); c.fat.extend((0..b.len() / 4).map(|i| u32at(b, i * 4))); }
        let dir_bytes: Vec<u8> = c.chain(u32at(d, 0x30), &c.fat).iter().flat_map(|&s| c.sector(s).to_vec()).collect();
        for e in dir_bytes.chunks_exact(128) {
            let n = u16at(e, 0x40).saturating_sub(2) / 2; let name: String = char::decode_utf16((0..n).map(|i| u16at(e, i * 2) as u16)).map(|r| r.unwrap_or('?')).collect();
            c.dir.push((if e[0x42] == 5 { "\0root".into() } else { name }, u32at(e, 0x74), u32at(e, 0x78) as usize));
        }
        let mf: Vec<u8> = c.chain(u32at(d, 0x3C), &c.fat).iter().flat_map(|&s| c.sector(s).to_vec()).collect();
        c.mfat = (0..mf.len() / 4).map(|i| u32at(&mf, i * 4)).collect();
        let _ = mshift;
        if let Some((_, st, sz)) = c.dir.first().cloned() { let mut m: Vec<u8> = c.chain(st, &c.fat).iter().flat_map(|&s| c.sector(s).to_vec()).collect(); m.truncate(sz); c.mini = m; }
        Ok(c)
    }
    pub fn stream(&self, name: &str) -> Option<Vec<u8>> {
        let (_, start, size) = self.dir.iter().skip(1).find(|e| e.0 == name)?.clone();
        let mut out: Vec<u8> = if size < self.cutoff { self.chain(start, &self.mfat).iter().flat_map(|&s| self.mini.get(s as usize * 64..(s as usize + 1) * 64).unwrap_or(&[]).to_vec()).collect() }
            else { self.chain(start, &self.fat).iter().flat_map(|&s| self.sector(s).to_vec()).collect() };
        out.truncate(size); Some(out)
    }
}
