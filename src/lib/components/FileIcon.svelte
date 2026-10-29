<script lang="ts" module>
  /** A folder with its own picture: the home folders in the sidebar. */
  export type SpecialFolder = "home" | "desktop" | "documents" | "downloads" | "music" | "pictures" | "videos" | "trash";
  export type IconTheme = "stingray" | "fluent" | "cupertino" | "outline";
  export const ICON_THEMES: { id: IconTheme; label: string; note: string }[] = [
    { id: "stingray", label: "Stingray", note: "Violet, the default" },
    { id: "fluent", label: "Fluent", note: "Warm yellow, layered" },
    { id: "cupertino", label: "Cupertino", note: "Soft blue, polished" },
    { id: "outline", label: "Outline", note: "Bold lines, clear when small" },
  ];
</script>

<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { fileKind, type FileKind } from "$lib/util/filekind";
  import { settings } from "$lib/stores/settings.svelte";

  /**
   * File and folder icons, drawn as SVG so they stay sharp at every size. Four themes share one
   * set of shapes: a folder (with a small picture for the home folders), a trash can, and a page
   * whose look shows the file's kind. Local pictures can show a thumbnail instead.
   */
  let {
    name,
    isDir,
    size = 64,
    localPath = null,
    special = null,
    theme = null,
  }: {
    name: string;
    isDir: boolean;
    size?: number;
    localPath?: string | null;
    special?: SpecialFolder | null;
    /** Overrides the theme from settings, for previews. */
    theme?: IconTheme | null;
  } = $props();

  const t = $derived<IconTheme>((theme ?? (settings.value?.iconTheme as IconTheme | undefined) ?? "stingray") as IconTheme);
  const kind = $derived<FileKind>(fileKind(name, isDir));
  const uid = $props.id();

  const FOLDER: Record<IconTheme, { back: string; front: [string, string]; edge: string; glyph: string }> = {
    stingray: { back: "#7a35ee", front: ["#b98bff", "#c25bf0"], edge: "none", glyph: "#ffffff" },
    fluent: { back: "#e9a514", front: ["#ffd75e", "#ffb92e"], edge: "#c8890b", glyph: "multi" },
    cupertino: { back: "#2c6fd1", front: ["#5aaef7", "#3b82e6"], edge: "#1f56a8", glyph: "#e8f2ff" },
    outline: { back: "#f5a623", front: ["#ffd25c", "#ffd25c"], edge: "#1d2533", glyph: "multi" },
  };

  /** Each kind's colour; Fluent fills the page with it, the others use it as an accent. */
  const KIND_COLOR: Record<FileKind, string> = {
    folder: "#8a3ffc",
    image: "#2f8cf0",
    video: "#8e5cf0",
    audio: "#1f3f8a",
    archive: "#f2b531",
    code: "#4a5568",
    spreadsheet: "#2ea44f",
    presentation: "#f26b3a",
    document: "#5b7fd6",
    pdf: "#e5322d",
    file: "#9aa3ae",
  };

  const filledPage = $derived(t === "fluent" && !["file", "document", "pdf"].includes(kind));
  const pageFill = $derived(filledPage ? KIND_COLOR[kind] : "var(--page-fill, #fff)");
  const stroke = $derived(t === "outline" ? "#1d2533" : t === "cupertino" ? "rgba(0,0,0,0.28)" : "var(--page-edge, rgba(0,0,0,0.16))");
  const strokeWidth = $derived(t === "outline" ? 2.6 : 1);
  const foldFill = $derived(t === "outline" ? "#4c9bf2" : filledPage ? "rgba(255,255,255,0.45)" : "var(--page-fold, #ececf0)");
  const ink = $derived(filledPage ? "#ffffff" : KIND_COLOR[kind]);
  const folder = $derived(FOLDER[t]);
  const multi = $derived(folder.glyph === "multi");

  /** Folder pictures: one colour for Stingray and Cupertino, each picture's own colour otherwise. */
  function g(colour: string): string {
    return multi ? colour : folder.glyph;
  }

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
</script>

{#snippet folderGlyph(which: SpecialFolder)}
  {#if which === "home"}
    <path d="M23 41 L32 33 L41 41 V49 H35.5 V44 H28.5 V49 H23 Z" fill={g("#ff6b4a")} />
  {:else if which === "desktop"}
    <rect x="22" y="32" width="20" height="13" rx="1.6" fill={g("#2f6fdc")} />
    <rect x="30" y="45" width="4" height="3" fill={g("#5b6475")} />
    <rect x="27" y="48" width="10" height="1.8" rx="0.9" fill={g("#5b6475")} />
  {:else if which === "documents"}
    <path d="M26 31 H35 L39 35 V49 H26 Z" fill={g("#ffffff")} stroke={multi ? "#5b6475" : "none"} stroke-width="1" />
    <rect x="28.5" y="38" width="8" height="1.4" rx="0.7" fill={multi ? "#5b7fd6" : "rgba(0,0,0,0.25)"} />
    <rect x="28.5" y="41.5" width="8" height="1.4" rx="0.7" fill={multi ? "#5b7fd6" : "rgba(0,0,0,0.25)"} />
    <rect x="28.5" y="45" width="5.5" height="1.4" rx="0.7" fill={multi ? "#5b7fd6" : "rgba(0,0,0,0.25)"} />
  {:else if which === "downloads"}
    <path d="M29 31 H35 V39 H40 L32 48 L24 39 H29 Z" fill={g("#2f8cf0")} />
  {:else if which === "music"}
    <path d="M28 46 a3.2 3.2 0 1 1 -1 -2.4 V33 L39 31 V43.5 a3.2 3.2 0 1 1 -1 -2.4 V34.6 L28 36.2 Z" fill={g("#8e5cf0")} />
  {:else if which === "pictures"}
    <rect x="22" y="33" width="20" height="15" rx="2" fill={g("#2f8cf0")} />
    <circle cx="37" cy="37.5" r="2" fill={multi ? "#ffd25c" : "rgba(0,0,0,0.25)"} />
    <path d="M23 47 L29 40 L33 44 L36 41 L41 47 Z" fill={multi ? "#2ea44f" : "rgba(0,0,0,0.25)"} />
  {:else if which === "videos"}
    <rect x="22" y="33" width="20" height="14" rx="2" fill={g("#5a46d6")} />
    <path d="M29.5 36.5 L36 40 L29.5 43.5 Z" fill={multi ? "#ffffff" : "rgba(0,0,0,0.3)"} />
  {/if}
{/snippet}

{#snippet pageGlyph()}
  {#if kind === "image"}
    <rect x="19" y="30" width="26" height="19" rx="2.5" fill={filledPage ? "rgba(255,255,255,0.9)" : "#dbeafe"} />
    <circle cx="39" cy="35" r="2.4" fill="#ffb02e" />
    <path d="M20 48 L28 38 L33 43 L37 39.5 L44 48 Z" fill="#2f8cf0" />
  {:else if kind === "video"}
    <rect x="19" y="31" width="26" height="18" rx="2.5" fill={filledPage ? "rgba(0,0,0,0.25)" : ink} />
    <path d="M29 35 L37 40 L29 45 Z" fill="#ffffff" />
  {:else if kind === "audio"}
    <path d="M27 47 a3.4 3.4 0 1 1 -1 -2.5 V33 L39 31 V44.5 a3.4 3.4 0 1 1 -1 -2.5 V35 L27 36.8 Z" fill={filledPage ? "#7cc0ff" : ink} />
  {:else if kind === "archive"}
    <rect x="29.5" y="8" width="5" height="44" fill={filledPage ? "rgba(0,0,0,0.18)" : "#e4e7eb"} />
    {#each [12, 17, 22, 27, 32, 37] as y (y)}
      <rect x="30.5" {y} width="3" height="2.4" rx="0.6" fill={filledPage ? "#5b6475" : "#8a94a3"} />
    {/each}
    <rect x="28.5" y="41" width="7" height="10" rx="2" fill={filledPage ? "#5b6475" : ink} />
  {:else if kind === "code"}
    <path d="M26 34 L20 40 L26 46 M38 34 L44 40 L38 46" fill="none" stroke={filledPage ? "#5ab0ff" : ink} stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M34.5 32 L29.5 48" stroke={filledPage ? "#ff9f43" : ink} stroke-width="3" stroke-linecap="round" />
  {:else if kind === "spreadsheet"}
    <rect x="19" y="29" width="26" height="20" rx="2" fill={filledPage ? "rgba(255,255,255,0.85)" : "#e8f6ec"} />
    {#each [34, 39, 44] as y (y)}<rect x="19" {y} width="26" height="0.9" fill={KIND_COLOR.spreadsheet} />{/each}
    {#each [27.5, 36.5] as x (x)}<rect {x} y="29" width="0.9" height="20" fill={KIND_COLOR.spreadsheet} />{/each}
  {:else if kind === "presentation"}
    <rect x="18" y="30" width="28" height="18" rx="2" fill={filledPage ? "rgba(255,255,255,0.25)" : "#fff1ea"} stroke={filledPage ? "rgba(255,255,255,0.7)" : ink} stroke-width="1.2" />
    <path d="M27 39 L27 33.5 A5.5 5.5 0 1 0 32.5 39 Z" fill={filledPage ? "#ffffff" : ink} />
    <rect x="35" y="35" width="8" height="1.6" rx="0.8" fill={filledPage ? "#ffffff" : ink} />
    <rect x="35" y="39" width="6" height="1.6" rx="0.8" fill={filledPage ? "#ffffff" : ink} />
  {:else if kind === "pdf"}
    <rect x="12" y="13" width="32" height="11" rx="2" fill={ink} />
    <text x="28" y="21.6" text-anchor="middle" font-size="8.5" font-weight="800" fill="#fff" font-family="Inter, sans-serif">PDF</text>
    <rect x="19" y="31" width="24" height="1.6" rx="0.8" fill="#c4c9d1" />
    <rect x="19" y="36" width="24" height="1.6" rx="0.8" fill="#c4c9d1" />
    <rect x="19" y="41" width="16" height="1.6" rx="0.8" fill="#c4c9d1" />
  {:else if kind === "document"}
    {#each [26, 31, 36, 41, 46] as y, i (y)}
      <rect x="19" {y} width={i === 4 ? 15 : 26} height="2" rx="1" fill={i % 2 ? "#9aa3ae" : ink} opacity="0.85" />
    {/each}
  {/if}
{/snippet}

{#if thumb}
  <span class="thumb" style:width="{size}px" style:height="{size}px">
    <img src={thumb} alt="" loading="lazy" decoding="async" onerror={() => (thumbFailed = true)} />
  </span>
{:else if special === "trash"}
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" class={t}>
    <defs>
      <linearGradient id="can-{uid}" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stop-color={t === "outline" ? "#c9d6ea" : "#e3e7ec"} />
        <stop offset="0.5" stop-color="#ffffff" />
        <stop offset="1" stop-color={t === "outline" ? "#aebfd9" : "#c4cad2"} />
      </linearGradient>
    </defs>
    <path d="M14 16 H50 L46 56 a3 3 0 0 1 -3 2.6 H21 a3 3 0 0 1 -3 -2.6 Z" fill="url(#can-{uid})" stroke={t === "outline" ? "#1d2533" : "#8a94a3"} stroke-width={t === "outline" ? 2.6 : 1.2} stroke-linejoin="round" />
    <ellipse cx="32" cy="16" rx="18.5" ry="4.5" fill={t === "outline" ? "#3b5b8f" : "#d5dae1"} stroke={t === "outline" ? "#1d2533" : "#8a94a3"} stroke-width={t === "outline" ? 2.6 : 1.2} />
    {#each [24, 32, 40] as x (x)}<path d="M{x} 25 V51" stroke={t === "outline" ? "#5b6f8f" : "#a3abb6"} stroke-width="1.6" stroke-linecap="round" />{/each}
  </svg>
{:else if isDir}
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" class={t}>
    <defs>
      <linearGradient id="front-{uid}" x1="0" y1="0" x2={t === "stingray" ? 1 : 0} y2="1">
        <stop offset="0" stop-color={folder.front[0]} />
        <stop offset="1" stop-color={folder.front[1]} />
      </linearGradient>
    </defs>
    {#if t === "outline"}
      <path d="M7 15 a3 3 0 0 1 3 -3 H24 l4 4 H40 a3 3 0 0 1 3 3 V48 L10 54 a3 3 0 0 1 -3 -3 Z" fill={folder.back} stroke={folder.edge} stroke-width="2.6" stroke-linejoin="round" />
      <path d="M14 23 L54 18 a3 3 0 0 1 3.3 3 L56 48 a3 3 0 0 1 -2.6 2.9 L13 56 a3 3 0 0 1 -3.3 -3 L11.2 26 a3 3 0 0 1 2.8 -3 Z" fill="url(#front-{uid})" stroke={folder.edge} stroke-width="2.6" stroke-linejoin="round" />
    {:else}
      <path d="M6 15 c0 -2.2 1.8 -4 4 -4 h14.5 c1.3 0 2.5 .6 3.2 1.6 L30.5 16 H54 c2.2 0 4 1.8 4 4 v4 H6 z" fill={folder.back} />
      <rect x="6" y="20" width="52" height="35" rx={t === "cupertino" ? 6 : 5} fill="url(#front-{uid})" stroke={folder.edge === "none" ? "none" : folder.edge} stroke-width="1.2" />
      <rect x="7" y="20.6" width="50" height="1.4" rx="0.7" fill="#fff" opacity={t === "fluent" ? 0.55 : 0.35} />
      {#if t === "cupertino"}
        <rect x="10" y="51" width="44" height="0.9" rx="0.45" fill="#ffffff" opacity="0.35" />
      {/if}
    {/if}
    {#if special}{@render folderGlyph(special)}{/if}
  </svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" class={t}>
    <defs>
      <linearGradient id="page-{uid}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#ffffff" stop-opacity={filledPage ? 0.28 : 0} />
        <stop offset="1" stop-color="#000000" stop-opacity={filledPage ? 0.08 : 0} />
      </linearGradient>
    </defs>
    <path d="M15 5 h24 l12 12 v40 a3 3 0 0 1 -3 3 H15 a3 3 0 0 1 -3 -3 V8 a3 3 0 0 1 3 -3 z" fill={pageFill} stroke={stroke} stroke-width={strokeWidth} stroke-linejoin="round" />
    <path d="M15 5 h24 l12 12 v40 a3 3 0 0 1 -3 3 H15 a3 3 0 0 1 -3 -3 V8 a3 3 0 0 1 3 -3 z" fill="url(#page-{uid})" />
    {#if t === "cupertino" && kind !== "file" && kind !== "document" && kind !== "pdf"}
      <path d="M15 5 h24 l3 3 V14 H12 V8 a3 3 0 0 1 3 -3 z" fill={KIND_COLOR[kind]} />
    {/if}
    <path d="M39 5 v9 a3 3 0 0 0 3 3 h9" fill={foldFill} stroke={stroke} stroke-width={strokeWidth} stroke-linejoin="round" />
    {@render pageGlyph()}
  </svg>
{/if}

<style>
  svg {
    display: block;
    flex: none;
    overflow: visible;
  }
  svg.stingray,
  svg.fluent,
  svg.cupertino {
    filter: drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.2));
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
