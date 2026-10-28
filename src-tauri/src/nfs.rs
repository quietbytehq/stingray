//! NFS shares. rclone has no NFS client, so Stingray asks the operating system to connect them:
//! on Linux `mount -t nfs` through `pkexec` (the desktop shows its own password prompt), on Windows
//! the built-in "Client for NFS" (`mount.exe`, Pro and Enterprise editions only). Once connected,
//! a share is an ordinary folder on this computer and is browsed like local files.
//!
//! The saved list holds only server, export and name. No secrets.

use crate::error::{AppError, AppResult, ErrorKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NfsShare {
    pub name: String,
    pub server: String,
    /// Exported path on the server, for example "/srv/share".
    pub export: String,
}

/// A saved share plus where it is connected right now, if anywhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfsShareView {
    #[serde(flatten)]
    pub share: NfsShare,
    pub connected_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfsSupport {
    pub available: bool,
    /// Plain-language reason when NFS can't be used here.
    pub message: Option<String>,
}

pub struct NfsStore {
    file: PathBuf,
    shares: Mutex<Vec<NfsShare>>,
}

impl NfsStore {
    pub fn load(file: PathBuf) -> Self {
        let shares = std::fs::read_to_string(&file)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self {
            file,
            shares: Mutex::new(shares),
        }
    }

    fn all(&self) -> Vec<NfsShare> {
        self.shares.lock().expect("nfs lock").clone()
    }

    fn find(&self, name: &str) -> AppResult<NfsShare> {
        self.all()
            .into_iter()
            .find(|share| share.name == name)
            .ok_or_else(|| AppError::invalid("That NFS share is no longer saved."))
    }

    fn save(&self, shares: Vec<NfsShare>) -> AppResult<()> {
        let raw = serde_json::to_string_pretty(&shares)
            .map_err(|err| AppError::internal(err.to_string()))?;
        if let Some(parent) = self.file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.file, raw).map_err(|err| {
            AppError::new(ErrorKind::Io, "Stingray couldn't save the NFS share list.")
                .with_details(err.to_string())
        })?;
        *self.shares.lock().expect("nfs lock") = shares;
        Ok(())
    }
}

/// "server:/export", the form both `mount` and the UI use.
pub fn source(share: &NfsShare) -> String {
    format!("{}:{}", share.server.trim(), share.export.trim())
}

/// Folder a share is connected at on Linux: ~/NFS/<name>.
fn linux_mount_point(name: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("NFS").join(name))
}

/// Finds where `source` is mounted, reading /proc/mounts on Linux.
fn linux_mounted_at(src: &str) -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    mounted_at_in(&mounts, src)
}

fn mounted_at_in(mounts: &str, src: &str) -> Option<String> {
    mounts.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let device = parts.next()?;
        let target = parts.next()?;
        let fstype = parts.next()?;
        (fstype.starts_with("nfs") && device.trim_end_matches('/') == src.trim_end_matches('/'))
            .then(|| target.replace("\\040", " "))
    })
}

/// Windows: the UNC form `\\server\export\path` that `mount.exe` takes.
fn windows_unc(share: &NfsShare) -> String {
    let export = share.export.trim().trim_matches('/').replace('/', "\\");
    format!("\\\\{}\\{}", share.server.trim(), export)
}

/// Windows: reads "Z: is now successfully connected" from mount.exe output.
fn drive_letter_in(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|word| {
        let bytes = word.as_bytes();
        (bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
            .then(|| format!("{}\\", word.to_ascii_uppercase()))
    })
}

fn connected_at(share: &NfsShare) -> Option<String> {
    if cfg!(target_os = "linux") {
        linux_mounted_at(&source(share))
    } else {
        None
    }
}

fn validate(share: &NfsShare) -> AppResult<NfsShare> {
    let name = share.name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return Err(AppError::invalid("Give the share a name without slashes."));
    }
    let server = share.server.trim();
    if server.is_empty() || server.contains(char::is_whitespace) {
        return Err(AppError::invalid("Enter the server's name or IP address."));
    }
    let export = share.export.trim();
    if !export.starts_with('/') {
        return Err(AppError::invalid("The shared folder starts with a slash, for example /srv/share."));
    }
    Ok(NfsShare {
        name: name.to_owned(),
        server: server.to_owned(),
        export: export.to_owned(),
    })
}

fn run(mut command: Command, failure: &str) -> AppResult<String> {
    let output = command.output().map_err(|err| {
        AppError::new(ErrorKind::Unsupported, failure).with_details(err.to_string())
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(AppError::new(ErrorKind::Io, failure).with_details(format!("{stdout}{stderr}")))
    }
}

#[tauri::command]
pub fn nfs_support() -> NfsSupport {
    if cfg!(windows) {
        let present = Path::new("C:\\Windows\\System32\\mount.exe").exists();
        NfsSupport {
            available: present,
            message: (!present).then(|| {
                "NFS needs Windows' \"Client for NFS\". Turn it on in Windows Features (Pro and Enterprise editions).".to_owned()
            }),
        }
    } else if cfg!(target_os = "linux") {
        let helper = Path::new("/sbin/mount.nfs").exists() || Path::new("/usr/sbin/mount.nfs").exists();
        NfsSupport {
            available: helper,
            message: (!helper).then(|| {
                "NFS needs the nfs-common package (Debian, Ubuntu) or nfs-utils (Fedora, Arch).".to_owned()
            }),
        }
    } else {
        NfsSupport {
            available: false,
            message: Some("NFS isn't supported on this system.".to_owned()),
        }
    }
}

#[tauri::command]
pub fn list_nfs_shares(store: State<'_, NfsStore>) -> Vec<NfsShareView> {
    store
        .all()
        .into_iter()
        .map(|share| NfsShareView {
            connected_at: connected_at(&share),
            share,
        })
        .collect()
}

#[tauri::command]
pub fn add_nfs_share(share: NfsShare, store: State<'_, NfsStore>) -> AppResult<NfsShare> {
    let share = validate(&share)?;
    let mut shares = store.all();
    if shares.iter().any(|existing| existing.name == share.name) {
        return Err(AppError::invalid("A share with that name is already saved."));
    }
    shares.push(share.clone());
    store.save(shares)?;
    Ok(share)
}

#[tauri::command]
pub fn remove_nfs_share(name: String, store: State<'_, NfsStore>) -> AppResult<()> {
    let shares = store.all().into_iter().filter(|share| share.name != name).collect();
    store.save(shares)
}

/// Connects a saved share and returns the folder it can be browsed at.
#[tauri::command]
pub async fn connect_nfs_share(name: String, store: State<'_, NfsStore>) -> AppResult<String> {
    let share = store.find(&name)?;
    if let Some(at) = connected_at(&share) {
        return Ok(at);
    }
    tauri::async_runtime::spawn_blocking(move || connect(&share))
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
}

fn connect(share: &NfsShare) -> AppResult<String> {
    if cfg!(windows) {
        let mut command = Command::new("mount");
        command.args(["-o", "anon"]).arg(windows_unc(share)).arg("*");
        let out = run(command, "Windows couldn't connect the NFS share. Check the server and folder.")?;
        return drive_letter_in(&out)
            .ok_or_else(|| AppError::new(ErrorKind::Io, "The share connected but Stingray couldn't tell which drive letter it got.").with_details(out));
    }
    let point = linux_mount_point(&share.name)
        .ok_or_else(|| AppError::internal("No home folder found"))?;
    std::fs::create_dir_all(&point).map_err(|err| {
        AppError::new(ErrorKind::Io, "Stingray couldn't create the folder to connect the share at.")
            .with_details(err.to_string())
    })?;
    let mut command = Command::new("pkexec");
    command
        .args(["mount", "-t", "nfs"])
        .arg(source(share))
        .arg(&point);
    run(command, "The NFS share didn't connect. Check the server name and shared folder, and that this computer is allowed to use it.")?;
    Ok(point.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn disconnect_nfs_share(name: String, store: State<'_, NfsStore>) -> AppResult<()> {
    let share = store.find(&name)?;
    tauri::async_runtime::spawn_blocking(move || {
        let failure = "The share is still in use. Close any files from it and try again.";
        if cfg!(windows) {
            let mut command = Command::new("umount");
            command.arg(windows_unc(&share));
            run(command, failure).map(|_| ())
        } else if let Some(at) = connected_at(&share) {
            let mut command = Command::new("pkexec");
            command.arg("umount").arg(at);
            run(command, failure).map(|_| ())
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share() -> NfsShare {
        NfsShare {
            name: "nas".into(),
            server: "192.168.1.10".into(),
            export: "/srv/share".into(),
        }
    }

    #[test]
    fn builds_sources() {
        assert_eq!(source(&share()), "192.168.1.10:/srv/share");
        assert_eq!(windows_unc(&share()), "\\\\192.168.1.10\\srv\\share");
    }

    #[test]
    fn finds_mounts() {
        let mounts = "sysfs /sys sysfs rw 0 0\n192.168.1.10:/srv/share /home/a/NFS/nas nfs4 rw 0 0\n";
        assert_eq!(
            mounted_at_in(mounts, "192.168.1.10:/srv/share"),
            Some("/home/a/NFS/nas".into())
        );
        assert_eq!(mounted_at_in(mounts, "other:/x"), None);
    }

    #[test]
    fn reads_drive_letter() {
        assert_eq!(
            drive_letter_in("z: is now successfully connected to \\\\nas\\share"),
            Some("Z:\\".into())
        );
    }

    #[test]
    fn validates_input() {
        assert!(validate(&share()).is_ok());
        let mut bad = share();
        bad.export = "srv".into();
        assert!(validate(&bad).is_err());
        let mut bad = share();
        bad.name = "a/b".into();
        assert!(validate(&bad).is_err());
    }
}
