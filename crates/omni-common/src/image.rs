// SPDX-License-Identifier: GPL-3.0-only
//! Image helpers without decoding: sniff the format and read pixel dimensions from PNG, JPEG and GIF headers.
/// (width, height) in pixels.
pub fn size(d: &[u8]) -> Option<(u32, u32)> {
    if d.len() > 24 && d.starts_with(&[0x89, b'P', b'N', b'G']) { return Some((u32::from_be_bytes(d[16..20].try_into().ok()?), u32::from_be_bytes(d[20..24].try_into().ok()?))); }
    if d.len() > 10 && (d.starts_with(b"GIF87a") || d.starts_with(b"GIF89a")) { return Some((u16::from_le_bytes([d[6], d[7]]) as u32, u16::from_le_bytes([d[8], d[9]]) as u32)); }
    if d.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < d.len() {
            if d[i] != 0xFF { i += 1; continue; }
            let m = d[i + 1]; let len = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&m) && !matches!(m, 0xC4 | 0xC8 | 0xCC) { return Some((u16::from_be_bytes([d[i + 7], d[i + 8]]) as u32, u16::from_be_bytes([d[i + 5], d[i + 6]]) as u32)); }
            i += 2 + len.max(2);
        }
    }
    None
}
/// File extension for the image data ("png", "jpeg", "gif") or None when unknown.
pub fn ext(d: &[u8]) -> Option<&'static str> {
    if d.starts_with(&[0x89, b'P', b'N', b'G']) { Some("png") } else if d.starts_with(&[0xFF, 0xD8]) { Some("jpeg") } else if d.starts_with(b"GIF8") { Some("gif") } else { None }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn sniff() {
        let mut png = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R']; png.extend(640u32.to_be_bytes()); png.extend(480u32.to_be_bytes()); png.extend([8, 6, 0, 0, 0]);
        assert_eq!((size(&png), ext(&png)), (Some((640, 480)), Some("png")));
        let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 17, 8, 0x01, 0xE0, 0x02, 0x80, 3, 0, 0, 0]; assert_eq!(size(&jpg), Some((640, 480)));
        let gif = [b'G', b'I', b'F', b'8', b'9', b'a', 0x40, 0x01, 0xF0, 0x00, 0, 0]; assert_eq!(size(&gif), Some((320, 240))); assert_eq!(size(b"junk"), None);
    }
}
