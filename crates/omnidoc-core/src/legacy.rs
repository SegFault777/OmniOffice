// SPDX-License-Identifier: GPL-3.0-only
//! Legacy binary formats: Word 97-2003 (.doc) and Hancom HWP 5.x (.hwp), both OLE2 compound files.
use crate::cfb::Cfb;
use crate::model::*;
use crate::zipio;

const CP1252: [char; 32] = ['€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ'];
fn cp1252(b: u8) -> char { if (0x80..0xA0).contains(&b) { CP1252[(b - 0x80) as usize] } else { b as char } }
fn u16le(b: &[u8], o: usize) -> usize { b.get(o..o + 2).map_or(0, |x| x[0] as usize | (x[1] as usize) << 8) }
fn u32le(b: &[u8], o: usize) -> usize { b.get(o..o + 4).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize) }

/// Word 97-2003: FIB -> CLX piece table -> text (or fcMin..fcMac for documents without a piece table).
pub fn read_doc(d: &[u8]) -> Result<Doc, String> {
    let c = Cfb::open(d)?;
    let wd = c.stream("WordDocument").ok_or("DOC: no WordDocument stream")?;
    if u16le(&wd, 0) != 0xA5EC { return Err("DOC: not a Word 97+ file (Word 6/95 files are not supported)".into()); }
    let flags = u16le(&wd, 0x0A);
    if flags & 0x100 != 0 { return Err("DOC: encrypted documents are not supported".into()); }
    let ccp = u32le(&wd, 0x4C); let mut text = String::new();
    let tbl = c.stream(if flags & 0x200 != 0 { "1Table" } else { "0Table" });
    let (fc, lcb) = (u32le(&wd, 0x1A2), u32le(&wd, 0x1A6));
    match tbl.as_ref().and_then(|t| t.get(fc..fc + lcb)).filter(|x| !x.is_empty()) {
        Some(clx) => {
            let mut i = 0; while clx.get(i) == Some(&1) { i += 3 + u16le(clx, i + 1); }
            if clx.get(i) != Some(&2) { return Err("DOC: piece table missing".into()); }
            let plc = clx.get(i + 5..i + 5 + u32le(clx, i + 1)).ok_or("DOC: bad piece table")?; let n = plc.len().saturating_sub(4) / 12;
            for k in 0..n {
                let (cp0, cp1) = (u32le(plc, k * 4), u32le(plc, (k + 1) * 4)); if cp0 >= ccp { break; } let len = cp1.min(ccp) - cp0;
                let fcv = u32le(plc, 4 * (n + 1) + k * 8 + 2);
                if fcv & 0x4000_0000 != 0 { let o = (fcv & 0x3FFF_FFFF) / 2; text.extend(wd.get(o..o + len).ok_or("DOC: bad text range")?.iter().map(|&b| cp1252(b))); }
                else { let u: Vec<u16> = (0..len).map(|j| u16le(&wd, fcv + 2 * j) as u16).collect(); text.extend(char::decode_utf16(u).map(|r| r.unwrap_or('?'))); }
            }
        }
        None => { // simple (non-complex) document
            let (a, b) = (u32le(&wd, 0x18), u32le(&wd, 0x1C)); let raw = wd.get(a..b.min(wd.len())).ok_or("DOC: bad text range")?;
            if flags & 0x1000 != 0 { text.extend(char::decode_utf16(raw.chunks_exact(2).map(|x| u16::from_le_bytes([x[0], x[1]]))).map(|r| r.unwrap_or('?'))); } else { text.extend(raw.iter().map(|&b| cp1252(b))); }
        }
    }
    Ok(text_to_doc(&text))
}
/// Word text stream -> paragraphs. \r ends a paragraph, 0x07 ends a table cell (twice = row), 0x13/0x14/0x15 delimit fields.
pub fn text_to_doc(text: &str) -> Doc {
    let (mut paras, mut cur, mut fields, mut cells) = (Vec::new(), String::new(), Vec::<bool>::new(), Vec::<String>::new());
    let ch: Vec<char> = text.chars().collect(); let mut i = 0;
    while i < ch.len() {
        let c = ch[i]; i += 1; let instr = fields.iter().any(|x| *x);
        match c {
            '\u{13}' => fields.push(true), '\u{14}' => if let Some(l) = fields.last_mut() { *l = false },
            '\u{15}' => { fields.pop(); }
            _ if instr => {}
            '\r' => { if !cells.is_empty() { cells.clear(); } paras.push(Para::plain(&std::mem::take(&mut cur))) }
            '\u{7}' => if ch.get(i) == Some(&'\u{7}') { // end of row: cell mark + row mark
                i += 1; cells.push(std::mem::take(&mut cur)); let mut p = Para::plain(&cells.join("\t")); p.table = true; cells.clear(); paras.push(p);
            } else { cells.push(std::mem::take(&mut cur)); },
            '\u{b}' => cur.push(' '), '\u{1e}' => cur.push('-'), c if c < ' ' && c != '\t' => {}, c => cur.push(c),
        }
    }
    if !cur.is_empty() || paras.is_empty() { paras.push(Para::plain(&cur)); }
    Doc { paras }
}

/// HWP 5.x: FileHeader flags, BodyText/SectionN streams (raw deflate when compressed), record stream -> paragraph text.
pub fn read_hwp(d: &[u8]) -> Result<Doc, String> {
    let c = Cfb::open(d)?;
    let fh = c.stream("FileHeader").ok_or("HWP: FileHeader missing")?;
    if !fh.starts_with(b"HWP Document File") { return Err("HWP: not an HWP 5.x document".into()); }
    let flags = u32le(&fh, 36);
    if flags & 2 != 0 { return Err("HWP: password-protected documents are not supported".into()); }
    if flags & 4 != 0 { return Err("HWP: distribution (view-only) documents are not supported".into()); }
    let mut paras = vec![];
    for i in 0..1000 {
        let Some(mut s) = c.stream(&format!("Section{i}")) else { break };
        if flags & 1 != 0 { s = zipio::inflate_raw(&s)?; }
        records(&s, &mut paras);
    }
    if paras.is_empty() { return Err("HWP: no body text found".into()); }
    Ok(Doc { paras })
}
fn records(b: &[u8], out: &mut Vec<Para>) {
    let (mut i, mut cur) = (0usize, None::<String>);
    while i + 4 <= b.len() {
        let h = u32le(b, i); i += 4; let tag = h & 0x3FF; let mut size = h >> 20;
        if size == 0xFFF { if i + 4 > b.len() { break; } size = u32le(b, i); i += 4; }
        let size = size; if i + size > b.len() { break; } let p = &b[i..i + size]; i += size;
        match tag {
            66 => { if let Some(t) = cur.take() { out.push(Para::plain(&t)); } cur = Some(String::new()); } // HWPTAG_PARA_HEADER
            67 => if let Some(t) = cur.as_mut() { para_text(p, t); }                                         // HWPTAG_PARA_TEXT
            _ => {} }
    }
    if let Some(t) = cur { out.push(Para::plain(&t)); }
}
/// UTF-16 text with HWP control characters: 8-code-unit inline/extended controls are skipped (tab becomes a space).
fn para_text(p: &[u8], out: &mut String) {
    let u: Vec<u16> = p.chunks_exact(2).map(|x| u16::from_le_bytes([x[0], x[1]])).collect();
    let (mut i, mut v) = (0, Vec::<u16>::new());
    while i < u.len() {
        let c = u[i]; i += 1;
        match c {
            9 => { v.push(' ' as u16); i += 7; } 10 | 30 | 31 => v.push(' ' as u16), 24 => v.push('-' as u16), 0 | 13 | 25..=29 => {}
            1..=8 | 11 | 12 | 14..=23 => i += 7, c => v.push(c),
        }
    }
    out.extend(char::decode_utf16(v).map(|r| r.unwrap_or('?')));
}
