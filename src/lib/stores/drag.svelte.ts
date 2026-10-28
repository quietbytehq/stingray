import type { FileEntry } from "$lib/api/types";
import type { Location } from "$lib/util/location";

/**
 * Drag and drop inside Stingray, done with pointer events rather than HTML drag and drop.
 * Tauri keeps the native drop handler on so files dragged in from other apps arrive with their
 * real paths; HTML drag and drop would then not work inside the window on Windows. Pointer events
 * work the same everywhere.
 *
 * Drop targets mark themselves with `data-drop="<key>"` and register their location with `target`.
 */
export interface DragPayload {
  from: Location;
  entries: FileEntry[];
}

const START_DISTANCE = 5;

class DragStore {
  payload = $state<DragPayload | null>(null);
  x = $state(0);
  y = $state(0);
  /** Key of the target under the pointer, for highlighting. */
  over = $state<string | null>(null);
  /** True while Ctrl (or Alt on Linux desktops that reserve Ctrl) forces a copy. */
  forceCopy = $state(false);
  #targets = new Map<string, Location>();
  #pending: { x: number; y: number; payload: DragPayload } | null = null;
  #onDrop: ((payload: DragPayload, to: Location, copy: boolean) => void) | null = null;

  constructor() {
    if (typeof window === "undefined") return;
    window.addEventListener("pointermove", (event) => this.#move(event));
    window.addEventListener("pointerup", (event) => this.#up(event));
    window.addEventListener("keydown", (event) => {
      if (event.key === "Escape") this.cancel();
      this.forceCopy = event.ctrlKey || event.altKey;
    });
    window.addEventListener("keyup", (event) => (this.forceCopy = event.ctrlKey || event.altKey));
  }

  /** Registers what a `data-drop` key stands for. Call again whenever it changes. */
  target(key: string, loc: Location) {
    this.#targets.set(key, loc);
  }

  handleDrop(fn: (payload: DragPayload, to: Location, copy: boolean) => void) {
    this.#onDrop = fn;
  }

  /** Called from pointerdown on an item; the drag starts only once the pointer has moved. */
  arm(event: PointerEvent, payload: DragPayload) {
    if (event.button !== 0) return;
    // Stops WebKitGTK from starting a text selection while the pointer moves.
    event.preventDefault();
    this.#pending = { x: event.clientX, y: event.clientY, payload };
  }

  cancel() {
    this.#pending = null;
    this.payload = null;
    this.over = null;
  }

  #keyAt(x: number, y: number): string | null {
    const el = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop]");
    return el?.dataset.drop ?? null;
  }

  #move(event: PointerEvent) {
    if (this.#pending && !this.payload) {
      const moved = Math.hypot(event.clientX - this.#pending.x, event.clientY - this.#pending.y);
      if (moved < START_DISTANCE) return;
      this.payload = this.#pending.payload;
    }
    if (!this.payload) return;
    this.x = event.clientX;
    this.y = event.clientY;
    this.forceCopy = event.ctrlKey || event.altKey;
    this.over = this.#keyAt(event.clientX, event.clientY);
  }

  #up(event: PointerEvent) {
    const payload = this.payload;
    this.#pending = null;
    if (!payload) return;
    const key = this.#keyAt(event.clientX, event.clientY);
    const to = key ? this.#targets.get(key) : undefined;
    this.payload = null;
    this.over = null;
    if (to) this.#onDrop?.(payload, to, event.ctrlKey || event.altKey);
  }
}

export const drag = new DragStore();

/** A stable key for a location, usable as a `data-drop` value. */
export function dropKey(loc: Location): string {
  return loc.kind === "local" ? `local:${loc.path}` : `remote:${loc.remote}:${loc.path}`;
}
