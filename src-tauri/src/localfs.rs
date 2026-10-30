//! Files on this computer: the home folders in the sidebar, folder listings, and the everyday
//! operations (new folder, rename, move to trash, open). Transfers between this computer and a
//! cloud or network drive go through the shared transfer queue instead.

use crate::error::{AppError, AppResult, ErrorKind};
use crate::files::FileEntry;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One entry in the "Places" part of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    /// Stable key the UI uses to pick an icon: home, desktop, documents, downloads, music,
    /// pictures, videos, trash, volume.
    pub kind: String,
    pub label: String,
    pub path: String,
}

fn place(kind: &str, label: &str, path: Option<PathBuf>) -> Option<Place> {
    let path = path?;
    path.is_dir().then(|| Place {
        kind: kind.to_owned(),
        label: label.to_owned(),
        path: path.to_string_lossy().into_owned(),
    })
}

/// The person's real home folder. Inside a snap, `$HOME` points at the snap's private folder
/// (~/snap/<name>/<revision>), so the real one comes from `SNAP_REAL_HOME` when it is set.
pub fn real_home() -> Option<PathBuf> {
    std::env::var_os("SNAP_REAL_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

/// The trash folder of the current user, when the desktop keeps one we can show.
fn trash_dir() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        real_home().map(|home| home.join(".local").join("share").join("Trash").join("files"))
    } else {
        None
    }
}

/// Mounted volumes worth showing (USB sticks, second disks), Linux only.
fn volumes() -> Vec<Place> {
    let mut found = Vec::new();
    if !cfg!(target_os = "linux") {
        return found;
    }
    let user = std::env::var("USER").unwrap_or_default();
    for base in [format!("/media/{user}"), format!("/run/media/{user}")] {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    found.push(Place {
                        kind: "volume".into(),
                        label: entry.file_name().to_string_lossy().into_owned(),
                        path: path.to_string_lossy().into_owned(),
                    });
                }
            }
        }
    }
    found
}

#[tauri::command]
pub fn home_places() -> Vec<Place> {
    let mut places: Vec<Place> = [
        place("home", "Home", real_home()),
        place("desktop", "Desktop", dirs::desktop_dir()),
        place("documents", "Documents", dirs::document_dir()),
        place("downloads", "Downloads", dirs::download_dir()),
        place("music", "Music", dirs::audio_dir()),
        place("pictures", "Pictures", dirs::picture_dir()),
        place("videos", "Videos", dirs::video_dir()),
        place("trash", "Trash", trash_dir()),
    ]
    .into_iter()
    .flatten()
    .collect();
    places.extend(volumes());
    places
}

fn io_error(message: &str, err: std::io::Error) -> AppError {
    AppError::new(ErrorKind::Io, message).with_details(err.to_string())
}

/// Lists one folder. Hidden files (dot files) are included; the UI decides whether to show them.
pub fn list(path: &Path) -> AppResult<Vec<FileEntry>> {
    let entries = std::fs::read_dir(path)
        .map_err(|err| io_error("Stingray can't open this folder. Check that it still exists and that you can read it.", err))?;
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let full = entry.path();
        // Follow symlinks for type and size, but keep broken links visible as files.
        let metadata = std::fs::metadata(&full).or_else(|_| entry.metadata());
        let Ok(metadata) = metadata else { continue };
        let mod_time = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|since| {
                chrono::DateTime::from_timestamp(since.as_secs() as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            });
        items.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: full.to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_dir() { -1 } else { metadata.len() as i64 },
            mod_time,
            mime_type: None,
        });
    }
    Ok(items)
}

fn checked_name(name: &str) -> AppResult<&str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\')
    {
        return Err(AppError::invalid("Use a name without slashes."));
    }
    Ok(name)
}

#[tauri::command]
pub async fn list_local(path: String) -> AppResult<Vec<FileEntry>> {
    tauri::async_runtime::spawn_blocking(move || list(Path::new(&path)))
        .await
        .map_err(|err| AppError::new(ErrorKind::Internal, "Listing the folder stopped unexpectedly.").with_details(err.to_string()))?
}

#[tauri::command]
pub fn local_create_folder(parent: String, name: String) -> AppResult<String> {
    let target = Path::new(&parent).join(checked_name(&name)?);
    if target.exists() {
        return Err(AppError::invalid("Something with that name is already here."));
    }
    std::fs::create_dir(&target).map_err(|err| io_error("Stingray couldn't create the folder.", err))?;
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn local_rename(path: String, new_name: String) -> AppResult<String> {
    let source = Path::new(&path);
    let parent = source
        .parent()
        .ok_or_else(|| AppError::invalid("This item can't be renamed."))?;
    let target = parent.join(checked_name(&new_name)?);
    if target.exists() {
        return Err(AppError::invalid("Something with that name is already here."));
    }
    std::fs::rename(source, &target).map_err(|err| io_error("Stingray couldn't rename this item.", err))?;
    Ok(target.to_string_lossy().into_owned())
}

/// Moves items to the desktop's trash, so a delete can be undone from the Trash folder.
#[tauri::command]
pub fn local_trash(paths: Vec<String>) -> AppResult<()> {
    trash::delete_all(&paths).map_err(|err| {
        AppError::new(ErrorKind::Io, "Stingray couldn't move these items to the trash.")
            .with_details(err.to_string())
    })
}

/// Opens a file with its default app.
#[tauri::command]
pub fn open_local(app: tauri::AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|err| AppError::new(ErrorKind::Io, "No app is set up to open this file.").with_details(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        let mut items = list(dir.path()).unwrap();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(items[0].name, "a.txt");
        assert_eq!(items[0].size, 5);
        assert!(!items[0].is_dir);
        assert!(items[1].is_dir);
        assert_eq!(items[1].size, -1);
    }

    #[test]
    fn rejects_bad_names() {
        assert!(checked_name("a/b").is_err());
        assert!(checked_name("..").is_err());
        assert!(checked_name("  ").is_err());
        assert_eq!(checked_name(" ok ").unwrap(), "ok");
    }

    #[test]
    fn creates_and_renames() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().to_string_lossy().into_owned();
        let made = local_create_folder(parent.clone(), "New".into()).unwrap();
        assert!(Path::new(&made).is_dir());
        assert!(local_create_folder(parent, "New".into()).is_err());
        let renamed = local_rename(made, "Renamed".into()).unwrap();
        assert!(renamed.ends_with("Renamed"));
    }
}
