<script lang="ts">
  import { api, asAppError } from "$lib/api";
  import type { AppError, DefaultFm } from "$lib/api/types";
  import Dialog from "./Dialog.svelte";
  import ErrorNote from "./ErrorNote.svelte";

  /** The first-start question: make Stingray the app that opens folders. */
  let { status, onClose }: { status: DefaultFm; onClose: () => void } = $props();
  let error = $state<AppError | null>(null);
  let copied = $state(false);

  function remember() {
    try {
      localStorage.setItem("stingray.askedDefault", "1");
    } catch {
      // Asked again next time; harmless.
    }
  }

  async function makeDefault() {
    try {
      await api.setDefaultFileManager(false);
      remember();
      onClose();
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function copyCommand() {
    if (!status.command) return;
    await navigator.clipboard.writeText(status.command).catch(() => {});
    copied = true;
  }
</script>

<Dialog title="Make Stingray your file manager?" onClose={() => onClose()}>
  <div class="body">
    <img src="/stingray.png" alt="" width="64" height="64" />
    <div>
      <p>
        Folders you open from other apps, the desktop, the dock or a browser's downloads will open in Stingray
        instead of {status.current.includes("Nautilus") ? "Ubuntu's Files" : "your current file manager"}.
      </p>
      <p class="muted">You can change this any time in Settings → Appearance.</p>
      {#if status.command}
        <p class="muted">
          The Snap Store version can't change this itself. Run this once in a terminal:
        </p>
        <div class="cmd">
          <code>{status.command}</code>
          <button type="button" class="button" onclick={() => void copyCommand()}>{copied ? "Copied" : "Copy"}</button>
        </div>
      {/if}
      <ErrorNote {error} />
    </div>
  </div>
  <div class="actions">
    <button type="button" class="button" onclick={() => { remember(); onClose(); }}>Don't ask again</button>
    <button type="button" class="button" onclick={() => onClose()}>Not now</button>
    {#if !status.command}
      <button type="button" class="button primary" onclick={() => void makeDefault()}>Make default</button>
    {:else}
      <button type="button" class="button primary" onclick={() => { remember(); onClose(); }}>Done</button>
    {/if}
  </div>
</Dialog>

<style>
  .body {
    display: flex;
    gap: 16px;
    max-width: 520px;
  }
  p {
    margin: 0 0 8px;
  }
  .muted {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .cmd {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
  }
  code {
    flex: 1;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--control-bg);
    font-size: 12px;
    user-select: all;
    -webkit-user-select: all;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
</style>
