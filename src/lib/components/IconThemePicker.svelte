<script lang="ts">
  import { settings } from "$lib/stores/settings.svelte";
  import { iconThemes, type IconKey } from "$lib/stores/iconThemes.svelte";
  import FileIcon, { ICON_THEMES, type SpecialFolder } from "./FileIcon.svelte";

  /**
   * Icon theme picker: a list of theme names on the left, a large preview on the right, and Apply.
   * Packs with light and dark variants preview both side by side.
   */
  const current = $derived(settings.value?.iconTheme ?? "stingray");
  let selected = $state<string | null>(null);
  const shown = $derived(selected ?? current);

  $effect(() => {
    if (shown.startsWith("system:")) void iconThemes.loadSystem(shown.slice(7));
  });

  const pack = $derived(shown.startsWith("pack:") ? iconThemes.packs.find((p) => `pack:${p.id}` === shown) : undefined);
  const twoModes = $derived(pack?.hasDark ?? false);

  /** Preview icons: name for FileIcon, whether it is a folder, and which home folder it is. */
  const PREVIEW: { key: IconKey; label: string; name: string; dir: boolean; special?: SpecialFolder }[] = [
    { key: "folder", label: "Folder", name: "", dir: true },
    { key: "home", label: "Home", name: "", dir: true, special: "home" },
    { key: "downloads", label: "Downloads", name: "", dir: true, special: "downloads" },
    { key: "music", label: "Music", name: "", dir: true, special: "music" },
    { key: "pictures", label: "Pictures", name: "", dir: true, special: "pictures" },
    { key: "trash", label: "Trash", name: "", dir: true, special: "trash" },
    { key: "pdf", label: "PDF", name: "a.pdf", dir: false },
    { key: "spreadsheet", label: "Sheet", name: "a.xlsx", dir: false },
    { key: "image", label: "Picture", name: "a.png", dir: false },
    { key: "video", label: "Video", name: "a.mp4", dir: false },
    { key: "archive", label: "Archive", name: "a.zip", dir: false },
    { key: "code", label: "Code", name: "a.rs", dir: false },
  ];

  async function apply() {
    await settings.patch({ iconTheme: shown });
    selected = null;
  }
</script>

{#snippet item(id: string, label: string, note: string)}
  <button
    type="button"
    class="item"
    class:selected={shown === id}
    aria-current={current === id ? "true" : undefined}
    onclick={() => (selected = id)}
  >
    <span class="name">{label}</span>
    <span class="note">{current === id ? "In use" : note}</span>
  </button>
{/snippet}

{#snippet grid(darkBg: boolean)}
  <div class="grid" class:dark={darkBg}>
    {#each PREVIEW as icon (icon.key)}
      <div class="cell">
        <FileIcon name={icon.name} isDir={icon.dir} special={icon.special ?? null} size={48} theme={shown} dark={darkBg} />
        <span>{icon.label}</span>
      </div>
    {/each}
  </div>
{/snippet}

<div class="picker">
  <div class="list" role="listbox" aria-label="Icon themes">
    <h4>Stingray designs</h4>
    {#each ICON_THEMES as theme (theme.id)}
      {@render item(theme.id, theme.label, theme.note)}
    {/each}
    {#if iconThemes.packs.length}
      <h4>Icon packs</h4>
      {#each iconThemes.packs as p (p.id)}
        {@render item(`pack:${p.id}`, p.name, p.hasDark ? "Light and dark" : `by ${p.author}`)}
      {/each}
    {/if}
    {#if iconThemes.system.length}
      <h4>Installed on this computer</h4>
      {#each iconThemes.system as s (s.id)}
        {@render item(`system:${s.id}`, s.name, s.id)}
      {/each}
    {/if}
  </div>

  <div class="preview">
    <div class="head">
      <div>
        <strong>{iconThemes.label(shown)}</strong>
        {#if pack}
          <span class="credit">by {pack.author} · {pack.license} · <a href={pack.url} target="_blank" rel="noopener">project page</a></span>
        {/if}
      </div>
      <button type="button" class="button primary" disabled={shown === current} onclick={() => void apply()}>
        {shown === current ? "Applied" : "Apply"}
      </button>
    </div>
    {#if twoModes}
      <p class="mode">Light mode</p>
      {@render grid(false)}
      <p class="mode">Dark mode</p>
      {@render grid(true)}
    {:else}
      {@render grid(iconThemes.dark)}
    {/if}
  </div>
</div>

<style>
  .picker {
    display: grid;
    grid-template-columns: 190px 1fr;
    gap: 14px;
    min-height: 340px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 420px;
    overflow-y: auto;
    padding-right: 4px;
  }
  h4 {
    margin: 10px 6px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  h4:first-child {
    margin-top: 0;
  }
  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 6px 10px;
    border: none;
    border-radius: 7px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--control-bg);
  }
  .item.selected {
    background: var(--sidebar-selected);
  }
  .name {
    font-weight: 600;
  }
  .note {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .item[aria-current="true"] .note {
    color: var(--accent);
    font-weight: 600;
  }
  .preview {
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }
  .head strong {
    display: block;
    font-size: 15px;
  }
  .credit {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .credit a {
    color: var(--accent);
  }
  .mode {
    margin: 6px 0 4px;
    font-size: 11px;
    color: var(--text-secondary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px 4px;
    padding: 14px 10px;
    border-radius: 12px;
    background: #f4f2f8;
    color: #2a2633;
  }
  .grid.dark {
    background: #1f1b28;
    color: #e8e4f0;
  }
  .cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
  }
</style>
