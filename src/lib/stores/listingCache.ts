import type { FileEntry } from "$lib/api/types";

/**
 * Remembered listings of cloud and network folders, so they open instantly. Stingray shows the
 * cached listing at once and refreshes it in the background. Kept in IndexedDB on this computer
 * (it can hold far more than localStorage), with a small in-memory copy for speed.
 */
const DB = "stingray-listings";
const STORE = "folders";
const MAX_AGE_MS = 30 * 24 * 3600 * 1000;

export interface Cached {
  entries: FileEntry[];
  /** When this listing was fetched, in ms since 1970. */
  at: number;
}

const memory = new Map<string, Cached>();
let dbPromise: Promise<IDBDatabase | null> | null = null;

function db(): Promise<IDBDatabase | null> {
  if (!dbPromise) {
    dbPromise = new Promise((resolve) => {
      try {
        const req = indexedDB.open(DB, 1);
        req.onupgradeneeded = () => req.result.createObjectStore(STORE);
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => resolve(null);
      } catch {
        resolve(null);
      }
    });
  }
  return dbPromise;
}

export function cacheKey(remote: string, path: string): string {
  return `${remote}:${path}`;
}

export async function getListing(key: string): Promise<Cached | null> {
  const hit = memory.get(key);
  if (hit) return hit;
  const d = await db();
  if (!d) return null;
  return new Promise((resolve) => {
    try {
      const req = d.transaction(STORE, "readonly").objectStore(STORE).get(key);
      req.onsuccess = () => {
        const value = req.result as Cached | undefined;
        if (value && Date.now() - value.at < MAX_AGE_MS) {
          memory.set(key, value);
          resolve(value);
        } else resolve(null);
      };
      req.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

export async function putListing(key: string, entries: FileEntry[]): Promise<void> {
  const value: Cached = { entries, at: Date.now() };
  memory.set(key, value);
  const d = await db();
  if (!d) return;
  try {
    d.transaction(STORE, "readwrite").objectStore(STORE).put(value, key);
  } catch {
    // The in-memory copy still works for this session.
  }
}

/** Forgets every cached folder of one connection (after it is removed or signed in again). */
export async function forgetRemote(remote: string): Promise<void> {
  for (const key of [...memory.keys()]) if (key.startsWith(`${remote}:`)) memory.delete(key);
  const d = await db();
  if (!d) return;
  try {
    const store = d.transaction(STORE, "readwrite").objectStore(STORE);
    const req = store.openCursor();
    req.onsuccess = () => {
      const cursor = req.result;
      if (!cursor) return;
      if (String(cursor.key).startsWith(`${remote}:`)) cursor.delete();
      cursor.continue();
    };
  } catch {
    // Nothing else to do.
  }
}

/** "2 minutes ago" for the Updating… note. */
export function age(at: number): string {
  const s = Math.round((Date.now() - at) / 1000);
  if (s < 60) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} minute${m === 1 ? "" : "s"} ago`;
  const h = Math.round(m / 60);
  if (h < 48) return `${h} hour${h === 1 ? "" : "s"} ago`;
  return `${Math.round(h / 24)} days ago`;
}
