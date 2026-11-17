// SPDX-License-Identifier: GPL-3.0-only
//! Format dispatch plus readers/writers: OMD, Markdown, TXT-like, RTF, DOCX, ODT/SXW, HWPX, HML, ABW.
use crate::model::*;
use crate::xml::{self, attr, esc, Ev};
use crate::zipio;
use std::collections::HashMap;

/// (extension, name, can_read, can_write)
pub const FORMATS: &[(&str, &str, bool, bool)] = &[
    ("omd", "OmniDoc", true, true), ("md", "Markdown", true, true), ("txt", "Plain Text", true, true),
    ("html", "HTML (source)", true, true), ("xhtml", "XHTML (source)", true, true), ("xml", "XML (source)", true, true),
    ("tex", "LaTeX (source)", true, true), ("rst", "reStructuredText (source)", true, true), ("org", "Org (source)", true, true),
    ("typ", "Typst (source)", true, true), ("wiki", "MediaWiki (source)", true, true), ("adoc", "AsciiDoc (source)", true, true),
    ("sgml", "SGML (source)", true, true), ("gml", "GML (source)", true, true),
    ("rtf", "Rich Text Format", true, true), ("docx", "Word", true, true), ("odt", "OpenDocument Text", true, true),
    ("docm", "Word (macro)", true, false), ("sxw", "OpenOffice 1.x", true, false), ("hwpx", "Hancom OWPML", true, false),
    ("hml", "Hancom Markup", true, false), ("abw", "AbiWord", true, false),
    ("hwp", "Hancom HWP", true, false), ("doc", "Word 97-2003", true, false), ("pdf", "PDF", false, false),
    ("pages", "Apple Pages", false, false), ("wpd", "WordPerfect", false, false), ("wps", "MS Works/WPS", false, false),
    ("lwp", "Lotus Word Pro", false, false), ("wri", "Windows Write", false, false), ("cwk", "ClarisWorks", false, false),
    ("uof", "UOF", false, false), ("gul", "JungUm GUL", false, false), ("jung", "JungUm", false, false),
];
const SOURCE: &[&str] = &["txt", "html", "xhtml", "xml", "tex", "rst", "org", "typ", "wiki", "adoc", "sgml", "gml"];

pub fn load(ext: &str, d: &[u8]) -> Result<Doc, String> {
    let e = ext.to_lowercase();
    let text = || String::from_utf8_lossy(d.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(d)).into_owned();
    match e.as_str() {
        x if SOURCE.contains(&x) => Ok(Doc::from_plain(&text())),
        "md" => Ok(read_md(&text(), false, None)),
        "omd" => { let c = zipio::read(d, "content.md")?.ok_or("OMD: content.md missing")?; let media = |n: &str| zipio::read(d, n).ok().flatten(); Ok(read_md(&String::from_utf8_lossy(&c), true, Some(&media))) }
        "rtf" => Ok(read_rtf(d)),
        "docx" | "docm" => crate::docx::read(d),
        "doc" => crate::legacy::read_doc(d),
        "hwp" => crate::legacy::read_hwp(d),
        "odt" | "sxw" => read_odf(d),
        "hwpx" => {
            let mut all = vec![];
            for i in 0..100 { match zipio::read(d, &format!("Contents/section{i}.xml"))? { Some(x) => all.extend(xml_paras(&String::from_utf8_lossy(&x), "p", "t")), None => break } }
            if all.is_empty() { return Err("HWPX: no sections found".into()); }
            Ok(Doc { paras: all })
        }
        "hml" => Ok(Doc { paras: xml_paras(&text(), "P", "CHAR") }),
        "abw" => Ok(Doc { paras: xml_paras(&text(), "p", "") }),
        _ => Err(format!("No reader for .{e} yet (see docs/project/roadmap.md)")),
    }
}
pub fn save(ext: &str, doc: &Doc) -> Result<Vec<u8>, String> {
    let e = ext.to_lowercase();
    match e.as_str() {
        x if SOURCE.contains(&x) => Ok(doc.plain().into_bytes()),
        "md" => Ok(write_md(doc, false).0.into_bytes()),
        "omd" => {
            let mut z = zipio::Writer::new();
            z.add("mimetype", b"application/vnd.omnioffice.omd+zip");
            z.add("manifest.json", format!("{{\"omd\":3,\"generator\":\"OmniDoc {}\"}}", crate::VERSION).as_bytes());
            let (md, media) = write_md(doc, true);
            z.add("content.md", md.as_bytes()); for (n, b) in media { z.add(&n, &b); }
            Ok(z.finish())
        }
        "rtf" => Ok(write_rtf(doc).into_bytes()),
        "docx" => Ok(crate::docx::write(doc)),
        "odt" => Ok(write_odt(doc)),
        _ => Err(format!("Saving .{e} is not supported")),
    }
}

// ---------- Markdown / OMD (OMD adds {c} {r} {j} alignment prefixes, <span s= c=> and media/ images) ----------
type Media<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;
#[derive(Clone, Default)]
struct F { b: bool, i: bool, u: bool, s: bool, size: Option<u8>, color: Option<u32> }
fn push_run(runs: &mut Vec<Run>, cur: &mut String, f: &F) { if !cur.is_empty() { runs.push(Run { text: std::mem::take(cur), bold: f.b, italic: f.i, underline: f.u, strike: f.s, size: f.size, color: f.color }); } }
fn inline(s: &str) -> Vec<Run> {
    let ch: Vec<char> = s.chars().collect();
    let (mut runs, mut cur, mut f, mut k) = (vec![], String::new(), F::default(), 0);
    let at = |k: usize, t: &str| ch[k..].iter().take(t.chars().count()).collect::<String>() == t;
    while k < ch.len() {
        if at(k, "**") { push_run(&mut runs, &mut cur, &f); f.b = !f.b; k += 2 }
        else if at(k, "~~") { push_run(&mut runs, &mut cur, &f); f.s = !f.s; k += 2 }
        else if ch[k] == '*' { push_run(&mut runs, &mut cur, &f); f.i = !f.i; k += 1 }
        else if at(k, "<u>") { push_run(&mut runs, &mut cur, &f); f.u = true; k += 3 }
        else if at(k, "</u>") { push_run(&mut runs, &mut cur, &f); f.u = false; k += 4 }
        else if at(k, "<span ") {
            push_run(&mut runs, &mut cur, &f); let end = ch[k..].iter().position(|c| *c == '>').map_or(ch.len() - 1, |p| k + p);
            for part in ch[k + 6..end].iter().collect::<String>().split_whitespace() {
                if let Some(v) = part.strip_prefix("s=") { f.size = v.trim_matches('"').parse().ok(); } else if let Some(v) = part.strip_prefix("c=") { f.color = u32::from_str_radix(v.trim_matches('"'), 16).ok(); } }
            k = end + 1;
        }
        else if at(k, "</span>") { push_run(&mut runs, &mut cur, &f); f.size = None; f.color = None; k += 7 }
        else if ch[k] == '\\' && k + 1 < ch.len() { cur.push(ch[k + 1]); k += 2 }
        else { cur.push(ch[k]); k += 1 }
    }
    push_run(&mut runs, &mut cur, &f); runs
}
fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim(); let t = t.strip_prefix('|').unwrap_or(t); let t = t.strip_suffix('|').unwrap_or(t);
    let (mut out, mut cur, mut esc) = (vec![], String::new(), false);
    for c in t.chars() { if esc { cur.push(c); esc = false; } else if c == '\\' { cur.push(c); esc = true; } else if c == '|' { out.push(cur.trim().to_string()); cur.clear(); } else { cur.push(c); } }
    out.push(cur.trim().to_string()); out
}
fn read_md(s: &str, omd: bool, media: Option<Media>) -> Doc {
    let mut paras = vec![];
    let src = s.replace("\r\n", "\n"); let lines: Vec<&str> = src.lines().collect();
    for line in lines {
        if line.trim().is_empty() { continue; }
        let mut l = line; let mut align = Align::Left;
        if omd { for (t, a) in [("{c}", Align::Center), ("{r}", Align::Right), ("{j}", Align::Justify)] { if let Some(x) = l.strip_prefix(t) { l = x; align = a; } } }
        let tl = l.trim();
        if tl.len() > 1 && tl.starts_with('|') && tl.ends_with('|') {
            let cells = split_cells(tl);
            if cells.iter().all(|c| !c.is_empty() && c.chars().all(|x| x == '-' || x == ':')) { continue; } // header separator
            let mut p = Para { table: true, ..Default::default() };
            for (i, c) in cells.iter().enumerate() { if i > 0 { p.runs.push(Run::plain("\t")); } p.runs.extend(inline(c)); }
            p.normalize(); paras.push(p); continue;
        }
        if let Some(rest) = tl.strip_prefix("![") {
            if let Some((alt, path)) = rest.split_once("](").and_then(|(a, p)| p.strip_suffix(')').map(|p| (a, p))) {
                match media.and_then(|m| m(path)) {
                    Some(d) => { let (w, h) = omni_common::image::size(&d).unwrap_or((0, 0)); paras.push(Para { align, image: Some(Image { data: std::sync::Arc::new(d), w, h, alt: alt.to_string() }), ..Default::default() }); }
                    None => paras.push(Para::plain(&format!("[image: {alt}]"))),
                }
                continue;
            }
        }
        let (style, body) = if let Some(x) = l.strip_prefix("### ") { (Style::H3, x) } else if let Some(x) = l.strip_prefix("## ") { (Style::H2, x) }
            else if let Some(x) = l.strip_prefix("# ") { (Style::H1, x) } else if let Some(x) = l.strip_prefix("% ") { (Style::Title, x) }
            else if let Some(x) = l.strip_prefix("> ") { (Style::Quote, x) } else if let Some(x) = l.strip_prefix("    ") { (Style::Code, x) } else { (Style::Normal, l) };
        let mut p = Para { style, align, runs: inline(body), ..Default::default() }; p.normalize(); paras.push(p);
    }
    if paras.is_empty() { paras.push(Para::default()); }
    Doc { paras }
}
fn runs_md(runs: &[Run], omd: bool, cell: bool) -> String {
    let mut o = String::new();
    for r in runs {
        let mut t = r.text.replace('\\', "\\\\").replace('*', "\\*").replace('~', "\\~").replace('<', "\\<"); if cell { t = t.replace('|', "\\|"); }
        // Plain Markdown needs whitespace outside the markers; OMD keeps runs exact.
        let (core, lead, trail) = if omd { (t.as_str(), "", "") } else { (t.trim(), &t[..t.len() - t.trim_start().len()], &t[t.trim_end().len()..]) };
        o.push_str(lead);
        if !core.is_empty() {
            let span = omd && (r.size.is_some() || r.color.is_some());
            if span { o.push_str("<span"); if let Some(s) = r.size { o.push_str(&format!(" s={s}")); } if let Some(c) = r.color { o.push_str(&format!(" c={c:06X}")); } o.push('>'); }
            if r.underline { o.push_str("<u>"); } if r.bold { o.push_str("**"); } if r.strike { o.push_str("~~"); } if r.italic { o.push('*'); }
            o.push_str(core);
            if r.italic { o.push('*'); } if r.strike { o.push_str("~~"); } if r.bold { o.push_str("**"); } if r.underline { o.push_str("</u>"); }
            if span { o.push_str("</span>"); }
        }
        o.push_str(if core.is_empty() { "" } else { trail });
    }
    o
}
/// Markdown text plus the media files it references (only OMD carries media).
fn write_md(d: &Doc, omd: bool) -> (String, Vec<(String, std::sync::Arc<Vec<u8>>)>) {
    let (mut o, mut media) = (String::new(), vec![]);
    let mut prev_table = false;
    for p in &d.paras {
        if omd { o.push_str(match p.align { Align::Center => "{c}", Align::Right => "{r}", Align::Justify => "{j}", _ => "" }); }
        if p.table {
            let cells = p.cells(); o.push('|'); for c in &cells { o.push(' '); o.push_str(&runs_md(c, omd, true)); o.push_str(" |"); } o.push('\n');
            if !prev_table { o.push('|'); for _ in &cells { o.push_str("---|"); } o.push('\n'); }
            prev_table = true; continue;
        }
        if prev_table { o.push('\n'); prev_table = false; }
        if let Some(img) = &p.image {
            if omd { let name = format!("media/img{}.{}", media.len() + 1, omni_common::image::ext(&img.data).unwrap_or("bin")); o.push_str(&format!("![{}]({})\n\n", img.alt.replace(']', ""), name)); media.push((name, img.data.clone())); }
            else { o.push_str(&format!("![{}](image)\n\n", img.alt.replace(']', ""))); }
            continue;
        }
        o.push_str(match p.style { Style::H1 => "# ", Style::H2 => "## ", Style::H3 => "### ", Style::Title => "% ", Style::Quote => "> ", Style::Code => "    ", _ => "" });
        o.push_str(&runs_md(&p.runs, omd, false)); o.push_str("\n\n");
    }
    if prev_table { o.push('\n'); }
    (o, media)
}

// ---------- RTF ----------
fn read_rtf(b: &[u8]) -> Doc {
    #[derive(Clone, Default)] struct S { bold: bool, italic: bool, ul: bool, skip: bool }
    let (mut st, mut stack, mut paras, mut cur, mut i) = (S::default(), vec![], vec![], Para::default(), 0);
    let mut skip_next = 0usize; let mut text = String::new();
    macro_rules! flush { () => { if !text.is_empty() { cur.runs.push(Run { text: std::mem::take(&mut text), bold: st.bold, italic: st.italic, underline: st.ul, ..Default::default() }); } } }
    while i < b.len() {
        match b[i] {
            b'{' => { flush!(); stack.push(st.clone()); i += 1 }
            b'}' => { flush!(); st = stack.pop().unwrap_or_default(); i += 1 }
            b'\\' => {
                i += 1; if i >= b.len() { break; }
                let c = b[i];
                if matches!(c, b'\\' | b'{' | b'}') { if !st.skip { text.push(c as char); } i += 1; }
                else if c == b'*' { st.skip = true; i += 1 }
                else if c == b'\'' { let h = std::str::from_utf8(b.get(i + 1..i + 3).unwrap_or(b"20")).ok().and_then(|x| u8::from_str_radix(x, 16).ok()).unwrap_or(b'?'); if !st.skip && skip_next == 0 { text.push(h as char); } skip_next = skip_next.saturating_sub(1); i += 3 }
                else if c.is_ascii_alphabetic() {
                    let s = i; while i < b.len() && b[i].is_ascii_alphabetic() { i += 1; }
                    let w = std::str::from_utf8(&b[s..i]).unwrap_or(""); let ns = i;
                    if i < b.len() && (b[i] == b'-' || b[i].is_ascii_digit()) { i += 1; while i < b.len() && b[i].is_ascii_digit() { i += 1; } }
                    let n: i32 = std::str::from_utf8(&b[ns..i]).ok().and_then(|x| x.parse().ok()).unwrap_or(1);
                    if i < b.len() && b[i] == b' ' { i += 1; }
                    match w {
                        "par" | "line" => { flush!(); cur.normalize(); paras.push(std::mem::take(&mut cur)); }
                        "tab" => text.push(' '),
                        "b" => { flush!(); st.bold = n != 0 } "i" => { flush!(); st.italic = n != 0 } "ul" => { flush!(); st.ul = n != 0 } "ulnone" => { flush!(); st.ul = false }
                        "plain" => { flush!(); st.bold = false; st.italic = false; st.ul = false }
                        "u" => { if !st.skip { text.push(char::from_u32((n as i16 as u16) as u32).unwrap_or('?')); } skip_next = 1; }
                        "fonttbl" | "colortbl" | "stylesheet" | "info" | "pict" | "header" | "footer" | "listtable" => st.skip = true,
                        _ => {}
                    }
                } else { i += 1 }
            }
            b'\r' | b'\n' => i += 1,
            c => { if skip_next > 0 { skip_next -= 1 } else if !st.skip { let l = std::str::from_utf8(&b[i..]).ok().and_then(|s| s.chars().next()).map_or(1, |c| c.len_utf8()); if c < 0x80 { text.push(c as char) } else { text.push(String::from_utf8_lossy(&b[i..i + l]).chars().next().unwrap_or('?')) }; i += l.max(1); continue; } i += 1 }
        }
    }
    flush!(); cur.normalize(); if !cur.runs.is_empty() || paras.is_empty() { paras.push(cur); }
    Doc { paras }
}
fn write_rtf(d: &Doc) -> String {
    let mut o = String::from("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Arial;}}\n");
    for p in &d.paras {
        o.push_str(match p.align { Align::Center => "\\qc", Align::Right => "\\qr", Align::Justify => "\\qj", _ => "\\ql" });
        o.push_str(match p.style { Style::Title => "{\\fs44\\b ", Style::H1 => "{\\fs36\\b ", Style::H2 => "{\\fs30\\b ", Style::H3 => "{\\fs26\\b ", Style::Code => "{\\fs20 ", _ => "{\\fs24 " });
        for r in &p.runs {
            o.push_str(&format!("{{{}{}{}", if r.bold { "\\b " } else { "" }, if r.italic { "\\i " } else { "" }, if r.underline { "\\ul " } else { "" }));
            for c in r.text.chars() { match c { '\\' | '{' | '}' => { o.push('\\'); o.push(c) } c if (c as u32) < 128 => o.push(c), c => { let mut u = [0u16; 2]; for x in c.encode_utf16(&mut u) { o.push_str(&format!("\\u{}?", *x as i16)); } } } }
            o.push('}');
        }
        o.push_str("}\\par\n");
    }
    o + "}"
}

// ---------- ODT / SXW ----------
fn read_odf(d: &[u8]) -> Result<Doc, String> {
    let x = zipio::read(d, "content.xml")?.ok_or("ODF: content.xml missing")?;
    let evs = xml::parse(&String::from_utf8_lossy(&x));
    let (mut tf, mut pa): (HashMap<String, (bool, bool, bool)>, HashMap<String, Align>) = Default::default();
    let mut cs = String::new();
    for ev in &evs { if let Ev::Start(n, a) = ev { match n.as_str() {
        "style" => cs = attr(a, "name").unwrap_or("").into(),
        "text-properties" => tf.insert(cs.clone(), (attr(a, "font-weight") == Some("bold"), attr(a, "font-style") == Some("italic"), attr(a, "text-underline-style").map_or(false, |v| v != "none"))).map(|_| ()).unwrap_or(()),
        "paragraph-properties" => { if let Some(t) = attr(a, "text-align") { pa.insert(cs.clone(), match t { "center" => Align::Center, "end" | "right" => Align::Right, "justify" => Align::Justify, _ => Align::Left }); } }
        _ => {} } } }
    let (mut paras, mut p, mut fs, mut inp) = (vec![], Para::default(), vec![(false, false, false)], false);
    let (mut row, mut cell, mut in_cell) = (Vec::<Vec<Run>>::new(), Vec::<Run>::new(), false);
    for ev in evs { match ev {
        Ev::Start(n, a) => match n.as_str() {
            "p" | "h" => { inp = true; p = Para::default(); let sn = attr(&a, "style-name").unwrap_or("");
                p.align = pa.get(sn).copied().unwrap_or_default();
                p.style = if n == "h" { Style::from_level(attr(&a, "outline-level").and_then(|v| v.parse().ok()).unwrap_or(1)) } else { match sn { "Title" => Style::Title, "Quotations" => Style::Quote, "Preformatted_20_Text" => Style::Code, _ => Style::Normal } }; }
            "span" => fs.push(tf.get(attr(&a, "style-name").unwrap_or("")).copied().unwrap_or(*fs.last().unwrap())),
            "tab" | "s" if inp => p.runs.push(Run::plain(" ")),
            "table-row" => row.clear(), "table-cell" => { in_cell = true; cell.clear(); } _ => {} },
        Ev::End(n) => match n.as_str() { "span" => { fs.pop(); }
            "p" | "h" if inp => { inp = false; p.normalize(); if in_cell { if !cell.is_empty() { cell.push(Run::plain(" ")); } cell.extend(std::mem::take(&mut p.runs)); } else { paras.push(std::mem::take(&mut p)); } }
            "table-cell" => { in_cell = false; row.push(std::mem::take(&mut cell)); }
            "table-row" => { let mut t = Para { table: true, ..Default::default() }; for (i, c) in row.drain(..).enumerate() { if i > 0 { t.runs.push(Run::plain("\t")); } t.runs.extend(c); } t.normalize(); paras.push(t); }
            _ => {} },
        Ev::Text(t) => if inp { let f = *fs.last().unwrap(); p.runs.push(Run { text: t, bold: f.0, italic: f.1, underline: f.2, ..Default::default() }); } } }
    if paras.is_empty() { paras.push(Para::default()); }
    Ok(Doc { paras })
}
fn write_odt(d: &Doc) -> Vec<u8> {
    let mut st = String::new();
    for m in 1..8u8 { st.push_str(&format!("<style:style style:name=\"T{m}\" style:family=\"text\"><style:text-properties{}{}{}/></style:style>", if m & 1 != 0 { " fo:font-weight=\"bold\"" } else { "" }, if m & 2 != 0 { " fo:font-style=\"italic\"" } else { "" }, if m & 4 != 0 { " style:text-underline-style=\"solid\"" } else { "" })); }
    for (n, a) in [("Pc", "center"), ("Pr", "end"), ("Pj", "justify")] { st.push_str(&format!("<style:style style:name=\"{n}\" style:family=\"paragraph\"><style:paragraph-properties fo:text-align=\"{a}\"/></style:style>")); }
    let spans = |runs: &[Run]| -> String { runs.iter().map(|r| { let m = r.bold as u8 | (r.italic as u8) << 1 | (r.underline as u8) << 2; if m == 0 { esc(&r.text) } else { format!("<text:span text:style-name=\"T{m}\">{}</text:span>", esc(&r.text)) } }).collect() };
    let mut body = String::new(); let mut prev_table = false;
    for (idx, p) in d.paras.iter().enumerate() {
        if p.table {
            let cells = p.cells();
            if !prev_table { let n = d.paras[idx..].iter().take_while(|q| q.table).map(|q| q.cells().len()).max().unwrap_or(1); body.push_str(&format!("<table:table table:name=\"Table{idx}\"><table:table-column table:number-columns-repeated=\"{n}\"/>")); }
            body.push_str("<table:table-row>"); for c in &cells { body.push_str(&format!("<table:table-cell office:value-type=\"string\"><text:p>{}</text:p></table:table-cell>", spans(c))); } body.push_str("</table:table-row>");
            prev_table = true; if !d.paras.get(idx + 1).map_or(false, |q| q.table) { body.push_str("</table:table>"); prev_table = false; } continue;
        }
        let sn = match p.align { Align::Center => "Pc", Align::Right => "Pr", Align::Justify => "Pj", _ => match p.style { Style::Title => "Title", Style::Quote => "Quotations", Style::Code => "Preformatted_20_Text", _ => "" } };
        let (open, close) = if p.style.level() > 0 { (format!("<text:h text:outline-level=\"{}\">", p.style.level()), "</text:h>") } else { (if sn.is_empty() { "<text:p>".into() } else { format!("<text:p text:style-name=\"{sn}\">") }, "</text:p>") };
        body.push_str(&open);
        for r in &p.runs { let m = r.bold as u8 | (r.italic as u8) << 1 | (r.underline as u8) << 2; if m == 0 { body.push_str(&esc(&r.text)) } else { body.push_str(&format!("<text:span text:style-name=\"T{m}\">{}</text:span>", esc(&r.text))) } }
        body.push_str(close);
    }
    let mut z = zipio::Writer::new();
    z.add("mimetype", b"application/vnd.oasis.opendocument.text");
    z.add("META-INF/manifest.xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>");
    z.add("content.xml", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" office:version=\"1.2\"><office:automatic-styles>{st}</office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document-content>").as_bytes());
    z.finish()
}

// ---------- generic XML paragraph extraction (HWPX/HML/ABW) ----------
fn xml_paras(s: &str, ptag: &str, ttag: &str) -> Vec<Para> {
    let (mut out, mut cur, mut depth, mut in_t) = (vec![], String::new(), 0usize, 0usize);
    for ev in xml::parse(s) { match ev {
        Ev::Start(n, _) if n == ptag => { depth += 1; if depth == 1 { cur.clear(); } }
        Ev::Start(n, _) if !ttag.is_empty() && n == ttag => in_t += 1,
        Ev::End(n) if !ttag.is_empty() && n == ttag => in_t = in_t.saturating_sub(1),
        Ev::End(n) if n == ptag => { depth = depth.saturating_sub(1); if depth == 0 { out.push(Para::plain(&cur)); } }
        Ev::Text(t) if depth > 0 && (ttag.is_empty() || in_t > 0) => cur.push_str(&t),
        _ => {} } }
    if out.is_empty() { out.push(Para::default()); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::{Editor, Fmt};
    fn sample() -> Doc {
        let mut e = Editor::new(Doc::default());
        e.insert("Title 한글"); e.set_style(Style::H1); e.insert("\nplain "); e.toggle(Fmt::Bold); e.insert("bold & <x>"); e.toggle(Fmt::Bold); e.toggle(Fmt::Italic); e.insert(" it*alic");
        e.insert("\nright"); e.set_align(Align::Right); e.doc
    }
    #[test] fn roundtrips() {
        let d = sample();
        for ext in ["omd", "docx", "odt", "rtf", "md"] {
            let back = load(ext, &save(ext, &d).unwrap()).unwrap();
            let (mut a, mut b) = (d.clone(), back); if ext == "md" || ext == "rtf" { a.paras.iter_mut().for_each(|p| p.align = Align::Left); b.paras.iter_mut().for_each(|p| p.align = Align::Left); }
            if ext == "md" { assert_eq!(a.plain(), b.plain()); assert_eq!(a.paras.iter().map(|p| p.style).collect::<Vec<_>>(), b.paras.iter().map(|p| p.style).collect::<Vec<_>>()); continue; }
            // RTF has no semantic heading styles here: headings come back as bold text.
            if ext == "rtf" { assert_eq!(a.plain(), b.plain()); assert_eq!(a.paras[1..], b.paras[1..]); continue; }
            assert_eq!(a, b, "roundtrip {ext}");
        }
    }
    #[test] fn undo_find_replace() {
        let mut e = Editor::new(Doc::from_plain("foo bar\nFoo baz"));
        assert_eq!(e.replace_all("foo", "x", false), 2); assert_eq!(e.doc.plain(), "x bar\nx baz\n");
        e.undo(); assert_eq!(e.doc.plain(), "foo bar\nFoo baz\n"); e.redo(); assert_eq!(e.doc.plain(), "x bar\nx baz\n");
        assert!(e.find_next("baz", true)); assert_eq!(e.range(), Some(((1, 2), (1, 5))));
        assert_eq!(crate::cmd::search("hea")[0].id, "style.h1");
    }
    #[test] fn real_world_inputs() {
        let rtf = br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}\pard Hello {\b bold} caf\'e9 \u54620?\u44544?\par second\par}";
        assert_eq!(load("rtf", rtf).unwrap().plain(), "Hello bold caf\u{e9} \u{d55c}\u{ae00}\nsecond\n");
    }
}
