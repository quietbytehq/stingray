<script lang="ts">
  import { api, asAppError } from "$lib/api";
  import type { AppError, FileEntry } from "$lib/api/types";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { viewPrefs } from "$lib/stores/viewPrefs.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { childOf, crumbs, parentOf, type Location } from "$lib/util/location";
  import { ArrowLeft, ArrowUp, RefreshCw, X } from "$lib/icons";
  import FileView from "./FileView.svelte";
  import ErrorNote from "./ErrorNote.svelte";
  import type { MenuItem } from "./ContextMenu.svelte";

  /**
   * The second pane of split view: its own folder, history and selection. Drag and drop works
   * between it and the main pane (the shared drag store handles drops onto either). Its menus
   * cover the everyday actions; the full set lives in the main pane.
   */
  let {
    start,
    label = "Home",
    onClose,
    onMenu,
    onPaste,
    canPaste,
    focused = false,
    onFocus,
  }: {
    start: Location;
    label?: string;
    onClose: () => void;
    onMenu: (event: MouseEvent, items: MenuItem[]) => void;
    /** Pastes the main window's clipboard into this pane's folder. */
    onPaste: (to: Location) => void;
    canPaste: boolean;
    focused?: boolean;
    onFocus: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let loc = $state<Location>(start);
  // svelte-ignore state_referenced_locally
  let rootLabel = $state(label);
  let history = $state<Location[]>([]);
  let entries = $state<FileEntry[]>([]);
  let selected = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state<AppError | null>(null);

  const visible = $derived(entries.filter((e) => !e.name.startsWith(".")).sort(viewPrefs.compare));
  const trail = $derived(crumbs(loc, rootLabel));

  let token = 0;
  async function load() {
    const mine = ++token;
    loading = true;
    error = null;
    try {
      const result = loc.kind === "local" ? await api.listLocal(loc.path) : await api.listFiles(loc.remote, loc.path);
      if (mine === token) entries = result;
    } catch (err) {
      if (mine === token) {
        entries = [];
        error = asAppError(err);
      }
    } finally {
      if (mine === token) loading = false;
    }
  }

  function go(next: Location, remember = true) {
    if (remember) history = [...history, loc];
    loc = next;
    selected = new Set();
    void load();
  }

  async function open(entry: FileEntry) {
    if (entry.isDir) return go(childOf(loc, entry));
    if (loc.kind !== "local") return;
    if (/\.(zip|jar|cbz|tar|tgz|tar\.gz)$/i.test(entry.name) && !entry.path.includes("!/")) return go({ kind: "local", path: `${entry.path}!/` });
    try {
      await api.openLocal(entry.path.includes("!/") ? await api.openFromArchive(entry.path) : entry.path);
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function trash(items: FileEntry[]) {
    if (loc.kind !== "local") return;
    try {
      await api.localTrash(items.map((e) => e.path));
      await load();
    } catch (err) {
      error = asAppError(err);
    }
  }

  function itemMenu(event: MouseEvent, entry: FileEntry) {
    const items = selected.has(entry.path) && selected.size > 1 ? visible.filter((e) => selected.has(e.path)) : [entry];
    const local = loc.kind === "local" && !loc.path.includes("!/");
    onMenu(event, [
      { label: "Open", action: () => void open(entry), disabled: items.length > 1 },
      { label: "Move to Trash", destructive: true, separator: true, action: () => void trash(items), disabled: !local },
    ]);
  }

  function backgroundMenu(event: MouseEvent) {
    onMenu(event, [
      { label: "Paste", action: () => onPaste(loc), disabled: !canPaste },
      { label: "Refresh", action: () => void load(), separator: true },
    ]);
  }

  // Reload when a transfer finishes, like the main pane.
  let lastActive = 0;
  $effect(() => {
    const active = transfers.active.length;
    if (active < lastActive) void load();
    lastActive = active;
  });

  $effect(() => {
    void load();
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<section class="pane" class:focused onpointerdown={onFocus}>
  <header>
    <button type="button" class="tool" title="Back" aria-label="Back" disabled={!history.length} onclick={() => { const prev = history.at(-1); if (prev) { history = history.slice(0, -1); go(prev, false); } }}>
      <ArrowLeft size={15} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="tool" title="Up" aria-label="Up" disabled={!parentOf(loc)} onclick={() => { const up = parentOf(loc); if (up) go(up); }}>
      <ArrowUp size={15} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <nav class="trail" aria-label="Folder path">
      {#each trail as crumb, i (i)}
        <button type="button" class="crumb" class:last={i === trail.length - 1} onclick={() => go(crumb.loc)}>{crumb.label}</button>
      {/each}
    </nav>
    <button type="button" class="tool" title="Refresh" aria-label="Refresh" onclick={() => void load()}>
      <RefreshCw size={15} strokeWidth={1.75} aria-hidden="true" class={loading ? "spin" : ""} />
    </button>
    <button type="button" class="tool" title="Close split view (F3)" aria-label="Close split view" onclick={onClose}>
      <X size={15} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </header>
  {#if error}<div class="err"><ErrorNote {error} /></div>{/if}
  <FileView
    {loc}
    entries={visible}
    mode="list"
    bind:selected
    iconSize={settings.value?.iconSize ?? 56}
    thumbnails={false}
    sortKey={viewPrefs.sort.key}
    sortDescending={viewPrefs.sort.descending}
    onSort={(key) => viewPrefs.sortBy(key)}
    onOpen={(entry) => void open(entry)}
    onItemMenu={itemMenu}
    onBackgroundMenu={backgroundMenu}
    onRename={() => {}}
  />
  <footer>{visible.length} {visible.length === 1 ? "item" : "items"}</footer>
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--separator);
  }
  .pane.focused header {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--separator);
  }
  .tool {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-secondary);
  }
  .tool:hover:not(:disabled) {
    background: var(--control-bg);
  }
  .tool:disabled {
    opacity: 0.35;
  }
  .trail {
    flex: 1;
    min-width: 0;
    display: flex;
    overflow: hidden;
  }
  .crumb {
    border: none;
    background: none;
    padding: 2px 5px;
    border-radius: 5px;
    color: var(--text-secondary);
    white-space: nowrap;
    font-size: 12px;
  }
  .crumb::after {
    content: " ›";
    opacity: 0.5;
  }
  .crumb.last {
    color: var(--text-primary);
    font-weight: 600;
  }
  .crumb.last::after {
    content: "";
  }
  .crumb:hover {
    background: var(--control-bg);
  }
  .err {
    padding: 8px 10px 0;
  }
  footer {
    padding: 4px 10px;
    font-size: 11px;
    color: var(--text-secondary);
    border-top: 1px solid var(--separator);
  }
</style>
