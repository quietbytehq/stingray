//! Archives: browse .zip and .tar(.gz) files as if they were folders, extract them, and open a
//! single file from inside one. Inside Stingray an archive's contents use paths like
//! "/home/ana/photos.zip!/2026/beach.jpg": everything before "!/" is the archive on disk,
//! everything after is the path inside it. Archives are read-only.

use crate::error::{AppError, AppResult, ErrorKind};
use crate::files::FileEntry;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};

pub const SEPARATOR: &str = "!/";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Zip,
    Tar,
    TarGz,
}

fn kind_of(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".zip") || name.ends_with(".jar") || name.ends_with(".cbz") {
        Some(Kind::Zip)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(Kind::TarGz)
    } else if name.ends_with(".tar") {
        Some(Kind::Tar)
    } else {
        None
    }
}

/// True for files Stingray can open as folders.
pub fn is_archive(path: &str) -> bool {
    kind_of(Path::new(path)).is_some()
}

/// Splits "/a/b.zip!/x/y" into ("/a/b.zip", "x/y"). Plain paths return None.
pub fn split(path: &str) -> Option<(String, String)> {
    let at = path.find(SEPARATOR)?;
    Some((path[..at].to_owned(), path[at + SEPARATOR.len()..].trim_matches('/').to_owned()))
}

fn err(message: &str, detail: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorKind::Io, message).with_details(detail.to_string())
}

/// One entry inside an archive: its inner path, whether it is a folder, size and modified time.
struct Item {
    path: String,
    is_dir: bool,
    size: u64,
    modified: Option<String>,
}

fn zip_items<R: Read + Seek>(reader: R) -> AppResult<Vec<Item>> {
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| err("Stingray couldn't read this zip file.", e))?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| err("Stingray couldn't read this zip file.", e))?;
        out.push(Item {
            path: f.name().trim_end_matches('/').to_owned(),
            is_dir: f.is_dir(),
            size: f.size(),
            modified: None,
        });
    }
    Ok(out)
}

fn tar_items<R: Read>(reader: R) -> AppResult<Vec<Item>> {
    let mut archive = tar::Archive::new(reader);
    let mut out = Vec::new();
    for entry in archive.entries().map_err(|e| err("Stingray couldn't read this tar file.", e))? {
        let entry = entry.map_err(|e| err("Stingray couldn't read this tar file.", e))?;
        let header = entry.header();
        let path = entry.path().map(|p| p.to_string_lossy().trim_start_matches("./").trim_end_matches('/').to_owned()).unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        let modified = header
            .mtime()
            .ok()
            .and_then(|t| chrono::DateTime::from_timestamp(t as i64, 0))
            .map(|d| d.to_rfc3339());
        out.push(Item { path, is_dir: header.entry_type().is_dir(), size: header.size().unwrap_or(0), modified });
    }
    Ok(out)
}

fn items(archive: &Path) -> AppResult<Vec<Item>> {
    let file = File::open(archive).map_err(|e| err("Stingray couldn't open this archive.", e))?;
    match kind_of(archive) {
        Some(Kind::Zip) => zip_items(file),
        Some(Kind::Tar) => tar_items(file),
        Some(Kind::TarGz) => tar_items(flate2::read::GzDecoder::new(file)),
        None => Err(AppError::invalid("This isn't an archive Stingray can open.")),
    }
}

/// Lists one folder inside an archive. Folders that exist only implicitly (because files sit in
/// them) are listed too.
pub fn list(path: &str) -> AppResult<Vec<FileEntry>> {
    let (archive, inner) = split(path).ok_or_else(|| AppError::invalid("That isn't a path inside an archive."))?;
    let prefix = if inner.is_empty() { String::new() } else { format!("{inner}/") };
    let mut children: BTreeMap<String, FileEntry> = BTreeMap::new();
    for item in items(Path::new(&archive))? {
        let Some(rest) = item.path.strip_prefix(&prefix) else { continue };
        if rest.is_empty() {
            continue;
        }
        let (name, deeper) = match rest.split_once('/') {
            Some((first, _)) => (first.to_owned(), true),
            None => (rest.to_owned(), false),
        };
        let full = format!("{archive}{SEPARATOR}{prefix}{name}");
        let is_dir = deeper || item.is_dir;
        children.entry(name.clone()).or_insert(FileEntry {
            name,
            path: full,
            is_dir,
            size: if is_dir { -1 } else { item.size as i64 },
            mod_time: if deeper { None } else { item.modified.clone() },
            mime_type: None,
        });
    }
    Ok(children.into_values().collect())
}

/// Rejects entries that would escape the target folder ("../../etc/passwd").
fn safe_join(base: &Path, inner: &str) -> Option<PathBuf> {
    let rel = Path::new(inner);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(base.join(rel))
}

/// Extracts everything, or only the entries under `only` (an inner path), into `target`.
fn extract_into(archive: &Path, target: &Path, only: Option<&str>) -> AppResult<()> {
    let wanted = |name: &str| match only {
        None => true,
        Some(o) => name == o || name.starts_with(&format!("{o}/")),
    };
    // Entries under `only` land directly in target, keeping only the last part of `only`.
    let strip = only.and_then(|o| o.rsplit_once('/').map(|(head, _)| format!("{head}/")));
    let rel_name = |name: &str| strip.as_deref().and_then(|s| name.strip_prefix(s)).unwrap_or(name).to_owned();
    std::fs::create_dir_all(target).map_err(|e| err("Stingray couldn't create the folder to extract into.", e))?;
    let file = File::open(archive).map_err(|e| err("Stingray couldn't open this archive.", e))?;
    let fail = |e: &dyn std::fmt::Display| err("Extracting stopped part-way.", e);
    match kind_of(archive) {
        Some(Kind::Zip) => {
            let mut zip = zip::ZipArchive::new(file).map_err(|e| fail(&e))?;
            for i in 0..zip.len() {
                let mut f = zip.by_index(i).map_err(|e| fail(&e))?;
                let name = f.name().trim_end_matches('/').to_owned();
                if name.is_empty() || !wanted(&name) {
                    continue;
                }
                let Some(dest) = safe_join(target, &rel_name(&name)) else { continue };
                if f.is_dir() {
                    std::fs::create_dir_all(&dest).map_err(|e| fail(&e))?;
                } else {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| fail(&e))?;
                    }
                    let mut out = File::create(&dest).map_err(|e| fail(&e))?;
                    std::io::copy(&mut f, &mut out).map_err(|e| fail(&e))?;
                }
            }
        }
        Some(kind) => {
            let reader: Box<dyn Read> = if kind == Kind::TarGz { Box::new(flate2::read::GzDecoder::new(file)) } else { Box::new(file) };
            let mut tar = tar::Archive::new(reader);
            for entry in tar.entries().map_err(|e| fail(&e))? {
                let mut entry = entry.map_err(|e| fail(&e))?;
                let name = entry.path().map(|p| p.to_string_lossy().trim_start_matches("./").trim_end_matches('/').to_owned()).unwrap_or_default();
                if name.is_empty() || !wanted(&name) {
                    continue;
                }
                let Some(dest) = safe_join(target, &rel_name(&name)) else { continue };
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| fail(&e))?;
                }
                entry.unpack(&dest).map_err(|e| fail(&e))?;
            }
        }
        None => return Err(AppError::invalid("This isn't an archive Stingray can open.")),
    }
    Ok(())
}

/// A folder name next to the archive that doesn't exist yet: "photos", then "photos (2)"…
fn free_folder(parent: &Path, stem: &str) -> PathBuf {
    let mut candidate = parent.join(stem);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{stem} ({n})"));
        n += 1;
    }
    candidate
}

fn stem_of(archive: &Path) -> String {
    let name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Archive".into());
    for ext in [".tar.gz", ".tgz", ".tar", ".zip", ".jar", ".cbz"] {
        if name.to_ascii_lowercase().ends_with(ext) {
            return name[..name.len() - ext.len()].to_owned();
        }
    }
    name
}

/// "Extract Here": unpacks the archive into a new folder beside it and returns that folder.
/// For a path inside an archive, extracts just that item next to the archive.
#[tauri::command]
pub async fn extract_archive(path: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (archive, inner) = match split(&path) {
            Some((a, i)) => (PathBuf::from(a), Some(i)),
            None => (PathBuf::from(&path), None),
        };
        let parent = archive.parent().ok_or_else(|| AppError::invalid("That archive can't be extracted here."))?.to_path_buf();
        let target = free_folder(&parent, &stem_of(&archive));
        extract_into(&archive, &target, inner.as_deref().filter(|i| !i.is_empty()))?;
        Ok(target.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Opens one file from inside an archive: extracts it to a temporary folder and returns its path.
#[tauri::command]
pub async fn open_from_archive(path: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (archive, inner) = split(&path).ok_or_else(|| AppError::invalid("That isn't a path inside an archive."))?;
        let tmp = std::env::temp_dir().join("stingray-archives").join(format!("{:x}", md5_like(&archive)));
        extract_into(Path::new(&archive), &tmp, Some(&inner))?;
        let name = inner.rsplit('/').next().unwrap_or(&inner);
        Ok(tmp.join(name).to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// A small stable hash for temp folder names (not for security).
fn md5_like(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(path: &Path) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let opts: zip::write::SimpleFileOptions = Default::default();
        zip.start_file("readme.txt", opts).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.start_file("photos/beach.jpg", opts).unwrap();
        zip.write_all(b"jpg").unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn splits_paths() {
        assert_eq!(split("/a/b.zip!/x/y"), Some(("/a/b.zip".into(), "x/y".into())));
        assert_eq!(split("/a/b.zip!/"), Some(("/a/b.zip".into(), "".into())));
        assert_eq!(split("/a/b.zip"), None);
        assert!(is_archive("/x/backup.tar.gz"));
        assert!(!is_archive("/x/notes.txt"));
    }

    #[test]
    fn lists_and_extracts_zip() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("pack.zip");
        make_zip(&zip);
        let root = list(&format!("{}!/", zip.display())).unwrap();
        let names: Vec<_> = root.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
        assert_eq!(names, vec![("photos", true), ("readme.txt", false)]);
        let inner = list(&format!("{}!/photos", zip.display())).unwrap();
        assert_eq!(inner[0].name, "beach.jpg");
        let target = dir.path().join("out");
        extract_into(&zip, &target, None).unwrap();
        assert_eq!(std::fs::read_to_string(target.join("readme.txt")).unwrap(), "hello");
        assert!(target.join("photos/beach.jpg").exists());
        assert_eq!(stem_of(Path::new("/x/backup.tar.gz")), "backup");
    }

    #[test]
    fn refuses_escaping_paths() {
        assert!(safe_join(Path::new("/tmp/x"), "../etc/passwd").is_none());
        assert!(safe_join(Path::new("/tmp/x"), "a/b").is_some());
    }
}
