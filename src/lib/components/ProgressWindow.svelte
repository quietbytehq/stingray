<script lang="ts">
  import { transfers } from "$lib/stores/transfers.svelte";
  import type { TransferItem } from "$lib/api/types";
  import { formatBytes, formatDuration } from "$lib/util/format";
  import FileIcon from "./FileIcon.svelte";

  /**
   * The progress window for copies, moves, uploads and downloads. It opens by itself when a
   * transfer starts, shows each item with its progress, speed and time left, and closes a few
   * seconds after everything has finished. The toolbar's progress button reopens it.
   */
  let { open = $bindable(false) }: { open?: boolean } = $props();

  const VERB = { upload: "Uploading", download: "Downloading", copy: "Copying", move: "Moving" } as const;
  const DONE = { upload: "Uploaded", download: "Downloaded", copy: "Copied", move: "Moved" } as const;

  const active = $derived(transfers.active);
  const total = $derived(active.reduce((t, i) => t + Math.max(i.totalBytes, 0), 0));
  const done = $derived(active.reduce((t, i) => t + Math.max(i.bytes, 0), 0));
  const overall = $derived(total > 0 ? Math.min(1, done / total) : 0);

  // Open when something starts; close a little after the last one ends.
  let lastCount = 0;
  let closeTimer: number | undefined;
  $effect(() => {
    const n = active.length;
    if (n > lastCount) {
      open = true;
      window.clearTimeout(closeTimer);
    }
    if (n === 0 && lastCount > 0) {
      const failed = transfers.items.some((i) => i.state.state === "failed");
      if (!failed) closeTimer = window.setTimeout(() => (open = false), 4000);
    }
    lastCount = n;
  });

  function fraction(item: TransferItem): number {
    if (item.state.state === "done") return 1;
    return item.totalBytes > 0 ? Math.min(1, item.bytes / item.totalBytes) : 0;
  }

  function line(item: TransferItem): string {
    switch (item.state.state) {
      case "queued":
        return "Waiting…";
      case "running": {
        if (item.totalBytes <= 0) return "Starting…";
        const speed = item.speed > 0 ? ` · ${formatBytes(item.speed)}/s` : "";
        const eta = item.etaSeconds !== null ? ` · ${formatDuration(item.etaSeconds)} left` : "";
        return `${formatBytes(item.bytes)} of ${formatBytes(item.totalBytes)}${speed}${eta}`;
      }
      case "done":
        return `${DONE[item.direction]} · ${formatBytes(Math.max(item.totalBytes, item.bytes))}`;
      case "cancelled":
        return "Cancelled";
      case "failed":
        return item.state.message;
    }
  }

  const where = (p: string) => p.split(/[\\/]/).filter(Boolean).slice(-1)[0] || p;
</script>

{#if open && transfers.items.length > 0}
  <section class="progress" aria-label="File operations">
    <header>
      <strong>
        {#if active.length}
          {active.length === 1 ? `${VERB[active[0].direction]} 1 item` : `${active.length} operations`}
        {:else}
          All done
        {/if}
      </strong>
      <div class="head-actions">
        {#if transfers.finished.length}
          <button type="button" class="link" onclick={() => transfers.clearFinished()}>Clear finished</button>
        {/if}
        <button type="button" class="x" aria-label="Hide" title="Hide (keeps running)" onclick={() => (open = false)}>×</button>
      </div>
    </header>
    {#if active.length > 1}
      <div class="overall">
        <div class="bar"><span style:width="{overall * 100}%"></span></div>
        <small>{formatBytes(done)} of {formatBytes(total)}</small>
      </div>
    {/if}
    <ul>
      {#each [...transfers.items].reverse() as item (item.id)}
        <li class:failed={item.state.state === "failed"} class:finished={item.state.state === "done"}>
          <FileIcon name={item.name} isDir={item.isDir} size={28} />
          <div class="body">
            <div class="title" title={`${item.from} → ${item.to}`}>
              <span class="name">{item.name}</span>
              <span class="to">→ {where(item.to)}</span>
            </div>
            <div class="bar" class:indeterminate={item.state.state === "running" && item.totalBytes <= 0}>
              <span style:width="{fraction(item) * 100}%"></span>
            </div>
            <small>{line(item)}</small>
          </div>
          {#if item.state.state === "queued" || item.state.state === "running"}
            <button type="button" class="x" aria-label="Cancel" title="Cancel" onclick={() => transfers.cancel(item.id)}>×</button>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .progress {
    position: fixed;
    right: 18px;
    bottom: 40px;
    z-index: 40;
    width: 380px;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--window-bg);
    border: 1px solid var(--separator);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.28);
    overflow: hidden;
    animation: rise 0.18s ease-out;
  }
  @keyframes rise {
    from {
      transform: translateY(10px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px 8px 14px;
  }
  .head-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12px;
  }
  .x {
    border: none;
    background: none;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 15px;
    flex: none;
  }
  .x:hover {
    background: var(--control-bg);
    color: var(--text-primary);
  }
  .overall {
    padding: 0 14px 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0 6px 8px;
    overflow-y: auto;
  }
  li {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px;
    border-radius: 10px;
  }
  li:hover {
    background: var(--control-bg);
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title {
    display: flex;
    gap: 6px;
    min-width: 0;
    font-size: 12.5px;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .to {
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  small {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .failed small {
    color: var(--status-offline);
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--control-bg);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent-gradient);
    transition: width 0.3s ease;
  }
  .finished .bar span {
    background: var(--status-connected, #2fbf71);
  }
  .bar.indeterminate span {
    width: 35% !important;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(290%);
    }
  }
</style>
