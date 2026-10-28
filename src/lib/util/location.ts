import type { Endpoint, FileEntry } from "$lib/api/types";

/**
 * Where the file view is. Local paths are absolute paths on this computer ("/home/ana" or
 * "C:\\Users\\ana"). Remote paths are relative to the drive's root, "/" separated, "" for the root.
 */
export type Location = { kind: "local"; path: string } | { kind: "remote"; remote: string; path: string };

/** Splits a local path on either separator, keeping Windows drive roots intact. */
function localParts(path: string): { root: string; parts: string[]; sep: string } {
  const sep = path.includes("\\") && !path.startsWith("/") ? "\\" : "/";
  const drive = /^[A-Za-z]:[\\/]?/.exec(path);
  const root = drive ? `${drive[0].slice(0, 2)}${sep}` : "/";
  const rest = drive ? path.slice(drive[0].length) : path;
  return { root, parts: rest.split(/[\\/]+/).filter(Boolean), sep };
}

export function joinLocal(parent: string, name: string): string {
  const { root, parts, sep } = localParts(parent);
  return root + [...parts, name].join(sep);
}

export function sameLocation(a: Location, b: Location): boolean {
  if (a.kind === "local" && b.kind === "local") return a.path === b.path;
  if (a.kind === "remote" && b.kind === "remote") return a.remote === b.remote && a.path === b.path;
  return false;
}

/** True when both places are on the same drive, so a drag moves instead of copying. */
export function sameDrive(a: Location, b: Location): boolean {
  if (a.kind === "remote" && b.kind === "remote") return a.remote === b.remote;
  if (a.kind === "local" && b.kind === "local") return localParts(a.path).root === localParts(b.path).root;
  return false;
}

export function parentOf(loc: Location): Location | null {
  if (loc.kind === "remote") {
    if (loc.path === "") return null;
    const cut = loc.path.lastIndexOf("/");
    return { ...loc, path: cut < 0 ? "" : loc.path.slice(0, cut) };
  }
  const { root, parts, sep } = localParts(loc.path);
  if (parts.length === 0) return null;
  return { kind: "local", path: root + parts.slice(0, -1).join(sep) };
}

/** The folder an entry from a listing opens to. */
export function childOf(loc: Location, entry: FileEntry): Location {
  return loc.kind === "local" ? { kind: "local", path: entry.path } : { ...loc, path: entry.path };
}

export function endpointOf(loc: Location): Endpoint {
  return loc.kind === "local" ? { kind: "local", path: loc.path } : { kind: "remote", remote: loc.remote, path: loc.path };
}

export interface Crumb {
  label: string;
  loc: Location;
}

/**
 * Breadcrumbs from the drive root down to `loc`. A local path under a known place (Home) starts
 * at that place's label instead of the file system root.
 */
export function crumbs(loc: Location, rootLabel: string, anchor?: { path: string; label: string }): Crumb[] {
  if (loc.kind === "remote") {
    const out: Crumb[] = [{ label: rootLabel, loc: { ...loc, path: "" } }];
    const parts = loc.path.split("/").filter(Boolean);
    parts.forEach((part, i) => out.push({ label: part, loc: { ...loc, path: parts.slice(0, i + 1).join("/") } }));
    return out;
  }
  const { root, parts, sep } = localParts(loc.path);
  let start = 0;
  const out: Crumb[] = [];
  if (anchor) {
    const a = localParts(anchor.path);
    const under = a.root === root && a.parts.every((part, i) => parts[i] === part);
    if (under) {
      out.push({ label: anchor.label, loc: { kind: "local", path: anchor.path } });
      start = a.parts.length;
    }
  }
  if (out.length === 0) out.push({ label: rootLabel, loc: { kind: "local", path: root } });
  for (let i = start; i < parts.length; i++) {
    out.push({ label: parts[i], loc: { kind: "local", path: root + parts.slice(0, i + 1).join(sep) } });
  }
  return out;
}

/** Splits an item's path into the folder it sits in and its name, for transfer requests. */
export function folderAndName(loc: Location, entry: FileEntry): { folder: Location; name: string } {
  return { folder: loc, name: entry.name };
}
