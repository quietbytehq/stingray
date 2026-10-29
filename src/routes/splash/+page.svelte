<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { FADE_OUT_MS, REDUCED_HOLD_MS } from "$lib/splash/timeline";

  /**
   * Stingray's opening: a short animation of the stingray swimming (an animated WebP, which every
   * web view shows without needing a video player), with the name settling in over it.
   * Click or press any key to skip. With reduced motion, a still frame shows briefly instead. If the
   * clip can't play (a missing video codec, say), the still frame shows and the app opens as usual.
   */
  /** Longest the splash may stay up, whatever happens to the clip. */
  const SAFETY_MS = 8000;
  /** The clip is longer than an intro should be; it is stopped here, fading out. */
  const CLIP_MS = 6000;
  /** The short intro on later launches stops the clip early. */
  const SHORT_MS = 6000;

  let still = $state(false);
  let leaving = $state(false);
  let failed = $state(false);
  let showName = $state(false);
  let version = $state("");
  let finished = false;

  async function finish() {
    if (finished) return;
    finished = true;
    leaving = true;
    await new Promise((resolve) => window.setTimeout(resolve, still ? 0 : FADE_OUT_MS));
    try {
      await api.splashFinished();
    } catch {
      // In a plain browser there is no backend to hand over to.
    }
  }

  onMount(() => {
    void api.appInfo().then((info) => (version = info.version)).catch(() => {});
    still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timers: number[] = [window.setTimeout(finish, SAFETY_MS)];
    const nameAt = window.setTimeout(() => (showName = true), still ? 0 : 1800);
    timers.push(nameAt);
    if (still) {
      timers.push(window.setTimeout(finish, REDUCED_HOLD_MS));
    } else {
      void api
        .getSplashPlan()
        .then((plan) => {
          timers.push(window.setTimeout(finish, plan === "short" ? SHORT_MS : CLIP_MS));
        })
        .catch(() => {});
    }
    return () => timers.forEach((t) => window.clearTimeout(t));
  });

  function videoFailed() {
    failed = true;
    showName = true;
    window.setTimeout(finish, REDUCED_HOLD_MS);
  }
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
  class:leaving
  role="button"
  tabindex="0"
  aria-label="Skip intro"
  onclick={finish}
  onkeydown={(event) => {
    event.preventDefault();
    void finish();
  }}
  style:transition-duration="{FADE_OUT_MS}ms"
>
  {#if still || failed}
    <img class="frame" src="/splash-still.jpg" alt="" />
  {:else}
    <img class="frame" src="/splash.webp" alt="" onerror={videoFailed} />
  {/if}
  <div class="name" class:shown={showName}>
    <h1>Stingray</h1>
    <p>Your files, your clouds, your servers. One window.</p>
  </div>
  <p class="footer" class:shown={showName}>{version ? `Version ${version} · ` : ""}QuietByte</p>
</div>

<style>
  .splash {
    position: fixed;
    inset: 0;
    border-radius: 18px;
    overflow: hidden;
    background: #1c1828;
    color: #fff;
    cursor: default;
    outline: none;
    transition-property: opacity;
  }
  .splash.leaving {
    opacity: 0;
  }
  .frame {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .frame.still {
    object-fit: contain;
    padding: 40px 0 90px;
    box-sizing: border-box;
  }
  /* A soft shade at the bottom keeps the name readable over the clip. */
  .splash::after {
    content: "";
    position: absolute;
    inset: auto 0 0 0;
    height: 45%;
    background: linear-gradient(to top, rgba(18, 14, 28, 0.85), transparent);
    pointer-events: none;
  }
  .name {
    position: absolute;
    left: 28px;
    bottom: 34px;
    z-index: 1;
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity 0.7s ease,
      transform 0.7s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @font-face {
    font-family: "Great Vibes";
    src: url("/fonts/GreatVibes-Regular.ttf") format("truetype");
    font-display: block;
  }
  .name h1 {
    margin: 0 0 -4px;
    font-family: "Great Vibes", "Segoe Script", cursive;
    font-size: 64px;
    font-weight: 400;
    line-height: 1.05;
    color: #ffffff;
    text-shadow:
      0 2px 12px rgba(138, 63, 252, 0.55),
      0 0 2px rgba(0, 0, 0, 0.4);
  }
  .name p {
    margin: 2px 0 0;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.75);
  }
  .footer {
    position: absolute;
    right: 22px;
    bottom: 16px;
    z-index: 1;
    margin: 0;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.55);
    opacity: 0;
    transition: opacity 0.7s ease 0.2s;
  }
  .shown {
    opacity: 1;
    transform: none;
  }
</style>
