import { api, asAppError } from "$lib/api";
import type { AppError, NfsShareView, Place, ProviderDef, RemoteSummary } from "$lib/api/types";

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

class DriveStore {
  places = $state<Place[]>([]);
  providers = $state<ProviderDef[]>([]);
  remotes = $state<RemoteSummary[]>([]);
  nfs = $state<NfsShareView[]>([]);
  lights = $state<Record<string, Light>>({});
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

  get network(): RemoteSummary[] {
    return this.remotes.filter((remote) => (NETWORK_PROVIDERS as readonly string[]).includes(remote.providerId ?? ""));
  }

  /** Other connections made elsewhere (for example in Orbit or by hand), shown under Cloud. */
  get otherClouds(): RemoteSummary[] {
    const fixed = this.remotes.filter((r) => (CLOUD_PROVIDERS as readonly string[]).includes(r.providerId ?? ""));
    const firstOfEach = CLOUD_PROVIDERS.map((id) => fixed.find((r) => r.providerId === id)).filter(Boolean);
    return this.remotes.filter(
      (r) => !firstOfEach.includes(r) && !(NETWORK_PROVIDERS as readonly string[]).includes(r.providerId ?? ""),
    );
  }
}

export const drives = new DriveStore();
