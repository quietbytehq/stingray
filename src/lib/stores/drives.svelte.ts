import { api, asAppError } from "$lib/api";
import type { AccountInfo, AppError, NfsShareView, Place, ProviderDef, RemoteSummary } from "$lib/api/types";
import { settings } from "$lib/stores/settings.svelte";

/**
 * Connection light for a drive in the sidebar:
 * - "connected": set up and reachable (green)
 * - "offline": set up but not reachable right now, or the sign-in expired (red)
 * - "checking": set up, check in progress (red until it answers, drawn pulsing)
 * - "none": not set up yet (grey)
 */
export type Light = "connected" | "offline" | "checking" | "none";

/** The three cloud services with a fixed place in the sidebar, in order. */
export const CLOUD_PROVIDERS = ["drive", "onedrive", "dropbox"] as const;
export type CloudProvider = (typeof CLOUD_PROVIDERS)[number];
export const NETWORK_PROVIDERS = ["smb", "sftp"] as const;

const RECHECK_MS = 60_000;

/** Badge colours for telling accounts of the same service apart. */
export const BADGE_COLORS = ["#8a3ffc", "#e81cc4", "#ff9f0a", "#30b0c7", "#34c759", "#ff453a", "#0a84ff", "#a2845e"];

export const CLOUD_TITLES: Record<string, string> = { drive: "Google Drive", onedrive: "OneDrive", dropbox: "Dropbox" };

class DriveStore {
  places = $state<Place[]>([]);
  providers = $state<ProviderDef[]>([]);
  remotes = $state<RemoteSummary[]>([]);
  nfs = $state<NfsShareView[]>([]);
  lights = $state<Record<string, Light>>({});
  /** Email, drive type and storage per connection, loaded in the background. */
  info = $state<Record<string, AccountInfo>>({});
  error = $state<AppError | null>(null);
  online = $state(typeof navigator === "undefined" ? true : navigator.onLine);
  #timer = 0;
  #started = false;

  async start() {
    if (this.#started) return;
    this.#started = true;
    try {
      this.places = await api.homePlaces();
    } catch (err) {
      this.error = asAppError(err);
    }
    window.addEventListener("online", () => {
      this.online = true;
      void this.checkAll();
    });
    window.addEventListener("offline", () => {
      this.online = false;
      for (const remote of this.remotes) this.lights[remote.name] = "offline";
    });
    this.#timer = window.setInterval(() => void this.checkAll(), RECHECK_MS);
  }

  stop() {
    window.clearInterval(this.#timer);
  }

  /** Reloads the list of drives once the engine is up, then checks each one. */
  async refresh() {
    try {
      if (this.providers.length === 0) this.providers = await api.listProviders();
      this.remotes = await api.listRemotes();
      this.nfs = await api.listNfsShares();
      this.error = null;
    } catch (err) {
      this.error = asAppError(err);
      return;
    }
    await this.checkAll();
    void this.loadInfo();
  }

  /** Fetches account details for every cloud connection, one at a time so the engine stays responsive. */
  async loadInfo() {
    for (const remote of this.remotes) {
      if ((NETWORK_PROVIDERS as readonly string[]).includes(remote.providerId ?? "")) continue;
      try {
        this.info[remote.name] = await api.accountInfo(remote.name);
      } catch {
        // Details are a nice-to-have; the account still works without them.
      }
    }
  }

  async checkAll() {
    await Promise.all(this.remotes.map((remote) => this.check(remote.name)));
    try {
      this.nfs = await api.listNfsShares();
    } catch {
      // Keep the last known list.
    }
  }

  async check(name: string) {
    if (!this.online) {
      this.lights[name] = "offline";
      return;
    }
    if (this.lights[name] !== "connected") this.lights[name] = "checking";
    try {
      const result = await api.testRemote(name);
      this.lights[name] = result.ok ? "connected" : "offline";
    } catch {
      this.lights[name] = "offline";
    }
  }

  light(name: string): Light {
    return this.lights[name] ?? "checking";
  }

  provider(id: string): ProviderDef | undefined {
    return this.providers.find((provider) => provider.id === id);
  }

  /** The connection set up for a cloud service, if any (the first one when there are several). */
  cloud(id: CloudProvider): RemoteSummary | undefined {
    return this.remotes.find((remote) => remote.providerId === id);
  }

  /** Every connection for a cloud service, in name order. */
  accountsOf(id: CloudProvider): RemoteSummary[] {
    return this.remotes.filter((remote) => remote.providerId === id).sort((a, b) => a.name.localeCompare(b.name));
  }

  /** What to call an account: your label, else its email, else the connection name. */
  displayName(remote: RemoteSummary): string {
    const look = settings.value?.accounts[remote.name];
    return look?.label?.trim() || this.info[remote.name]?.user || remote.name;
  }

  /** The account's badge colour: yours if set, otherwise a stable colour from its name. */
  badgeColor(remote: RemoteSummary): string {
    const chosen = settings.value?.accounts[remote.name]?.color;
    if (chosen) return chosen;
    let hash = 0;
    for (const ch of remote.name) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
    return BADGE_COLORS[hash % BADGE_COLORS.length];
  }

  get network(): RemoteSummary[] {
    return this.remotes.filter((remote) => (NETWORK_PROVIDERS as readonly string[]).includes(remote.providerId ?? ""));
  }

  /** True when an account shares its service with another one, so it needs a badge to tell them apart. */
  needsBadge(remote: RemoteSummary): boolean {
    return this.remotes.filter((r) => r.providerId === remote.providerId).length > 1;
  }

  /** Other connections made elsewhere (for example in Orbit or by hand), shown under Cloud. */
  get otherClouds(): RemoteSummary[] {
    const known = [...CLOUD_PROVIDERS, ...NETWORK_PROVIDERS] as readonly string[];
    return this.remotes.filter((r) => !known.includes(r.providerId ?? ""));
  }
}

export const drives = new DriveStore();
