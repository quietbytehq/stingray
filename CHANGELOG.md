# Changelog

## 1.4.0 (unreleased)

- New splash: a short clip of the stingray swimming, with the name fading in over it. Plays for six
  seconds on first launch and after updates, about three seconds otherwise; click or press any key
  to skip. Reduced motion, or a system that can't play the clip, shows the logo instead.

## 1.3.0 (unreleased)

- About page in Settings, with the version, QuietByte's website and the project links.
- The splash stingray swims: its wings rise and dip as it glides in.
- Folders that take a while to open show a clear loading indicator, and after a few seconds explain
  why (with a Google Drive tip about using your own client ID).

## 1.2.0 (unreleased)

- OneDrive: setup now opens your own OneDrive ("My files") instead of the first library Microsoft
  lists; Settings → Accounts can switch to any other library on the account.
- "Sign in again" really signs in again through the browser, for Google Drive, OneDrive and Dropbox.
- A slow service shows an amber light ("Connected, but slow to respond") instead of red, and checks
  wait longer. The misleading "lost contact with its storage engine" message on slow replies is gone.
- Google Drive: a tip and an Edit settings button for using your own Google client ID.
- Sidebar places use the chosen icon theme; bigger Stingray logo.

## 1.1.0 (unreleased)

- Settings window (gear next to the logo, or Ctrl+,) with Accounts and Appearance.
- Accounts: several accounts per cloud service; add, sign in again, remove. Each shows its email,
  drive type (personal, work or school, SharePoint library) and storage used, with your own label
  and badge colour.
- Sidebar shows every account with the service's official logo; accounts of the same service get a
  coloured initial badge.
- Four icon themes: Stingray, Fluent, Cupertino and Outline. Home folders get their own pictures.
- Icon size slider (32 to 128 px), also Ctrl + and Ctrl −; switch for picture thumbnails.
- Bigger Stingray logo in the sidebar.

## 1.0.0 (2026-10-28)

First release of Stingray File Manager.

- Sidebar with Home, Desktop, Documents, Downloads, Music, Pictures, Videos, Trash and mounted volumes.
- Google Drive, OneDrive and Dropbox in the sidebar, set up with a guided wizard and browser sign-in.
- Connection lights: green when connected, red when set up but not reachable, grey when not set up.
- Network drives: SMB and SFTP through the wizard, NFS through the system's NFS client.
- Drag and drop between any two places: folders on this computer, cloud drives and network drives.
  Same drive moves, different drive copies; hold Ctrl or Alt to copy. Files dragged in from other apps are copied in.
- Copy, cut and paste; rename in place; new folder; move to Trash; search the current folder; icon and list views.
- Transfer panel with progress, speed, time left and cancel.
- Splash screen and macOS-style look in light and dark mode.
- Builds for Linux (AppImage, .deb) and Windows (.msi).
