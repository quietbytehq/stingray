import { invoke } from "@tauri-apps/api/core";
import { settings } from "$lib/stores/settings.svelte";

/**
 * Icon packs and installed icon themes. Built-in themes are drawn by FileIcon itself; this store
 * covers the bundled packs (static/icon-packs, light and sometimes dark variants) and icon themes
 * installed on this computer (read through the backend as data URIs).
 *
 * Theme ids: "stingray" | "fluent" | "cupertino" | "outline" | "pack:<id>" | "system:<folder>".
 */
export interface IconPack {
  id: string;
  name: string;
  author: string;
  url: string;
  license: string;
  hasDark: boolean;
  icons: string[];
}

export interface SystemTheme {
  id: string;
  name: string;
}

/** Stingray's icon keys, in preview order. */
export const ICON_KEYS = [
  "folder", "home", "desktop", "documents", "downloads", "music", "pictures", "videos", "trash",
  "file", "document", "pdf", "spreadsheet", "presentation", "image", "video", "audio", "archive", "code",
] as const;
export type IconKey = (typeof ICON_KEYS)[number];

class IconThemes {
  packs = $state<IconPack[]>([]);
  system = $state<SystemTheme[]>([]);
  /** Icons of installed themes, loaded when first needed. */
  systemIcons = $state<Record<string, Record<string, string>>>({});
  /** True when the window is showing dark colours. */
  dark = $state(false);
  #loading = new Set<string>();
  #started = false;

  async start() {
    if (this.#started) return;
    this.#started = true;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => {
      const mode = settings.value?.theme ?? "system";
      this.dark = mode === "dark" || (mode === "system" && media.matches);
    };
    update();
    media.addEventListener("change", update);
    $effect.root(() => {
      $effect(() => {
        void settings.value?.theme;
        update();
      });
    });
    try {
      const response = await fetch("/icon-packs/index.json");
      this.packs = await response.json();
    } catch {
      this.packs = [];
    }
    try {
      this.system = await invoke<SystemTheme[]>("list_system_icon_themes");
    } catch {
      this.system = []; // Windows, or no themes installed
    }
  }

  async loadSystem(id: string) {
    if (this.systemIcons[id] || this.#loading.has(id)) return;
    this.#loading.add(id);
    try {
      this.systemIcons[id] = await invoke<Record<string, string>>("system_icon_theme_icons", { id });
    } catch {
      this.systemIcons[id] = {};
    } finally {
      this.#loading.delete(id);
    }
  }

  /** Display name for any theme id. */
  label(theme: string): string {
    if (theme.startsWith("pack:")) return this.packs.find((p) => `pack:${p.id}` === theme)?.name ?? theme.slice(5);
    if (theme.startsWith("system:")) return this.system.find((t) => `system:${t.id}` === theme)?.name ?? theme.slice(7);
    return theme.charAt(0).toUpperCase() + theme.slice(1);
  }

  /** Image URL for one icon of a pack or installed theme, or null to fall back to the built-in drawing. */
  url(theme: string, key: string, dark = this.dark): string | null {
    if (theme.startsWith("pack:")) {
      const pack = this.packs.find((p) => `pack:${p.id}` === theme);
      if (!pack || !pack.icons.includes(key)) return null;
      const variant = dark && pack.hasDark ? "dark" : "light";
      return `/icon-packs/${pack.id}/${variant}/${key}.svg`;
    }
    if (theme.startsWith("system:")) {
      const id = theme.slice(7);
      const icons = this.systemIcons[id];
      if (!icons) {
        void this.loadSystem(id);
        return null;
      }
      return icons[key] ?? null;
    }
    return null;
  }
}

export const iconThemes = new IconThemes();
