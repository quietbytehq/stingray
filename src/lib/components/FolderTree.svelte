<script lang="ts" module>
  import type { Location } from "$lib/util/location";

  /** One folder in the tree. Children load the first time it is opened. */
  export interface TreeNode {
    key: string;
    label: string;
    loc: Location | null;
    /** Devices that are not mounted yet: opening them mounts first. */
    device?: string;
    children: TreeNode[] | null;
    open: boolean;
    loading: boolean;
    failed: boolean;
  }
</script>

<script lang="ts">
  import { api } from "$lib/api";
  import type { Drive, FileEntry } from "$lib/api/types";
  import { drives } from "$lib/stores/drives.svelte";
  import { viewPrefs } from "$lib/stores/viewPrefs.svelte";
  import { cacheKey, getListing, putListing } from "$lib/stores/listingCache";
  import { childOf, sameLocation } from "$lib/util/location";
  import FileIcon from "./FileIcon.svelte";
  import { ChevronRight } from "$lib/icons";

  /**
   * A tree of every place Stingray can reach: home folders, bookmarks, cloud drives, network
   * servers and devices. Used by the Folders panel and by the Move to / Copy to picker.
   */
  let {
    selected = null,
    onSelect,
    compact = false,
  }: {
    selected?: Location | null;
    onSelect: (loc: Location, label: string) => void;
    compact?: boolean;
  } = $props();

  function node(key: string, label: string, loc: Location | null, extra: Partial<TreeNode> = {}): TreeNode {
    const n: TreeNode = $state({ key, label, loc, children: null, open: false, loading: false, failed: false, ...extra });
    return n;
  }

  let disks = $state<Drive[]>([]);
  $effect(() => {
    void api.listDrives().then((d: Drive[]) => (disks = d)).catch(() => {});
  });

  /** Top-level groups, rebuilt when places, bookmarks, connections or drives change. */
  const groups = $derived.by(() => {
    const out: { title: string; nodes: TreeNode[] }[] = [];
    const places = drives.places.filter((p) => p.kind !== "trash" && p.kind !== "volume");
    out.push({ title: "Places", nodes: places.map((p) => node(`p:${p.path}`, p.label, { kind: "local", path: p.path })) });
    if (viewPrefs.bookmarks.length) {
      out.push({ title: "Bookmarks", nodes: viewPrefs.bookmarks.map((b) => node(`b:${b.path}`, b.label, { kind: "local", path: b.path })) });
    }
    if (drives.remotes.length) {
      out.push({ title: "Cloud & Network", nodes: drives.remotes.map((r) => node(`r:${r.name}`, r.name, { kind: "remote", remote: r.name, path: "" })) });
    }
    if (disks.length) {
      out.push({
        title: "Devices",
        nodes: disks.map((d) =>
          node(`d:${d.device}`, d.label, d.mountPoint ? { kind: "local", path: d.mountPoint } : null, { device: d.mountPoint ? undefined : d.device }),
        ),
      });
    }
    return out;
  });

  // Open/loaded state survives regrouping, keyed by node key. The map itself isn't reactive;
  // each node is, so registering one while drawing is safe.
  const known = new Map<string, TreeNode>();
  function live(n: TreeNode): TreeNode {
    const existing = known.get(n.key);
    if (existing) return existing;
    known.set(n.key, n);
    return n;
  }

  async function listDirs(loc: Location): Promise<FileEntry[]> {
    if (loc.kind === "local") return (await api.listLocal(loc.path)).filter((e) => e.isDir && !e.name.startsWith("."));
    const key = cacheKey(loc.remote, loc.path);
    const cached = await getListing(key);
    const fresh = api.listFiles(loc.remote, loc.path).then((r) => {
      void putListing(key, r);
      return r;
    });
    const result = cached ? cached.entries : await fresh;
    return result.filter((e) => e.isDir && !e.name.startsWith("."));
  }

  async function toggle(n: TreeNode) {
    const s = live(n);
    if (s.open) {
      s.open = false;
      return;
    }
    s.open = true;
    if (s.children) return;
    s.loading = true;
    try {
      if (!s.loc && s.device) {
        const at = await api.mountDrive(s.device);
        s.loc = { kind: "local", path: at };
      }
      const loc = s.loc!;
      const dirs = await listDirs(loc);
      s.children = dirs
        .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }))
        .map((e) => {
          const child = childOf(loc, e);
          return node(`${s.key}/${e.name}`, e.name, child);
        });
    } catch {
      s.failed = true;
      s.children = [];
    } finally {
      s.loading = false;
    }
  }

  async function choose(n: TreeNode) {
    const s = live(n);
    if (!s.loc && s.device) {
      try {
        s.loc = { kind: "local", path: await api.mountDrive(s.device) };
      } catch {
        s.failed = true;
        return;
      }
    }
    if (s.loc) onSelect(s.loc, s.label);
  }
</script>

{#snippet branch(n: TreeNode, depth: number)}
  {@const s = live(n)}
  <div
    class="row"
    class:selected={s.loc !== null && selected !== null && sameLocation(s.loc, selected)}
    style:padding-left="{6 + depth * 14}px"
  >
    <button type="button" class="twisty" class:open={s.open} aria-label={s.open ? "Collapse" : "Expand"} onclick={() => void toggle(n)}>
      {#if !(s.children && s.children.length === 0 && s.open)}
        <ChevronRight size={13} strokeWidth={2} aria-hidden="true" />
      {/if}
    </button>
    <button type="button" class="name" onclick={() => void choose(n)} ondblclick={() => void toggle(n)} title={s.loc?.kind === "local" ? s.loc.path : s.label}>
      <FileIcon name="" isDir={true} size={compact ? 16 : 18} />
      <span>{s.label}</span>
      {#if s.loading}<span class="dim">…</span>{/if}
      {#if s.failed}<span class="dim">unavailable</span>{/if}
    </button>
  </div>
  {#if s.open && s.children}
    {#each s.children as child (child.key)}
      {@render branch(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<div class="tree" role="tree">
  {#each groups as group (group.title)}
    <h3>{group.title}</h3>
    {#each group.nodes as n (n.key)}
      {@render branch(n, 0)}
    {/each}
  {/each}
</div>

<style>
  .tree {
    overflow-y: auto;
    min-height: 0;
    padding: 4px 4px 10px;
  }
  h3 {
    margin: 10px 8px 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: 6px;
  }
  .row:hover {
    background: var(--control-bg);
  }
  .row.selected {
    background: var(--sidebar-selected);
  }
  .twisty {
    display: grid;
    place-items: center;
    width: 18px;
    height: 22px;
    flex: none;
    border: none;
    background: none;
    color: var(--text-secondary);
    padding: 0;
    transition: transform 0.12s ease;
  }
  .twisty.open {
    transform: rotate(90deg);
  }
  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 4px;
    border: none;
    background: none;
    text-align: left;
    color: inherit;
  }
  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dim {
    color: var(--text-secondary);
    font-size: 11px;
  }
</style>
