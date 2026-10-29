<script lang="ts">
  import type { Component } from "svelte";
  import type { NfsShareView, Place, RemoteSummary } from "$lib/api/types";
  import { drives, CLOUD_PROVIDERS, CLOUD_TITLES, type CloudProvider, type Light } from "$lib/stores/drives.svelte";
  import BrandIcon from "./BrandIcon.svelte";
  import FileIcon, { type SpecialFolder } from "./FileIcon.svelte";
  import { drag, dropKey } from "$lib/stores/drag.svelte";
  import { sameLocation, type Location } from "$lib/util/location";
  import {
    Cloud,
    Download,
    Film,
    FileText,
    HardDrive,
    House,
    Image,
    Monitor,
    Music,
    Network,
    Plus,
    Server,
    Settings,
    Terminal,
    Trash,
  } from "$lib/icons";
  import type { MenuItem } from "./ContextMenu.svelte";

  let {
    current,
    onNavigate,
    onSetUpCloud,
    onConnectServer,
    onOpenNfs,
    onMenu,
    onSettings,
  }: {
    current: Location;
    onNavigate: (loc: Location, label: string) => void;
    onSetUpCloud: (provider: CloudProvider) => void;
    onConnectServer: () => void;
    onOpenNfs: (share: NfsShareView) => void;
    onMenu: (event: MouseEvent, items: MenuItem[]) => void;
    /** Opens Settings, optionally at a section ("accounts", "appearance"). */
    onSettings: (section?: string) => void;
  } = $props();

  const PLACE_ICONS: Record<Place["kind"], Component> = {
    home: House,
    desktop: Monitor,
    documents: FileText,
    downloads: Download,
    music: Music,
    pictures: Image,
    videos: Film,
    trash: Trash,
    volume: HardDrive,
  };
  const LIGHT_TEXT: Record<Light, string> = {
    connected: "Connected",
    offline: "Not connected",
    checking: "Checking connection",
    slow: "Connected, but slow to respond",
    none: "Not set up",
  };

  const places = $derived(drives.places.filter((p) => p.kind !== "volume"));
  const volumes = $derived(drives.places.filter((p) => p.kind === "volume"));

  function remoteLoc(remote: RemoteSummary): Location {
    return { kind: "remote", remote: remote.name, path: remote.startPath };
  }

  function isCurrentRemote(remote: RemoteSummary): boolean {
    return current.kind === "remote" && current.remote === remote.name;
  }

  // Every sidebar entry you can drop on registers its location with the drag store.
  $effect(() => {
    for (const place of drives.places) drag.target(dropKey({ kind: "local", path: place.path }), { kind: "local", path: place.path });
    for (const remote of drives.remotes) drag.target(dropKey(remoteLoc(remote)), remoteLoc(remote));
    for (const share of drives.nfs) {
      if (share.connectedAt) drag.target(dropKey({ kind: "local", path: share.connectedAt }), { kind: "local", path: share.connectedAt });
    }
  });

  function remoteMenu(event: MouseEvent, remote: RemoteSummary) {
    onMenu(event, [
      { label: "Open", action: () => onNavigate(remoteLoc(remote), remote.name) },
      { label: "Check connection", action: () => void drives.check(remote.name) },
      { label: "Account settings…", action: () => onSettings("accounts") },
      {
        label: `Remove ${remote.name}…`,
        destructive: true,
        separator: true,
        action: async () => {
          if (!window.confirm(`Remove ${remote.name} from Stingray? The sign-in is forgotten. Your files stay where they are.`)) return;
          const { api } = await import("$lib/api");
          await api.deleteRemote(remote.name).catch(() => {});
          await drives.refresh();
        },
      },
    ]);
  }
</script>

{#snippet dot(light: Light)}
  <span class="dot {light}" title={LIGHT_TEXT[light]} aria-label={LIGHT_TEXT[light]}></span>
{/snippet}

<aside class="sidebar" aria-label="Locations">
  <div class="brand">
    <img src="/stingray.png" alt="" width="50" height="50" />
    <span>Stingray</span>
    <button type="button" class="gear" title="Settings (Ctrl+,)" aria-label="Settings" onclick={() => onSettings()}>
      <Settings size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </div>

  <nav>
    <h2>Places</h2>
    {#each places as place (place.path)}
      {@const loc = { kind: "local" as const, path: place.path }}
      <button
        type="button"
        class="item"
        class:current={sameLocation(current, loc)}
        class:drop={drag.over === dropKey(loc)}
        data-drop={place.kind === "trash" ? undefined : dropKey(loc)}
        onclick={() => onNavigate(loc, place.label)}
      >
        <span class="place-icon"><FileIcon name="" isDir={true} size={20} special={place.kind as SpecialFolder} /></span>
        <span class="label">{place.label}</span>
      </button>
    {/each}
    {#each volumes as place (place.path)}
      {@const loc = { kind: "local" as const, path: place.path }}
      <button type="button" class="item" class:current={sameLocation(current, loc)} class:drop={drag.over === dropKey(loc)} data-drop={dropKey(loc)} onclick={() => onNavigate(loc, place.label)}>
        <HardDrive size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="label">{place.label}</span>
      </button>
    {/each}

    <h2 class="with-action">
      Cloud
      <button type="button" class="mini" title="Manage accounts" aria-label="Manage accounts" onclick={() => onSettings("accounts")}>
        <Plus size={13} strokeWidth={2} aria-hidden="true" />
      </button>
    </h2>
    {#each CLOUD_PROVIDERS as id (id)}
      {@const accounts = drives.accountsOf(id)}
      {#each accounts as remote (remote.name)}
        {@const loc = remoteLoc(remote)}
        {@const name = drives.displayName(remote)}
        <button
          type="button"
          class="item"
          class:current={isCurrentRemote(remote)}
          class:drop={drag.over === dropKey(loc)}
          data-drop={dropKey(loc)}
          title={drives.info[remote.name]?.user ?? remote.name}
          onclick={() => onNavigate(loc, accounts.length > 1 ? name : CLOUD_TITLES[id])}
          oncontextmenu={(e) => remoteMenu(e, remote)}
        >
          <BrandIcon
            provider={id}
            badge={accounts.length > 1 ? { letter: name.slice(0, 1).toUpperCase(), color: drives.badgeColor(remote) } : null}
          />
          <span class="label">{accounts.length > 1 ? name : CLOUD_TITLES[id]}</span>
          {@render dot(drives.light(remote.name))}
        </button>
      {:else}
        <button type="button" class="item idle" onclick={() => onSetUpCloud(id)} title="Set up {CLOUD_TITLES[id]}">
          <span class="faded"><BrandIcon provider={id} /></span>
          <span class="label">{CLOUD_TITLES[id]}</span>
          {@render dot("none")}
        </button>
      {/each}
    {/each}
    {#each drives.otherClouds as remote (remote.name)}
      {@const loc = remoteLoc(remote)}
      <button type="button" class="item" class:current={isCurrentRemote(remote)} class:drop={drag.over === dropKey(loc)} data-drop={dropKey(loc)} onclick={() => onNavigate(loc, drives.displayName(remote))} oncontextmenu={(e) => remoteMenu(e, remote)}>
        <BrandIcon provider={null} />
        <span class="label">{drives.displayName(remote)}</span>
        {@render dot(drives.light(remote.name))}
      </button>
    {/each}

    <h2>Network</h2>
    {#each drives.network as remote (remote.name)}
      {@const loc = remoteLoc(remote)}
      <button type="button" class="item" class:current={isCurrentRemote(remote)} class:drop={drag.over === dropKey(loc)} data-drop={dropKey(loc)} onclick={() => onNavigate(loc, remote.name)} oncontextmenu={(e) => remoteMenu(e, remote)}>
        {#if remote.providerId === "sftp"}
          <Terminal size={16} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <Network size={16} strokeWidth={1.75} aria-hidden="true" />
        {/if}
        <span class="label">{remote.name}</span>
        {@render dot(drives.light(remote.name))}
      </button>
    {/each}
    {#each drives.nfs as share (share.name)}
      {@const loc = share.connectedAt ? { kind: "local" as const, path: share.connectedAt } : null}
      <button
        type="button"
        class="item"
        class:current={loc !== null && sameLocation(current, loc)}
        class:drop={loc !== null && drag.over === dropKey(loc)}
        data-drop={loc ? dropKey(loc) : undefined}
        onclick={() => onOpenNfs(share)}
        oncontextmenu={(e) =>
          onMenu(e, [
            { label: share.connectedAt ? "Open" : "Connect", action: () => onOpenNfs(share) },
            {
              label: "Disconnect",
              disabled: !share.connectedAt,
              action: async () => {
                const { api } = await import("$lib/api");
                await api.disconnectNfsShare(share.name).catch((err) => window.alert(err?.message ?? String(err)));
                await drives.refresh();
              },
            },
            {
              label: `Remove ${share.name}…`,
              destructive: true,
              separator: true,
              action: async () => {
                if (!window.confirm(`Remove ${share.name} from Stingray?`)) return;
                const { api } = await import("$lib/api");
                await api.removeNfsShare(share.name).catch(() => {});
                await drives.refresh();
              },
            },
          ])}
      >
        <Server size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="label">{share.name}</span>
        {@render dot(share.connectedAt ? "connected" : "offline")}
      </button>
    {/each}
    <button type="button" class="item add" onclick={onConnectServer}>
      <Plus size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="label">Connect to server…</span>
    </button>
  </nav>
</aside>

<style>
  .sidebar {
    background: var(--sidebar-bg);
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    border-right: 1px solid var(--separator);
    padding: 14px 10px 16px;
    overflow-y: auto;
    min-height: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 2px 4px 8px 6px;
    font-weight: 750;
    font-size: 22px;
    letter-spacing: -0.02em;
  }
  .brand img {
    filter: drop-shadow(0 2px 4px rgba(138, 63, 252, 0.35));
  }
  .gear,
  .mini {
    margin-left: auto;
    display: grid;
    place-items: center;
    border: none;
    background: none;
    color: var(--text-secondary);
    border-radius: 6px;
    cursor: default;
  }
  .gear {
    width: 28px;
    height: 26px;
  }
  .mini {
    width: 20px;
    height: 18px;
  }
  .gear:hover,
  .mini:hover {
    background: var(--control-bg);
    color: var(--text-primary);
  }
  h2.with-action {
    display: flex;
    align-items: center;
  }
  .place-icon {
    display: inline-grid;
    width: 20px;
    height: 20px;
    flex: none;
  }
  .faded {
    display: inline-grid;
    opacity: 0.45;
    filter: grayscale(0.6);
  }
  h2 {
    margin: 16px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: default;
  }
  .item :global(svg) {
    color: var(--accent);
    flex: none;
  }
  .item:hover {
    background: var(--control-bg);
  }
  .item.current {
    background: var(--sidebar-selected);
    font-weight: 500;
  }
  .item.drop {
    background: var(--accent);
    color: #fff;
  }
  .item.drop :global(svg) {
    color: #fff;
  }
  .item.idle {
    color: var(--text-secondary);
  }
  .item.idle :global(svg) {
    color: var(--text-secondary);
    opacity: 0.7;
  }
  .item.add {
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .item.add :global(svg) {
    color: var(--text-secondary);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .dot.connected {
    background: var(--status-synced);
    box-shadow: 0 0 6px rgba(52, 199, 89, 0.6);
  }
  .dot.offline {
    background: var(--status-offline);
  }
  .dot.checking {
    background: var(--status-offline);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .dot.slow {
    background: #ff9f0a;
  }
  .dot.none {
    background: var(--status-idle);
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
