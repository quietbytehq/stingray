<script lang="ts">
  import { onMount } from "svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, NfsSupport, ProviderDef } from "$lib/api/types";
  import { drives } from "$lib/stores/drives.svelte";
  import { Network, Server, Terminal } from "$lib/icons";
  import Dialog from "./Dialog.svelte";
  import ErrorNote from "./ErrorNote.svelte";
  import RemoteWizard from "./RemoteWizard.svelte";

  /** Connects a network drive: SMB and SFTP through the setup wizard, NFS through the system. */
  let { onClose }: { onClose: () => void } = $props();

  type Choice = "pick" | "smb" | "sftp" | "nfs";
  let choice = $state<Choice>("pick");
  let nfsSupport = $state<NfsSupport | null>(null);
  let nfs = $state({ name: "", server: "", export: "" });
  let busy = $state(false);
  let error = $state<AppError | null>(null);

  onMount(() => {
    api.nfsSupport().then((s) => (nfsSupport = s)).catch(() => {});
  });

  const provider = $derived<ProviderDef | undefined>(
    choice === "smb" || choice === "sftp" ? drives.provider(choice) : undefined,
  );

  async function done() {
    await drives.refresh();
    onClose();
  }

  async function saveNfs(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      const share = await api.addNfsShare({ ...nfs });
      await api.connectNfsShare(share.name).catch((err) => {
        // Saved even when the first connect fails; it shows red in the sidebar and can be retried.
        error = asAppError(err);
      });
      await drives.refresh();
      if (!error) onClose();
    } catch (err) {
      error = asAppError(err);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog title="Connect to server" {onClose}>
  {#if choice === "pick"}
    <p class="intro">Pick how the server shares its files. If you're not sure, Windows and most home NAS boxes use SMB.</p>
    <div class="choices">
      <button type="button" class="choice" onclick={() => (choice = "smb")}>
        <Network size={22} strokeWidth={1.6} aria-hidden="true" />
        <strong>SMB</strong>
        <span>Windows shares, Samba, most NAS boxes</span>
      </button>
      <button type="button" class="choice" onclick={() => (choice = "sftp")}>
        <Terminal size={22} strokeWidth={1.6} aria-hidden="true" />
        <strong>SFTP</strong>
        <span>Linux servers you reach over SSH</span>
      </button>
      <button type="button" class="choice" onclick={() => (choice = "nfs")} disabled={nfsSupport !== null && !nfsSupport.available}>
        <Server size={22} strokeWidth={1.6} aria-hidden="true" />
        <strong>NFS</strong>
        <span>Unix and Linux file servers</span>
      </button>
    </div>
    {#if nfsSupport && !nfsSupport.available && nfsSupport.message}
      <p class="help">{nfsSupport.message}</p>
    {/if}
  {:else if choice === "nfs"}
    <form class="form" onsubmit={saveNfs}>
      <label>Name <input bind:value={nfs.name} placeholder="Office NAS" spellcheck="false" required /></label>
      <label>Server <input bind:value={nfs.server} placeholder="192.168.1.20 or nas.local" spellcheck="false" required /></label>
      <label>Shared folder <input bind:value={nfs.export} placeholder="/srv/share" spellcheck="false" required /></label>
      <p class="help">Your computer asks for your password to connect the share. It then opens like any other folder.</p>
      <ErrorNote {error} />
      <div class="actions">
        <button type="button" class="button" onclick={() => (choice = "pick")}>Back</button>
        <button type="submit" class="button primary" disabled={busy}>{busy ? "Connecting…" : "Connect"}</button>
      </div>
    </form>
  {:else if provider}
    <RemoteWizard {provider} onBack={() => (choice = "pick")} onDone={() => void done()} />
  {:else}
    <p class="help">Stingray's storage engine is still starting. Try again in a moment.</p>
  {/if}
</Dialog>

<style>
  .intro {
    margin: 0 0 14px;
    color: var(--text-secondary);
    max-width: 52ch;
  }
  .choices {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 14px;
    border: 1px solid var(--separator);
    border-radius: 12px;
    background: var(--control-bg);
    text-align: left;
    cursor: pointer;
  }
  .choice :global(svg) {
    color: var(--accent);
    margin-bottom: 4px;
  }
  .choice span {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .choice:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .choice:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 380px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-weight: 500;
  }
  input {
    padding: 6px 9px;
    border-radius: 7px;
    border: 1px solid var(--separator);
    background: var(--window-bg);
    user-select: text;
    font-weight: 400;
  }
  .help {
    margin: 10px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }
</style>
