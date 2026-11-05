//! Living on the desktop: the tray icon (Show / Exit), keeping Stingray in the background when
//! its window is closed, starting at login, and folders handed over by other apps or a second
//! launch (`stingray ~/Downloads`, or a desktop "open folder" request).

use crate::error::{AppError, AppResult};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Sent to the window when it should open a folder (and select a file in it, if given).
pub const OPEN_PATH_EVENT: &str = "desktop://open-path";

/// Started with --background: stay hidden until the tray icon or a new launch asks for the window.
pub struct BackgroundStart(pub AtomicBool);

/// A folder given on the command line, waiting for the window to ask for it.
pub struct InitialPath(pub Mutex<Option<String>>);

/// Turns a command-line argument or file:// URI into a local path, if it is one.
pub fn path_from_arg(arg: &str) -> Option<String> {
    if arg.starts_with("--") {
        return None;
    }
    let raw = arg.strip_prefix("file://").unwrap_or(arg);
    let decoded = percent_decode(raw);
    let path = PathBuf::from(&decoded);
    path.exists().then_some(decoded)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16)
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

/// The first path among the arguments (skipping the program name).
pub fn first_path(args: &[String]) -> Option<String> {
    args.iter().skip(1).find_map(|a| path_from_arg(a))
}

#[tauri::command]
pub fn take_initial_path(state: tauri::State<'_, InitialPath>) -> Option<String> {
    state.0.lock().ok()?.take()
}

/// Shows the window at a folder. Used by a second launch and by the tray.
pub fn open_path(app: &AppHandle, path: Option<String>) {
    crate::reveal_main_window(app);
    if let Some(path) = path {
        let _ = app.emit(OPEN_PATH_EVENT, path);
    }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Stingray", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit Stingray", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &PredefinedMenuItem::separator(app)?, &exit])?;
    let mut builder = TrayIconBuilder::with_id("stingray")
        .tooltip("Stingray File Manager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => open_path(app, None),
            "exit" => crate::quit::request(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                open_path(tray.app_handle(), None);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Hides the window instead of quitting, when the person keeps Stingray in the background.
pub fn keep_in_background(app: &AppHandle) -> bool {
    let keep = app
        .try_state::<crate::settings::SettingsStore>()
        .map(|s| s.get().keep_running)
        .unwrap_or(false);
    if keep {
        for window in app.webview_windows().values() {
            let _ = window.hide();
        }
    }
    keep
}

pub fn mark_shown(app: &AppHandle) {
    if let Some(flag) = app.try_state::<BackgroundStart>() {
        flag.0.store(false, Ordering::SeqCst);
    }
}

pub fn started_in_background(app: &AppHandle) -> bool {
    app.try_state::<BackgroundStart>().is_some_and(|f| f.0.load(Ordering::SeqCst))
}

// ---- Start at login ------------------------------------------------------------------------

fn autostart_file() -> Option<PathBuf> {
    crate::localfs::real_home().map(|h| h.join(".config/autostart/stingray.desktop"))
}

/// Adds or removes Stingray from the desktop's start-at-login list (Linux).
#[tauri::command]
pub fn set_start_at_login(enabled: bool) -> AppResult<()> {
    let file = autostart_file().ok_or_else(|| AppError::internal("No home folder"))?;
    if !enabled {
        let _ = std::fs::remove_file(&file);
        return Ok(());
    }
    let command = if std::env::var_os("SNAP").is_some() { "/snap/bin/stingray" } else { "stingray" };
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Stingray File Manager\nExec={command} --background\nIcon=stingray\nX-GNOME-Autostart-enabled=true\nNoDisplay=true\n"
    );
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| AppError::invalid(format!("Stingray couldn't set itself to start at login: {e}")))?;
    }
    std::fs::write(&file, entry).map_err(|e| AppError::invalid(format!("Stingray couldn't set itself to start at login: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_paths_and_uris() {
        assert_eq!(path_from_arg("/tmp"), Some("/tmp".into()));
        assert_eq!(path_from_arg("file:///tmp"), Some("/tmp".into()));
        assert_eq!(path_from_arg("--background"), None);
        assert_eq!(path_from_arg("/definitely/not/here"), None);
        assert_eq!(first_path(&["stingray".into(), "--background".into(), "/tmp".into()]), Some("/tmp".into()));
    }
}
