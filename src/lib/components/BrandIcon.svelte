<script lang="ts">
  import { Cloud } from "$lib/icons";

  /**
   * A cloud service's official logo, from static/brands, shown unchanged as its brand
   * guidelines ask. Optionally carries a small coloured badge with an initial, so two accounts
   * of the same service can be told apart at a glance.
   */
  let {
    provider,
    size = 16,
    badge = null,
  }: { provider: string | null; size?: number; badge?: { letter: string; color: string } | null } = $props();

  const FILES: Record<string, string> = {
    drive: "/brands/google-drive.png",
    onedrive: "/brands/onedrive.png",
    dropbox: "/brands/dropbox.png",
  };
  const src = $derived(provider ? FILES[provider] : undefined);
  let failed = $state(false);
</script>

<span class="brand" style:width="{size}px" style:height="{size}px">
  {#if src && !failed}
    <img {src} alt="" width={size} height={size} onerror={() => (failed = true)} />
  {:else}
    <Cloud size={size} strokeWidth={1.75} aria-hidden="true" />
  {/if}
  {#if badge}
    <span class="badge" style:background={badge.color}>{badge.letter}</span>
  {/if}
</span>

<style>
  .brand {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }
  img {
    display: block;
    object-fit: contain;
  }
  .badge {
    position: absolute;
    right: -5px;
    bottom: -4px;
    min-width: 11px;
    height: 11px;
    padding: 0 2px;
    border-radius: 6px;
    color: #fff;
    font-size: 7.5px;
    font-weight: 700;
    line-height: 11px;
    text-align: center;
    box-shadow: 0 0 0 1.5px var(--sidebar-solid, #f0eef5);
  }
</style>
