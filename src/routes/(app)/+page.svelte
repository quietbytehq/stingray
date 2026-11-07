<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { api, asAppError } from "$lib/api";
  import type { AppError, FileEntry, NfsShareView, TransferRequest } from "$lib/api/types";
  import { drives, type CloudProvider } from "$lib/stores/drives.svelte";
  import { drag, type DragPayload } from "$lib/stores/drag.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { transfers } from "$lib/stores/transfers.svelte";
  import {
    childOf,
    crumbs,
    endpointOf,
    joinLocal,
    parentOf,
    sameDrive,
    sameLocation,
    type Location,
  } from "$lib/util/location";
  import { Plus, Columns2, SquareTerminal, Rows3, Info, ArrowLeft, ArrowRight, ChevronRight, Eye, EyeOff, FolderPlus, LayoutGrid, List, RefreshCw, Search } from "$lib/icons";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import FileView from "$lib/components/FileView.svelte";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";
  import Dialog from "$lib/components/Dialog.svelte";
  import RemoteWizard from "$lib/components/RemoteWizard.svelte";
  import ConnectServer from "$lib/components/ConnectServer.svelte";
  import SettingsDialog from "$lib/components/SettingsDialog.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import TransferPanel from "$lib/components/TransferPanel.svelte";
  import OpenWithDialog from "$lib/components/OpenWithDialog.svelte";
  import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
  import { viewPrefs, type SortKey } from "$lib/stores/viewPrefs.svelte";
  import type { Drive } from "$lib/api/types";
  import { formatBytes } from "$lib/util/format";
  import { listen } from "@tauri-apps/api/event";
  import DefaultFmDialog from "$lib/components/DefaultFmDialog.svelte";
  import BatchRenameDialog from "$lib/components/BatchRenameDialog.svelte";
  import SplitPane from "$lib/components/SplitPane.svelte";
  import FolderPickerDialog from "$lib/components/FolderPickerDialog.svelte";
  import FolderTree from "$lib/components/FolderTree.svelte";
  import TerminalPanel from "$lib/components/TerminalPanel.svelte";
  import InfoPanel from "$lib/components/InfoPanel.svelte";
  import { iconThemes } from "$lib/stores/iconThemes.svelte";
  import { age, cacheKey, getListing, putListing } from "$lib/stores/listingCache";
  import type { DefaultFm } from "$lib/api/types";

  // ---- Where we are -------------------------------------------------------------------------
  let loc = $state<Location>({ kind: "local", path: "/" });
  /** Name of the drive at the top of the breadcrumbs (Home, Google Drive, a server name). */
  let rootLabel = $state("Home");
  let back = $state<{ loc: Location; label: string }[]>([]);
  let forward = $state<{ loc: Location; label: string }[]>([]);

  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  const onGoogle = $derived(
    loc.kind === "remote" && drives.remotes.find((r) => loc.kind === "remote" && r.name === loc.remote)?.providerId === "drive",
  );
  /** True once a folder has been loading for a while, to explain the wait. */
  let slowLoad = $state(false);
  $effect(() => {
    if (!loading) {
      slowLoad = false;
      return;
    }
    const timer = window.setTimeout(() => (slowLoad = true), 6000);
    return () => window.clearTimeout(timer);
  });
  let error = $state<AppError | null>(null);
  let notice = $state<string | null>(null);

  let mode = $state<"grid" | "list" | "compact">("grid");
  /** Recent and Starred are lists, not folders; while one is shown it replaces the folder listing. */
  let virtualView = $state<null | "recent" | "starred">(null);
  let openWithPath = $state<string | null>(null);
  let askDefault = $state<DefaultFm | null>(null);
  let batchRename = $state<FileEntry[] | null>(null);

  // ---- Tabs ----------------------------------------------------------------------------------
  /** Everything a tab remembers. The window shows one tab at a time; switching swaps these in. */
  interface TabState {
    loc: Location;
    rootLabel: string;
    back: { loc: Location; label: string }[];
    forward: { loc: Location; label: string }[];
    virtualView: null | "recent" | "starred";
  }
  let tabs = $state<TabState[]>([]);
  let activeTab = $state(0);
  let closedTabs = $state<TabState[]>([]);

  function snapshot(): TabState {
    return { loc, rootLabel, back: [...back], forward: [...forward], virtualView };
  }

  function applyTab(tab: TabState) {
    loc = tab.loc;
    rootLabel = tab.rootLabel;
    back = [...tab.back];
    forward = [...tab.forward];
    virtualView = tab.virtualView;
    selected = new Set();
    renaming = null;
    query = "";
    void load();
  }

  function switchTab(index: number) {
    if (index === activeTab || index < 0 || index >= tabs.length) return;
    tabs[activeTab] = snapshot();
    activeTab = index;
    applyTab(tabs[index]);
  }

  function newTab(at?: Location, label?: string) {
    if (!tabs.length) tabs = [snapshot()];
    tabs[activeTab] = snapshot();
    const start = at ?? (home ? { kind: "local" as const, path: home.path } : loc);
    tabs = [...tabs, { loc: start, rootLabel: label ?? (at ? rootLabel : (home?.label ?? "Home")), back: [], forward: [], virtualView: null }];
    activeTab = tabs.length - 1;
    applyTab(tabs[activeTab]);
  }

  function closeTab(index: number) {
    if (tabs.length <= 1) return;
    const closing = index === activeTab ? snapshot() : tabs[index];
    closedTabs = [...closedTabs.slice(-19), closing];
    tabs = tabs.filter((_, i) => i !== index);
    if (index === activeTab) {
      activeTab = Math.min(index, tabs.length - 1);
      applyTab(tabs[activeTab]);
    } else if (index < activeTab) {
      activeTab -= 1;
    }
  }

  function reopenTab() {
    const tab = closedTabs.at(-1);
    if (!tab) return;
    closedTabs = closedTabs.slice(0, -1);
    if (!tabs.length) tabs = [snapshot()];
    tabs[activeTab] = snapshot();
    tabs = [...tabs, tab];
    activeTab = tabs.length - 1;
    applyTab(tab);
  }

  /** A tab's title: its folder name. The active tab follows the live breadcrumbs. */
  function tabTitle(tab: TabState, index: number): string {
    if (index === activeTab) return trail.at(-1)?.label ?? rootLabel;
    if (tab.virtualView) return tab.virtualView === "recent" ? "Recent" : "Starred";
    const p = tab.loc.path;
    return (p.split(/[\\/]/).filter(Boolean).pop() ?? "").replace(/!$/, "") || tab.rootLabel;
  }

  // ---- Split view ----------------------------------------------------------------------------
  let split = $state(false);
  let splitStart = $state<Location | null>(null);
  let splitFocused = $state(false);

  function toggleSplit() {
    split = !split;
    if (split) splitStart = { ...loc };
  }

  // ---- Undo / Redo ---------------------------------------------------------------------------
  /** One reversible step: what it was called, and how to undo and redo it. */
  interface Step {
    label: string;
    undo: () => Promise<void>;
    redo: () => Promise<void>;
  }
  let undoStack = $state<Step[]>([]);
  let redoStack = $state<Step[]>([]);

  function record(step: Step) {
    undoStack = [...undoStack.slice(-49), step];
    redoStack = [];
  }

  async function undo() {
    const step = undoStack.at(-1);
    if (!step) return;
    undoStack = undoStack.slice(0, -1);
    try {
      await step.undo();
      redoStack = [...redoStack, step];
      notice = `Undid: ${step.label}`;
    } catch (err) {
      error = asAppError(err);
    }
    await load();
  }

  async function redo() {
    const step = redoStack.at(-1);
    if (!step) return;
    redoStack = redoStack.slice(0, -1);
    try {
      await step.redo();
      undoStack = [...undoStack, step];
      notice = `Redid: ${step.label}`;
    } catch (err) {
      error = asAppError(err);
    }
    await load();
  }

  const baseName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
  const parentPath = (p: string) => p.slice(0, Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"))) || "/";

  // ---- Archives ----------------------------------------------------------------------------
  const ARCHIVE = /\.(zip|jar|cbz|tar|tgz|tar\.gz)$/i;
  const inArchive = $derived(loc.kind === "local" && loc.path.includes("!/"));
  const isArchiveFile = (e: FileEntry) => !e.isDir && ARCHIVE.test(e.name);

  async function extract(entry: FileEntry) {
    try {
      const folder = await api.extractArchive(entry.path);
      notice = `Extracted to ${baseName(folder)}`;
      record({ label: `Extract ${entry.name}`, undo: () => api.localTrash([folder]), redo: async () => void (await api.extractArchive(entry.path)) });
      if (!inArchive) await load();
    } catch (err) {
      error = asAppError(err);
    }
  }

  /** Renames several items at once, as one step that undo can reverse. */
  async function applyBatchRename(renames: { entry: FileEntry; name: string }[]) {
    batchRename = null;
    const done: { from: string; to: string }[] = [];
    try {
      for (const { entry, name } of renames) {
        const to = await api.localRename(entry.path, name);
        done.push({ from: entry.path, to });
      }
    } catch (err) {
      error = asAppError(err);
    }
    if (done.length) {
      record({
        label: `Rename ${done.length} items`,
        undo: async () => {
          for (const d of [...done].reverse()) await api.localRename(d.to, baseName(d.from));
        },
        redo: async () => {
          for (const d of done) await api.localRename(d.from, baseName(d.to));
        },
      });
    }
    await load();
  }

  /** Opens a folder handed over by another app or a second launch; a file opens its folder, selected. */
  async function openGivenPath(path: string) {
    try {
      const info = await api.fileProperties(path);
      if (info.isDir) {
        go({ kind: "local", path });
      } else {
        go({ kind: "local", path: info.location });
        selected = new Set([path]);
      }
    } catch (err) {
      error = asAppError(err);
    }
  }
  let propertiesPath = $state<string | null>(null);
  let showHidden = $state(false);
  let query = $state("");
  let selected = $state<Set<string>>(new Set());
  let renaming = $state<string | null>(null);

  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  let wizardFor = $state<CloudProvider | null>(null);
  let connectServer = $state(false);
  /** Open Settings section, or null when Settings is closed. */
  let settingsAt = $state<string | null>(null);
  let clipboard = $state<{ payload: DragPayload; cut: boolean } | null>(null);
  let externalOver = $state(false);

  const home = $derived(drives.places.find((p) => p.kind === "home"));

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return entries
      .filter((e) => showHidden || !e.name.startsWith("."))
      .filter((e) => q === "" || e.name.toLowerCase().includes(q))
      .sort(viewPrefs.compare);
  });

  const trail = $derived(
    virtualView
      ? [{ label: virtualView === "recent" ? "Recent" : "Starred", loc }]
      : crumbs(loc, rootLabel, loc.kind === "local" && home ? { path: home.path, label: home.label } : undefined),
  );
  const selectedEntries = $derived(visible.filter((e) => selected.has(e.path)));

  /** Like Ubuntu's Files: “name” selected (size), or N items selected (total). */
  let folderCount = $state<number | null>(null);
  $effect(() => {
    folderCount = null;
    const only = selectedEntries.length === 1 ? selectedEntries[0] : null;
    if (only?.isDir && loc.kind === "local") {
      api.listLocal(only.path).then((items) => (folderCount = items.length)).catch(() => {});
    }
  });
  const statusText = $derived.by(() => {
    const n = selectedEntries.length;
    if (n === 0) return `${visible.length} ${visible.length === 1 ? "item" : "items"}`;
    if (n === 1) {
      const e = selectedEntries[0];
      if (e.isDir) return folderCount === null ? `“${e.name}” selected` : `“${e.name}” selected (containing ${folderCount} ${folderCount === 1 ? "item" : "items"})`;
      return `“${e.name}” selected${e.size >= 0 ? ` (${formatBytes(e.size)})` : ""}`;
    }
    const total = selectedEntries.filter((e) => !e.isDir && e.size >= 0).reduce((t, e) => t + e.size, 0);
    return `${n} items selected${total ? ` (${formatBytes(total)})` : ""}`;
  });

  // ---- Loading ------------------------------------------------------------------------------
  let loadToken = 0;
  /** Set while a cached cloud listing is shown and the fresh one is on its way. */
  let refreshingSince = $state<number | null>(null);

  async function load() {
    const token = ++loadToken;
    loading = true;
    error = null;
    refreshingSince = null;
    // Cloud and network folders: show the remembered listing at once, then refresh it quietly.
    let cached = null as Awaited<ReturnType<typeof getListing>>;
    if (!virtualView && loc.kind === "remote") {
      const here = loc;
      cached = await getListing(cacheKey(here.remote, here.path));
      if (token !== loadToken) return;
      if (cached) {
        entries = cached.entries;
        refreshingSince = cached.at;
      }
    }
    try {
      const result = virtualView
        ? await virtualEntries(virtualView)
        : loc.kind === "local"
          ? await api.listLocal(loc.path)
          : await api.listFiles(loc.remote, loc.path);
      if (token !== loadToken) return;
      entries = result;
      if (loc.kind === "remote") {
        drives.lights[loc.remote] = "connected";
        void putListing(cacheKey(loc.remote, loc.path), result);
      }
    } catch (err) {
      if (token !== loadToken) return;
      if (cached) {
        // Keep showing what we had; say how old it is instead of emptying the folder.
        notice = `Couldn't reach ${rootLabel} right now. Showing files as they were ${age(cached.at)}.`;
      } else {
        entries = [];
        error = asAppError(err);
      }
      if (loc.kind === "remote") drives.lights[loc.remote] = asAppError(err).kind === "timeout" ? "slow" : "offline";
    } finally {
      if (token === loadToken) {
        loading = false;
        refreshingSince = null;
      }
    }
  }

  async function warmCache() {
    for (const remote of drives.remotes) {
      try {
        const top = await api.listFiles(remote.name, "");
        await putListing(cacheKey(remote.name, ""), top);
        for (const dir of top.filter((e) => e.isDir).slice(0, 12)) {
          try {
            await putListing(cacheKey(remote.name, dir.path), await api.listFiles(remote.name, dir.path));
          } catch {
            break; // Slow or offline: stop warming this drive.
          }
        }
      } catch {
        // Not connected; it will be cached when opened.
      }
    }
  }

  /** Entries for Recent (files opened lately on this desktop) or Starred. */
  async function virtualEntries(view: "recent" | "starred"): Promise<FileEntry[]> {
    const name = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
    if (view === "recent") {
      const recent = await api.recentFiles();
      return recent.map((r) => ({ name: name(r.path), path: r.path, isDir: false, size: -1, modTime: r.modified || null, mimeType: null }));
    }
    const out: FileEntry[] = [];
    for (const path of viewPrefs.starred) {
      try {
        const p = await api.fileProperties(path);
        out.push({ name: p.name, path, isDir: p.isDir, size: p.isDir ? -1 : p.size, modTime: p.modified, mimeType: null });
      } catch {
        // Moved or deleted since it was starred.
      }
    }
    return out;
  }

  function showVirtual(view: "recent" | "starred") {
    virtualView = view;
    selected = new Set();
    query = "";
    void load();
  }

  async function openDrive(drive: Drive) {
    try {
      const at = drive.mountPoint ?? (await api.mountDrive(drive.device));
      go({ kind: "local", path: at }, drive.label);
    } catch (err) {
      error = asAppError(err);
    }
  }

  function go(next: Location, label?: string, record = true) {
    const wasVirtual = virtualView !== null;
    virtualView = null;
    if (!wasVirtual && sameLocation(next, loc) && entries.length) return;
    if (record) {
      back = [...back, { loc, label: rootLabel }];
      forward = [];
    }
    if (label) rootLabel = label;
    else if (next.kind === "local" && loc.kind !== "local") rootLabel = "Computer";
    loc = next;
    selected = new Set();
    renaming = null;
    query = "";
    void load();
  }

  function goBack() {
    const prev = back.at(-1);
    if (!prev) return;
    back = back.slice(0, -1);
    forward = [...forward, { loc, label: rootLabel }];
    go(prev.loc, prev.label, false);
  }

  function goForward() {
    const next = forward.at(-1);
    if (!next) return;
    forward = forward.slice(0, -1);
    back = [...back, { loc, label: rootLabel }];
    go(next.loc, next.label, false);
  }

  function goUp() {
    const up = parentOf(loc);
    if (up) go(up);
  }

  // ---- Opening ------------------------------------------------------------------------------
  async function open(entry: FileEntry) {
    if (entry.isDir) {
      go(childOf(loc, entry));
      return;
    }
    if (loc.kind === "local") {
      if (entry.path.includes("!/")) {
        // A file inside an archive: unpack just that file to a temporary folder, then open it.
        try {
          await api.openLocal(await api.openFromArchive(entry.path));
        } catch (err) {
          error = asAppError(err);
        }
        return;
      }
      if (isArchiveFile(entry)) {
        go({ kind: "local", path: `${entry.path}!/` }, undefined);
        return;
      }
      await api.openLocal(entry.path).catch((err) => (error = asAppError(err)));
      return;
    }
    // Files on a cloud or network drive are downloaded first, to Downloads.
    const downloads = drives.places.find((p) => p.kind === "downloads") ?? home;
    if (!downloads) return;
    await queue([request({ from: loc, entries: [entry] }, { kind: "local", path: downloads.path }, false, entry)]);
    notice = `Downloading ${entry.name} to ${downloads.label}.`;
  }

  async function openNfs(share: NfsShareView) {
    try {
      const at = share.connectedAt ?? (await api.connectNfsShare(share.name));
      await drives.refresh();
      go({ kind: "local", path: at }, share.name);
    } catch (err) {
      error = asAppError(err);
    }
  }

  // ---- Transfers (drag and drop, copy and paste) --------------------------------------------
  function request(payload: DragPayload, to: Location, move: boolean, entry: FileEntry): TransferRequest {
    return {
      source: endpointOf(payload.from),
      destination: endpointOf(to),
      mode: move ? "move" : "copy",
      name: entry.name,
      isDir: entry.isDir,
    };
  }

  async function queue(requests: TransferRequest[]) {
    if (requests.length === 0) return;
    await transfers.enqueue(requests);
    if (transfers.error) error = transfers.error;
  }

  /** A folder can't be put inside itself or its own subfolders. */
  function intoItself(payload: DragPayload, to: Location, entry: FileEntry): boolean {
    const inside = childOf(payload.from, entry);
    if (to.kind !== inside.kind) return false;
    if (to.kind === "local" && inside.kind === "local") return to.path === inside.path || to.path.startsWith(inside.path + "/") || to.path.startsWith(inside.path + "\\");
    if (to.kind === "remote" && inside.kind === "remote") return to.remote === inside.remote && (to.path === inside.path || to.path.startsWith(inside.path + "/"));
    return false;
  }

  async function transfer(payload: DragPayload, to: Location, forceCopy: boolean, forceMove = false) {
    if (sameLocation(payload.from, to)) return;
    // Like Finder: on the same drive a drag moves, across drives it copies. Ctrl or Alt copies.
    const fromArchive = payload.from.kind === "local" && payload.from.path.includes("!/");
    if (fromArchive || (to.kind === "local" && to.path.includes("!/"))) {
      notice = fromArchive ? "Archives are read-only here: use Extract to take files out." : "Stingray can't add files to an archive.";
      return;
    }
    const move = forceMove || (!forceCopy && sameDrive(payload.from, to));
    const items = payload.entries.filter((e) => !intoItself(payload, to, e));
    if (items.length < payload.entries.length) notice = "A folder can't go inside itself, so it was skipped.";
    await queue(items.map((e) => request(payload, to, move, e)));
    if (payload.from.kind === "local" && to.kind === "local" && items.length) {
      const fromDir = payload.from.path;
      const toDir = to.path;
      const names = items.map((e) => e.name);
      const back = (dir: string) => ({ kind: "local" as const, path: dir });
      const what = names.length === 1 ? names[0] : `${names.length} items`;
      record(
        move
          ? {
              label: `Move ${what}`,
              undo: () => queue(items.map((e) => request({ from: back(toDir), entries: [e] }, back(fromDir), true, e))),
              redo: () => queue(items.map((e) => request({ from: back(fromDir), entries: [e] }, back(toDir), true, e))),
            }
          : {
              label: `Copy ${what}`,
              undo: () => api.localTrash(names.map((n) => joinLocal(toDir, n))),
              redo: () => queue(items.map((e) => request({ from: back(fromDir), entries: [e] }, back(toDir), false, e))),
            },
      );
    }
  }

  drag.handleDrop((payload, to, copy) => void transfer(payload, to, copy));

  function copySelection(cut: boolean) {
    if (selectedEntries.length === 0) return;
    clipboard = { payload: { from: loc, entries: selectedEntries }, cut };
    notice = `${selectedEntries.length === 1 ? selectedEntries[0].name : `${selectedEntries.length} items`} ${cut ? "ready to move" : "copied"}. Paste in another folder.`;
  }

  async function paste() {
    if (!clipboard) return;
    const { payload, cut } = clipboard;
    await transfer(payload, loc, !cut, cut);
    if (cut) clipboard = null;
  }

  // Reload the folder on screen when a transfer into or out of it finishes.
  let lastActive = 0;
  $effect(() => {
    const active = transfers.active.length;
    if (active < lastActive) void load();
    lastActive = active;
  });

  // ---- Everyday operations ------------------------------------------------------------------
  async function newFolder() {
    const base = "untitled folder";
    let name = base;
    for (let n = 2; entries.some((e) => e.name === name); n++) name = `${base} ${n}`;
    try {
      if (loc.kind === "local") {
        const path = await api.localCreateFolder(loc.path, name);
        const parent = loc.path;
        record({ label: `New folder`, undo: () => api.localTrash([path]), redo: async () => void (await api.localCreateFolder(parent, baseName(path))) });
        await load();
        renaming = path;
      } else {
        const path = loc.path ? `${loc.path}/${name}` : name;
        await api.createFolder(loc.remote, path);
        await load();
        renaming = path;
      }
      selected = new Set([renaming ?? ""]);
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function rename(entry: FileEntry, name: string | null) {
    renaming = null;
    const next = name?.trim();
    if (!next || next === entry.name) return;
    try {
      if (loc.kind === "local") {
        const to = await api.localRename(entry.path, next);
        const from = entry.path;
        record({ label: `Rename ${entry.name}`, undo: async () => void (await api.localRename(to, baseName(from))), redo: async () => void (await api.localRename(from, next)) });
      } else await api.renameItem(loc.remote, entry.path, entry.isDir, next);
      await load();
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function remove(items: FileEntry[]) {
    if (items.length === 0) return;
    const what = items.length === 1 ? `"${items[0].name}"` : `${items.length} items`;
    try {
      if (loc.kind === "local") {
        const paths = items.map((e) => e.path);
        await api.localTrash(paths);
        notice = `Moved ${what} to the Trash. Ctrl+Z puts it back.`;
        record({ label: `Move ${what} to the Trash`, undo: () => api.restoreFromTrash(paths), redo: () => api.localTrash(paths) });
      } else {
        if (!window.confirm(`Delete ${what} from ${rootLabel}? This can't be undone from Stingray.`)) return;
        await api.deleteItems(loc.remote, items.map((e) => ({ path: e.path, isDir: e.isDir })));
      }
      selected = new Set();
      await load();
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function copyPath(entry: FileEntry) {
    const text = loc.kind === "local" ? entry.path : `${loc.remote}:${entry.path}`;
    await navigator.clipboard.writeText(text).catch(() => {});
  }

  // ---- Menus --------------------------------------------------------------------------------
  function showMenu(event: MouseEvent, items: MenuItem[]) {
    event.preventDefault();
    menu = { x: event.clientX, y: event.clientY, items };
  }

  /** Move to… / Copy to…: Stingray's own picker, so cloud, network and devices are destinations too. */
  let sendPicker = $state<{ items: FileEntry[]; move: boolean; from: Location } | null>(null);
  function sendTo(items: FileEntry[], move: boolean) {
    sendPicker = { items, move, from: loc };
  }

  /** The Folders panel (F7): a tree beside the file view. */
  let showTree = $state(false);
  /** The terminal panel (F4). */
  let showTerminal = $state(false);
  /** The information panel (F11). */
  let showInfo = $state(false);
  const terminalCwd = $derived(loc.kind === "local" && !loc.path.includes("!/") ? loc.path : (home?.path ?? ""));

  async function compressItems(items: FileEntry[]) {
    const suggested = items.length === 1 ? items[0].name.replace(/\.[^.]+$/, "") : "Archive";
    const name = window.prompt("Archive name", suggested);
    if (!name) return;
    try {
      await api.compress(items.map((e) => e.path), name);
      notice = `Created ${name.replace(/\.zip$/, "")}.zip`;
      await load();
    } catch (err) {
      error = asAppError(err);
    }
  }

  function itemMenu(event: MouseEvent, entry: FileEntry) {
    const many = selected.size > 1;
    const items = many ? selectedEntries : [entry];
    const local = loc.kind === "local";
    const starred = items.every((e) => viewPrefs.isStarred(e.path));
    showMenu(event, [
      { label: entry.isDir ? "Open" : local ? "Open" : "Download", action: () => void open(entry), disabled: many },
      { label: "Open With…", action: () => (openWithPath = entry.path), disabled: many || !local || entry.isDir },
      { label: "Open in New Tab", action: () => newTab(childOf(loc, entry), entry.name), disabled: many || !entry.isDir },
      { label: "Open in Split View", action: () => { split = true; splitStart = childOf(loc, entry); }, disabled: many || !entry.isDir },
      { label: "Cut", action: () => copySelection(true), separator: true },
      { label: "Copy", action: () => copySelection(false) },
      { label: "Move to…", action: () => void sendTo(items, true) },
      { label: "Copy to…", action: () => void sendTo(items, false) },
      { label: many ? `Rename ${items.length} items…` : "Rename…", action: () => (many ? (batchRename = items) : (renaming = entry.path)), disabled: inArchive || (many && !local), separator: true },
      { label: "Extract Here", action: () => void extract(entry), disabled: many || !local || !(isArchiveFile(entry) || inArchive) },
      { label: "Add to Places", action: () => viewPrefs.addBookmark(entry.path, entry.name), disabled: many || !local || !entry.isDir || inArchive },
      {
        label: "Paste Into Folder",
        action: () => clipboard && void transfer(clipboard.payload, childOf(loc, entry), !clipboard.cut, clipboard.cut),
        disabled: !clipboard || many || !entry.isDir,
      },
      { label: "Compress…", action: () => void compressItems(items), disabled: !local || inArchive },
      { label: "Email…", action: () => void api.emailFiles(items.map((e) => e.path)).catch((err) => (error = asAppError(err))), disabled: !local || items.some((e) => e.isDir) },
      { label: starred ? "Remove from Starred" : "Star", action: () => { viewPrefs.toggleStar(items.map((e) => e.path)); if (virtualView === "starred") void load(); }, disabled: !local },
      { label: "Copy path", action: () => void copyPath(entry), disabled: many },
      { label: local ? "Move to Trash" : "Delete…", destructive: true, separator: true, action: () => void remove(items), disabled: inArchive },
      { label: "Properties", action: () => (propertiesPath = entry.path), disabled: many || !local, separator: true },
    ]);
  }

  const SORT_LABELS: [SortKey, string][] = [["name", "Name"], ["size", "Size"], ["type", "Type"], ["modified", "Modified"]];
  const tick = (on: boolean, label: string) => (on ? `✓  ${label}` : `     ${label}`);

  function backgroundMenu(event: MouseEvent) {
    const s = viewPrefs.sort;
    showMenu(event, [
      { label: undoStack.length ? `Undo ${undoStack.at(-1)?.label}` : "Undo", action: () => void undo(), disabled: !undoStack.length },
      { label: redoStack.length ? `Redo ${redoStack.at(-1)?.label}` : "Redo", action: () => void redo(), disabled: !redoStack.length },
      { label: "New folder", action: () => void newFolder(), disabled: virtualView !== null || inArchive, separator: true },
      { label: "Paste", action: () => void paste(), disabled: !clipboard || virtualView !== null || inArchive },
      ...SORT_LABELS.map(([key, label], i) => ({
        label: tick(s.key === key, `Arrange by ${label}`),
        action: () => viewPrefs.setSort({ key }),
        separator: i === 0,
      })),
      { label: tick(!s.descending, "Ascending"), action: () => viewPrefs.setSort({ descending: false }), separator: true },
      { label: tick(s.descending, "Descending"), action: () => viewPrefs.setSort({ descending: true }) },
      { label: tick(s.foldersFirst, "Folders first"), action: () => viewPrefs.setSort({ foldersFirst: !s.foldersFirst }) },
      { label: showHidden ? "Hide hidden files" : "Show hidden files", action: () => (showHidden = !showHidden), separator: true },
      { label: "Refresh", action: () => void load() },
    ]);
  }

  // ---- Keyboard -----------------------------------------------------------------------------
  function keydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (target.tagName === "INPUT" || menu || wizardFor || connectServer || settingsAt) return;
    const zoom = event.ctrlKey && (event.key === "+" || event.key === "=" || event.key === "-");
    if (zoom && settings.value) {
      const step = event.key === "-" ? -8 : 8;
      const next = Math.max(32, Math.min(128, settings.value.iconSize + step));
      if (next !== settings.value.iconSize) void settings.patch({ iconSize: next });
      event.preventDefault();
      return;
    }
    if (event.ctrlKey && event.key === ",") {
      settingsAt = "accounts";
      event.preventDefault();
      return;
    }
    const ctrl = event.ctrlKey || event.metaKey;
    const single = selectedEntries.length === 1 ? selectedEntries[0] : null;
    if (event.altKey && event.key === "ArrowLeft") goBack();
    else if (event.altKey && event.key === "ArrowRight") goForward();
    else if ((event.altKey && event.key === "ArrowUp") || event.key === "Backspace") goUp();
    else if (event.key === "Enter" && single) void open(single);
    else if (event.key === "F3") toggleSplit();
    else if (event.key === "F7") showTree = !showTree;
    else if (event.key === "F4") showTerminal = !showTerminal;
    else if (event.key === "F11") {
      event.preventDefault();
      showInfo = !showInfo;
    }
    else if (ctrl && event.shiftKey && event.key.toLowerCase() === "t") reopenTab();
    else if (ctrl && !event.shiftKey && event.key.toLowerCase() === "t") newTab();
    else if (ctrl && event.key.toLowerCase() === "w") {
      if (tabs.length > 1) closeTab(activeTab);
    } else if (ctrl && event.key === "Tab") switchTab((activeTab + (event.shiftKey ? tabs.length - 1 : 1)) % Math.max(tabs.length, 1));
    else if (event.altKey && /^[1-9]$/.test(event.key)) switchTab(Number(event.key) - 1);
    else if (event.key === "F2" && single) renaming = single.path;
    else if (event.key === "F2" && selectedEntries.length > 1 && loc.kind === "local" && !inArchive) batchRename = selectedEntries;
    else if (ctrl && !event.shiftKey && event.key.toLowerCase() === "z") void undo();
    else if (ctrl && ((event.shiftKey && event.key.toLowerCase() === "z") || event.key.toLowerCase() === "y")) void redo();
    else if (event.key === "Delete") void remove(selectedEntries);
    else if (event.key === "F5") void load();
    else if (ctrl && event.shiftKey && event.key.toLowerCase() === "n") void newFolder();
    else if (ctrl && event.key.toLowerCase() === "h") showHidden = !showHidden;
    else if (ctrl && event.key.toLowerCase() === "a") selected = new Set(visible.map((e) => e.path));
    else if (ctrl && event.key.toLowerCase() === "c") copySelection(false);
    else if (ctrl && event.key.toLowerCase() === "x") copySelection(true);
    else if (ctrl && event.key.toLowerCase() === "v") void paste();
    else if (ctrl && event.key.toLowerCase() === "f") document.getElementById("search")?.focus();
    else return;
    event.preventDefault();
  }

  // ---- Start-up -----------------------------------------------------------------------------
  onMount(() => {
    let unlisten: (() => void) | undefined;
    let stopOpenPath: (() => void) | undefined;
    void listen<string>("desktop://open-path", (event) => void openGivenPath(event.payload)).then((stop) => (stopOpenPath = stop));
    void (async () => {
      await drives.start();
      const given = await api.takeInitialPath().catch(() => null);
      if (given) await openGivenPath(given);
      else if (home) go({ kind: "local", path: home.path }, home.label, false);
      tabs = [snapshot()];
      // Warm the cache: quietly list the top of each cloud drive and its first folders.
      window.setTimeout(() => void warmCache(), 4000);
      // First start: offer to become the default file manager (once, unless postponed).
      let asked = false;
      try {
        asked = localStorage.getItem("stingray.askedDefault") === "1";
      } catch {
        asked = false;
      }
      if (!asked) {
        const status = await api.defaultFileManager().catch(() => null);
        if (status && !status.isDefault) askDefault = status;
      }
    })();
    // Files dragged in from other apps arrive here with their real paths.
    getCurrentWebview()
      .onDragDropEvent(async (event) => {
        const kind = event.payload.type;
        if (kind === "enter" || kind === "over") externalOver = true;
        else if (kind === "leave") externalOver = false;
        else if (kind === "drop") {
          externalOver = false;
          try {
            const items = await api.inspectLocalPaths(event.payload.paths);
            await queue(
              items.map((item) => ({
                source: { kind: "local", path: item.parent },
                destination: endpointOf(loc),
                mode: "copy",
                name: item.name,
                isDir: item.isDir,
              })),
            );
          } catch (err) {
            error = asAppError(err);
          }
        }
      })
      .then((stop) => (unlisten = stop))
      .catch(() => {});
    return () => {
      unlisten?.();
      stopOpenPath?.();
      drives.stop();
    };
  });

  // Load drives once the storage engine is ready, and again whenever it restarts.
  $effect(() => {
    if (rcd.state.state === "healthy") void drives.refresh();
  });

  $effect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => (notice = null), 4000);
    return () => window.clearTimeout(timer);
  });

  const cloudTitles: Record<CloudProvider, string> = { drive: "Google Drive", onedrive: "OneDrive", dropbox: "Dropbox" };
</script>

<svelte:window onkeydown={keydown} />

<div class="window" class:dragging={drag.payload !== null}>
  <Sidebar
    current={loc}
    onNavigate={(next, label) => go(next, label)}
    onSetUpCloud={(provider) => (wizardFor = provider)}
    onConnectServer={() => (connectServer = true)}
    onOpenNfs={(share) => void openNfs(share)}
    onMenu={showMenu}
    onSettings={(section) => (settingsAt = section ?? "accounts")}
    virtual={virtualView}
    onVirtual={showVirtual}
    onOpenDrive={(drive) => void openDrive(drive)}
  />

  <section class="main">
    {#if tabs.length > 1}
      <div class="tabs" role="tablist" aria-label="Tabs">
        {#each tabs as tab, i (i)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="tab"
            role="tab"
            tabindex="0"
            aria-selected={i === activeTab}
            onclick={() => switchTab(i)}
            onauxclick={(e) => { if (e.button === 1) closeTab(i); }}
            onkeydown={(e) => { if (e.key === "Enter") switchTab(i); }}
          >
            <span class="tab-title">{tabTitle(tab, i)}</span>
            <button type="button" class="tab-close" aria-label="Close tab" onclick={(e) => { e.stopPropagation(); closeTab(i); }}>×</button>
          </div>
        {/each}
        <button type="button" class="tab-new" title="New tab (Ctrl+T)" aria-label="New tab" onclick={() => newTab()}>+</button>
      </div>
    {/if}
    <header class="toolbar">
      <div class="nav">
        <button type="button" class="tool" onclick={goBack} disabled={back.length === 0} title="Back (Alt+Left)" aria-label="Back">
          <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button type="button" class="tool" onclick={goForward} disabled={forward.length === 0} title="Forward (Alt+Right)" aria-label="Forward">
          <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
      <nav class="trail" aria-label="Folder path">
        {#each trail as crumb, i (i)}
          {#if i > 0}<ChevronRight size={13} strokeWidth={2} aria-hidden="true" />{/if}
          <button type="button" class="crumb" class:last={i === trail.length - 1} onclick={() => go(crumb.loc)}>{crumb.label}</button>
        {/each}
      </nav>
      <div class="tools">
        <button type="button" class="tool" onclick={() => void newFolder()} title="New folder (Ctrl+Shift+N)" aria-label="New folder">
          <FolderPlus size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button type="button" class="tool" onclick={() => (showHidden = !showHidden)} title={showHidden ? "Hide hidden files (Ctrl+H)" : "Show hidden files (Ctrl+H)"} aria-pressed={showHidden}>
          {#if showHidden}<Eye size={16} strokeWidth={1.75} aria-hidden="true" />{:else}<EyeOff size={16} strokeWidth={1.75} aria-hidden="true" />{/if}
        </button>
        <button type="button" class="tool" onclick={() => void load()} title="Refresh (F5)" aria-label="Refresh">
          <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" class={loading ? "spin" : ""} />
        </button>
        <button type="button" class="tool" onclick={() => newTab()} title="New tab (Ctrl+T)" aria-label="New tab">
          <Plus size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button type="button" class="tool" onclick={() => (showInfo = !showInfo)} title="Information (F11)" aria-label="Information" aria-pressed={showInfo}>
          <Info size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button type="button" class="tool" onclick={() => (showTerminal = !showTerminal)} title="Terminal (F4)" aria-label="Terminal" aria-pressed={showTerminal}>
          <SquareTerminal size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button type="button" class="tool" onclick={toggleSplit} title="Split view (F3)" aria-label="Split view" aria-pressed={split}>
          <Columns2 size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <div class="segmented" role="group" aria-label="View">
          <button type="button" aria-pressed={mode === "grid"} onclick={() => (mode = "grid")} title="Icons" aria-label="Icons">
            <LayoutGrid size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
          <button type="button" aria-pressed={mode === "compact"} onclick={() => (mode = "compact")} title="Compact" aria-label="Compact">
            <Rows3 size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
          <button type="button" aria-pressed={mode === "list"} onclick={() => (mode = "list")} title="List" aria-label="List">
            <List size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
        <label class="search">
          <Search size={14} strokeWidth={1.75} aria-hidden="true" />
          <input id="search" type="search" placeholder="Search this folder" bind:value={query} spellcheck="false" />
        </label>
      </div>
    </header>

    {#if error}
      <div class="banner"><ErrorNote {error} /></div>
    {/if}

    <div
      class="panes"
      style:grid-template-columns={[showTree ? "230px" : "", "minmax(0, 1fr)", split ? "minmax(0, 1fr)" : "", showInfo ? "280px" : ""].filter(Boolean).join(" ")}
    >
      {#if showTree}
        <aside class="tree-panel" aria-label="Folders">
          <div class="tree-head">
            <span>Folders</span>
            <button type="button" class="tab-close" aria-label="Close Folders panel" title="Close (F7)" onclick={() => (showTree = false)}>×</button>
          </div>
          <FolderTree compact selected={loc} onSelect={(l, label) => go(l, label)} />
        </aside>
      {/if}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="left" onpointerdown={() => (splitFocused = false)}>
    {#if loading && entries.length === 0}
      <div class="empty" role="status">
        <span class="loader" aria-hidden="true"></span>
        <p>Opening {trail.at(-1)?.label ?? "folder"}…</p>
        {#if slowLoad}
          <p class="hint">
            This service is answering slowly.{#if onGoogle}
              Google limits apps that share a sign-in; adding your own Google client ID in Settings → Accounts makes it fast.{/if}
          </p>
        {/if}
      </div>
    {:else if !error && !loading && visible.length === 0}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="empty" oncontextmenu={(e) => { e.preventDefault(); backgroundMenu(e); }}>
        <img src="/stingray.png" alt="" width="88" height="88" />
        <p>{query ? `Nothing here matches "${query}".` : "This folder is empty. Drag files here, or paste with Ctrl+V."}</p>
      </div>
    {:else}
      <FileView
        {loc}
        sortKey={viewPrefs.sort.key}
        sortDescending={viewPrefs.sort.descending}
        onSort={(key) => viewPrefs.sortBy(key)}
        entries={visible}
        {mode}
        bind:selected
        {renaming}
        iconSize={settings.value?.iconSize ?? 56}
        thumbnails={settings.value?.showThumbnails ?? true}
        onOpen={(entry) => void open(entry)}
        onItemMenu={itemMenu}
        onBackgroundMenu={backgroundMenu}
        onRename={(entry, name) => void rename(entry, name)}
      />
    {/if}
      </div>
      {#if split && splitStart}
        {#key splitStart}
          <SplitPane
            start={splitStart}
            label={rootLabel}
            focused={splitFocused}
            onFocus={() => (splitFocused = true)}
            onClose={() => (split = false)}
            onMenu={showMenu}
            canPaste={!!clipboard}
            onPaste={(to) => clipboard && void transfer(clipboard.payload, to, !clipboard.cut, clipboard.cut)}
          />
        {/key}
      {/if}
      {#if showInfo}
        <InfoPanel entry={selectedEntries.length === 1 ? selectedEntries[0] : null} count={selectedEntries.length} local={loc.kind === "local"} onClose={() => (showInfo = false)} />
      {/if}
    </div>

    {#if showTerminal}
      <TerminalPanel
        cwd={terminalCwd}
        dark={iconThemes.dark}
        onClose={() => (showTerminal = false)}
        onCwd={(path) => {
          if (loc.kind !== "local" || loc.path !== path) go({ kind: "local", path });
        }}
      />
    {/if}

    <footer class="status">
      <span>{statusText}</span>
      {#if refreshingSince}<span class="refreshing" role="status">Updating… (shown from {age(refreshingSince)})</span>{/if}
      {#if notice}<span class="notice" role="status">{notice}</span>{/if}
    </footer>
    <TransferPanel />
  </section>
</div>

{#if externalOver}
  <div class="external" aria-hidden="true"><span>Drop to copy into {trail.at(-1)?.label}</span></div>
{/if}

{#if drag.payload}
  {@const count = drag.payload.entries.length}
  <div class="ghost" style:left="{drag.x + 14}px" style:top="{drag.y + 10}px" aria-hidden="true">
    {count === 1 ? drag.payload.entries[0].name : `${count} items`}
    {#if drag.forceCopy}<b>Copy</b>{/if}
  </div>
{/if}

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => (menu = null)} />
{/if}

{#if wizardFor}
  {@const provider = drives.provider(wizardFor)}
  <Dialog title="Set up {cloudTitles[wizardFor]}" onClose={() => (wizardFor = null)}>
    {#if provider}
      <RemoteWizard
        {provider}
        suggestedName={cloudTitles[wizardFor].replace(/\s+/g, "")}
        onBack={() => (wizardFor = null)}
        onDone={async (name) => {
          wizardFor = null;
          await drives.refresh();
          const remote = name ? drives.remotes.find((r) => r.name === name) : undefined;
          if (remote) go({ kind: "remote", remote: remote.name, path: remote.startPath }, cloudTitles[provider.id as CloudProvider]);
        }}
      />
    {:else}
      <p>Stingray's storage engine is still starting. Try again in a moment.</p>
    {/if}
  </Dialog>
{/if}

{#if sendPicker}
  {@const job = sendPicker}
  <FolderPickerDialog
    title={`${job.move ? "Move" : "Copy"} ${job.items.length === 1 ? `“${job.items[0].name}”` : `${job.items.length} items`} to…`}
    action={job.move ? "Move Here" : "Copy Here"}
    onClose={() => (sendPicker = null)}
    onPick={(to) => {
      sendPicker = null;
      void transfer({ from: job.from, entries: job.items }, to, !job.move, job.move);
    }}
  />
{/if}

{#if batchRename}
  <BatchRenameDialog items={batchRename} onApply={(r) => void applyBatchRename(r)} onClose={() => (batchRename = null)} />
{/if}

{#if askDefault}
  <DefaultFmDialog status={askDefault} onClose={() => (askDefault = null)} />
{/if}

{#if openWithPath}
  <OpenWithDialog path={openWithPath} onClose={() => (openWithPath = null)} />
{/if}

{#if propertiesPath}
  <PropertiesDialog path={propertiesPath} onClose={() => (propertiesPath = null)} />
{/if}

{#if settingsAt}
  <SettingsDialog section={settingsAt} onClose={() => (settingsAt = null)} onConnectServer={() => { settingsAt = null; connectServer = true; }} />
{/if}

{#if connectServer}
  <ConnectServer onClose={() => (connectServer = false)} />
{/if}

<style>
  .window {
    display: grid;
    grid-template-columns: 228px 1fr;
    height: 100vh;
    background: var(--window-bg);
  }
  .window.dragging {
    cursor: grabbing;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--separator);
    min-width: 0;
  }
  .nav,
  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .tools {
    gap: 6px;
  }
  .tool {
    display: grid;
    place-items: center;
    width: 30px;
    height: 28px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--text-secondary);
    cursor: default;
  }
  .tool:hover:not(:disabled) {
    background: var(--control-bg);
    color: var(--text-primary);
  }
  .tool:disabled {
    opacity: 0.35;
  }
  .tool[aria-pressed="true"] {
    color: var(--accent);
  }
  .trail {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 2px;
    overflow: hidden;
    color: var(--text-secondary);
  }
  .crumb {
    border: none;
    background: none;
    padding: 3px 6px;
    border-radius: 6px;
    white-space: nowrap;
    color: var(--text-secondary);
    cursor: default;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crumb:hover {
    background: var(--control-bg);
  }
  .crumb.last {
    color: var(--text-primary);
    font-weight: 650;
    font-size: 15px;
    letter-spacing: -0.01em;
    flex-shrink: 0;
  }
  .segmented {
    display: flex;
    padding: 2px;
    border-radius: 8px;
    background: var(--control-bg);
  }
  .segmented button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 24px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-secondary);
  }
  .segmented button[aria-pressed="true"] {
    background: var(--window-bg);
    color: var(--accent);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 9px;
    height: 28px;
    width: 190px;
    border-radius: 8px;
    background: var(--control-bg);
    color: var(--text-secondary);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    outline: none;
    user-select: text;
  }
  @media (max-width: 1000px) {
    .search {
      width: 130px;
    }
  }
  :global(.spin) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .banner {
    padding: 12px 16px 0;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-secondary);
    text-align: center;
    padding: 24px;
  }
  .loader {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 3px solid var(--separator);
    border-top-color: var(--accent);
    animation: spin 0.9s linear infinite;
  }
  .hint {
    font-size: 12px;
    max-width: 46ch;
  }
  .empty img {
    opacity: 0.35;
    filter: saturate(0.6);
  }
  .empty p {
    margin: 0;
    max-width: 40ch;
  }
  .tabs {
    display: flex;
    align-items: stretch;
    gap: 2px;
    padding: 6px 10px 0;
    border-bottom: 1px solid var(--separator);
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 200px;
    padding: 5px 6px 5px 12px;
    border-radius: 8px 8px 0 0;
    color: var(--text-secondary);
    cursor: default;
  }
  .tab:hover {
    background: var(--control-bg);
  }
  .tab[aria-selected="true"] {
    background: var(--window-bg);
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tab-close,
  .tab-new {
    border: none;
    background: none;
    color: var(--text-secondary);
    border-radius: 5px;
    width: 20px;
    height: 20px;
    line-height: 1;
    font-size: 15px;
  }
  .tab-close:hover,
  .tab-new:hover {
    background: var(--control-bg-hover);
    color: var(--text-primary);
  }
  .tab-new {
    align-self: center;
    margin-left: 4px;
  }
  .panes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .tree-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--separator);
  }
  .tree-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--separator);
  }
  .left {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .refreshing {
    color: var(--text-secondary);
    font-style: italic;
  }
  .status {
    display: flex;
    gap: 16px;
    padding: 5px 16px;
    font-size: 11px;
    color: var(--text-secondary);
    border-top: 1px solid var(--separator);
  }
  .notice {
    margin-left: auto;
    color: var(--text-primary);
  }
  .ghost {
    position: fixed;
    z-index: 60;
    pointer-events: none;
    padding: 4px 10px;
    border-radius: 7px;
    background: var(--accent-gradient);
    color: #fff;
    font-size: 12px;
    font-weight: 500;
    box-shadow: 0 6px 18px rgba(138, 63, 252, 0.4);
    white-space: nowrap;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ghost b {
    margin-left: 6px;
    padding: 0 5px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.25);
    font-weight: 600;
  }
  .external {
    position: fixed;
    inset: 8px;
    z-index: 55;
    pointer-events: none;
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    border-radius: 14px;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .external span {
    padding: 8px 14px;
    border-radius: 10px;
    background: var(--accent-gradient);
    color: #fff;
    font-weight: 600;
  }
</style>
