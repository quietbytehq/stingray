<script lang="ts">
  import { onMount } from "svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, OpenWithApp } from "$lib/api/types";
  import Dialog from "./Dialog.svelte";
  import ErrorNote from "./ErrorNote.svelte";

  /** "Open With…": every installed app that says it can open this file, default first. */
  let { path, onClose }: { path: string; onClose: () => void } = $props();
  let apps = $state<OpenWithApp[] | null>(null);
  let error = $state<AppError | null>(null);
  const name = $derived(path.split(/[\\/]/).pop() ?? path);

  onMount(async () => {
    try {
      apps = await api.appsForFile(path);
    } catch (err) {
      error = asAppError(err);
      apps = [];
    }
  });

  async function launch(app: OpenWithApp) {
    try {
      await api.openWith(path, app.id);
      onClose();
    } catch (err) {
      error = asAppError(err);
    }
  }
</script>

<Dialog title="Open “{name}” with" {onClose}>
  <ErrorNote {error} />
  {#if apps === null}
    <p class="muted">Looking for apps…</p>
  {:else if apps.length === 0}
    <p class="muted">No installed app says it can open this kind of file.</p>
  {:else}
    <div class="apps">
      {#each apps as app (app.id)}
        <button type="button" class="app" onclick={() => void launch(app)}>
          <span>{app.name}</span>
          {#if app.default}<span class="tag">Default</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</Dialog>

<style>
  .apps {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 340px;
    max-height: 360px;
    overflow-y: auto;
  }
  .app {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border: none;
    border-radius: 8px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .app:hover {
    background: var(--sidebar-selected);
  }
  .tag {
    font-size: 11px;
    color: var(--accent);
    font-weight: 600;
  }
  .muted {
    color: var(--text-secondary);
  }
</style>
