//! Desktop integration: drives, Open With, Compress, Email, Properties, Recent files, the boot id
//! (for the once-per-boot splash) and being the default file manager. Linux uses the usual
//! desktop tools (lsblk, udisksctl, gio, xdg-email, xdg-mime); Windows uses drive letters.

use crate::error::{AppError, AppResult, ErrorKind};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

fn io(message: &str, err: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorKind::Io, message).with_details(err.to_string())
}

fn run(cmd: &mut Command, failure: &str) -> AppResult<String> {
    let out = cmd.output().map_err(|e| io(failure, e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() {
        Ok(stdout)
    } else {
        Err(io(failure, format!("{stdout}{}", String::from_utf8_lossy(&out.stderr))))
    }
}

// ---- Drives --------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Device path ("/dev/nvme1n1p2") on Linux, drive root ("D:\\") on Windows.
    pub device: String,
    pub label: String,
    pub size: u64,
    pub filesystem: String,
    /// Where it is mounted, if it is.
    pub mount_point: Option<String>,
    /// USB sticks and other removable drives, which get an eject button.
    pub removable: bool,
}

/// Turns a byte count into "512 GB" the way disk makers count (powers of 1000).
fn human_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 1000.0 {
        format!("{:.1} TB", gb / 1000.0)
    } else if gb >= 1.0 {
        format!("{:.0} GB", gb)
    } else {
        format!("{:.0} MB", bytes as f64 / 1e6)
    }
}

/// Partitions a file manager should not show: system, boot and recovery ones.
fn hidden_partition(fs: &str, part_type: &str, mount: Option<&str>) -> bool {
    let kind = part_type.to_ascii_lowercase();
    fs.is_empty()
        || fs == "swap"
        || fs == "squashfs"
        || kind.contains("efi")
        || kind.contains("microsoft reserved")
        || kind.contains("recovery")
        || kind.contains("bios boot")
        || matches!(mount, Some("/") | Some("/boot") | Some("/boot/efi") | Some("/efi"))
}

#[cfg(target_os = "linux")]
fn linux_drives() -> AppResult<Vec<Drive>> {
    let json = run(
        Command::new("lsblk").args(["-J", "-b", "-o", "PATH,LABEL,SIZE,FSTYPE,MOUNTPOINT,TYPE,HOTPLUG,RM,PARTTYPENAME"]),
        "Stingray couldn't list the drives.",
    )?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| io("Stingray couldn't read the drive list.", e))?;
    let mut out = Vec::new();
    fn walk(node: &serde_json::Value, parent_removable: bool, out: &mut Vec<Drive>) {
        let s = |k: &str| node.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
        let b = |k: &str| node.get(k).map(|v| v.as_bool().unwrap_or(v.as_str() == Some("1"))).unwrap_or(false);
        let kind = s("type");
        let removable = parent_removable || b("hotplug") || b("rm");
        let fs = s("fstype");
        let mount = node.get("mountpoint").and_then(|v| v.as_str()).map(str::to_owned);
        if (kind == "part" || kind == "disk" || kind == "crypt") && !hidden_partition(&fs, &s("parttypename"), mount.as_deref()) {
            let size = node.get("size").and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok())).unwrap_or(0);
            let label = s("label");
            out.push(Drive {
                device: s("path"),
                label: if label.is_empty() { format!("{} Volume", human_size(size)) } else { label },
                size,
                filesystem: fs,
                mount_point: mount,
                removable,
            });
        }
        if kind == "loop" {
            return;
        }
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                walk(child, removable, out);
            }
        }
    }
    for dev in value.get("blockdevices").and_then(|v| v.as_array()).into_iter().flatten() {
        walk(dev, false, &mut out);
    }
    Ok(out)
}

#[cfg(windows)]
fn windows_drives() -> Vec<Drive> {
    (b'A'..=b'Z')
        .map(|c| format!("{}:\\", c as char))
        .filter(|root| Path::new(root).exists())
        .map(|root| Drive {
            label: format!("Local Disk ({})", &root[..2]),
            device: root.clone(),
            size: 0,
            filesystem: String::new(),
            mount_point: Some(root),
            removable: false,
        })
        .collect()
}

#[tauri::command]
pub fn list_drives() -> AppResult<Vec<Drive>> {
    #[cfg(target_os = "linux")]
    return linux_drives();
    #[cfg(windows)]
    return Ok(windows_drives());
    #[allow(unreachable_code)]
    Ok(Vec::new())
}

/// Mounts a drive (the desktop asks for a password if needed) and returns where it is mounted.
#[tauri::command]
pub async fn mount_drive(device: String) -> AppResult<String> {
    if !device.starts_with("/dev/") {
        return Err(AppError::invalid("That isn't a drive Stingray can mount."));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let out = run(
            Command::new("udisksctl").args(["mount", "-b", &device]),
            "The drive didn't mount. It may be in use by Windows (Fast Startup) or need a password.",
        )?;
        // "Mounted /dev/nvme1n1p2 at /media/faisal/Data"
        out.split(" at ")
            .nth(1)
            .map(|p| p.trim().trim_end_matches('.').to_owned())
            .ok_or_else(|| io("The drive mounted but Stingray couldn't tell where.", out))
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
pub async fn unmount_drive(device: String, eject: bool) -> AppResult<()> {
    if !device.starts_with("/dev/") {
        return Err(AppError::invalid("That isn't a drive Stingray can unmount."));
    }
    tauri::async_runtime::spawn_blocking(move || {
        run(Command::new("udisksctl").args(["unmount", "-b", &device]), "The drive is still in use. Close any files on it and try again.")?;
        if eject {
            let _ = Command::new("udisksctl").args(["power-off", "-b", &device]).output();
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

// ---- Open With -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    /// Desktop entry id, e.g. "org.gnome.TextEditor.desktop".
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[cfg(target_os = "linux")]
fn mime_of(path: &str) -> Option<String> {
    let out = Command::new("gio").args(["info", "-a", "standard::content-type", path]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("standard::content-type:").map(|m| m.trim().to_owned()))
}

/// Apps that say they can open this file, default first.
#[tauri::command]
pub fn apps_for_file(path: String) -> AppResult<Vec<App>> {
    #[cfg(target_os = "linux")]
    {
        let Some(mime) = mime_of(&path) else { return Ok(Vec::new()) };
        let out = run(Command::new("gio").args(["mime", &mime]), "Stingray couldn't list apps for this file.")?;
        let mut default = None;
        let mut ids: Vec<String> = Vec::new();
        for line in out.lines() {
            let line = line.trim();
            if let Some(d) = line.strip_prefix("Default application for") {
                default = d.split(':').nth(1).map(|s| s.trim().to_owned());
            } else if line.ends_with(".desktop") && !ids.contains(&line.to_owned()) {
                ids.push(line.to_owned());
            }
        }
        if let Some(d) = &default
            && !ids.contains(d)
        {
            ids.insert(0, d.clone());
        }
        let mut apps: Vec<App> = ids
            .into_iter()
            .map(|id| App { name: desktop_name(&id).unwrap_or_else(|| id.trim_end_matches(".desktop").to_owned()), default: Some(&id) == default.as_ref(), id })
            .collect();
        apps.sort_by_key(|a| (!a.default, a.name.to_lowercase()));
        return Ok(apps);
    }
    #[allow(unreachable_code)]
    Ok(Vec::new())
}

/// "Name=" from a desktop entry, looked up in the usual application folders.
fn desktop_name(id: &str) -> Option<String> {
    let mut dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Some(home) = crate::localfs::real_home() {
        dirs.insert(0, home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    dirs.iter().find_map(|d| {
        let text = std::fs::read_to_string(d.join(id)).ok()?;
        text.lines().find_map(|l| l.strip_prefix("Name=").map(str::to_owned))
    })
}

#[tauri::command]
pub fn open_with(path: String, app_id: String) -> AppResult<()> {
    if app_id.contains('/') || !app_id.ends_with(".desktop") {
        return Err(AppError::invalid("That isn't an app Stingray can start."));
    }
    Command::new("gio")
        .args(["launch", &app_id, &path])
        .spawn()
        .map_err(|e| io("Stingray couldn't start that app.", e))?;
    Ok(())
}

// ---- Compress and Email --------------------------------------------------------------------

/// Packs files and folders into a .zip next to them. Uses the system `zip`, or Python's zipfile
/// when `zip` isn't installed (it is part of every Ubuntu desktop).
#[tauri::command]
pub async fn compress(paths: Vec<String>, name: String) -> AppResult<String> {
    let first = paths.first().ok_or_else(|| AppError::invalid("Choose something to compress."))?;
    let parent = Path::new(first).parent().ok_or_else(|| AppError::invalid("That can't be compressed."))?.to_path_buf();
    let mut file = name.trim().trim_end_matches(".zip").to_owned();
    if file.is_empty() || file.contains('/') {
        return Err(AppError::invalid("Give the archive a name without slashes."));
    }
    file.push_str(".zip");
    let target = parent.join(&file);
    if target.exists() {
        return Err(AppError::invalid("An archive with that name is already here."));
    }
    let names: Vec<String> = paths
        .iter()
        .filter_map(|p| Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    tauri::async_runtime::spawn_blocking(move || {
        let has_zip = Command::new("zip").arg("-v").output().is_ok();
        if has_zip {
            run(Command::new("zip").current_dir(&parent).arg("-r").arg("-q").arg(&file).args(&names), "Stingray couldn't create the archive.")?;
        } else {
            let script = "import sys,zipfile,os\nz=zipfile.ZipFile(sys.argv[1],'w',zipfile.ZIP_DEFLATED)\nfor n in sys.argv[2:]:\n  if os.path.isdir(n):\n    for r,_,fs in os.walk(n):\n      for f in fs: z.write(os.path.join(r,f))\n  else: z.write(n)\nz.close()";
            run(Command::new("python3").current_dir(&parent).arg("-c").arg(script).arg(&file).args(&names), "Stingray couldn't create the archive.")?;
        }
        Ok(target.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Opens a new email in the person's mail app with the files attached.
#[tauri::command]
pub fn email_files(paths: Vec<String>) -> AppResult<()> {
    let mut cmd = Command::new("xdg-email");
    for p in &paths {
        cmd.arg("--attach").arg(p);
    }
    cmd.spawn().map_err(|e| io("Stingray couldn't open your mail app.", e))?;
    Ok(())
}

// ---- Properties ----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    pub name: String,
    pub location: String,
    pub is_dir: bool,
    /// Total size in bytes (for folders, everything inside).
    pub size: u64,
    /// Files and folders inside (folders only).
    pub items: u64,
    pub content_type: String,
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub created: Option<String>,
    /// "rwxr-xr-x" on Linux.
    pub permissions: String,
    pub owner: String,
}

fn when(t: std::io::Result<std::time::SystemTime>) -> Option<String> {
    let secs = t.ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0).map(|d| d.to_rfc3339())
}

fn folder_totals(dir: &Path, size: &mut u64, items: &mut u64, budget: &mut u32) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        *items += 1;
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            folder_totals(&entry.path(), size, items, budget);
        } else {
            *size += meta.len();
        }
    }
}

#[tauri::command]
pub async fn file_properties(path: String) -> AppResult<Properties> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        let meta = std::fs::metadata(p).map_err(|e| io("Stingray couldn't read this item.", e))?;
        let mut props = Properties {
            name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone()),
            location: p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified: when(meta.modified()),
            accessed: when(meta.accessed()),
            created: when(meta.created()),
            ..Default::default()
        };
        if meta.is_dir() {
            let (mut size, mut items, mut budget) = (0, 0, 200_000);
            folder_totals(p, &mut size, &mut items, &mut budget);
            props.size = size;
            props.items = items;
            props.content_type = "Folder".into();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let mode = meta.permissions().mode();
            props.permissions = (0..9)
                .map(|i| {
                    let bit = 1 << (8 - i);
                    if mode & bit == 0 { '-' } else { ['r', 'w', 'x'][i % 3] }
                })
                .collect();
            let uid = meta.uid();
            props.owner = std::fs::read_to_string("/etc/passwd")
                .ok()
                .and_then(|t| t.lines().find_map(|l| {
                    let f: Vec<&str> = l.split(':').collect();
                    (f.len() > 2 && f[2] == uid.to_string()).then(|| f[0].to_owned())
                }))
                .unwrap_or_else(|| uid.to_string());
            if !meta.is_dir() {
                #[cfg(target_os = "linux")]
                {
                    props.content_type = mime_of(&path).unwrap_or_default();
                }
            }
        }
        Ok(props)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

// ---- Recent files --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub path: String,
    pub modified: String,
}

/// Files recently opened on this desktop, newest first, from ~/.local/share/recently-used.xbel.
#[tauri::command]
pub fn recent_files() -> Vec<RecentFile> {
    let Some(home) = crate::localfs::real_home() else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(home.join(".local/share/recently-used.xbel")) else { return Vec::new() };
    parse_recent(&text)
}

fn unescape_uri(uri: &str) -> String {
    let raw = uri.strip_prefix("file://").unwrap_or(uri).replace("&amp;", "&");
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&raw[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_recent(text: &str) -> Vec<RecentFile> {
    let mut items: Vec<RecentFile> = text
        .split("<bookmark ")
        .skip(1)
        .filter_map(|chunk| {
            let attr = |name: &str| {
                let key = format!("{name}=\"");
                let start = chunk.find(&key)? + key.len();
                let end = chunk[start..].find('"')?;
                Some(chunk[start..start + end].to_owned())
            };
            let href = attr("href")?;
            if !href.starts_with("file://") {
                return None;
            }
            let path = unescape_uri(&href);
            Path::new(&path).exists().then(|| RecentFile { path, modified: attr("modified").or_else(|| attr("visited")).unwrap_or_default() })
        })
        .collect();
    items.sort_by(|a, b| b.modified.cmp(&a.modified));
    items.truncate(100);
    items
}

// ---- Boot id and default file manager ------------------------------------------------------

/// Changes every time the computer starts; the splash plays once per boot.
#[tauri::command]
pub fn boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultFm {
    /// Desktop entry currently opening folders, e.g. "org.gnome.Nautilus.desktop".
    pub current: String,
    /// Stingray's own desktop entry id.
    pub ours: String,
    pub is_default: bool,
    /// Inside a snap the default can't be changed from the app; this is the command to run instead.
    pub command: Option<String>,
}

fn our_desktop_id() -> String {
    if std::env::var_os("SNAP").is_some() { "stingray_stingray.desktop".into() } else { "stingray.desktop".into() }
}

#[tauri::command]
pub fn default_file_manager() -> DefaultFm {
    let current = Command::new("xdg-mime")
        .args(["query", "default", "inode/directory"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let ours = our_desktop_id();
    let in_snap = std::env::var_os("SNAP").is_some();
    DefaultFm {
        is_default: current == ours,
        command: in_snap.then(|| format!("xdg-mime default {ours} inode/directory")),
        current,
        ours,
    }
}

/// Makes Stingray (or, with `restore`, Ubuntu's Files) open folders.
#[tauri::command]
pub fn set_default_file_manager(restore: bool) -> AppResult<()> {
    let id = if restore { "org.gnome.Nautilus.desktop".to_owned() } else { our_desktop_id() };
    run(Command::new("xdg-mime").args(["default", &id, "inode/directory"]), "Stingray couldn't change the default file manager.").map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_like_disk_labels() {
        assert_eq!(human_size(512_110_190_592), "512 GB");
        assert_eq!(human_size(2_000_398_934_016), "2.0 TB");
    }

    #[test]
    fn hides_system_partitions() {
        assert!(hidden_partition("vfat", "EFI System", Some("/boot/efi")));
        assert!(hidden_partition("", "Microsoft reserved", None));
        assert!(hidden_partition("ext4", "Linux filesystem", Some("/")));
        assert!(!hidden_partition("ntfs", "Microsoft basic data", None));
    }

    #[test]
    fn reads_recent_files() {
        let xbel = r#"<xbel><bookmark href="file:///tmp" added="a" modified="2026-11-01T10:00:00Z" visited="x"></bookmark>
            <bookmark href="https://example.com" modified="2026-11-02T10:00:00Z"></bookmark></xbel>"#;
        let items = parse_recent(xbel);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "/tmp");
        assert_eq!(unescape_uri("file:///home/a/My%20File.txt"), "/home/a/My File.txt");
    }
}
