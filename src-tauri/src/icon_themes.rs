//! Icon themes installed on this computer (Linux). Stingray lists the freedesktop icon themes in
//! the usual folders and, when one is chosen, reads the few icons it needs from it: folders, the
//! home folders, trash and common file types. Icons are returned as data URIs so the window can
//! show them without opening up file access to system folders.

use crate::error::{AppError, AppResult};
use base64::Engine;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTheme {
    /// Folder name, used as the id.
    pub id: String,
    /// Display name from index.theme.
    pub name: String,
}

/// Stingray key -> freedesktop names to try, best first. Same list as the bundled packs.
const ICONS: &[(&str, &[&str])] = &[
    ("folder", &["folder", "inode-directory"]),
    ("home", &["user-home", "folder-home"]),
    ("desktop", &["user-desktop", "folder-desktop"]),
    ("documents", &["folder-documents", "folder-document"]),
    ("downloads", &["folder-download", "folder-downloads"]),
    ("music", &["folder-music"]),
    ("pictures", &["folder-pictures", "folder-images"]),
    ("videos", &["folder-videos", "folder-video"]),
    ("trash", &["user-trash", "user-trash-empty"]),
    ("file", &["text-x-generic", "unknown"]),
    ("document", &["x-office-document", "text-x-generic"]),
    ("pdf", &["application-pdf"]),
    ("spreadsheet", &["x-office-spreadsheet"]),
    ("presentation", &["x-office-presentation"]),
    ("image", &["image-x-generic"]),
    ("video", &["video-x-generic"]),
    ("audio", &["audio-x-generic"]),
    ("archive", &["package-x-generic", "application-zip", "application-x-archive"]),
    ("code", &["text-x-script", "text-x-source"]),
];

fn roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = crate::localfs::real_home() {
        out.push(home.join(".local/share/icons"));
        out.push(home.join(".icons"));
    }
    out.push(PathBuf::from("/usr/share/icons"));
    out.push(PathBuf::from("/usr/local/share/icons"));
    out
}

fn theme_name(dir: &Path) -> Option<String> {
    let index = std::fs::read_to_string(dir.join("index.theme")).ok()?;
    if !dir.join("places").exists() && !index.contains("places") {
        return None; // cursor-only or symbolic-only themes have nothing for a file manager
    }
    index
        .lines()
        .find_map(|l| l.strip_prefix("Name="))
        .map(|n| n.trim().to_owned())
}

#[tauri::command]
pub fn list_system_icon_themes() -> Vec<SystemTheme> {
    let mut found: BTreeMap<String, SystemTheme> = BTreeMap::new();
    for root in roots() {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let dir = entry.path();
            let id = entry.file_name().to_string_lossy().into_owned();
            if id == "hicolor" || id == "default" || found.contains_key(&id) {
                continue;
            }
            if let Some(name) = theme_name(&dir) {
                found.insert(id.clone(), SystemTheme { id, name });
            }
        }
    }
    found.into_values().collect()
}

fn theme_dir(id: &str) -> Option<PathBuf> {
    if id.contains('/') || id.contains("..") {
        return None;
    }
    roots().into_iter().map(|r| r.join(id)).find(|d| d.join("index.theme").exists())
}

/// Themes this one inherits from (index.theme "Inherits="), for icons it doesn't have itself.
fn parents(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("index.theme"))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("Inherits=")).map(str::to_owned))
        .map(|s| s.split(',').map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()).collect())
        .unwrap_or_default()
}

fn size_score(path: &Path) -> i32 {
    let s = path.to_string_lossy();
    if s.contains("scalable") {
        return 1000;
    }
    s.split('/')
        .filter_map(|part| part.split('x').next()?.parse::<i32>().ok())
        .next()
        .unwrap_or(48)
}

/// Finds the best file for one icon name inside a theme folder.
fn find_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut best: Option<(i32, PathBuf)> = None;
    let walker = walk(dir, 3);
    for path in walker {
        let file = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if file != name {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
        if ext != "svg" && ext != "png" {
            continue;
        }
        let text = path.to_string_lossy();
        if text.contains("symbolic") || text.contains("/16") || text.contains("/22") || text.contains("/24") {
            continue;
        }
        let score = size_score(&path) + if ext == "svg" { 1 } else { 0 };
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Lists files up to `depth` folders down (icon themes are size/context/name or context/size/name).
fn walk(dir: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                out.extend(walk(&path, depth - 1));
            }
        } else {
            out.push(path);
        }
    }
    out
}

fn data_uri(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mime = if path.extension().is_some_and(|e| e == "svg") { "image/svg+xml" } else { "image/png" };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// All of Stingray's icons from one installed theme, keyed like the bundled packs.
#[tauri::command]
pub async fn system_icon_theme_icons(id: String) -> AppResult<BTreeMap<String, String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = theme_dir(&id).ok_or_else(|| AppError::invalid("That icon theme isn't installed."))?;
        let mut chain = vec![dir.clone()];
        for parent in parents(&dir) {
            if let Some(p) = theme_dir(&parent) {
                chain.push(p);
            }
        }
        let mut out = BTreeMap::new();
        for (key, names) in ICONS {
            let file = chain
                .iter()
                .find_map(|d| names.iter().find_map(|n| find_in(d, n)));
            if let Some(uri) = file.as_deref().and_then(data_uri) {
                out.insert((*key).to_owned(), uri);
            }
        }
        Ok(out)
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_scalable_and_large() {
        assert!(size_score(Path::new("/x/scalable/places/folder.svg")) > size_score(Path::new("/x/48x48/places/folder.svg")));
        assert!(size_score(Path::new("/x/64/places/folder.svg")) > size_score(Path::new("/x/32/places/folder.svg")));
    }

    #[test]
    fn rejects_paths_in_ids() {
        assert!(theme_dir("../etc").is_none());
    }
}
