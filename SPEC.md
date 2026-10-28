# Stingray File Manager — Specification

Version 1.0. If code and spec disagree, fix one of them in the same change.

## 1. Purpose

A modern, easy file manager for Linux and Windows that shows local folders, cloud drives and
network drives in one window, with a macOS-inspired look.

## 2. Architecture

Tauri 2. Rust backend in `src-tauri/`, SvelteKit (SPA, adapter-static) + TypeScript in `src/`.

```
UI (splash window, main window)
  │ invoke() / events
Rust backend
  ├─ localfs   home folders, volumes, list, new folder, rename, trash, open
  ├─ nfs       saved NFS shares; connect with the OS (pkexec mount / Windows mount.exe)
  ├─ rclone    spawns and supervises `rclone rcd` (from Orbit)
  ├─ remotes   provider definitions, setup wizard, sign-in, connection test (from Orbit)
  ├─ files     remote listing and operations, transfer queue (from Orbit)
  ├─ quit      clean shutdown when the window closes
  └─ settings  theme, concurrency, splash (no secrets)
  │ HTTP 127.0.0.1:<random port>, Basic auth with a random token
rclone rcd (bundled, pinned)
```

Credentials live only in rclone's config file. Stingray's own files (`settings.json`, `nfs.json`)
hold none.

## 3. Window

- **Splash** (420×420, frameless): the logo glides in, a light sweep passes over it, then the
  wordmark and tagline. Full intro on first launch and after updates, short otherwise. Click or any
  key skips. Reduced motion shows a still frame.
- **Sidebar**: Places (home folders, Trash, volumes), Cloud (Google Drive, OneDrive, Dropbox always
  listed, plus other rclone connections), Network (SMB, SFTP, NFS, and "Connect to server…").
- **Toolbar**: back, forward, breadcrumbs (anchored at Home for local paths), new folder, hidden
  files toggle, refresh, icon/list view, search in this folder.
- **Status bar** and **transfer panel** at the bottom.

### 3.1 Connection lights

| Light | Meaning |
|---|---|
| Green `#34C759` | Set up and the last check succeeded |
| Red `#FF3B30` | Set up, but the last check failed or the computer is offline; pulses while checking |
| Grey | Not set up. Clicking opens the setup wizard |

Checks run when the engine starts, every 60 s, when the network comes back, and when a drive is
opened. Going offline turns every set-up drive red at once.

### 3.2 Drag and drop

Pointer-event drag and drop (works the same on WebKitGTK and WebView2 while Tauri's native drop
handler stays on for files from other apps). Targets: sidebar entries, folders in the view, and the
view itself. Same drive moves; different drives copy; Ctrl or Alt forces a copy. A folder can't be
dropped into itself. Every transfer goes through the shared queue.

### 3.3 Design tokens

Accent violet `#8A3FFC` into magenta `#E81CC4`, from the logo. Folder icons in the accent violet.
Light and dark themes follow the system. Font: system UI, then bundled Inter.

## 4. Platforms

Linux (AppImage, .deb) and Windows (.msi). NFS on Windows needs Client for NFS (Pro, Enterprise).

## 5. Roadmap

Tabs, previews (Quick Look style), more cloud services in the sidebar, a settings window, and a
Snap package.
