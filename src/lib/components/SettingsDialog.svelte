<script lang="ts">
  import { untrack } from "svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, OneDriveLibrary, ProviderDef, RemoteForm, RemoteSummary, Theme } from "$lib/api/types";
  import { onSignInUrl } from "$lib/api";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { drives, BADGE_COLORS, CLOUD_PROVIDERS, CLOUD_TITLES } from "$lib/stores/drives.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { formatBytes } from "$lib/util/format";
  import { Network, Plus, Server, Terminal, X } from "$lib/icons";
  import BrandIcon from "./BrandIcon.svelte";
  import IconThemePicker from "./IconThemePicker.svelte";
  import ErrorNote from "./ErrorNote.svelte";
  import RemoteWizard from "./RemoteWizard.svelte";

  /** Stingray's settings: accounts and appearance. Every change is saved as it is made. */
  let {
    section = "accounts",
    onClose,
    onConnectServer,
  }: { section?: string; onClose: () => void; onConnectServer: () => void } = $props();

  let tab = $state(untrack(() => section));
  let error = $state<AppError | null>(null);
  /** The setup wizard shown in place of the list, for a new account or a sign-in again. */
  let wizard = $state<{ provider: ProviderDef; existing: RemoteForm | null } | null>(null);

  const TABS = [
    { id: "accounts", label: "Accounts" },
    { id: "appearance", label: "Appearance" },
    { id: "about", label: "About" },
  ];

  let version = $state("");
  $effect(() => {
    if (tab === "about" && !version) void api.appInfo().then((info) => (version = info.version)).catch(() => {});
  });

  function open(url: string) {
    void openUrl(url).catch(() => window.open(url, "_blank"));
  }

  const DRIVE_TYPES: Record<string, string> = {
    personal: "Personal",
    business: "Work or school",
    documentLibrary: "SharePoint library",
  };

  function addAccount(id: string) {
    const provider = drives.provider(id);
    if (provider) wizard = { provider, existing: null };
    else error = { kind: "notRunning", message: "The storage engine is still starting. Try again in a moment." } as AppError;
  }

  const BROWSER = ["drive", "onedrive", "dropbox"];
  /** Connection currently waiting for the browser sign-in, with its sign-in link once known. */
  let signingIn = $state<{ name: string; url: string | null } | null>(null);

  /** Cloud accounts: run the browser sign-in again, keeping the account's name and settings. */
  async function signInAgain(remote: RemoteSummary) {
    signingIn = { name: remote.name, url: null };
    error = null;
    const stop = await onSignInUrl((url) => {
      if (signingIn) signingIn.url = url;
      void openUrl(url).catch(() => {});
    });
    try {
      await api.reconnectRemote(remote.name);
      await drives.refresh();
    } catch (err) {
      error = asAppError(err);
    } finally {
      stop();
      signingIn = null;
    }
  }

  async function cancelSignIn() {
    await api.cancelBrowserSignIn().catch(() => {});
  }

  /** OneDrive: libraries the account can open, loaded when asked for. */
  let libraries = $state<Record<string, OneDriveLibrary[] | "loading">>({});

  async function loadLibraries(remote: RemoteSummary) {
    libraries[remote.name] = "loading";
    try {
      libraries[remote.name] = await api.onedriveLibraries(remote.name);
    } catch (err) {
      delete libraries[remote.name];
      error = asAppError(err);
    }
  }

  async function chooseLibrary(remote: RemoteSummary, id: string) {
    try {
      await api.setOnedriveLibrary(remote.name, id);
      await loadLibraries(remote);
      await drives.refresh();
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function reconnect(remote: RemoteSummary) {
    const provider = drives.provider(remote.providerId ?? "");
    if (!provider) return;
    try {
      wizard = { provider, existing: await api.getRemoteForm(remote.name) };
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function remove(remote: RemoteSummary) {
    const name = drives.displayName(remote);
    if (!window.confirm(`Remove ${name} from Stingray? Its sign-in is forgotten. Your files stay where they are.`)) return;
    try {
      await api.deleteRemote(remote.name);
      const accounts = { ...settings.value?.accounts };
      delete accounts[remote.name];
      await settings.patch({ accounts });
      await drives.refresh();
    } catch (err) {
      error = asAppError(err);
    }
  }

  async function setLook(remote: RemoteSummary, change: { label?: string; color?: string }) {
    const current = settings.value?.accounts[remote.name] ?? { label: "", color: "" };
    await settings.patch({ accounts: { ...settings.value?.accounts, [remote.name]: { ...current, ...change } } });
  }

  async function wizardDone() {
    wizard = null;
    await drives.refresh();
  }

  const networkAccounts = $derived(drives.network);
  const iconSize = $derived(settings.value?.iconSize ?? 56);
</script>

{#snippet account(remote: RemoteSummary, id: string | null)}
  {@const info = drives.info[remote.name]}
  {@const look = settings.value?.accounts[remote.name]}
  {@const color = drives.badgeColor(remote)}
  <div class="account">
    <BrandIcon provider={id} size={28} badge={{ letter: drives.displayName(remote).slice(0, 1).toUpperCase(), color }} />
    <div class="details">
      <input
        class="label-input"
        value={look?.label ?? ""}
        placeholder={info?.user ?? remote.name}
        aria-label="Label for {remote.name}"
        spellcheck="false"
        onchange={(e) => void setLook(remote, { label: (e.currentTarget as HTMLInputElement).value })}
      />
      <div class="meta">
        {#if info?.user}<span>{info.user}</span>{/if}
        {#if info?.driveType}<span>{DRIVE_TYPES[info.driveType] ?? info.driveType}</span>{/if}
        {#if info?.usedBytes != null && info?.totalBytes}
          <span>{formatBytes(info.usedBytes)} of {formatBytes(info.totalBytes)} used</span>
        {/if}
        <span class="conn">{remote.name}</span>
      </div>
      {#if info?.usedBytes != null && info?.totalBytes}
        <div class="bar"><span style:width="{Math.min(100, (info.usedBytes / info.totalBytes) * 100)}%"></span></div>
      {/if}
      <div class="swatches" role="radiogroup" aria-label="Badge colour">
        {#each BADGE_COLORS as swatch (swatch)}
          <button
            type="button"
            class="swatch"
            role="radio"
            aria-checked={swatch === color}
            aria-label="Colour {swatch}"
            style:background={swatch}
            onclick={() => void setLook(remote, { color: swatch })}
          ></button>
        {/each}
      </div>
    </div>
    <div class="row-actions">
      {#if signingIn?.name === remote.name}
        <span class="waiting">Waiting for your browser…</span>
        {#if signingIn.url}<a class="link" href={signingIn.url} target="_blank" rel="noopener">Open sign-in page</a>{/if}
        <button type="button" class="button" onclick={() => void cancelSignIn()}>Cancel</button>
      {:else}
        <button
          type="button"
          class="button"
          disabled={signingIn !== null}
          onclick={() => void (BROWSER.includes(remote.providerId ?? "") ? signInAgain(remote) : reconnect(remote))}
        >Sign in again</button>
        {#if remote.providerId === "drive"}
          <button type="button" class="button" onclick={() => void reconnect(remote)}>Edit settings</button>
        {/if}
        <button type="button" class="button destructive" onclick={() => void remove(remote)}>Remove</button>
      {/if}
    </div>
  </div>
  {#if remote.providerId === "onedrive"}
    {@const libs = libraries[remote.name]}
    <div class="library">
      {#if libs === undefined}
        <button type="button" class="linkish" onclick={() => void loadLibraries(remote)}>Choose which OneDrive library to open…</button>
      {:else if libs === "loading"}
        <span class="waiting">Asking Microsoft for your libraries…</span>
      {:else}
        <label>
          Library
          <select onchange={(e) => void chooseLibrary(remote, (e.currentTarget as HTMLSelectElement).value)}>
            {#each libs as lib (lib.id)}
              <option value={lib.id} selected={lib.current}>{lib.name}{lib.main ? " (My files)" : ""}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>
  {/if}
  {#if remote.providerId === "drive" && drives.lights[remote.name] !== "connected"}
    <p class="tip">
      Slow or red? Google is retiring the shared sign-in many apps use. Add your own free Google client ID with
      <strong>Edit settings</strong>, then <strong>Sign in again</strong>. Steps:
      <a href="https://rclone.org/drive/#making-your-own-client-id" target="_blank" rel="noopener">making your own client ID</a>.
    </p>
  {/if}
{/snippet}

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="window" role="dialog" aria-modal="true" aria-label="Settings">
    <nav class="tabs" aria-label="Settings sections">
      <h2>Settings</h2>
      {#each TABS as t (t.id)}
        <button type="button" class:active={tab === t.id} onclick={() => { tab = t.id; wizard = null; }}>{t.label}</button>
      {/each}
    </nav>
    <section class="content">
      <button type="button" class="close" aria-label="Close settings" onclick={onClose}><X size={16} strokeWidth={2} aria-hidden="true" /></button>
      <ErrorNote {error} />

      {#if wizard}
        <h3>{wizard.existing ? `Sign in again: ${wizard.existing.name}` : `Add ${wizard.provider.title}`}</h3>
        <RemoteWizard
          provider={wizard.provider}
          existing={wizard.existing}
          suggestedName={CLOUD_TITLES[wizard.provider.id]?.replace(/\s+/g, "") ?? ""}
          onBack={() => (wizard = null)}
          onDone={() => void wizardDone()}
        />
      {:else if tab === "accounts"}
        {#each CLOUD_PROVIDERS as id (id)}
          {@const list = drives.accountsOf(id)}
          <div class="group">
            <div class="group-head">
              <BrandIcon provider={id} size={18} />
              <h3>{CLOUD_TITLES[id]}</h3>
              <button type="button" class="button add" onclick={() => addAccount(id)}>
                <Plus size={13} strokeWidth={2} aria-hidden="true" /> Add account
              </button>
            </div>
            {#each list as remote (remote.name)}
              {@render account(remote, id)}
            {:else}
              <p class="empty">No {CLOUD_TITLES[id]} account yet.</p>
            {/each}
          </div>
        {/each}
        {#if drives.otherClouds.length}
          <div class="group">
            <div class="group-head"><BrandIcon provider={null} size={18} /><h3>Other storage</h3></div>
            {#each drives.otherClouds as remote (remote.name)}
              {@render account(remote, null)}
            {/each}
          </div>
        {/if}
        <div class="group">
          <div class="group-head">
            <Network size={18} strokeWidth={1.75} aria-hidden="true" />
            <h3>Network</h3>
            <button type="button" class="button add" onclick={onConnectServer}>
              <Plus size={13} strokeWidth={2} aria-hidden="true" /> Connect to server
            </button>
          </div>
          {#each networkAccounts as remote (remote.name)}
            <div class="account simple">
              {#if remote.providerId === "sftp"}<Terminal size={22} strokeWidth={1.6} aria-hidden="true" />{:else}<Network size={22} strokeWidth={1.6} aria-hidden="true" />{/if}
              <div class="details"><strong>{remote.name}</strong><div class="meta"><span>{remote.providerTitle}</span></div></div>
              <div class="row-actions">
                <button type="button" class="button" onclick={() => void reconnect(remote)}>Edit</button>
                <button type="button" class="button destructive" onclick={() => void remove(remote)}>Remove</button>
              </div>
            </div>
          {/each}
          {#each drives.nfs as share (share.name)}
            <div class="account simple">
              <Server size={22} strokeWidth={1.6} aria-hidden="true" />
              <div class="details"><strong>{share.name}</strong><div class="meta"><span>NFS · {share.server}:{share.export}</span></div></div>
            </div>
          {/each}
          {#if networkAccounts.length === 0 && drives.nfs.length === 0}
            <p class="empty">No servers yet.</p>
          {/if}
        </div>
      {:else if tab === "about"}
        <div class="about">
          <img src="/stingray.png" alt="" width="96" height="96" />
          <h3 class="about-name">Stingray File Manager</h3>
          <p class="about-version">Version {version || "…"}</p>
          <p class="about-text">
            A modern file manager for Linux and Windows that puts your folders, your cloud drives and your servers in
            one window. Free and open source.
          </p>
          <div class="about-links">
            <button type="button" class="button primary" onclick={() => open("https://quietbytehq.github.io")}>QuietByte website</button>
            <button type="button" class="button" onclick={() => open("https://github.com/quietbytehq/stingray")}>Source code</button>
            <button type="button" class="button" onclick={() => open("https://github.com/quietbytehq/stingray/issues")}>Report a problem</button>
          </div>
          <div class="maker">
            <strong>Made by QuietByte</strong>
            <p>
              QuietByte builds small, friendly open-source desktop apps for Linux and Windows, including Stingray,
              Orbit and Timewizer. See every project at
              <button type="button" class="linkish" onclick={() => open("https://quietbytehq.github.io")}>quietbytehq.github.io</button>.
            </p>
          </div>
          <p class="credits">
            Apache License 2.0. Built with Tauri and Svelte. Bundled icon packs (Candy, Kora, Newaita Reborn, Papirus, WhiteSur, Nordzy, Gruvbox Plus) are by their authors under GPL-3.0; see each pack in Settings. Cloud and network drives by rclone. Icons by Lucide.
            Google Drive, OneDrive and Dropbox logos belong to their owners and are shown only to identify each service.
          </p>
        </div>
      {:else if tab === "appearance"}
        <div class="field">
          <span class="field-label">Icon theme</span>
          <IconThemePicker />
        </div>
        <div class="field">
          <span class="field-label">Mode</span>
          <div class="segmented" role="radiogroup" aria-label="Mode">
            {#each [["system", "Automatic"], ["light", "Light"], ["dark", "Dark"]] as [value, label] (value)}
              <button type="button" role="radio" aria-checked={settings.value?.theme === value} onclick={() => void settings.patch({ theme: value as Theme })}>{label}</button>
            {/each}
          </div>
        </div>
        <div class="field">
          <label class="field-label" for="icon-size">Icon size</label>
          <div class="slider">
            <span class="small">A</span>
            <input
              id="icon-size"
              type="range"
              min="32"
              max="128"
              step="8"
              value={iconSize}
              oninput={(e) => {
                if (settings.value) settings.value.iconSize = Number((e.currentTarget as HTMLInputElement).value);
              }}
              onchange={(e) => void settings.patch({ iconSize: Number((e.currentTarget as HTMLInputElement).value) })}
            />
            <span class="big">A</span>
            <span class="value">{iconSize} px</span>
          </div>
          <p class="hint">Ctrl + plus and Ctrl + minus change it from the file view too.</p>
        </div>
        <div class="field">
          <label class="toggle">
            <input
              type="checkbox"
              checked={settings.value?.showThumbnails ?? true}
              onchange={(e) => void settings.patch({ showThumbnails: (e.currentTarget as HTMLInputElement).checked })}
            />
            <span>Show pictures as thumbnails</span>
          </label>
          <p class="hint">When off, pictures show their file icon. Turning this off makes big folders of photos open faster.</p>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.28);
  }
  .window {
    display: grid;
    grid-template-columns: 180px 1fr;
    width: min(920px, calc(100vw - 48px));
    height: min(640px, calc(100vh - 48px));
    border-radius: 14px;
    overflow: hidden;
    background: var(--window-bg);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35), 0 0 0 0.5px var(--separator);
  }
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 10px;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--separator);
  }
  .tabs h2 {
    margin: 0 8px 12px;
    font-size: 15px;
  }
  .tabs button {
    text-align: left;
    border: none;
    background: none;
    padding: 6px 10px;
    border-radius: 7px;
    cursor: default;
  }
  .tabs button:hover {
    background: var(--control-bg);
  }
  .tabs button.active {
    background: var(--sidebar-selected);
    font-weight: 600;
  }
  .content {
    position: relative;
    overflow-y: auto;
    padding: 20px 24px 28px;
  }
  .close {
    position: absolute;
    top: 12px;
    right: 12px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--text-secondary);
  }
  .close:hover {
    background: var(--control-bg);
  }
  h3 {
    margin: 0;
    font-size: 13px;
  }
  .group {
    margin-bottom: 22px;
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    padding-right: 36px;
  }
  .group-head :global(svg) {
    color: var(--accent);
  }
  .add {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: 12px;
  }
  .account {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px;
    margin-bottom: 8px;
    border-radius: 10px;
    background: var(--control-bg);
  }
  .account.simple {
    align-items: center;
  }
  .account.simple :global(svg) {
    color: var(--accent);
  }
  .details {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label-input {
    font-weight: 600;
    border: 1px solid transparent;
    background: none;
    padding: 2px 6px;
    margin-left: -6px;
    border-radius: 6px;
    user-select: text;
    -webkit-user-select: text;
  }
  .label-input:hover,
  .label-input:focus {
    border-color: var(--separator);
    background: var(--window-bg);
    outline: none;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .conn {
    opacity: 0.7;
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--separator);
    overflow: hidden;
    max-width: 280px;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent-gradient);
  }
  .swatches {
    display: flex;
    gap: 5px;
    margin-top: 2px;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .swatch[aria-checked="true"] {
    box-shadow: 0 0 0 2px var(--window-bg), 0 0 0 3.5px var(--text-secondary);
  }
  .row-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row-actions .button {
    font-size: 12px;
    padding: 4px 10px;
    background: var(--window-bg);
  }
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 12px 36px 0;
  }
  .about img {
    filter: drop-shadow(0 6px 14px rgba(138, 63, 252, 0.35));
  }
  .about-name {
    margin-top: 10px;
    font-size: 20px;
  }
  .about-version {
    margin: 2px 0 12px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .about-text {
    max-width: 46ch;
    margin: 0 0 16px;
  }
  .about-links {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .maker {
    margin-top: 22px;
    padding: 14px 18px;
    border-radius: 12px;
    background: var(--control-bg);
    max-width: 52ch;
  }
  .maker p {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .credits {
    margin-top: 18px;
    font-size: 11px;
    color: var(--text-secondary);
    max-width: 60ch;
  }
  .waiting {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .link,
  .linkish {
    font-size: 12px;
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .library {
    margin: -4px 0 10px 52px;
    font-size: 12px;
  }
  .library label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .library select {
    padding: 3px 6px;
    border-radius: 6px;
    border: 1px solid var(--separator);
    background: var(--window-bg);
    color: inherit;
  }
  .tip {
    margin: -4px 0 12px 52px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .tip a {
    color: var(--accent);
  }
  .empty {
    margin: 0 0 4px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .field {
    margin-bottom: 22px;
    padding-right: 36px;
  }
  .field-label {
    display: block;
    font-weight: 600;
    margin-bottom: 8px;
  }
  .segmented {
    display: inline-flex;
    padding: 2px;
    border-radius: 8px;
    background: var(--control-bg);
  }
  .segmented button {
    border: none;
    background: none;
    padding: 4px 14px;
    border-radius: 6px;
  }
  .segmented button[aria-checked="true"] {
    background: var(--window-bg);
    color: var(--accent);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .slider {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 420px;
  }
  .slider input {
    flex: 1;
    accent-color: var(--accent);
  }
  .small {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .big {
    font-size: 17px;
    color: var(--text-secondary);
  }
  .value {
    width: 52px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .toggle input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
  .hint {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
