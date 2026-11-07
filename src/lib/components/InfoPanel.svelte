<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "$lib/api";
  import type { FileEntry, FileProperties } from "$lib/api/types";
  import { fileKind } from "$lib/util/filekind";
  import { formatBytes } from "$lib/util/format";
  import FileIcon from "./FileIcon.svelte";

  /**
   * The information panel (F11): a large preview of what is selected (pictures, video and audio
   * that play right here, the start of text files) and its details. Previews are for files on
   * this computer; cloud files show their icon and details.
   */
  let {
    entry,
    count,
    local,
    onClose,
  }: { entry: FileEntry | null; count: number; local: boolean; onClose: () => void } = $props();

  const kind = $derived(entry ? fileKind(entry.name, entry.isDir) : "file");
  const canPreview = $derived(!!entry && local && !entry.isDir && !entry.path.includes("!/"));
  const src = $derived(canPreview && entry ? convertFileSrc(entry.path) : "");
  const TEXT = /\.(txt|md|log|json|ya?ml|toml|ini|conf|cfg|csv|tsv|sh|py|rs|js|ts|svelte|html|css|xml|c|h|cpp|java|go|sql)$/i;

  let text = $state<string | null>(null);
  let details = $state<FileProperties | null>(null);
  let mediaFailed = $state(false);

  $effect(() => {
    const current = entry;
    text = null;
    details = null;
    mediaFailed = false;
    if (!current) return;
    if (local && !current.path.includes("!/")) {
      void api.fileProperties(current.path).then((p) => {
        if (entry === current) details = p;
      }).catch(() => {});
    }
    if (canPreview && TEXT.test(current.name) && current.size < 2_000_000) {
      void fetch(src)
        .then((r) => r.text())
        .then((t) => {
          if (entry === current) text = t.slice(0, 4000);
        })
        .catch(() => {});
    }
  });

  function date(iso: string | null | undefined): string {
    return iso ? new Date(iso).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" }) : "—";
  }
</script>

<aside class="info" aria-label="Information">
  <header>
    <span>Information</span>
    <button type="button" aria-label="Close information panel" title="Close (F11)" onclick={onClose}>×</button>
  </header>
  {#if !entry}
    <p class="empty">{count > 1 ? `${count} items selected` : "Select a file or folder to see it here."}</p>
  {:else}
    <div class="preview">
      {#if canPreview && kind === "image" && !mediaFailed}
        <img {src} alt="" onerror={() => (mediaFailed = true)} />
      {:else if canPreview && kind === "video" && !mediaFailed}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video {src} controls preload="metadata" onerror={() => (mediaFailed = true)}></video>
      {:else if canPreview && kind === "audio" && !mediaFailed}
        <div class="audio">
          <FileIcon name={entry.name} isDir={false} size={96} />
          <audio {src} controls preload="metadata" onerror={() => (mediaFailed = true)}></audio>
        </div>
      {:else if text !== null}
        <pre>{text}</pre>
      {:else}
        <FileIcon name={entry.name} isDir={entry.isDir} size={128} />
      {/if}
    </div>
    <h3 title={entry.name}>{entry.name}</h3>
    <dl>
      <dt>Type</dt>
      <dd>{details?.contentType || (entry.isDir ? "Folder" : kind)}</dd>
      <dt>Size</dt>
      <dd>
        {#if entry.isDir}
          {details ? `${formatBytes(details.size)} · ${details.items.toLocaleString()} items` : "—"}
        {:else}
          {entry.size >= 0 ? formatBytes(entry.size) : "—"}
        {/if}
      </dd>
      <dt>Modified</dt>
      <dd>{date(details?.modified ?? entry.modTime)}</dd>
      {#if details?.created}
        <dt>Created</dt>
        <dd>{date(details.created)}</dd>
      {/if}
      {#if details?.location}
        <dt>Location</dt>
        <dd class="path">{details.location}</dd>
      {/if}
    </dl>
  {/if}
</aside>

<style>
  .info {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    border-left: 1px solid var(--separator);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--separator);
  }
  header button {
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 15px;
    border-radius: 5px;
    width: 20px;
    height: 20px;
  }
  header button:hover {
    background: var(--control-bg);
  }
  .empty {
    padding: 20px 14px;
    color: var(--text-secondary);
    text-align: center;
  }
  .preview {
    display: grid;
    place-items: center;
    padding: 14px;
    min-height: 160px;
  }
  .preview img,
  .preview video {
    max-width: 100%;
    max-height: 300px;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
  }
  .audio {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    width: 100%;
  }
  .audio audio {
    width: 100%;
  }
  pre {
    width: 100%;
    max-height: 300px;
    overflow: auto;
    margin: 0;
    padding: 8px;
    border-radius: 8px;
    background: var(--control-bg);
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
    -webkit-user-select: text;
  }
  h3 {
    margin: 0 14px 8px;
    font-size: 14px;
    word-break: break-all;
  }
  dl {
    display: grid;
    grid-template-columns: 72px 1fr;
    gap: 5px 10px;
    margin: 0 14px 14px;
    font-size: 12px;
  }
  dt {
    color: var(--text-secondary);
  }
  dd {
    margin: 0;
  }
  .path {
    word-break: break-all;
  }
</style>
