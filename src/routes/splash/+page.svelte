<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { FADE_OUT_MS, REDUCED_HOLD_MS, phaseTiming, timeline, type Timeline } from "$lib/splash/timeline";
  import { applyTheme } from "$lib/util/theme";

  /**
   * Stingray's opening: the ray glides in from the lower right along its own heading, the
   * wordmark settles under it, then the tagline. Click or press any key to skip.
   * Phases reuse the shared timeline: "ring" is the glide, "cloud" the light sweep across the ray.
   */
  let still = $state(false);
  let leaving = $state(false);
  let finished = false;
  let line = $state<Timeline | null>(null);
  let version = $state("");

  const glide = $derived(line ? phaseTiming(line, "ring") : [0, 0]);
  const sweep = $derived(line ? phaseTiming(line, "cloud") : [0, 0]);
  const word = $derived(line ? phaseTiming(line, "wordmark") : [0, 0]);
  const tag = $derived(line ? phaseTiming(line, "tagline") : [0, 0]);
  const foot = $derived(line ? phaseTiming(line, "footer") : [0, 0]);

  async function finish() {
    if (finished) return;
    finished = true;
    leaving = true;
    if (!still) await new Promise((resolve) => window.setTimeout(resolve, FADE_OUT_MS));
    try {
      await api.splashFinished();
    } catch {
      // In a plain browser there is no backend to hand over to.
    }
  }

  onMount(() => {
    void api.getSettings().then((s) => applyTheme(s.theme)).catch(() => {});
    void api.appInfo().then((info) => (version = info.version)).catch(() => {});
    still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let timer = 0;
    void api
      .getSplashPlan()
      .then((plan) => (plan === "short" ? "short" : "full"))
      .catch(() => "full" as const)
      .then((which) => {
        if (finished) return;
        line = timeline(which);
        timer = window.setTimeout(finish, still ? REDUCED_HOLD_MS : line.sequenceMs);
      });
    return () => window.clearTimeout(timer);
  });
</script>

<svelte:head>
  <style>
    html,
    body {
      background: transparent !important;
    }
  </style>
</svelte:head>

<svelte:window onkeydown={() => void finish()} />

<div
  class="splash"
  class:still
  class:leaving
  class:ready={line !== null}
  role="button"
  tabindex="0"
  aria-label="Skip intro"
  onclick={finish}
  onkeydown={(event) => {
    event.preventDefault();
    void finish();
  }}
>
  <div class="content" style:transition-duration="{FADE_OUT_MS}ms">
    <div class="ray" style:animation-delay="{glide[0]}ms" style:animation-duration="{glide[1]}ms">
      <img src="/stingray.png" alt="" width="190" height="190" />
      <span class="sheen" style:animation-delay="{sweep[0]}ms" style:animation-duration="{sweep[1]}ms"></span>
    </div>
    <h1 style:animation-delay="{word[0]}ms" style:animation-duration="{word[1]}ms">Stingray</h1>
    <p class="tagline" style:animation-delay="{tag[0]}ms" style:animation-duration="{tag[1]}ms">
      Your files, your clouds, your servers. One window.
    </p>
    <p class="footer" style:animation-delay="{foot[0]}ms" style:animation-duration="{foot[1]}ms">
      {version ? `Version ${version}` : ""} · QuietByte
    </p>
  </div>
</div>

<style>
  .splash {
    position: fixed;
    inset: 0;
    border-radius: 22px;
    background: var(--splash-bg);
    color: var(--splash-ink);
    display: grid;
    place-items: center;
    overflow: hidden;
    cursor: default;
    outline: none;
  }
  .content {
    display: flex;
    flex-direction: column;
    align-items: center;
    transition-property: opacity;
  }
  .leaving .content {
    opacity: 0;
  }
  .ray {
    position: relative;
    width: 190px;
    height: 190px;
    opacity: 0;
    -webkit-mask: url(/stingray.png) center / contain no-repeat;
    mask: url(/stingray.png) center / contain no-repeat;
  }
  .ray img {
    display: block;
  }
  /* A band of light passes over the ray once it has arrived, clipped to its shape by the mask. */
  .sheen {
    position: absolute;
    inset: 0;
    background: linear-gradient(115deg, transparent 35%, rgba(255, 255, 255, 0.55) 50%, transparent 65%);
    transform: translateX(-120%);
    opacity: 0;
  }
  h1 {
    margin: 6px 0 0;
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -0.035em;
    background: linear-gradient(120deg, #8a3ffc, #e81cc4);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    opacity: 0;
  }
  .tagline {
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--splash-muted);
    opacity: 0;
  }
  .footer {
    margin: 26px 0 0;
    font-size: 11px;
    color: var(--splash-muted);
    opacity: 0;
  }
  .ready .ray {
    animation-name: glide;
    animation-timing-function: cubic-bezier(0.16, 1, 0.3, 1);
    animation-fill-mode: forwards;
  }
  .ready .sheen {
    animation-name: sheen;
    animation-timing-function: ease-in-out;
    animation-fill-mode: forwards;
  }
  .ready h1,
  .ready .tagline,
  .ready .footer {
    animation-name: rise;
    animation-timing-function: cubic-bezier(0.2, 0.8, 0.2, 1);
    animation-fill-mode: forwards;
  }
  @keyframes glide {
    from {
      opacity: 0;
      transform: translate(70px, 60px) rotate(8deg) scale(0.82);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes sheen {
    from {
      opacity: 1;
      transform: translateX(-120%);
    }
    to {
      opacity: 1;
      transform: translateX(120%);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .still .ray,
  .still h1,
  .still .tagline,
  .still .footer {
    animation: none !important;
    opacity: 1;
  }
</style>
