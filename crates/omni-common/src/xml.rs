// SPDX-License-Identifier: GPL-3.0-only
//! Tiny non-validating XML tokenizer (local names only).
pub enum Ev { Start(String, Vec<(String, String)>), End(String), Text(String) }
pub fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
pub fn unesc(s: &str) -> String {
    let mut o = String::new(); let mut r = s;
    while let Some(i) = r.find('&') {
        o.push_str(&r[..i]); r = &r[i..];
        let Some(j) = r.find(';').filter(|j| *j < 10) else { o.push('&'); r = &r[1..]; continue };
        let e = &r[1..j];
        let c = match e { "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'), "apos" => Some('\''),
            _ if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            _ if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32), _ => None };
        match c { Some(c) => { o.push(c); r = &r[j + 1..]; } None => { o.push('&'); r = &r[1..]; } }
    }
    o.push_str(r); o
}
fn local(n: &str) -> String { n.rsplit(':').next().unwrap_or(n).to_string() }
fn attrs(mut s: &str) -> Vec<(String, String)> {
    let mut v = vec![];
    while let Some(eq) = s.find('=') {
        let k = local(s[..eq].trim()); let rest = s[eq + 1..].trim_start();
        let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else { break };
        let Some(end) = rest[1..].find(q) else { break };
        v.push((k, unesc(&rest[1..1 + end]))); s = &rest[end + 2..];
    }
    v
}
pub fn attr<'a>(a: &'a [(String, String)], k: &str) -> Option<&'a str> { a.iter().find(|x| x.0 == k).map(|x| x.1.as_str()) }
pub fn parse(s: &str) -> Vec<Ev> {
    let (mut i, mut v) = (0, vec![]);
    while i < s.len() {
        if s.as_bytes()[i] == b'<' {
            if s[i..].starts_with("<!--") { i = s[i..].find("-->").map_or(s.len(), |x| i + x + 3); continue; }
            if s[i..].starts_with("<?") || s[i..].starts_with("<!") { i = s[i..].find('>').map_or(s.len(), |x| i + x + 1); continue; }
            let Some(e) = s[i..].find('>').map(|x| i + x) else { break };
            let tag = &s[i + 1..e];
            if let Some(t) = tag.strip_prefix('/') { v.push(Ev::End(local(t.trim()))); }
            else {
                let sc = tag.ends_with('/'); let tag = tag.trim_end_matches('/');
                let (n, rest) = tag.split_at(tag.find(char::is_whitespace).unwrap_or(tag.len()));
                v.push(Ev::Start(local(n), attrs(rest)));
                if sc { v.push(Ev::End(local(n))); }
            }
            i = e + 1;
        } else {
            let e = s[i..].find('<').map_or(s.len(), |x| i + x);
            v.push(Ev::Text(unesc(&s[i..e]))); i = e;
        }
    }
    v
}
