<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { fileKind, type FileKind } from "$lib/util/filekind";

  /**
   * File and folder icons drawn in the spirit of macOS: a soft folder in the Stingray violet,
   * and a page with a folded corner and a coloured band for its kind. Local pictures show a
   * thumbnail instead, loaded lazily through Tauri's asset protocol.
   */
  let {
    name,
    isDir,
    size = 64,
    localPath = null,
  }: { name: string; isDir: boolean; size?: number; localPath?: string | null } = $props();

  const kind = $derived<FileKind>(fileKind(name, isDir));
  const ext = $derived.by(() => {
    const dot = name.lastIndexOf(".");
    return !isDir && dot > 0 ? name.slice(dot + 1).slice(0, 4).toUpperCase() : "";
  });

  const BAND: Record<FileKind, string> = {
    folder: "#8a3ffc",
    image: "#ff9f0a",
    video: "#ff375f",
    audio: "#e81cc4",
    archive: "#a2845e",
    code: "#30b0c7",
    spreadsheet: "#30d158",
    presentation: "#ff9f0a",
    document: "#0a84ff",
    pdf: "#ff453a",
    file: "#8e8e93",
  };

  const THUMB_EXT = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "avif", "svg"]);
  const canThumb = $derived(
    localPath !== null && kind === "image" && THUMB_EXT.has(name.slice(name.lastIndexOf(".") + 1).toLowerCase()),
  );
  let thumbFailed = $state(false);
  const thumb = $derived.by(() => {
    if (!canThumb || thumbFailed || !localPath) return null;
    try {
      return convertFileSrc(localPath);
    } catch {
      return null;
    }
  });
  const uid = $props.id();
</script>

{#if thumb}
  <span class="thumb" style:width="{size}px" style:height="{size}px">
    <img src={thumb} alt="" loading="lazy" decoding="async" onerror={() => (thumbFailed = true)} />
  </span>
{:else if isDir}
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
    <defs>
      <linearGradient id="back-{uid}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#9d63ff" />
        <stop offset="1" stop-color="#7a35ee" />
      </linearGradient>
      <linearGradient id="front-{uid}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#b98bff" />
        <stop offset="0.6" stop-color="#a064ff" />
        <stop offset="1" stop-color="#c25bf0" />
      </linearGradient>
    </defs>
    <path d="M6 15c0-2.2 1.8-4 4-4h14.5c1.3 0 2.5.6 3.2 1.6L30.5 16H54c2.2 0 4 1.8 4 4v4H6z" fill="url(#back-{uid})" />
    <rect x="6" y="20" width="52" height="34" rx="5" fill="url(#front-{uid})" />
    <rect x="6" y="20" width="52" height="1.2" fill="#fff" opacity="0.35" />
  </svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
    <path d="M15 5h24l12 12v40a3 3 0 0 1-3 3H15a3 3 0 0 1-3-3V8a3 3 0 0 1 3-3z" fill="var(--page-fill, #fff)" stroke="var(--page-edge, rgba(0,0,0,0.14))" stroke-width="1" />
    <path d="M39 5v9a3 3 0 0 0 3 3h9" fill="var(--page-fold, #ececf0)" stroke="var(--page-edge, rgba(0,0,0,0.14))" stroke-width="1" />
    {#if ext}
      <rect x="16" y="40" width="31" height="12" rx="3" fill={BAND[kind]} />
      <text x="31.5" y="49" text-anchor="middle" font-size="8" font-weight="700" fill="#fff" font-family="Inter, sans-serif">{ext}</text>
    {:else}
      <rect x="18" y="26" width="24" height="2.5" rx="1.25" fill={BAND[kind]} opacity="0.7" />
      <rect x="18" y="32" width="20" height="2.5" rx="1.25" fill={BAND[kind]} opacity="0.5" />
    {/if}
  </svg>
{/if}

<style>
  svg {
    display: block;
    flex: none;
    filter: drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.18));
  }
  .thumb {
    display: grid;
    place-items: center;
    flex: none;
  }
  img {
    max-width: 100%;
    max-height: 100%;
    border-radius: 4px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    object-fit: contain;
  }
</style>
