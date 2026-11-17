// SPDX-License-Identifier: GPL-3.0-only
//! DOCX reader/writer. The reader resolves paragraph styles through styles.xml (so localized heading names such as
//! "제목 1" work), numbering.xml (bullets/numbers become text prefixes), tables (cells joined by spaces, one
//! paragraph per row), text boxes, tracked changes, hidden text and mc:AlternateContent fallbacks.
use crate::model::*;
use crate::xml::{self, attr, esc, Ev};
use crate::zipio;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default, Clone)]
struct St { name: String, based: Option<String>, outline: Option<u32>, b: bool, i: bool, u: bool, jc: Option<Align> }
fn on(a: &[(String, String)]) -> bool { !matches!(attr(a, "val"), Some("0") | Some("false") | Some("none")) }
fn align(v: Option<&str>) -> Align { match v { Some("center") => Align::Center, Some("right") | Some("end") => Align::Right, Some("both") | Some("distribute") => Align::Justify, _ => Align::Left } }

fn parse_styles(x: &str) -> HashMap<String, St> {
    let (mut m, mut cur, mut in_rpr, mut in_ppr) = (HashMap::new(), None::<(String, St)>, false, false);
    for ev in xml::parse(x) { match ev {
        Ev::Start(n, a) => match n.as_str() {
            "style" => cur = Some((attr(&a, "styleId").unwrap_or("").to_string(), St::default())),
            "name" => if let Some(c) = cur.as_mut() { c.1.name = attr(&a, "val").unwrap_or("").to_string(); },
            "basedOn" => if let Some(c) = cur.as_mut() { c.1.based = attr(&a, "val").map(String::from); },
            "outlineLvl" => if let Some(c) = cur.as_mut() { c.1.outline = attr(&a, "val").and_then(|v| v.parse().ok()); },
            "rPr" => in_rpr = true, "pPr" => in_ppr = true,
            "b" if in_rpr => if let Some(c) = cur.as_mut() { c.1.b = on(&a); }, "i" if in_rpr => if let Some(c) = cur.as_mut() { c.1.i = on(&a); },
            "u" if in_rpr => if let Some(c) = cur.as_mut() { c.1.u = on(&a); },
            "jc" if in_ppr => if let Some(c) = cur.as_mut() { c.1.jc = Some(align(attr(&a, "val"))); },
            _ => {} },
        Ev::End(n) => match n.as_str() { "rPr" => in_rpr = false, "pPr" => in_ppr = false, "style" => if let Some((id, s)) = cur.take() { m.insert(id, s); }, _ => {} },
        _ => {} } }
    m
}
fn chain<'a>(m: &'a HashMap<String, St>, id: &str) -> Vec<&'a St> {
    let (mut v, mut cur) = (vec![], m.get(id));
    while let Some(s) = cur { v.push(s); if v.len() > 8 { break; } cur = s.based.as_ref().and_then(|b| m.get(b)); }
    v
}
fn para_style(m: &HashMap<String, St>, id: &str) -> Style {
    for s in chain(m, id) {
        let n: String = s.name.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
        let digits = |p: &str| n.strip_prefix(p).and_then(|r| r.parse::<u32>().ok());
        if let Some(l) = digits("heading").or_else(|| digits("제목")) { return Style::from_level(l.min(3)); }
        if n == "title" || n == "제목" { return Style::Title; }
        if n == "quote" || n == "인용" || n == "intensequote" { return Style::Quote; }
        if n.contains("code") || n.contains("preformatted") { return Style::Code; }
        if let Some(l) = s.outline { return Style::from_level((l + 1).min(3)); }
    }
    Style::Normal
}
#[derive(Default)]
struct Nums { fmt: HashMap<(String, u32), String>, num: HashMap<String, String> }
fn parse_numbering(x: &str) -> Nums {
    let (mut n, mut abs, mut lvl, mut num) = (Nums::default(), String::new(), 0u32, String::new());
    for ev in xml::parse(x) { if let Ev::Start(t, a) = ev { match t.as_str() {
        "abstractNum" => abs = attr(&a, "abstractNumId").unwrap_or("").into(), "lvl" => lvl = attr(&a, "ilvl").and_then(|v| v.parse().ok()).unwrap_or(0),
        "numFmt" => { n.fmt.insert((abs.clone(), lvl), attr(&a, "val").unwrap_or("").into()); }
        "num" => num = attr(&a, "numId").unwrap_or("").into(), "abstractNumId" => { n.num.insert(num.clone(), attr(&a, "val").unwrap_or("").into()); }
        _ => {} } } }
    n
}
pub fn read(d: &[u8]) -> Result<Doc, String> {
    let x = zipio::read(d, "word/document.xml")?.ok_or("DOCX: word/document.xml missing")?;
    let styles = zipio::read(d, "word/styles.xml").ok().flatten().map(|b| parse_styles(&String::from_utf8_lossy(&b))).unwrap_or_default();
    let nums = zipio::read(d, "word/numbering.xml").ok().flatten().map(|b| parse_numbering(&String::from_utf8_lossy(&b))).unwrap_or_default();
    let rels: HashMap<String, String> = zipio::read(d, "word/_rels/document.xml.rels").ok().flatten().map(|b| xml::parse(&String::from_utf8_lossy(&b)).into_iter().filter_map(|e| match e { Ev::Start(n, a) if n == "Relationship" => Some((attr(&a, "Id")?.to_string(), attr(&a, "Target")?.to_string())), _ => None }).collect()).unwrap_or_default();
    let (mut in_draw, mut ext, mut descr, mut embed, mut pend) = (false, (0u32, 0u32), String::new(), None::<String>, Vec::<Image>::new());
    let (mut out, mut stack, mut r, mut hidden) = (Vec::<Para>::new(), Vec::<Para>::new(), Run::default(), false);
    let (mut in_t, mut in_rpr, mut in_ppr, mut skip, mut pf) = (false, false, false, 0usize, (false, false, false));
    let (mut tbl, mut row, mut cell) = (0usize, Vec::<Vec<Run>>::new(), Vec::<Run>::new());
    let (mut num_id, mut num_lvl, mut counters) = (String::new(), 0u32, HashMap::<(String, u32), u32>::new());
    for ev in xml::parse(&String::from_utf8_lossy(&x)) {
        if skip > 0 { match &ev { Ev::Start(n, _) if n == "Fallback" => skip += 1, Ev::End(n) if n == "Fallback" => skip -= 1, _ => {} } continue; }
        match ev {
            Ev::Start(n, a) => match n.as_str() {
                "Fallback" => skip = 1,
                "p" => { stack.push(Para::default()); pf = (false, false, false); num_id.clear(); num_lvl = 0; }
                "pPr" => in_ppr = true, "rPr" => in_rpr = true, "t" => in_t = true,
                "r" => { r = Run { bold: pf.0, italic: pf.1, underline: pf.2, ..Default::default() }; hidden = false; }
                "pStyle" => if let Some(p) = stack.last_mut() {
                    let id = attr(&a, "val").unwrap_or(""); p.style = para_style(&styles, id);
                    for s in chain(&styles, id) { if let Some(j) = s.jc { p.align = j; break; } }
                    pf = if p.style == Style::Normal { chain(&styles, id).iter().fold((false, false, false), |f, s| (f.0 | s.b, f.1 | s.i, f.2 | s.u)) } else { (false, false, false) };
                },
                "rStyle" if in_rpr => for s in chain(&styles, attr(&a, "val").unwrap_or("")) { r.bold |= s.b; r.italic |= s.i; r.underline |= s.u; },
                "jc" if in_ppr => if let Some(p) = stack.last_mut() { p.align = align(attr(&a, "val")); },
                "ilvl" if in_ppr => num_lvl = attr(&a, "val").and_then(|v| v.parse().ok()).unwrap_or(0),
                "numId" if in_ppr => num_id = attr(&a, "val").unwrap_or("").into(),
                "drawing" => { in_draw = true; ext = (0, 0); descr.clear(); embed = None; }
                "extent" if in_draw && ext == (0, 0) => ext = (attr(&a, "cx").and_then(|v| v.parse().ok()).unwrap_or(0), attr(&a, "cy").and_then(|v| v.parse().ok()).unwrap_or(0)),
                "docPr" if in_draw => descr = attr(&a, "descr").unwrap_or("").to_string(), "blip" if in_draw => embed = attr(&a, "embed").map(String::from),
                "strike" if in_rpr => r.strike = on(&a), "sz" if in_rpr => r.size = attr(&a, "val").and_then(|v| v.parse::<u32>().ok()).map(|v| (v / 2).clamp(1, 255) as u8),
                "color" if in_rpr => r.color = attr(&a, "val").and_then(|v| u32::from_str_radix(v, 16).ok()),
                "b" if in_rpr => r.bold = on(&a), "i" if in_rpr => r.italic = on(&a), "u" if in_rpr => r.underline = on(&a), "vanish" if in_rpr => hidden = on(&a),
                "tab" | "br" if !in_ppr => if let Some(p) = stack.last_mut() { p.runs.push(Run { text: if n == "tab" { "    " } else { " " }.into(), ..r.clone() }); },
                "tbl" => tbl += 1, "tr" if tbl == 1 => row.clear(), "tc" if tbl == 1 => cell.clear(),
                _ => {} },
            Ev::End(n) => match n.as_str() {
                "pPr" => { in_ppr = false;
                    if let (false, Some(p)) = (num_id.is_empty() || num_id == "0", stack.last_mut()) {
                        let abs = nums.num.get(&num_id).cloned().unwrap_or_default(); let bullet = nums.fmt.get(&(abs, num_lvl)).map_or(true, |f| f == "bullet");
                        counters.retain(|k, _| k.0 != num_id || k.1 <= num_lvl); let c = counters.entry((num_id.clone(), num_lvl)).or_insert(0); *c += 1;
                        let prefix = format!("{}{}", "  ".repeat(num_lvl as usize), if bullet { "\u{2022} ".to_string() } else { format!("{}. ", c) });
                        p.runs.insert(0, Run::plain(&prefix));
                    } }
                "rPr" => in_rpr = false, "t" => in_t = false,
                "drawing" => { in_draw = false;
                    if let Some(t) = embed.take().and_then(|id| rels.get(&id).cloned()) {
                        let path = if let Some(a) = t.strip_prefix('/') { a.to_string() } else { format!("word/{t}") };
                        if let Some(data) = zipio::read(d, &path).ok().flatten() {
                            let (w, h) = if ext != (0, 0) { (ext.0 / 9525, ext.1 / 9525) } else { omni_common::image::size(&data).unwrap_or((0, 0)) };
                            pend.push(Image { data: Arc::new(data), w, h, alt: std::mem::take(&mut descr) });
                        }
                    } }
                "p" => if let Some(mut p) = stack.pop() {
                    p.normalize();
                    if stack.is_empty() && tbl == 0 && p.runs.is_empty() && pend.len() == 1 { p.image = pend.pop(); }
                    if tbl > 0 && stack.is_empty() { if !cell.is_empty() { cell.push(Run::plain(" ")); } cell.extend(p.runs); pend.clear(); }
                    else { out.push(p); if stack.is_empty() { for im in pend.drain(..) { out.push(Para { image: Some(im), ..Default::default() }); } } }
                },
                "tc" if tbl == 1 => row.push(std::mem::take(&mut cell)),
                "tr" if tbl == 1 => { let mut p = Para { table: true, ..Default::default() }; for (i, c) in row.drain(..).enumerate() { if i > 0 { p.runs.push(Run::plain("\t")); } p.runs.extend(c); } p.normalize(); out.push(p); }
                "tbl" => tbl = tbl.saturating_sub(1),
                _ => {} },
            Ev::Text(t) => if in_t && !hidden { if let Some(p) = stack.last_mut() { p.runs.push(Run { text: t, ..r.clone() }); } },
        }
    }
    if out.is_empty() { out.push(Para::default()); }
    Ok(Doc { paras: out })
}
fn run_xml(r: &Run) -> String {
    format!("<w:r><w:rPr>{}{}{}{}{}{}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>", if r.bold { "<w:b/>" } else { "" }, if r.italic { "<w:i/>" } else { "" }, if r.underline { "<w:u w:val=\"single\"/>" } else { "" },
        if r.strike { "<w:strike/>" } else { "" }, r.color.map_or(String::new(), |c| format!("<w:color w:val=\"{c:06X}\"/>")), r.size.map_or(String::new(), |s| format!("<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>", s as u32 * 2, s as u32 * 2)), esc(&r.text).replace('\t', "</w:t><w:tab/><w:t xml:space=\"preserve\">"))
}
pub fn write(d: &Doc) -> Vec<u8> {
    let w = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"";
    let wns = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"";
    let (mut body, mut media, mut i) = (String::new(), Vec::<(String, Arc<Vec<u8>>)>::new(), 0usize);
    while i < d.paras.len() {
        let p = &d.paras[i];
        if p.table {
            let j = (i..d.paras.len()).take_while(|k| d.paras[*k].table).last().unwrap() + 1;
            let rows: Vec<Vec<Vec<Run>>> = d.paras[i..j].iter().map(|p| p.cells()).collect(); let n = rows.iter().map(|r| r.len()).max().unwrap_or(1); let cw = 9000 / n;
            let b = |s: &str| format!("<w:{s} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"808080\"/>");
            body.push_str(&format!("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/><w:tblBorders>{}{}{}{}{}{}</w:tblBorders></w:tblPr><w:tblGrid>{}</w:tblGrid>", b("top"), b("left"), b("bottom"), b("right"), b("insideH"), b("insideV"), format!("<w:gridCol w:w=\"{cw}\"/>").repeat(n)));
            for row in &rows { body.push_str("<w:tr>"); for k in 0..n { body.push_str(&format!("<w:tc><w:tcPr><w:tcW w:w=\"{cw}\" w:type=\"dxa\"/></w:tcPr><w:p>{}</w:p></w:tc>", row.get(k).map_or(String::new(), |c| c.iter().map(run_xml).collect::<String>()))); } body.push_str("</w:tr>"); }
            body.push_str("</w:tbl>"); if j >= d.paras.len() { body.push_str("<w:p/>"); } i = j; continue;
        }
        if let Some(img) = &p.image {
            let n = media.len() + 1; let ext = omni_common::image::ext(&img.data).unwrap_or("png"); media.push((format!("media/img{n}.{ext}"), img.data.clone()));
            let (cx, cy) = (img.w.max(1) as u64 * 9525, img.h.max(1) as u64 * 9525);
            body.push_str(&format!("<w:p><w:pPr>{}</w:pPr><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\" descr=\"{}\"/><a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"img{n}.{ext}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"rIdImg{n}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>",
                match p.align { Align::Center => "<w:jc w:val=\"center\"/>", Align::Right => "<w:jc w:val=\"right\"/>", _ => "" }, esc(&img.alt), ));
            i += 1; continue;
        }
        body.push_str("<w:p><w:pPr>");
        body.push_str(match p.style { Style::Title => "<w:pStyle w:val=\"Title\"/>", Style::H1 => "<w:pStyle w:val=\"Heading1\"/>", Style::H2 => "<w:pStyle w:val=\"Heading2\"/>", Style::H3 => "<w:pStyle w:val=\"Heading3\"/>", Style::Quote => "<w:pStyle w:val=\"Quote\"/>", Style::Code => "<w:pStyle w:val=\"Code\"/>", _ => "" });
        body.push_str(match p.align { Align::Center => "<w:jc w:val=\"center\"/>", Align::Right => "<w:jc w:val=\"right\"/>", Align::Justify => "<w:jc w:val=\"both\"/>", _ => "" });
        body.push_str("</w:pPr>"); for r in &p.runs { body.push_str(&run_xml(r)); } body.push_str("</w:p>"); i += 1;
    }
    let sty = |id: &str, name: &str, ppr: &str, rpr: &str| format!("<w:style w:type=\"paragraph\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:pPr>{ppr}</w:pPr><w:rPr>{rpr}</w:rPr></w:style>");
    let styles = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles {wns}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:eastAsia=\"Malgun Gothic\" w:cs=\"Calibri\"/><w:sz w:val=\"22\"/><w:lang w:val=\"en-US\" w:eastAsia=\"ko-KR\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"120\" w:line=\"276\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>{}{}{}{}{}{}</w:styles>",
        sty("Title", "Title", "", "<w:b/><w:sz w:val=\"56\"/>"), sty("Heading1", "heading 1", "<w:keepNext/><w:outlineLvl w:val=\"0\"/>", "<w:b/><w:sz w:val=\"40\"/>"),
        sty("Heading2", "heading 2", "<w:keepNext/><w:outlineLvl w:val=\"1\"/>", "<w:b/><w:sz w:val=\"32\"/>"), sty("Heading3", "heading 3", "<w:keepNext/><w:outlineLvl w:val=\"2\"/>", "<w:b/><w:sz w:val=\"26\"/>"),
        sty("Quote", "Quote", "<w:ind w:left=\"720\"/>", "<w:i/>"), sty("Code", "Code", "", "<w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\"/>"));
    let sect = "<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr>";
    let mut rels = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>");
    for (k, (n, _)) in media.iter().enumerate() { rels.push_str(&format!("<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"{n}\"/>", k + 1)); }
    rels.push_str("</Relationships>");
    let mut z = zipio::Writer::new();
    z.add("[Content_Types].xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/><Default Extension=\"gif\" ContentType=\"image/gif\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/></Types>");
    z.add("_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>");
    z.add("docProps/core.xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:creator>OmniDoc</dc:creator></cp:coreProperties>");
    z.add("word/_rels/document.xml.rels", rels.as_bytes()); z.add("word/styles.xml", styles.as_bytes());
    z.add("word/document.xml", format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document {w}><w:body>{body}{sect}</w:body></w:document>").as_bytes());
    for (n, data) in &media { z.add(&format!("word/{n}"), data); }
    z.finish()
}
