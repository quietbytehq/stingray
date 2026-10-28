<script lang="ts">
  import type { FileEntry } from "$lib/api/types";
  import { drag, dropKey } from "$lib/stores/drag.svelte";
  import { childOf, type Location } from "$lib/util/location";
  import { formatBytes } from "$lib/util/format";
  import FileIcon from "./FileIcon.svelte";

  let {
    loc,
    entries,
    mode,
    selected = $bindable(),
    renaming = null,
    onOpen,
    onItemMenu,
    onBackgroundMenu,
    onRename,
  }: {
    loc: Location;
    entries: FileEntry[];
    mode: "grid" | "list";
    selected: Set<string>;
    /** Path of the item being renamed in place, if any. */
    renaming?: string | null;
    onOpen: (entry: FileEntry) => void;
    onItemMenu: (event: MouseEvent, entry: FileEntry) => void;
    onBackgroundMenu: (event: MouseEvent) => void;
    onRename: (entry: FileEntry, name: string | null) => void;
  } = $props();

  let anchor = $state<string | null>(null);

  // Folders in the listing accept drops; so does the view itself (the folder being shown).
  $effect(() => {
    drag.target(dropKey(loc), loc);
    for (const entry of entries) if (entry.isDir) drag.target(dropKey(childOf(loc, entry)), childOf(loc, entry));
  });

  function click(event: MouseEvent, entry: FileEntry) {
    const next = new Set(event.ctrlKey || event.metaKey ? selected : []);
    if (event.shiftKey && anchor) {
      const a = entries.findIndex((e) => e.path === anchor);
      const b = entries.findIndex((e) => e.path === entry.path);
      const [from, to] = a < b ? [a, b] : [b, a];
      for (const e of entries.slice(from, to + 1)) next.add(e.path);
    } else if ((event.ctrlKey || event.metaKey) && selected.has(entry.path)) {
      next.delete(entry.path);
    } else {
      next.add(entry.path);
      anchor = entry.path;
    }
    selected = next;
  }

  function pointerdown(event: PointerEvent, entry: FileEntry) {
    if (renaming === entry.path) return;
    // Dragging an unselected item drags just that item.
    const dragged = selected.has(entry.path) ? entries.filter((e) => selected.has(e.path)) : [entry];
    drag.arm(event, { from: loc, entries: dragged });
  }

  function menu(event: MouseEvent, entry: FileEntry) {
    event.preventDefault();
    event.stopPropagation();
    if (!selected.has(entry.path)) selected = new Set([entry.path]);
    onItemMenu(event, entry);
  }

  function renameKey(event: KeyboardEvent, entry: FileEntry) {
    const input = event.currentTarget as HTMLInputElement;
    if (event.key === "Enter") onRename(entry, input.value);
    else if (event.key === "Escape") onRename(entry, null);
    event.stopPropagation();
  }

  /** Selects the name without its extension, as Finder does. */
  function focusName(node: HTMLInputElement) {
    node.focus();
    const dot = node.value.lastIndexOf(".");
    node.setSelectionRange(0, dot > 0 ? dot : node.value.length);
  }

  function modified(iso: string | null): string {
    if (!iso) return "—";
    const date = new Date(iso);
    return Number.isNaN(date.getTime()) ? "—" : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  }

  const localPath = (entry: FileEntry) => (loc.kind === "local" ? entry.path : null);
</script>

{#snippet name(entry: FileEntry)}
  {#if renaming === entry.path}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="rename"
      value={entry.name}
      spellcheck="false"
      use:focusName
      onkeydown={(e) => renameKey(e, entry)}
      onblur={(e) => onRename(entry, (e.currentTarget as HTMLInputElement).value)}
      onclick={(e) => e.stopPropagation()}
      onpointerdown={(e) => e.stopPropagation()}
    />
  {:else}
    <span class="name" title={entry.name}>{entry.name}</span>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="surface"
  class:drop={drag.over === dropKey(loc)}
  data-drop={dropKey(loc)}
  onclick={(e) => { if (e.target === e.currentTarget) selected = new Set(); }}
  oncontextmenu={(e) => { if (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains("grid") || (e.target as HTMLElement).tagName === "TBODY") { e.preventDefault(); selected = new Set(); onBackgroundMenu(e); } }}
>
  {#if mode === "grid"}
    <div class="grid" role="listbox" aria-multiselectable="true" aria-label="Files">
      {#each entries as entry (entry.path)}
        {@const target = entry.isDir ? dropKey(childOf(loc, entry)) : undefined}
        <div
          class="tile"
          role="option"
          tabindex="-1"
          aria-selected={selected.has(entry.path)}
          class:selected={selected.has(entry.path)}
          class:drop={target !== undefined && drag.over === target}
          class:dragging={drag.payload?.entries.some((e) => e.path === entry.path)}
          data-drop={target}
          onclick={(e) => click(e, entry)}
          ondblclick={() => onOpen(entry)}
          onpointerdown={(e) => pointerdown(e, entry)}
          oncontextmenu={(e) => menu(e, entry)}
        >
          <span class="icon"><FileIcon name={entry.name} isDir={entry.isDir} size={56} localPath={localPath(entry)} /></span>
          {@render name(entry)}
        </div>
      {/each}
    </div>
  {:else}
    <table>
      <thead>
        <tr><th class="col-name">Name</th><th>Modified</th><th class="num">Size</th></tr>
      </thead>
      <tbody>
        {#each entries as entry (entry.path)}
          {@const target = entry.isDir ? dropKey(childOf(loc, entry)) : undefined}
          <tr
            aria-selected={selected.has(entry.path)}
            class:selected={selected.has(entry.path)}
            class:drop={target !== undefined && drag.over === target}
            data-drop={target}
            onclick={(e) => click(e, entry)}
            ondblclick={() => onOpen(entry)}
            onpointerdown={(e) => pointerdown(e, entry)}
            oncontextmenu={(e) => menu(e, entry)}
          >
            <td class="col-name"><span class="row-name"><FileIcon name={entry.name} isDir={entry.isDir} size={20} />{@render name(entry)}</span></td>
            <td class="muted">{modified(entry.modTime)}</td>
            <td class="num muted">{entry.isDir ? "—" : formatBytes(entry.size)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .surface {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 14px 16px 24px;
    outline: 2px solid transparent;
    outline-offset: -4px;
    border-radius: 8px;
    transition: outline-color 0.12s;
  }
  .surface.drop {
    outline-color: var(--accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 6px 4px;
    align-content: start;
    min-height: 100%;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 4px 6px;
    border-radius: 10px;
    cursor: default;
    outline: none;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 64px;
    height: 60px;
    border-radius: 8px;
  }
  .tile.selected .icon {
    background: var(--control-bg);
  }
  .tile .name {
    max-width: 100%;
    padding: 1px 6px;
    border-radius: 4px;
    text-align: center;
    font-size: 12px;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }
  .tile.selected .name {
    background: var(--accent);
    color: #fff;
  }
  .tile.drop .icon,
  tr.drop {
    background: var(--sidebar-selected);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .tile.dragging {
    opacity: 0.45;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    position: sticky;
    top: -14px;
    background: var(--window-bg);
    text-align: left;
    font-weight: 500;
    font-size: 11px;
    color: var(--text-secondary);
    padding: 4px 8px;
    border-bottom: 1px solid var(--separator);
  }
  td {
    padding: 3px 8px;
    white-space: nowrap;
  }
  tbody tr:nth-child(even) {
    background: color-mix(in srgb, var(--control-bg) 45%, transparent);
  }
  tbody tr.selected {
    background: var(--accent);
    color: #fff;
  }
  tbody tr.selected .muted {
    color: rgba(255, 255, 255, 0.85);
  }
  .col-name {
    width: 60%;
  }
  .row-name {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .row-name .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .num {
    text-align: right;
  }
  .muted {
    color: var(--text-secondary);
  }
  .rename {
    width: 100%;
    max-width: 180px;
    font-size: 12px;
    padding: 1px 4px;
    border: none;
    border-radius: 4px;
    box-shadow: 0 0 0 2px var(--accent);
    background: var(--window-bg);
    user-select: text;
  }
</style>
