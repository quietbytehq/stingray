<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";

  /**
   * The terminal panel (F4): a real shell that starts in the folder being viewed. When the folder
   * changes in Stingray and the shell is idle at its prompt, Stingray types `cd` for you; when you
   * `cd` in the shell, Stingray follows (Linux).
   */
  let {
    cwd,
    dark = false,
    onCwd,
    onClose,
  }: { cwd: string; dark?: boolean; onCwd: (path: string) => void; onClose: () => void } = $props();

  let host: HTMLDivElement;
  let id = 0;
  let term: Terminal | null = null;
  /** The folder the shell is in, as last reported; avoids echoing a cd back and forth. */
  let shellCwd = "";

  const THEMES = {
    dark: { background: "#17141f", foreground: "#e8e4f0", cursor: "#b98bff", selectionBackground: "#8a3ffc55" },
    light: { background: "#fbfaff", foreground: "#2a2633", cursor: "#8a3ffc", selectionBackground: "#8a3ffc33" },
  };

  function decode(b64: string): Uint8Array {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  /** Quotes a path for a POSIX shell. */
  const quote = (p: string) => `'${p.replace(/'/g, `'\\''`)}'`;

  onMount(() => {
    term = new Terminal({
      fontFamily: '"Ubuntu Mono", "DejaVu Sans Mono", "Cascadia Mono", Consolas, monospace',
      fontSize: 13,
      cursorBlink: true,
      allowProposedApi: false,
      theme: dark ? THEMES.dark : THEMES.light,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();

    const stops: (() => void)[] = [];
    let disposed = false;

    void (async () => {
      try {
        id = await invoke<number>("term_open", { cwd, cols: term!.cols, rows: term!.rows });
      } catch (err) {
        term?.writeln(`\r\nStingray couldn't start a terminal: ${(err as { message?: string })?.message ?? err}`);
        return;
      }
      shellCwd = cwd;
      if (disposed) return void invoke("term_close", { id });
      stops.push(await listen<{ id: number; data: string }>("term://data", (e) => {
        if (e.payload.id === id) term?.write(decode(e.payload.data));
      }));
      stops.push(await listen<number>("term://exit", (e) => {
        if (e.payload === id) onClose();
      }));
      stops.push(await listen<{ id: number; path: string }>("term://cwd", (e) => {
        if (e.payload.id !== id || e.payload.path === shellCwd) return;
        shellCwd = e.payload.path;
        onCwd(e.payload.path);
      }));
      term!.onData((data) => void invoke("term_write", { id, data }));
      term!.onResize(({ cols, rows }) => void invoke("term_resize", { id, cols, rows }));
      term!.focus();
    })();

    const observer = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {
        // Hidden or zero-sized for a moment.
      }
    });
    observer.observe(host);

    return () => {
      disposed = true;
      observer.disconnect();
      stops.forEach((s) => s());
      if (id) void invoke("term_close", { id });
      term?.dispose();
    };
  });

  // Stingray moved to another folder: follow it in the shell, but only when nothing is running.
  $effect(() => {
    const target = cwd;
    if (!id || !target || target === shellCwd || target.includes("!/")) return;
    void (async () => {
      const idle = await invoke<boolean>("term_idle", { id }).catch(() => false);
      if (!idle) return;
      shellCwd = target;
      await invoke("term_write", { id, data: ` cd -- ${quote(target)}\r` });
    })();
  });

  $effect(() => {
    if (term) term.options.theme = dark ? THEMES.dark : THEMES.light;
  });
</script>

<section class="terminal" aria-label="Terminal">
  <header>
    <span>Terminal</span>
    <button type="button" aria-label="Close terminal" title="Close (F4)" onclick={onClose}>×</button>
  </header>
  <div class="host" bind:this={host}></div>
</section>

<style>
  .terminal {
    display: flex;
    flex-direction: column;
    height: 240px;
    min-height: 120px;
    border-top: 1px solid var(--separator);
    resize: vertical;
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--separator);
  }
  header button {
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 15px;
    border-radius: 5px;
    width: 20px;
    height: 20px;
  }
  header button:hover {
    background: var(--control-bg);
    color: var(--text-primary);
  }
  .host {
    flex: 1;
    min-height: 0;
    padding: 4px 0 0 6px;
  }
</style>
