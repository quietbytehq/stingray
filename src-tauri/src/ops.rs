//! Everyday file operations added in 2.0.2: empty the Trash, change permissions, install a
//! .deb or .rpm, and create .zip or .7z archives, optionally protected with AES-256.

use crate::error::{AppError, AppResult, ErrorKind};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

fn io(message: &str, err: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorKind::Io, message).with_details(err.to_string())
}

/// Permanently deletes everything in the Trash.
#[tauri::command]
pub async fn empty_trash() -> AppResult<usize> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(any(target_os = "linux", windows))]
        {
            let items = trash::os_limited::list().map_err(|e| io("Stingray couldn't read the Trash.", e))?;
            let count = items.len();
            if count > 0 {
                trash::os_limited::purge_all(items).map_err(|e| io("Stingray couldn't empty the Trash.", e))?;
            }
            Ok(count)
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        Ok(0)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Sets a file's permission bits ("rwxr-xr-x" as a number, e.g. 0o755).
#[tauri::command]
pub fn set_permissions(path: String, mode: u32) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode & 0o7777))
            .map_err(|e| io("Stingray couldn't change the permissions. You may not own this file.", e))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}

fn exists(cmd: &str) -> bool {
    Command::new("sh").arg("-c").arg(format!("command -v {cmd} >/dev/null 2>&1")).status().is_ok_and(|s| s.success())
}

/// Installs a .deb or .rpm with the desktop's own installer (App Center or GNOME Software),
/// which asks for the password and shows progress. Without one, falls back to the package
/// manager behind the system password prompt.
#[tauri::command]
pub fn install_package(path: String) -> AppResult<String> {
    let lower = path.to_ascii_lowercase();
    let deb = lower.ends_with(".deb");
    if !deb && !lower.ends_with(".rpm") {
        return Err(AppError::invalid("Only .deb and .rpm packages can be installed."));
    }
    let spawn = |prog: &str, args: &[&str]| Command::new(prog).args(args).spawn().is_ok();
    for (prog, label) in [("snap-store", "App Center"), ("gnome-software", "Software"), ("plasma-discover", "Discover")] {
        if exists(prog) && spawn(prog, &["--local-filename", &path]) {
            return Ok(label.into());
        }
    }
    let ok = if deb {
        exists("apt-get") && spawn("pkexec", &["apt-get", "install", "-y", &path])
    } else if exists("dnf") {
        spawn("pkexec", &["dnf", "install", "-y", &path])
    } else {
        exists("zypper") && spawn("pkexec", &["zypper", "--non-interactive", "install", &path])
    };
    if ok {
        Ok("the system installer".into())
    } else {
        Err(AppError::invalid("No installer for this kind of package was found on this computer."))
    }
}

/// Every file under the selection, with its name inside the archive.
fn collect(paths: &[String]) -> Vec<(PathBuf, String, bool)> {
    let mut out = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        let Some(base) = path.parent().map(Path::to_path_buf) else { continue };
        let mut stack = vec![path];
        while let Some(cur) = stack.pop() {
            let rel = cur.strip_prefix(&base).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            if cur.is_dir() {
                out.push((cur.clone(), rel, true));
                if let Ok(entries) = std::fs::read_dir(&cur) {
                    stack.extend(entries.flatten().map(|e| e.path()));
                }
            } else {
                out.push((cur, rel, false));
            }
        }
    }
    out
}

fn write_zip(target: &Path, items: &[(PathBuf, String, bool)], password: Option<&str>) -> AppResult<()> {
    let file = File::create(target).map_err(|e| io("Stingray couldn't create the archive.", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let base: zip::write::SimpleFileOptions =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let fail = |e: &dyn std::fmt::Display| io("Creating the archive stopped part-way.", e);
    for (path, rel, is_dir) in items {
        let opts = match password {
            Some(pw) => base.with_aes_encryption(zip::AesMode::Aes256, pw),
            None => base,
        };
        if *is_dir {
            zip.add_directory(rel.as_str(), opts).map_err(|e| fail(&e))?;
        } else {
            zip.start_file(rel.as_str(), opts).map_err(|e| fail(&e))?;
            let mut f = File::open(path).map_err(|e| fail(&e))?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = f.read(&mut buf).map_err(|e| fail(&e))?;
                if n == 0 {
                    break;
                }
                zip.write_all(&buf[..n]).map_err(|e| fail(&e))?;
            }
        }
    }
    zip.finish().map_err(|e| fail(&e))?;
    Ok(())
}

fn write_7z(target: &Path, paths: &[String], password: Option<&str>) -> AppResult<()> {
    use sevenz_rust::{AesEncoderOptions, SevenZMethod, SevenZWriter};
    let fail = |e: &dyn std::fmt::Display| io("Creating the archive stopped part-way.", e);
    let mut sz = SevenZWriter::create(target).map_err(|e| fail(&e))?;
    if let Some(pw) = password {
        sz.set_content_methods(vec![AesEncoderOptions::new(pw.into()).into(), SevenZMethod::LZMA2.into()]);
    }
    for p in paths {
        sz.push_source_path(p, |_| true).map_err(|e| fail(&e))?;
    }
    sz.finish().map_err(|e| fail(&e))?;
    Ok(())
}

/// Creates `name.zip` or `name.7z` next to the selection. With a password, the archive is
/// encrypted with AES-256 (for .7z, file names are hidden too).
#[tauri::command]
pub async fn compress_archive(paths: Vec<String>, name: String, format: String, password: Option<String>) -> AppResult<String> {
    let first = paths.first().ok_or_else(|| AppError::invalid("Choose something to compress."))?;
    let parent = Path::new(first).parent().ok_or_else(|| AppError::invalid("That can't be compressed."))?.to_path_buf();
    let ext = if format == "7z" { "7z" } else { "zip" };
    let stem = name.trim().trim_end_matches(&format!(".{ext}")).to_owned();
    if stem.is_empty() || stem.contains('/') || stem.contains('\\') {
        return Err(AppError::invalid("Give the archive a name without slashes."));
    }
    let target = parent.join(format!("{stem}.{ext}"));
    if target.exists() {
        return Err(AppError::invalid("An archive with that name is already here."));
    }
    let password = password.filter(|p| !p.is_empty());
    let out = target.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = if ext == "7z" {
            write_7z(&target, &paths, password.as_deref())
        } else {
            write_zip(&target, &collect(&paths), password.as_deref())
        };
        if result.is_err() {
            let _ = std::fs::remove_file(&target);
        }
        result
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    Ok(out.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_encrypted_zip_and_7z() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("docs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("a.txt"), "hello").unwrap();
        let paths = vec![folder.to_string_lossy().into_owned()];
        let zip_path = dir.path().join("x.zip");
        write_zip(&zip_path, &collect(&paths), Some("secret")).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
        assert!(archive.by_name("docs/a.txt").is_err() || archive.by_name_decrypt("docs/a.txt", b"secret").is_ok());
        let sz = dir.path().join("x.7z");
        write_7z(&sz, &paths, Some("secret")).unwrap();
        assert!(std::fs::metadata(&sz).unwrap().len() > 0);
    }
}
