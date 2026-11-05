<script lang="ts">
  import { onMount } from "svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, FileProperties } from "$lib/api/types";
  import { formatBytes } from "$lib/util/format";
  import Dialog from "./Dialog.svelte";
  import ErrorNote from "./ErrorNote.svelte";
  import FileIcon from "./FileIcon.svelte";

  /** Properties of one file or folder: size (and contents for folders), type, place, dates, access. */
  let { path, onClose }: { path: string; onClose: () => void } = $props();
  let info = $state<FileProperties | null>(null);
  let error = $state<AppError | null>(null);

  onMount(async () => {
    try {
      info = await api.fileProperties(path);
    } catch (err) {
      error = asAppError(err);
    }
  });

  function date(iso: string | null): string {
    if (!iso) return "—";
    return new Date(iso).toLocaleString(undefined, { dateStyle: "long", timeStyle: "short" });
  }

  /** "rwxr-xr-x" as words: who can read, change and open it. */
  function access(perm: string, offset: number): string {
    const p = perm.slice(offset, offset + 3);
    if (!p) return "—";
    const parts = [p[0] === "r" && "read", p[1] === "w" && "change", p[2] === "x" && (info?.isDir ? "open" : "run")].filter(Boolean);
    return parts.length ? parts.join(", ") : "none";
  }
</script>

<Dialog title="Properties" {onClose}>
  <ErrorNote {error} />
  {#if info}
    <div class="head">
      <FileIcon name={info.name} isDir={info.isDir} size={56} />
      <div>
        <strong>{info.name}</strong>
        <span class="muted">{info.contentType || (info.isDir ? "Folder" : "File")}</span>
      </div>
    </div>
    <dl>
      <dt>Size</dt>
      <dd>
        {formatBytes(info.size)}
        {#if info.isDir}<span class="muted"> · {info.items.toLocaleString()} items inside</span>{/if}
      </dd>
      <dt>Location</dt>
      <dd class="path">{info.location}</dd>
      <dt>Modified</dt>
      <dd>{date(info.modified)}</dd>
      <dt>Accessed</dt>
      <dd>{date(info.accessed)}</dd>
      <dt>Created</dt>
      <dd>{date(info.created)}</dd>
      {#if info.permissions}
        <dt>Owner</dt>
        <dd>{info.owner}: {access(info.permissions, 0)}</dd>
        <dt>Group</dt>
        <dd>{access(info.permissions, 3)}</dd>
        <dt>Others</dt>
        <dd>{access(info.permissions, 6)} <span class="muted">({info.permissions})</span></dd>
      {/if}
    </dl>
  {:else if !error}
    <p class="muted">Reading…</p>
  {/if}
</Dialog>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
    min-width: 380px;
  }
  .head strong {
    display: block;
    font-size: 15px;
    word-break: break-all;
  }
  dl {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 6px 12px;
    margin: 0;
  }
  dt {
    color: var(--text-secondary);
    text-align: right;
  }
  dd {
    margin: 0;
    user-select: text;
    -webkit-user-select: text;
  }
  .path {
    word-break: break-all;
  }
  .muted {
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
