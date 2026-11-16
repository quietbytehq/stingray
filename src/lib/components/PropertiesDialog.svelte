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

  const BITS = [
    ["Owner", 0],
    ["Group", 3],
    ["Others", 6],
  ] as const;
  const RIGHTS = [
    ["Read", 0, 4],
    ["Change", 1, 2],
    ["Run", 2, 1],
  ] as const;
  let saving = $state(false);

  /** Turns one tick box on or off and saves straight away. */
  async function toggle(offset: number, index: number) {
    if (!info) return;
    const chars = info.permissions.split("");
    const letter = ["r", "w", "x"][index];
    chars[offset + index] = chars[offset + index] === "-" ? letter : "-";
    const next = chars.join("");
    const mode = [0, 3, 6].reduce((m, o) => m * 8 + (next[o] !== "-" ? 4 : 0) + (next[o + 1] !== "-" ? 2 : 0) + (next[o + 2] !== "-" ? 1 : 0), 0);
    saving = true;
    try {
      await api.setPermissions(path, mode);
      info.permissions = next;
    } catch (err) {
      error = asAppError(err);
    } finally {
      saving = false;
    }
  }

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
    {#if info.permissions}
      <h4 class="perm-title">Permissions {#if saving}<span class="muted">saving…</span>{/if}</h4>
      <div class="perms">
        <span></span>
        {#each RIGHTS as [label] (label)}<span class="col">{label === "Run" && info.isDir ? "Open" : label}</span>{/each}
        {#each BITS as [who, offset] (who)}
          <span class="who">{who}</span>
          {#each RIGHTS as [label, index] (label)}
            <input type="checkbox" aria-label="{who} {label}" checked={info.permissions[offset + index] !== "-"} onchange={() => void toggle(offset, index)} />
          {/each}
        {/each}
      </div>
    {/if}
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
  .perm-title {
    margin: 16px 0 6px;
    font-size: 12px;
  }
  .perms {
    display: grid;
    grid-template-columns: 90px repeat(3, 64px);
    gap: 6px 4px;
    align-items: center;
    justify-items: center;
    font-size: 12px;
  }
  .perms .who {
    justify-self: end;
    color: var(--text-secondary);
    padding-right: 8px;
  }
  .perms .col {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .path {
    word-break: break-all;
  }
  .muted {
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
