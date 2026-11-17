// SPDX-License-Identifier: GPL-3.0-only
use omnidoc_core::{cmd, edit::*, formats, model::*, zipio};

fn png(w: u32, h: u32) -> Vec<u8> { let mut p = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R']; p.extend(w.to_be_bytes()); p.extend(h.to_be_bytes()); p.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]); p }
fn rich() -> Doc {
    let mut e = Editor::new(Doc::from_plain("Intro")); e.select_all(); e.change_size(4); e.set_color(Some(0xC00000)); e.toggle(Fmt::Strike); e.mv((0, 5), false);
    e.insert("\nafter"); e.insert_table(2, 3); e.insert("a"); e.table_tab(false); e.insert("b*|~"); e.table_tab(false); e.insert("c"); e.table_tab(false); e.insert("d");
    e.insert_image(png(120, 80), "a pic").unwrap(); e.insert("tail"); e.doc
}
#[test] fn run_formatting_lists_indent() {
    let mut e = Editor::new(Doc::from_plain("one\ntwo\nthree")); e.select_all(); e.toggle_list(true); assert_eq!(e.doc.plain(), "\u{2022} one\n\u{2022} two\n\u{2022} three\n");
    e.toggle_list(true); assert_eq!(e.doc.plain(), "one\ntwo\nthree\n"); e.toggle_list(false); assert_eq!(e.doc.plain(), "1. one\n2. two\n3. three\n");
    e.toggle_list(true); assert_eq!(e.doc.plain(), "\u{2022} one\n\u{2022} two\n\u{2022} three\n"); e.indent(1); assert_eq!(e.doc.plain(), "    \u{2022} one\n    \u{2022} two\n    \u{2022} three\n"); e.indent(-1); e.indent(-1);
    assert!(e.doc.plain().starts_with("\u{2022} one"));
    let mut e = Editor::new(Doc::from_plain("size")); e.select_all(); cmd::exec(&mut e, "font.bigger"); cmd::exec(&mut e, "font.bigger"); cmd::exec(&mut e, "color.blue"); cmd::exec(&mut e, "fmt.strike");
    assert_eq!((e.doc.paras[0].runs[0].size, e.doc.paras[0].runs[0].color, e.doc.paras[0].runs[0].strike), (Some(14), Some(0x1F4E9E), true));
    cmd::exec(&mut e, "font.smaller"); cmd::exec(&mut e, "color.auto"); assert_eq!((e.doc.paras[0].runs[0].size, e.doc.paras[0].runs[0].color), (Some(13), None));
}
#[test] fn tables_editing() {
    let mut e = Editor::new(Doc::default()); e.insert_table(2, 2); assert_eq!(e.doc.paras.iter().filter(|p| p.table).count(), 2); assert!(e.doc.paras[0].table, "empty paragraph is replaced");
    e.insert("a"); assert!(e.table_tab(false)); e.insert("b"); assert!(e.table_tab(false)); assert_eq!(e.cur, (1, 0)); e.insert("c"); e.table_tab(false); e.insert("d");
    assert_eq!(e.doc.plain(), "a\tb\nc\td\n\n");
    e.table_tab(false); assert_eq!(e.doc.paras.iter().filter(|p| p.table).count(), 3, "tab in the last cell adds a row");
    e.table_add_col(); assert_eq!(e.doc.paras[0].cells().len(), 3); e.table_del_col(); assert_eq!(e.doc.paras[0].cells().len(), 2);
    e.table_del_row(); assert_eq!(e.doc.paras.iter().filter(|p| p.table).count(), 2); assert!(!Editor::new(Doc::from_plain("t")).table_tab(false));
    let mut e = Editor::new(Doc::default()); e.insert_table(1, 2); e.insert("A"); e.insert("\n"); assert_eq!(e.cur.0, 0); assert_eq!(e.cur.1, 2, "Enter in a table moves to the next cell");
}
#[test] fn omd_docx_odt_md_roundtrip_rich() {
    let d = rich();
    for ext in ["omd", "docx"] { let back = formats::load(ext, &formats::save(ext, &d).unwrap()).unwrap();
        // docx adds an empty trailing paragraph only when a table ends the document; here the document ends with text
        assert_eq!(back, d, "roundtrip {ext}"); }
    let odt = formats::load("odt", &formats::save("odt", &d).unwrap()).unwrap();
    assert_eq!(odt.paras.iter().filter(|p| p.table).map(|p| p.text()).collect::<Vec<_>>(), ["a\tb*|~\tc".to_string(), "d\t\t".to_string()]);
    let md = String::from_utf8(formats::save("md", &d).unwrap()).unwrap();
    assert!(md.contains("| a | b\\*\\|\\~ | c |") && md.contains("|---|---|---|") && md.contains("![a pic](image)"), "{md}");
    let back = formats::load("md", md.as_bytes()).unwrap(); assert_eq!(back.paras.iter().filter(|p| p.table).count(), 2);
}
#[test] fn docx_image_and_run_properties_in_xml() {
    let b = formats::save("docx", &rich()).unwrap();
    let doc = String::from_utf8(zipio::read(&b, "word/document.xml").unwrap().unwrap()).unwrap();
    for needle in ["<w:strike/>", "<w:sz w:val=\"32\"/>", "<w:color w:val=\"C00000\"/>", "<w:tbl>", "<w:gridCol", "<wp:extent cx=\"1143000\" cy=\"762000\"/>", "r:embed=\"rIdImg1\""] { assert!(doc.contains(needle), "{needle}"); }
    assert_eq!(zipio::read(&b, "word/media/img1.png").unwrap().unwrap().len(), png(120, 80).len());
    assert!(String::from_utf8(zipio::read(&b, "word/_rels/document.xml.rels").unwrap().unwrap()).unwrap().contains("media/img1.png"));
    assert!(formats::load("docx", &b).unwrap().paras.iter().any(|p| p.image.as_ref().map_or(false, |i| (i.w, i.h) == (120, 80))));
    assert!(Editor::new(Doc::default()).insert_image(b"not an image".to_vec(), "x").is_err());
}
