<script lang="ts">
  import type { Component } from "svelte";
  import type { NfsShareView, Place, RemoteSummary } from "$lib/api/types";
  import { drives, CLOUD_PROVIDERS, type CloudProvider, type Light } from "$lib/stores/drives.svelte";
  import { drag, dropKey } from "$lib/stores/drag.svelte";
  import { sameLocation, type Location } from "$lib/util/location";
  import {
    Box,
    Cloud,
    CloudUpload,
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
  }: {
    current: Location;
    onNavigate: (loc: Location, label: string) => void;
    onSetUpCloud: (provider: CloudProvider) => void;
    onConnectServer: () => void;
    onOpenNfs: (share: NfsShareView) => void;
    onMenu: (event: MouseEvent, items: MenuItem[]) => void;
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
  const CLOUD_ICONS: Record<CloudProvider, Component> = { drive: Cloud, onedrive: CloudUpload, dropbox: Box };
  const CLOUD_TITLES: Record<CloudProvider, string> = { drive: "Google Drive", onedrive: "OneDrive", dropbox: "Dropbox" };
  const LIGHT_TEXT: Record<Light, string> = {
    connected: "Connected",
    offline: "Not connected",
    checking: "Checking connection",
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
    <img src="/favicon.png" alt="" width="20" height="20" />
    <span>Stingray</span>
  </div>

  <nav>
    <h2>Places</h2>
    {#each places as place (place.path)}
      {@const Icon = PLACE_ICONS[place.kind]}
      {@const loc = { kind: "local" as const, path: place.path }}
      <button
        type="button"
        class="item"
        class:current={sameLocation(current, loc)}
        class:drop={drag.over === dropKey(loc)}
        data-drop={place.kind === "trash" ? undefined : dropKey(loc)}
        onclick={() => onNavigate(loc, place.label)}
      >
        <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
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

    <h2>Cloud</h2>
    {#each CLOUD_PROVIDERS as id (id)}
      {@const Icon = CLOUD_ICONS[id]}
      {@const remote = drives.cloud(id)}
      {#if remote}
        {@const loc = remoteLoc(remote)}
        <button
          type="button"
          class="item"
          class:current={isCurrentRemote(remote)}
          class:drop={drag.over === dropKey(loc)}
          data-drop={dropKey(loc)}
          onclick={() => onNavigate(loc, CLOUD_TITLES[id])}
          oncontextmenu={(e) => remoteMenu(e, remote)}
        >
          <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
          <span class="label">{CLOUD_TITLES[id]}</span>
          {@render dot(drives.light(remote.name))}
        </button>
      {:else}
        <button type="button" class="item idle" onclick={() => onSetUpCloud(id)} title="Set up {CLOUD_TITLES[id]}">
          <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
          <span class="label">{CLOUD_TITLES[id]}</span>
          {@render dot("none")}
        </button>
      {/if}
    {/each}
    {#each drives.otherClouds as remote (remote.name)}
      {@const loc = remoteLoc(remote)}
      <button type="button" class="item" class:current={isCurrentRemote(remote)} class:drop={drag.over === dropKey(loc)} data-drop={dropKey(loc)} onclick={() => onNavigate(loc, remote.name)} oncontextmenu={(e) => remoteMenu(e, remote)}>
        <Cloud size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="label">{remote.name}</span>
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
    gap: 8px;
    padding: 2px 8px 6px;
    font-weight: 650;
    font-size: 14px;
    letter-spacing: -0.01em;
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
  .dot.none {
    background: var(--status-idle);
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
