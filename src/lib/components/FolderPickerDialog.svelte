<script lang="ts">
  import { api, asAppError } from "$lib/api";
  import type { AppError } from "$lib/api/types";
  import type { Location } from "$lib/util/location";
  import Dialog from "./Dialog.svelte";
  import ErrorNote from "./ErrorNote.svelte";
  import FolderTree from "./FolderTree.svelte";

  /**
   * Stingray's own "Move to… / Copy to…" window: any folder Stingray can reach (local, cloud,
   * network, devices), recently used destinations, and a New Folder button.
   */
  let {
    title,
    action,
    onPick,
    onClose,
  }: {
    title: string;
    action: string;
    onPick: (loc: Location) => void;
    onClose: () => void;
  } = $props();

  const RECENT_KEY = "stingray.recentDestinations";
  type Recent = { loc: Location; label: string };
  let recent = $state<Recent[]>((() => {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    } catch {
      return [];
    }
  })());

  let chosen = $state<Location | null>(null);
  let chosenLabel = $state("");
  let error = $state<AppError | null>(null);

  function where(loc: Location): string {
    return loc.kind === "local" ? loc.path : `${loc.remote}:${loc.path ? ` /${loc.path}` : ""}`;
  }

  function pick(loc: Location, label: string) {
    chosen = loc;
    chosenLabel = label;
  }

  async function newFolder() {
    if (!chosen) return;
    const name = window.prompt("New folder name", "New Folder");
    if (!name?.trim()) return;
    try {
      if (chosen.kind === "local") {
        const path = await api.localCreateFolder(chosen.path, name.trim());
        pick({ kind: "local", path }, name.trim());
      } else {
        const path = chosen.path ? `${chosen.path}/${name.trim()}` : name.trim();
        await api.createFolder(chosen.remote, path);
        pick({ kind: "remote", remote: chosen.remote, path }, name.trim());
      }
    } catch (err) {
      error = asAppError(err);
    }
  }

  function confirm() {
    if (!chosen) return;
    const entry = { loc: chosen, label: chosenLabel };
    const key = where(chosen);
    recent = [entry, ...recent.filter((r) => where(r.loc) !== key)].slice(0, 6);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(recent));
    } catch {
      // Recent list just won't be kept.
    }
    onPick(chosen);
  }
</script>

<Dialog {title} {onClose}>
  <div class="picker">
    {#if recent.length}
      <div class="recent">
        <span class="hint">Recently used</span>
        <div class="chips">
          {#each recent as r (where(r.loc))}
            <button type="button" class="chip" class:on={chosen !== null && where(chosen) === where(r.loc)} title={where(r.loc)} onclick={() => pick(r.loc, r.label)}>{r.label}</button>
          {/each}
        </div>
      </div>
    {/if}
    <div class="tree-box">
      <FolderTree selected={chosen} onSelect={pick} />
    </div>
    <ErrorNote {error} />
    <div class="footer">
      <span class="path" title={chosen ? where(chosen) : ""}>{chosen ? where(chosen) : "Choose a folder"}</span>
      <button type="button" class="button" disabled={!chosen} onclick={() => void newFolder()}>New Folder</button>
      <button type="button" class="button" onclick={onClose}>Cancel</button>
      <button type="button" class="button primary" disabled={!chosen} onclick={confirm}>{action}</button>
    </div>
  </div>
</Dialog>

<style>
  .picker {
    width: min(560px, 80vw);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .tree-box {
    height: 360px;
    display: flex;
    flex-direction: column;
    border-radius: 10px;
    background: var(--control-bg);
    overflow: hidden;
  }
  .recent .hint {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  .chip {
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--separator);
    background: none;
    font-size: 12px;
  }
  .chip.on,
  .chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
