// SPDX-License-Identifier: GPL-3.0-only
//! Compatibility tests built from synthetic files (hand-written XML / generated OLE2 containers).
use omnidoc_core::{formats, model::*, zipio};

fn docx(document: &str, styles: &str, numbering: &str) -> Vec<u8> {
    let mut z = zipio::Writer::new();
    z.add("word/document.xml", format!("<w:document xmlns:w=\"w\" xmlns:mc=\"mc\"><w:body>{document}</w:body></w:document>").as_bytes());
    if !styles.is_empty() { z.add("word/styles.xml", format!("<w:styles xmlns:w=\"w\">{styles}</w:styles>").as_bytes()); }
    if !numbering.is_empty() { z.add("word/numbering.xml", format!("<w:numbering xmlns:w=\"w\">{numbering}</w:numbering>").as_bytes()); }
    z.finish()
}
fn p(style: &str, text: &str) -> String { format!("<w:p><w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>") }

#[test] fn docx_localized_styles_and_inheritance() {
    let styles = "<w:style w:type=\"paragraph\" w:styleId=\"1\"><w:name w:val=\"heading 1\"/></w:style>\
        <w:style w:type=\"paragraph\" w:styleId=\"Kor2\"><w:name w:val=\"제목 2\"/></w:style>\
        <w:style w:type=\"paragraph\" w:styleId=\"Mine\"><w:name w:val=\"My Heading\"/><w:basedOn w:val=\"1\"/></w:style>\
        <w:style w:type=\"paragraph\" w:styleId=\"Out\"><w:name w:val=\"Custom\"/><w:pPr><w:outlineLvl w:val=\"1\"/></w:pPr></w:style>\
        <w:style w:type=\"paragraph\" w:styleId=\"Strong\"><w:name w:val=\"Emphasis para\"/><w:rPr><w:b/></w:rPr></w:style>";
    let d = formats::load("docx", &docx(&(p("1", "A") + &p("Kor2", "B") + &p("Mine", "C") + &p("Out", "D") + &p("Strong", "E")), styles, "")).unwrap();
    let s: Vec<Style> = d.paras.iter().map(|x| x.style).collect();
    assert_eq!(s, [Style::H1, Style::H2, Style::H1, Style::H2, Style::Normal]);
    assert!(d.paras[4].runs[0].bold, "bold inherited from paragraph style");
}
#[test] fn docx_tables_lists_textboxes_fallback_hidden() {
    let numbering = "<w:abstractNum w:abstractNumId=\"0\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"bullet\"/></w:lvl></w:abstractNum>\
        <w:abstractNum w:abstractNumId=\"1\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"decimal\"/></w:lvl></w:abstractNum>\
        <w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num><w:num w:numId=\"2\"><w:abstractNumId w:val=\"1\"/></w:num>";
    let li = |id: &str, t: &str| format!("<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"{id}\"/></w:numPr></w:pPr><w:r><w:t>{t}</w:t></w:r></w:p>");
    let cell = |t: &str| format!("<w:tc><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:tc>");
    let body = li("1", "dot") + &li("2", "one") + &li("2", "two")
        + &format!("<w:tbl><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>", cell("a"), cell("b"), cell("c"), cell("d"))
        + "<mc:AlternateContent><mc:Choice><w:p><w:r><w:t>real</w:t></w:r></w:p></mc:Choice><mc:Fallback><w:p><w:r><w:t>dup</w:t></w:r></w:p></mc:Fallback></mc:AlternateContent>"
        + "<w:p><w:r><w:t>shown </w:t></w:r><w:r><w:rPr><w:vanish/></w:rPr><w:t>hidden</w:t></w:r><w:del><w:r><w:delText>gone</w:delText></w:r></w:del><w:ins><w:r><w:t>added</w:t></w:r></w:ins></w:p>";
    let d = formats::load("docx", &docx(&body, "", numbering)).unwrap();
    assert_eq!(d.plain(), "\u{2022} dot\n1. one\n2. two\na\tb\nc\td\nreal\nshown added\n");
}
#[test] fn docx_write_has_styles_and_roundtrips() {
    let mut d = Doc::from_plain("Title\nbody"); d.paras[0].style = Style::H1; d.paras[1].align = Align::Justify;
    let b = formats::save("docx", &d).unwrap();
    for part in ["word/styles.xml", "docProps/core.xml", "word/_rels/document.xml.rels"] { assert!(zipio::read(&b, part).unwrap().is_some(), "{part}"); }
    let doc = String::from_utf8(zipio::read(&b, "word/document.xml").unwrap().unwrap()).unwrap(); assert!(doc.contains("<w:sectPr>"));
    assert_eq!(formats::load("docx", &b).unwrap(), d);
}

/// Generic OLE2 builder: header, one FAT sector, directory sectors, then the streams (each >= 4096 bytes, so no mini stream).
fn cfb(streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let ndir = (streams.len() + 1 + 3) / 4;
    let pad = |v: &Vec<u8>| { let mut x = v.clone(); x.resize((v.len().max(4096) + 511) / 512 * 512, 0); x };
    let data: Vec<Vec<u8>> = streams.iter().map(|s| pad(&s.1)).collect();
    let total = 1 + ndir + data.iter().map(|d| d.len() / 512).sum::<usize>(); assert!(total <= 128);
    let mut f = vec![0u8; 512 * (1 + total)]; f[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let put = |f: &mut Vec<u8>, o: usize, v: u32| f[o..o + 4].copy_from_slice(&v.to_le_bytes());
    f[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes()); f[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    for (o, v) in [(0x2C, 1), (0x30, 1), (0x38, 4096), (0x3C, 0xFFFF_FFFE), (0x40, 0), (0x44, 0xFFFF_FFFE), (0x48, 0)] { put(&mut f, o, v); }
    for i in 0..109 { put(&mut f, 0x4C + i * 4, if i == 0 { 0 } else { 0xFFFF_FFFF }); }
    let mut fat = vec![0xFFFF_FFFFu32; 128]; fat[0] = 0xFFFF_FFFD;
    for s in 1..=ndir { fat[s] = if s == ndir { 0xFFFF_FFFE } else { s as u32 + 1 }; }
    let mut starts = vec![]; let mut next = 1 + ndir;
    for d in &data { starts.push(next); let n = d.len() / 512; for k in 0..n { fat[next + k] = if k + 1 == n { 0xFFFF_FFFE } else { (next + k + 1) as u32 }; } next += n; }
    for (i, v) in fat.iter().enumerate() { put(&mut f, 512 + i * 4, *v); }
    let mut ents: Vec<(String, u8, u32, u32)> = vec![("Root Entry".into(), 5, 0xFFFF_FFFE, 0)];
    for (k, (n, raw)) in streams.iter().enumerate() { ents.push((n.to_string(), 2, starts[k] as u32, raw.len().max(4096) as u32)); }
    for (k, (name, ty, start, size)) in ents.iter().enumerate() {
        let e = 1024 + k * 128; for (j, c) in name.encode_utf16().enumerate() { f[e + j * 2..e + j * 2 + 2].copy_from_slice(&c.to_le_bytes()); }
        f[e + 0x40..e + 0x42].copy_from_slice(&(((name.len() + 1) * 2) as u16).to_le_bytes()); f[e + 0x42] = *ty; put(&mut f, e + 0x74, *start); put(&mut f, e + 0x78, *size);
    }
    for (k, d) in data.iter().enumerate() { f[(1 + starts[k]) * 512..(1 + starts[k]) * 512 + d.len()].copy_from_slice(d); }
    f
}
/// Word binary with several pieces: (compressed 8-bit text?, text). Text is stored from offset 0x800 on.
fn word(pieces: &[(bool, &str)]) -> Vec<u8> {
    let (mut wd, mut cps, mut pcds, mut off, mut cp) = (vec![0u8; 0x800], vec![0u32], vec![], 0x800usize, 0u32);
    for (comp, t) in pieces {
        let bytes: Vec<u8> = if *comp { t.chars().map(|c| match c { '‘' => 0x91, '’' => 0x92, c => c as u8 }).collect() } else { t.encode_utf16().flat_map(|c| c.to_le_bytes()).collect() };
        let n = if *comp { bytes.len() } else { bytes.len() / 2 } as u32; cp += n; cps.push(cp);
        let fc = if *comp { (off as u32 * 2) | 0x4000_0000 } else { off as u32 };
        pcds.extend([0u8, 0]); pcds.extend(fc.to_le_bytes()); pcds.extend([0u8, 0]); wd.extend(&bytes); off += bytes.len();
    }
    let set = |v: &mut Vec<u8>, o: usize, x: u32| v[o..o + 4].copy_from_slice(&x.to_le_bytes());
    wd[0..2].copy_from_slice(&0xA5ECu16.to_le_bytes()); wd[0x0A..0x0C].copy_from_slice(&0x0200u16.to_le_bytes());
    let mut plc: Vec<u8> = cps.iter().flat_map(|c| c.to_le_bytes()).collect(); plc.extend(pcds);
    let mut clx = vec![2u8]; clx.extend((plc.len() as u32).to_le_bytes()); clx.extend(plc);
    set(&mut wd, 0x4C, cp); set(&mut wd, 0x1A2, 0); set(&mut wd, 0x1A6, clx.len() as u32);
    cfb(&[("WordDocument", wd), ("1Table", clx)])
}
#[test] fn doc_pieces_cp1252_tables_fields() {
    let pieces = [(true, "It‘s caf\u{e9}\r"), (false, "한글 문서\r"),
                  (true, "x\u{7}y\u{7}\u{7}"), (true, "see \u{13} HYPERLINK \"http://a\" \u{14}link\u{15} end\r")];
    let d = formats::load("doc", &word(&pieces)).unwrap();
    assert_eq!(d.plain(), "It‘s café\n한글 문서\nx\ty\nsee link end\n");
    assert!(formats::load("doc", b"junk").is_err());
}
/// HWP record: tag 10 bits, level 10 bits, size 12 bits.
fn rec(tag: u32, level: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = if payload.len() < 0xFFF { ((tag | level << 10 | (payload.len() as u32) << 20)).to_le_bytes().to_vec() } else { let mut h = (tag | level << 10 | 0xFFF << 20).to_le_bytes().to_vec(); h.extend((payload.len() as u32).to_le_bytes()); h };
    v.extend(payload); v
}
fn hwp_text(t: &str) -> Vec<u8> { let mut u: Vec<u16> = t.encode_utf16().collect(); u.push(13); u.iter().flat_map(|c| c.to_le_bytes()).collect() }
fn hwp(flags: u32, paras: &[&str]) -> Vec<u8> {
    let mut fh = vec![0u8; 256]; fh[..17].copy_from_slice(b"HWP Document File"); fh[36..40].copy_from_slice(&flags.to_le_bytes());
    let mut sec = vec![]; for t in paras { sec.extend(rec(66, 0, &[0u8; 22])); sec.extend(rec(67, 1, &hwp_text(t))); }
    if flags & 1 != 0 { sec = miniz_oxide::deflate::compress_to_vec(&sec, 6); }
    cfb(&[("FileHeader", fh), ("Section0", sec)])
}
#[test] fn hwp_plain_and_compressed() {
    let mut ctrl: Vec<u16> = vec![2, 0, 0, 0, 0, 0, 0, 2]; ctrl.extend("본문".encode_utf16()); ctrl.push(9); ctrl.extend([0u16; 7]); ctrl.extend("끝".encode_utf16()); ctrl.push(13); // extended control, text, tab, text
    let mut sec = rec(66, 0, &[0u8; 22]); sec.extend(rec(67, 1, &ctrl.iter().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>()));
    let mut fh = vec![0u8; 256]; fh[..17].copy_from_slice(b"HWP Document File");
    assert_eq!(formats::load("hwp", &cfb(&[("FileHeader", fh), ("Section0", sec)])).unwrap().plain(), "본문 끝\n");
    assert_eq!(formats::load("hwp", &hwp(0, &["안녕하세요", "Second 문단"])).unwrap().plain(), "안녕하세요\nSecond 문단\n");
    let long: String = (0..4000u32).map(|i| char::from_u32(0xAC00 + (i.wrapping_mul(2654435761u32) >> 20) % 11000).unwrap()).collect(); // hard to compress -> stays > 4096 bytes
    let d = formats::load("hwp", &hwp(1, &[&long, "tail"])).unwrap(); assert_eq!(d.paras[0].text(), long); assert_eq!(d.paras[1].text(), "tail");
    assert!(formats::load("hwp", &hwp(2, &["x"])).unwrap_err().contains("password"));
    assert!(formats::load("hwp", &hwp(4, &["x"])).unwrap_err().contains("distribution"));
}
