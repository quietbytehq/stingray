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
  import { ArrowLeft, ArrowRight, ChevronRight, Eye, EyeOff, FolderPlus, LayoutGrid, List, RefreshCw, Search } from "$lib/icons";
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

  // ---- Where we are -------------------------------------------------------------------------
  let loc = $state<Location>({ kind: "local", path: "/" });
  /** Name of the drive at the top of the breadcrumbs (Home, Google Drive, a server name). */
  let rootLabel = $state("Home");
  let back = $state<{ loc: Location; label: string }[]>([]);
  let forward = $state<{ loc: Location; label: string }[]>([]);

  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let error = $state<AppError | null>(null);
  let notice = $state<string | null>(null);

  let mode = $state<"grid" | "list">("grid");
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
      .sort((a, b) => Number(b.isDir) - Number(a.isDir) || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
  });

  const trail = $derived(
    crumbs(loc, rootLabel, loc.kind === "local" && home ? { path: home.path, label: home.label } : undefined),
  );
  const selectedEntries = $derived(visible.filter((e) => selected.has(e.path)));

  // ---- Loading ------------------------------------------------------------------------------
  let loadToken = 0;
  async function load() {
    const token = ++loadToken;
    loading = true;
    error = null;
    try {
      const result =
        loc.kind === "local" ? await api.listLocal(loc.path) : await api.listFiles(loc.remote, loc.path);
      if (token !== loadToken) return;
      entries = result;
      if (loc.kind === "remote") drives.lights[loc.remote] = "connected";
    } catch (err) {
      if (token !== loadToken) return;
      entries = [];
      error = asAppError(err);
      if (loc.kind === "remote") drives.lights[loc.remote] = "offline";
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  function go(next: Location, label?: string, record = true) {
    if (sameLocation(next, loc) && entries.length) return;
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
    const move = forceMove || (!forceCopy && sameDrive(payload.from, to));
    const items = payload.entries.filter((e) => !intoItself(payload, to, e));
    if (items.length < payload.entries.length) notice = "A folder can't go inside itself, so it was skipped.";
    await queue(items.map((e) => request(payload, to, move, e)));
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
      if (loc.kind === "local") await api.localRename(entry.path, next);
      else await api.renameItem(loc.remote, entry.path, entry.isDir, next);
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
        await api.localTrash(items.map((e) => e.path));
        notice = `Moved ${what} to the Trash.`;
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

  function itemMenu(event: MouseEvent, entry: FileEntry) {
    const many = selected.size > 1;
    const items = many ? selectedEntries : [entry];
    showMenu(event, [
      { label: entry.isDir ? "Open" : loc.kind === "local" ? "Open" : "Download", action: () => void open(entry), disabled: many },
      { label: "Rename…", action: () => (renaming = entry.path), disabled: many },
      { label: "Copy", action: () => copySelection(false), separator: true },
      { label: "Cut", action: () => copySelection(true) },
      { label: "Copy path", action: () => void copyPath(entry), disabled: many },
      { label: loc.kind === "local" ? "Move to Trash" : "Delete…", destructive: true, separator: true, action: () => void remove(items) },
    ]);
  }

  function backgroundMenu(event: MouseEvent) {
    showMenu(event, [
      { label: "New folder", action: () => void newFolder() },
      { label: "Paste", action: () => void paste(), disabled: !clipboard },
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
    else if (event.key === "F2" && single) renaming = single.path;
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
    void (async () => {
      await drives.start();
      if (home) go({ kind: "local", path: home.path }, home.label, false);
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
  />

  <section class="main">
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
        <div class="segmented" role="group" aria-label="View">
          <button type="button" aria-pressed={mode === "grid"} onclick={() => (mode = "grid")} title="Icons" aria-label="Icons">
            <LayoutGrid size={15} strokeWidth={1.75} aria-hidden="true" />
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

    {#if !error && !loading && visible.length === 0}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="empty" oncontextmenu={(e) => { e.preventDefault(); backgroundMenu(e); }}>
        <img src="/stingray.png" alt="" width="88" height="88" />
        <p>{query ? `Nothing here matches "${query}".` : "This folder is empty. Drag files here, or paste with Ctrl+V."}</p>
      </div>
    {:else}
      <FileView
        {loc}
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

    <footer class="status">
      <span>
        {visible.length} {visible.length === 1 ? "item" : "items"}{selected.size ? `, ${selected.size} selected` : ""}
      </span>
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
  .empty img {
    opacity: 0.35;
    filter: saturate(0.6);
  }
  .empty p {
    margin: 0;
    max-width: 40ch;
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
