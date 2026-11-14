// SPDX-License-Identifier: GPL-3.0-only
//! Command registry shared by the ribbon, keyboard shortcuts and the command palette.
use crate::edit::{Case, Editor, Fmt};
use crate::model::{Align, Style};
pub struct Command { pub id: &'static str, pub title: &'static str, pub tab: &'static str, pub group: &'static str, pub keys: &'static str }
const fn c(id: &'static str, title: &'static str, tab: &'static str, group: &'static str, keys: &'static str) -> Command { Command { id, title, tab, group, keys } }
pub const COMMANDS: &[Command] = &[
    c("file.new", "New", "File", "File", "Ctrl+N"), c("file.open", "Open", "File", "File", "Ctrl+O"),
    c("file.save", "Save", "File", "File", "Ctrl+S"), c("file.saveas", "Save As", "File", "File", "Ctrl+Shift+S"),
    c("edit.undo", "Undo", "Home", "Undo", "Ctrl+Z"), c("edit.redo", "Redo", "Home", "Undo", "Ctrl+Y"),
    c("edit.cut", "Cut", "Home", "Clipboard", "Ctrl+X"), c("edit.copy", "Copy", "Home", "Clipboard", "Ctrl+C"), c("edit.paste", "Paste", "Home", "Clipboard", "Ctrl+V"),
    c("fmt.painter", "Format Painter", "Home", "Clipboard", "Ctrl+Shift+C"),
    c("case.upper", "UPPERCASE", "Home", "Case", ""), c("case.lower", "lowercase", "Home", "Case", ""), c("case.title", "Title Case", "Home", "Case", ""), c("case.sentence", "Sentence case", "Home", "Case", ""),
    c("edit.sort", "Sort A-Z", "Home", "Paragraph", ""), c("edit.sortdesc", "Sort Z-A", "Home", "Paragraph", ""), c("view.marks", "Show Marks", "Home", "Paragraph", "Ctrl+Shift+*"),
    c("tools.stats", "Statistics", "Review", "Proofing", "Ctrl+Shift+G"), c("tools.autocorrect", "AutoCorrect", "Review", "Proofing", ""), c("tools.complete", "Word Completion", "Review", "Proofing", ""),
    c("find.regex", "Regex Find", "Review", "Find", "Alt+R"),
    c("fmt.bold", "Bold", "Home", "Font", "Ctrl+B"), c("fmt.italic", "Italic", "Home", "Font", "Ctrl+I"),
    c("fmt.underline", "Underline", "Home", "Font", "Ctrl+U"),
    c("align.left", "Align Left", "Home", "Paragraph", "Ctrl+L"), c("align.center", "Center", "Home", "Paragraph", "Ctrl+E"),
    c("align.right", "Align Right", "Home", "Paragraph", "Ctrl+R"), c("align.justify", "Justify", "Home", "Paragraph", "Ctrl+J"),
    c("style.normal", "Normal", "Home", "Styles", "Ctrl+Alt+0"), c("style.title", "Title", "Home", "Styles", ""),
    c("style.h1", "Heading 1", "Home", "Styles", "Ctrl+Alt+1"), c("style.h2", "Heading 2", "Home", "Styles", "Ctrl+Alt+2"),
    c("style.h3", "Heading 3", "Home", "Styles", "Ctrl+Alt+3"), c("style.quote", "Quote", "Home", "Styles", ""), c("style.code", "Code", "Home", "Styles", ""),
    c("edit.selectall", "Select All", "Home", "Editing", "Ctrl+A"), c("find.open", "Find and Replace", "Home", "Editing", "Ctrl+F"),
    c("view.outline", "Navigation Pane", "View", "Show", "F9"), c("view.focus", "Focus Mode", "View", "Show", "F11"),
    c("view.dark", "Dark Mode", "View", "Show", "Ctrl+Shift+D"), c("view.zoomin", "Zoom In", "View", "Zoom", "Ctrl+="),
    c("view.zoomout", "Zoom Out", "View", "Zoom", "Ctrl+-"), c("view.ribbon", "Collapse Ribbon", "View", "Show", "Ctrl+F1"),
    c("palette.open", "Command Palette", "View", "Show", "Ctrl+Shift+P"),
];
/// Fuzzy (subsequence) search over command titles, best match first.
pub fn search(q: &str) -> Vec<&'static Command> {
    let q: Vec<char> = q.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    let mut v: Vec<(usize, &Command)> = COMMANDS.iter().filter_map(|cm| {
        let t: Vec<char> = cm.title.to_lowercase().chars().collect(); let (mut i, mut gap, mut last) = (0, 0, 0);
        for (k, ch) in t.iter().enumerate() { if i < q.len() && *ch == q[i] { if i > 0 { gap += k - last - 1; } last = k; i += 1; } }
        (i == q.len()).then_some((gap + t.len() / 4, cm))
    }).collect();
    v.sort_by_key(|x| x.0); v.into_iter().map(|x| x.1).collect()
}
/// Execute an editing command. Returns false for UI-level commands (file.*, view.*, find.*).
pub fn exec(e: &mut Editor, id: &str) -> bool {
    match id {
        "fmt.painter" => match e.painter.take() { Some(p) => e.paint_format(&p), None => e.painter = Some(e.copy_format()) },
        "case.upper" => e.change_case(Case::Upper), "case.lower" => e.change_case(Case::Lower), "case.title" => e.change_case(Case::Title), "case.sentence" => e.change_case(Case::Sentence),
        "edit.sort" => e.sort_paras(false), "edit.sortdesc" => e.sort_paras(true),
        "tools.autocorrect" => e.autocorrect_on = !e.autocorrect_on, "tools.complete" => e.completion_on = !e.completion_on,
        "edit.undo" => e.undo(), "edit.redo" => e.redo(), "edit.selectall" => e.select_all(),
        "fmt.bold" => e.toggle(Fmt::Bold), "fmt.italic" => e.toggle(Fmt::Italic), "fmt.underline" => e.toggle(Fmt::Underline),
        "align.left" => e.set_align(Align::Left), "align.center" => e.set_align(Align::Center),
        "align.right" => e.set_align(Align::Right), "align.justify" => e.set_align(Align::Justify),
        "style.normal" => e.set_style(Style::Normal), "style.title" => e.set_style(Style::Title), "style.h1" => e.set_style(Style::H1),
        "style.h2" => e.set_style(Style::H2), "style.h3" => e.set_style(Style::H3), "style.quote" => e.set_style(Style::Quote), "style.code" => e.set_style(Style::Code),
        _ => return false,
    }
    true
}

/// Right-click menu entries ("-" is a separator).
pub fn context_menu(has_selection: bool) -> Vec<&'static str> {
    if has_selection { vec!["edit.cut", "edit.copy", "edit.paste", "-", "fmt.bold", "fmt.italic", "fmt.underline", "-", "case.upper", "case.lower", "case.title", "-", "fmt.painter", "edit.selectall"] }
    else { vec!["edit.paste", "edit.selectall", "-", "style.h1", "style.h2", "style.normal", "-", "fmt.painter", "find.open"] }
}
