<script lang="ts">
  import type { FileEntry } from "$lib/api/types";
  import Dialog from "./Dialog.svelte";

  /**
   * Batch rename: renames several items in one go. Three ways, each with a live preview:
   * find and replace, add text before or after, or a new name with a number ("Holiday 1, 2, 3…").
   * File extensions are kept unless the person includes them in the text.
   */
  let {
    items,
    onApply,
    onClose,
  }: { items: FileEntry[]; onApply: (renames: { entry: FileEntry; name: string }[]) => void; onClose: () => void } = $props();

  let mode = $state<"replace" | "add" | "number">("replace");
  let find = $state("");
  let replaceWith = $state("");
  let before = $state("");
  let after = $state("");
  let base = $state("File");
  let start = $state(1);
  let digits = $state(1);

  function split(name: string, isDir: boolean): [string, string] {
    const dot = name.lastIndexOf(".");
    return !isDir && dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  }

  const preview = $derived(
    items.map((entry, i) => {
      const [stem, ext] = split(entry.name, entry.isDir);
      let name = entry.name;
      if (mode === "replace" && find) name = entry.name.split(find).join(replaceWith);
      else if (mode === "add") name = `${before}${stem}${after}${ext}`;
      else if (mode === "number") name = `${base} ${String(start + i).padStart(digits, "0")}${ext}`;
      return { entry, name };
    }),
  );

  const problem = $derived.by(() => {
    const names = preview.map((p) => p.name);
    if (names.some((n) => !n.trim() || n.includes("/") || n.includes("\\"))) return "Names can't be empty or contain slashes.";
    if (new Set(names).size !== names.length) return "Two items would end up with the same name.";
    return null;
  });
  const changed = $derived(preview.filter((p) => p.name !== p.entry.name));
</script>

<Dialog title="Rename {items.length} items" {onClose}>
  <div class="wrap">
    <div class="modes" role="radiogroup" aria-label="How to rename">
      {#each [["replace", "Find and replace"], ["add", "Add text"], ["number", "Name and number"]] as [id, label] (id)}
        <button type="button" role="radio" aria-checked={mode === id} onclick={() => (mode = id as typeof mode)}>{label}</button>
      {/each}
    </div>

    <div class="fields">
      {#if mode === "replace"}
        <label>Find <input bind:value={find} spellcheck="false" placeholder="IMG_" /></label>
        <label>Replace with <input bind:value={replaceWith} spellcheck="false" placeholder="Holiday_" /></label>
      {:else if mode === "add"}
        <label>Before the name <input bind:value={before} spellcheck="false" placeholder="2026-" /></label>
        <label>After the name <input bind:value={after} spellcheck="false" placeholder=" (final)" /></label>
      {:else}
        <label>New name <input bind:value={base} spellcheck="false" /></label>
        <label class="small">Start at <input type="number" min="0" bind:value={start} /></label>
        <label class="small">Digits <input type="number" min="1" max="6" bind:value={digits} /></label>
      {/if}
    </div>

    <div class="preview">
      {#each preview as p (p.entry.path)}
        <div class="row" class:same={p.name === p.entry.name}>
          <span class="old">{p.entry.name}</span>
          <span class="arrow">→</span>
          <span class="new">{p.name}</span>
        </div>
      {/each}
    </div>
    {#if problem}<p class="problem">{problem}</p>{/if}

    <div class="actions">
      <button type="button" class="button" onclick={onClose}>Cancel</button>
      <button
        type="button"
        class="button primary"
        disabled={!!problem || changed.length === 0}
        onclick={() => onApply(changed)}
      >Rename {changed.length || ""}</button>
    </div>
  </div>
</Dialog>

<style>
  .wrap {
    width: min(620px, 80vw);
  }
  .modes {
    display: inline-flex;
    padding: 2px;
    border-radius: 8px;
    background: var(--control-bg);
    margin-bottom: 12px;
  }
  .modes button {
    border: none;
    background: none;
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
  }
  .modes button[aria-checked="true"] {
    background: var(--window-bg);
    color: var(--accent);
    font-weight: 600;
  }
  .fields {
    display: flex;
    gap: 10px;
    margin-bottom: 12px;
  }
  label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
  }
  label.small {
    flex: 0 0 90px;
  }
  input {
    padding: 6px 8px;
    border-radius: 7px;
    border: 1px solid var(--separator);
    background: var(--window-bg);
    color: inherit;
    font-weight: 400;
    user-select: text;
    -webkit-user-select: text;
  }
  .preview {
    max-height: 260px;
    overflow-y: auto;
    border-radius: 10px;
    background: var(--control-bg);
    padding: 6px 10px;
    font-size: 12px;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 8px;
    padding: 3px 0;
  }
  .row.same {
    opacity: 0.5;
  }
  .old,
  .new {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .new {
    font-weight: 600;
  }
  .arrow {
    color: var(--text-secondary);
  }
  .problem {
    color: var(--status-offline);
    font-size: 12px;
    margin: 8px 0 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
</style>
