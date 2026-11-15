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
