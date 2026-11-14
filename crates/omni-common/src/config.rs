// SPDX-License-Identifier: GPL-3.0-only
//! Per-user configuration directory, recent-file lists and autosave paths.
use std::path::PathBuf;
pub fn config_dir() -> PathBuf {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(windows) { env("APPDATA") } else if cfg!(target_os = "macos") { env("HOME").map(|h| h.join("Library/Application Support")) }
        else { env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config"))) };
    base.unwrap_or_else(std::env::temp_dir).join("OmniOffice")
}
/// Move/insert `path` to the front of the list, remove duplicates, keep at most `max` entries.
pub fn recent_push(list: &mut Vec<String>, path: &str, max: usize) { list.retain(|p| p != path); list.insert(0, path.to_string()); list.truncate(max); }
pub fn recent_load(app: &str) -> Vec<String> {
    std::fs::read_to_string(config_dir().join(format!("{app}-recent.txt"))).map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default()
}
pub fn recent_save(app: &str, list: &[String]) { let d = config_dir(); let _ = std::fs::create_dir_all(&d); let _ = std::fs::write(d.join(format!("{app}-recent.txt")), list.join("\n")); }
pub fn autosave_path(app: &str, ext: &str) -> PathBuf { config_dir().join("autosave").join(format!("{app}.{ext}")) }
#[cfg(test)]
mod tests { use super::*;
    #[test] fn recent() { let mut v = vec!["a".to_string(), "b".into()]; recent_push(&mut v, "b", 3); recent_push(&mut v, "c", 3); recent_push(&mut v, "d", 3); assert_eq!(v, ["d", "c", "b"]); } }
