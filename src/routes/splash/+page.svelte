<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { FADE_OUT_MS, REDUCED_HOLD_MS } from "$lib/splash/timeline";
  import SplashScene from "$lib/components/SplashScene.svelte";


  /**
   * Stingray's opening: the white ray from the logo glides in over glowing water (SplashScene).
   * Click or press any key to skip. With reduced motion, one still frame shows briefly instead.
   */
  /** Longest the splash may stay up, whatever happens. */
  const SAFETY_MS = 8000;
  /** How long the swim plays before the splash fades. */
  const CLIP_MS = 4000;
  /** The same length on later launches. */
  const SHORT_MS = 4000;

  let still = $state(false);
  let leaving = $state(false);
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
    const nameAt = window.setTimeout(() => (showName = true), still ? 0 : 1200);
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
    return () => {
      timers.forEach((t) => window.clearTimeout(t));
    };
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
    <SplashScene {still} />
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
  /* A soft shade at the bottom keeps the name readable over the clip. */
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
