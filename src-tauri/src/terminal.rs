//! The terminal panel: a real shell in a pseudo-terminal. Output is streamed to the window as
//! events; typing, resizing and closing come back as commands. On Linux Stingray also watches the
//! shell's working folder, so `cd` in the terminal moves the file view, and can send `cd` the
//! other way when the shell is idle at its prompt.

use crate::error::{AppError, AppResult};
use base64::Engine;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub const DATA_EVENT: &str = "term://data";
pub const EXIT_EVENT: &str = "term://exit";
pub const CWD_EVENT: &str = "term://cwd";

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    pid: Option<u32>,
}

#[derive(Default)]
pub struct Terminals {
    sessions: Mutex<HashMap<u32, Session>>,
    next: AtomicU32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Chunk {
    id: u32,
    /// Raw bytes, base64, so multi-byte characters split across reads survive.
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Cwd {
    id: u32,
    path: String,
}

fn shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("powershell.exe")
    }
    #[cfg(not(windows))]
    {
        let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        let mut cmd = CommandBuilder::new(sh);
        cmd.arg("-l");
        cmd
    }
}

fn err(message: &str, e: impl std::fmt::Display) -> AppError {
    AppError::invalid(format!("{message} ({e})"))
}

/// Starts a shell in `cwd` and returns its id.
#[tauri::command]
pub fn term_open(app: AppHandle, cwd: String, cols: u16, rows: u16) -> AppResult<u32> {
    let pair = native_pty_system()
        .openpty(PtySize { rows: rows.max(2), cols: cols.max(10), pixel_width: 0, pixel_height: 0 })
        .map_err(|e| err("Stingray couldn't open a terminal", e))?;
    let mut cmd = shell();
    if std::path::Path::new(&cwd).is_dir() {
        cmd.cwd(&cwd);
    } else if let Some(home) = crate::localfs::real_home() {
        cmd.cwd(home);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    let child = pair.slave.spawn_command(cmd).map_err(|e| err("Stingray couldn't start your shell", e))?;
    drop(pair.slave);
    let pid = child.process_id();
    let mut reader = pair.master.try_clone_reader().map_err(|e| err("Stingray couldn't read the terminal", e))?;
    let writer = pair.master.take_writer().map_err(|e| err("Stingray couldn't write to the terminal", e))?;

    let state = app.state::<Terminals>();
    let id = state.next.fetch_add(1, Ordering::SeqCst) + 1;
    state.sessions.lock().expect("terminals").insert(id, Session { master: pair.master, writer, child, pid });

    // Output → window.
    let out = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = base64::engine::general_purpose::STANDARD.encode(&buf[..n]);
                    let _ = out.emit(DATA_EVENT, Chunk { id, data });
                }
            }
        }
        let _ = out.emit(EXIT_EVENT, id);
        if let Some(t) = out.try_state::<Terminals>() {
            t.sessions.lock().expect("terminals").remove(&id);
        }
    });

    // Working folder → window (Linux): poll the shell's cwd and report changes.
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        let watch = app.clone();
        std::thread::spawn(move || {
            let mut last = String::new();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(400));
                let alive = watch.try_state::<Terminals>().is_some_and(|t| t.sessions.lock().expect("terminals").contains_key(&id));
                if !alive {
                    break;
                }
                if let Ok(p) = std::fs::read_link(format!("/proc/{pid}/cwd")) {
                    let path = p.to_string_lossy().into_owned();
                    if path != last {
                        last = path.clone();
                        let _ = watch.emit(CWD_EVENT, Cwd { id, path });
                    }
                }
            }
        });
    }
    Ok(id)
}

#[tauri::command]
pub fn term_write(state: tauri::State<'_, Terminals>, id: u32, data: String) -> AppResult<()> {
    let mut sessions = state.sessions.lock().expect("terminals");
    let s = sessions.get_mut(&id).ok_or_else(|| AppError::invalid("That terminal has closed."))?;
    s.writer.write_all(data.as_bytes()).map_err(|e| err("Typing into the terminal failed", e))?;
    let _ = s.writer.flush();
    Ok(())
}

#[tauri::command]
pub fn term_resize(state: tauri::State<'_, Terminals>, id: u32, cols: u16, rows: u16) -> AppResult<()> {
    let sessions = state.sessions.lock().expect("terminals");
    if let Some(s) = sessions.get(&id) {
        let _ = s.master.resize(PtySize { rows: rows.max(2), cols: cols.max(10), pixel_width: 0, pixel_height: 0 });
    }
    Ok(())
}

/// True when the shell is waiting at its prompt (no program running in the foreground), so it is
/// safe to type a `cd` for the person.
#[tauri::command]
pub fn term_idle(state: tauri::State<'_, Terminals>, id: u32) -> bool {
    let sessions = state.sessions.lock().expect("terminals");
    let Some(s) = sessions.get(&id) else { return false };
    #[cfg(unix)]
    {
        match (s.master.process_group_leader(), s.pid) {
            (Some(fg), Some(pid)) => fg as u32 == pid,
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = s;
        true
    }
}

#[tauri::command]
pub fn term_close(state: tauri::State<'_, Terminals>, id: u32) {
    if let Some(mut s) = state.sessions.lock().expect("terminals").remove(&id) {
        let _ = s.child.kill();
    }
}

/// Closes every terminal when Stingray quits.
pub fn close_all(app: &AppHandle) {
    if let Some(t) = app.try_state::<Terminals>() {
        for (_, mut s) in t.sessions.lock().expect("terminals").drain() {
            let _ = s.child.kill();
        }
    }
}
