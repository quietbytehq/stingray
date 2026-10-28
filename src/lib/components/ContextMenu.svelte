<script lang="ts" module>
  export interface MenuItem {
    label: string;
    action: () => void;
    destructive?: boolean;
    disabled?: boolean;
    /** Draw a separator above this item. */
    separator?: boolean;
  }
</script>

<script lang="ts">
  import { onMount, tick } from "svelte";

  let { x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void } = $props();
  let menu: HTMLDivElement;
  let left = $state(0);
  let top = $state(0);

  onMount(async () => {
    left = x;
    top = y;
    await tick();
    const rect = menu.getBoundingClientRect();
    left = Math.min(x, window.innerWidth - rect.width - 6);
    top = Math.min(y, window.innerHeight - rect.height - 6);
    menu.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
  });

  function pick(item: MenuItem) {
    onClose();
    item.action();
  }

  function keydown(event: KeyboardEvent) {
    const buttons = [...menu.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === "Escape") onClose();
    else if (event.key === "ArrowDown") buttons[(at + 1) % buttons.length]?.focus();
    else if (event.key === "ArrowUp") buttons[(at - 1 + buttons.length) % buttons.length]?.focus();
    else return;
    event.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onpointerdown={onClose} oncontextmenu={(e) => { e.preventDefault(); onClose(); }}></div>
<div class="menu" role="menu" tabindex="-1" bind:this={menu} style:left="{left}px" style:top="{top}px" onkeydown={keydown}>
  {#each items as item (item.label)}
    {#if item.separator}<div class="sep" role="separator"></div>{/if}
    <button type="button" role="menuitem" class:destructive={item.destructive} disabled={item.disabled} onclick={() => pick(item)}>
      {item.label}
    </button>
  {/each}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: fixed;
    z-index: 41;
    min-width: 190px;
    padding: 5px;
    border-radius: 10px;
    background: var(--panel-bg);
    backdrop-filter: blur(40px) saturate(180%);
    -webkit-backdrop-filter: blur(40px) saturate(180%);
    box-shadow: 0 0 0 0.5px var(--separator), 0 10px 30px rgba(0, 0, 0, 0.22);
  }
  button {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    padding: 4px 10px;
    border-radius: 5px;
    cursor: default;
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--accent);
    color: #fff;
    outline: none;
  }
  button:disabled {
    opacity: 0.4;
  }
  .destructive {
    color: var(--status-disconnected);
  }
  .sep {
    height: 1px;
    margin: 4px 8px;
    background: var(--separator);
  }
</style>
