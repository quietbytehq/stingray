<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { FADE_OUT_MS, REDUCED_HOLD_MS } from "$lib/splash/timeline";
  import { FACET_COUNT, renderFrame, type Frame } from "$lib/splash/stingray3d";

  /** The live 3D ray. Drawn every animation frame; one still frame with reduced motion. */
  let frame = $state<Frame>(renderFrame(0));
  const facetSlots = Array.from({ length: FACET_COUNT }, (_, i) => i);

  /**
   * Stingray's opening: a 3D low-poly stingray swims, drawn live, then settles as the name appears.
   * Click or press any key to skip. With reduced motion, one still frame shows briefly instead.
   */
  /** Longest the splash may stay up, whatever happens. */
  const SAFETY_MS = 8000;
  /** How long the swim plays before the splash fades. */
  const CLIP_MS = 6000;
  /** The same length on later launches. */
  const SHORT_MS = 6000;

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
    let raf = 0;
    if (!still) {
      const start = performance.now();
      const tick = (now: number) => {
        const t = (now - start) / 1000;
        // Calm the wings over the last 1.2 seconds so the ray settles as the splash fades.
        const settle = Math.max(0, (t - (CLIP_MS / 1000 - 1.2)) / 1.2);
        frame = renderFrame(t, settle);
        if (!finished) raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    }
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
    return () => {
      timers.forEach((t) => window.clearTimeout(t));
      cancelAnimationFrame(raf);
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
    <svg class="frame" viewBox="-80 150 1160 652" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <defs>
        <linearGradient id="rg1" x1="0%" y1="0%" x2="100%" y2="0%"><stop offset="0%" stop-color="#e879f9" stop-opacity="0" /><stop offset="35%" stop-color="#c026d3" stop-opacity="0.55" /><stop offset="70%" stop-color="#7c3aed" stop-opacity="0.35" /><stop offset="100%" stop-color="#4f46e5" stop-opacity="0" /></linearGradient>
        <linearGradient id="rg2" x1="0%" y1="0%" x2="100%" y2="0%"><stop offset="0%" stop-color="#a855f7" stop-opacity="0" /><stop offset="45%" stop-color="#e879f9" stop-opacity="0.45" /><stop offset="80%" stop-color="#9333ea" stop-opacity="0.2" /><stop offset="100%" stop-color="#6d28d9" stop-opacity="0" /></linearGradient>
        <radialGradient id="aura" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#c026d3" stop-opacity="0.25" /><stop offset="55%" stop-color="#6d28d9" stop-opacity="0.08" /><stop offset="100%" stop-color="#000000" stop-opacity="0" /></radialGradient>
      </defs>
      <rect x="-80" y="150" width="1160" height="652" fill="#06040d" />
      <ellipse cx="500" cy="500" rx="340" ry="270" fill="url(#aura)" />
      <path d={frame.ribbons[0]} fill="none" stroke="url(#rg1)" stroke-width="14" stroke-linecap="round" />
      <path d={frame.ribbons[1]} fill="none" stroke="url(#rg2)" stroke-width="9" stroke-linecap="round" />
      <path d={frame.ribbons[2]} fill="none" stroke="url(#rg1)" stroke-width="5" stroke-linecap="round" />
      {#each facetSlots as i (i)}
        {@const f = frame.facets[i]}
        <path d={f.d} fill={f.fill} stroke={f.stroke} stroke-width="0.9" stroke-linejoin="round" />
      {/each}
      <path d={frame.ribbons[3]} fill="none" stroke="url(#rg2)" stroke-width="3.5" stroke-linecap="round" />
    </svg>
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
