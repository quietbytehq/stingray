<script lang="ts">
  import Dialog from "./Dialog.svelte";

  /**
   * Compress…: archive name, format (.zip or .7z) and an optional password. With a password the
   * archive is encrypted with AES-256; .7z also hides the file names.
   */
  let {
    suggested,
    count,
    onCreate,
    onClose,
  }: {
    suggested: string;
    count: number;
    onCreate: (name: string, format: "zip" | "7z", password: string | null) => void;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let name = $state(suggested);
  let format = $state<"zip" | "7z">("zip");
  let protect = $state(false);
  let password = $state("");
  let confirm = $state("");
  let show = $state(false);

  const problem = $derived(
    !name.trim() || /[\\/]/.test(name)
      ? "Give the archive a name without slashes."
      : protect && password.length < 4
        ? "Use a password of at least 4 characters."
        : protect && password !== confirm
          ? "The passwords don't match."
          : null,
  );

  function create() {
    if (problem) return;
    onCreate(name.trim(), format, protect ? password : null);
  }
</script>

<Dialog title={`Compress ${count === 1 ? "1 item" : `${count} items`}`} {onClose}>
  <form class="form" onsubmit={(e) => { e.preventDefault(); create(); }}>
    <label>Archive name
      <span class="name">
        <input bind:value={name} spellcheck="false" />
        <span class="ext">.{format}</span>
      </span>
    </label>

    <div class="formats" role="radiogroup" aria-label="Format">
      <button type="button" role="radio" aria-checked={format === "zip"} onclick={() => (format = "zip")}>
        <strong>.zip</strong><small>Opens everywhere, including Windows</small>
      </button>
      <button type="button" role="radio" aria-checked={format === "7z"} onclick={() => (format = "7z")}>
        <strong>.7z</strong><small>Smaller files; a password also hides file names</small>
      </button>
    </div>

    <label class="check"><input type="checkbox" bind:checked={protect} /> Protect with a password (AES-256 encryption)</label>
    {#if protect}
      <label>Password
        <input type={show ? "text" : "password"} bind:value={password} autocomplete="new-password" />
      </label>
      <label>Confirm password
        <input type={show ? "text" : "password"} bind:value={confirm} autocomplete="new-password" />
      </label>
      <label class="check small"><input type="checkbox" bind:checked={show} /> Show password</label>
      <p class="hint">Keep this password safe. Without it, nobody (including you) can open the archive.</p>
    {/if}

    {#if problem && (name || protect)}<p class="problem">{problem}</p>{/if}
    <div class="actions">
      <button type="button" class="button" onclick={onClose}>Cancel</button>
      <button type="submit" class="button primary" disabled={!!problem}>Create</button>
    </div>
  </form>
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(440px, calc(100vw - 140px));
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .name input {
    flex: 1;
  }
  .ext {
    color: var(--text-secondary);
    font-weight: 400;
  }
  input:not([type="checkbox"]) {
    padding: 7px 9px;
    border-radius: 7px;
    border: 1px solid var(--separator);
    background: var(--window-bg);
    color: inherit;
    font-weight: 400;
    user-select: text;
    -webkit-user-select: text;
  }
  .formats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .formats button {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    padding: 8px 10px;
    border-radius: 9px;
    border: 1px solid var(--separator);
    background: none;
  }
  .formats button[aria-checked="true"] {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .formats small {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-weight: 400;
    font-size: 13px;
  }
  .check.small {
    font-size: 12px;
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .problem {
    margin: 0;
    font-size: 12px;
    color: var(--status-offline);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
