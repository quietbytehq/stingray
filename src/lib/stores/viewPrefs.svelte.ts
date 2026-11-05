import type { FileEntry } from "$lib/api/types";
import { fileKind } from "$lib/util/filekind";

/**
 * View preferences that live on this computer only: how folders are sorted, and starred items.
 * Kept in the browser storage of Stingray's window, which persists between runs.
 */
export type SortKey = "name" | "size" | "type" | "modified";

const SORT_KEY = "stingray.sort";
const STAR_KEY = "stingray.starred";

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

class ViewPrefs {
  sort = $state(read(SORT_KEY, { key: "name" as SortKey, descending: false, foldersFirst: true }));
  /** Starred items: local paths, or "remote:path" for cloud and network drives. */
  starred = $state<string[]>((() => {
    try {
      return JSON.parse(localStorage.getItem(STAR_KEY) ?? "[]");
    } catch {
      return [];
    }
  })());

  setSort(change: Partial<{ key: SortKey; descending: boolean; foldersFirst: boolean }>) {
    this.sort = { ...this.sort, ...change };
    try {
      localStorage.setItem(SORT_KEY, JSON.stringify(this.sort));
    } catch {
      // Sorting still works for this session.
    }
  }

  /** Clicking a column: same column flips the order, another column sorts by it ascending. */
  sortBy(key: SortKey) {
    this.setSort(this.sort.key === key ? { descending: !this.sort.descending } : { key, descending: false });
  }

  isStarred(id: string): boolean {
    return this.starred.includes(id);
  }

  toggleStar(ids: string[]) {
    const all = ids.every((id) => this.starred.includes(id));
    this.starred = all ? this.starred.filter((s) => !ids.includes(s)) : [...new Set([...this.starred, ...ids])];
    try {
      localStorage.setItem(STAR_KEY, JSON.stringify(this.starred));
    } catch {
      // Stars still work for this session.
    }
  }

  /** Sorts a listing the way the person chose. */
  compare = (a: FileEntry, b: FileEntry): number => {
    const { key, descending, foldersFirst } = this.sort;
    if (foldersFirst && a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    let order = 0;
    if (key === "size") order = (a.isDir ? -1 : a.size) - (b.isDir ? -1 : b.size);
    else if (key === "modified") order = (a.modTime ?? "").localeCompare(b.modTime ?? "");
    else if (key === "type") {
      const ext = (n: string) => (n.includes(".") ? n.slice(n.lastIndexOf(".") + 1).toLowerCase() : "");
      order = fileKind(a.name, a.isDir).localeCompare(fileKind(b.name, b.isDir)) || ext(a.name).localeCompare(ext(b.name));
    }
    if (order === 0) order = a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" });
    return descending ? -order : order;
  };
}

export const viewPrefs = new ViewPrefs();
