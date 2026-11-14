// SPDX-License-Identifier: GPL-3.0-only
use omnidoc_core::{cmd, edit::*, formats, model::*};

fn ed(s: &str) -> Editor { Editor::new(Doc::from_plain(s)) }

#[test] fn change_case_and_painter() {
    let mut e = ed("hello wORLD. next one");
    e.select_all(); e.change_case(Case::Title); assert_eq!(e.doc.plain(), "Hello World. Next One\n");
    e.change_case(Case::Sentence); assert_eq!(e.doc.plain(), "Hello world. Next one\n");
    e.mv((0, 2), false); e.change_case(Case::Upper); assert_eq!(e.doc.plain(), "HELLO world. Next one\n"); // word under cursor
    let mut e = ed("bold\nplain"); e.mv((0, 0), false); e.mv((0, 4), true); e.toggle(Fmt::Bold); e.set_style(Style::H2);
    cmd::exec(&mut e, "fmt.painter"); e.mv((1, 0), false); e.mv((1, 5), true); cmd::exec(&mut e, "fmt.painter");
    assert!(e.doc.paras[1].runs[0].bold && e.doc.paras[1].style == Style::H2);
}
#[test] fn sort_autocorrect_completion() {
    let mut e = ed("pear\nApple\nfig"); e.sort_paras(false); assert_eq!(e.doc.plain(), "Apple\nfig\npear\n");
    e.sort_paras(true); assert_eq!(e.doc.plain(), "pear\nfig\nApple\n");
    let mut e = ed(""); e.insert("teh"); assert!(e.autocorrect()); e.insert(" (c)"); assert!(e.autocorrect()); e.insert(" i"); assert!(e.autocorrect());
    assert_eq!(e.doc.plain(), "The © I\n"); e.undo(); assert_eq!(e.doc.plain(), "The © i\n");
    let mut e = ed("internationalization internal\ninter"); e.mv((1, 5), false); assert_eq!(e.completion().as_deref(), Some("nal")); // most frequent? tie -> shorter
    assert!(e.accept_completion()); assert_eq!(e.doc.paras[1].text(), "internal");
}
#[test] fn regex_and_stats() {
    let mut e = ed("a1 b22\nc333");
    assert_eq!(e.replace_all_regex(r"(\w)(\d+)", "$2$1", true).unwrap(), 3); assert_eq!(e.doc.plain(), "1a 22b\n333c\n");
    e.mv((0, 0), false); assert!(e.find_next_regex(r"\d{3}", true).unwrap()); assert_eq!(e.range(), Some(((1, 0), (1, 3))));
    assert!(e.find_next_regex("(", true).is_err());
    let s = ed("One. Two! Three?").stats(); assert_eq!((s.words, s.sentences, s.paras, s.pages), (3, 3, 1, 1));
    assert!(cmd::context_menu(true).contains(&"edit.cut") && !cmd::context_menu(false).contains(&"edit.cut"));
}
fn mk_doc_file(text: &str) -> Vec<u8> {
    let u: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect(); let n = (u.len() / 2) as u32;
    let mut wd = vec![0u8; 4096]; wd[0..2].copy_from_slice(&0xA5ECu16.to_le_bytes()); wd[0x0A..0x0C].copy_from_slice(&0x0200u16.to_le_bytes());
    wd[0x4C..0x50].copy_from_slice(&n.to_le_bytes()); wd[0x1A2..0x1A6].copy_from_slice(&0u32.to_le_bytes()); wd[0x1A6..0x1AA].copy_from_slice(&21u32.to_le_bytes()); // CLX length (table offset 0)
    wd[0x800..0x800 + u.len()].copy_from_slice(&u);
    let mut tb = vec![0u8; 4096]; tb[0] = 2; tb[1..5].copy_from_slice(&16u32.to_le_bytes());
    tb[5..9].copy_from_slice(&0u32.to_le_bytes()); tb[9..13].copy_from_slice(&n.to_le_bytes()); tb[15..19].copy_from_slice(&0x800u32.to_le_bytes());
    let mut f = vec![0xFFu8; 512 * 19]; f[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let put = |f: &mut Vec<u8>, o: usize, v: u32| f[o..o + 4].copy_from_slice(&v.to_le_bytes());
    f[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes()); f[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    put(&mut f, 0x2C, 1); put(&mut f, 0x30, 1); put(&mut f, 0x38, 4096); put(&mut f, 0x3C, 0xFFFF_FFFE); put(&mut f, 0x40, 0); put(&mut f, 0x44, 0xFFFF_FFFE); put(&mut f, 0x48, 0); put(&mut f, 0x4C, 0);
    let fat: Vec<u32> = (0..128).map(|i| match i { 0 => 0xFFFF_FFFD, 1 | 9 | 17 => 0xFFFF_FFFE, 2..=8 | 10..=16 => i + 1, _ => 0xFFFF_FFFF }).collect();
    for (i, v) in fat.iter().enumerate() { put(&mut f, 512 + i * 4, *v); }
    f[1024..1536].fill(0);
    for (k, (name, ty, start, size)) in [("Root Entry", 5u8, 0xFFFF_FFFEu32, 0u32), ("WordDocument", 2, 2, 4096), ("1Table", 2, 10, 4096)].iter().enumerate() {
        let e = 1024 + k * 128; for (j, c) in name.encode_utf16().enumerate() { f[e + j * 2..e + j * 2 + 2].copy_from_slice(&c.to_le_bytes()); }
        f[e + 0x40..e + 0x42].copy_from_slice(&(((name.len() + 1) * 2) as u16).to_le_bytes()); f[e + 0x42] = *ty; put(&mut f, e + 0x74, *start); put(&mut f, e + 0x78, *size);
    }
    f[1536..1536 + 4096].copy_from_slice(&wd); f[1536 + 4096..1536 + 8192].copy_from_slice(&tb); f
}
#[test] fn doc_import_synthetic() {
    let d = formats::load("doc", &mk_doc_file("Hello\rWorld 한글\r")).unwrap();
    assert_eq!(d.plain(), "Hello\nWorld 한글\n");
    assert!(formats::load("doc", b"not a doc").is_err());
}
