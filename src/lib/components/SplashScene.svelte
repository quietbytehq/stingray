<script lang="ts">
  /**
   * Stingray 2.0's opening scene, drawn as vector graphics so it is sharp at any size: the white
   * ray from the logo glides in over glowing waves, with light shafts, rising particles and two
   * distant rays for depth, then the name and slogan appear. `still` shows the final frame only.
   */
  let { still = false }: { still?: boolean } = $props();

  const RAY =
    "M10,95 C45,102 80,92 105,72 C140,45 180,20 205,-5 C200,40 188,72 162,100 C148,116 150,132 172,152 C195,172 222,196 240,215 C210,198 172,176 140,158 C110,140 70,130 35,118 C20,112 8,104 10,95 Z";
  const RAY_DOWN =
    "M10,108 C45,108 80,98 105,80 C140,58 180,45 210,30 C200,62 188,84 162,106 C148,120 150,135 172,154 C195,174 222,197 240,215 C210,198 172,177 140,160 C110,145 70,138 35,128 C20,122 8,116 10,108 Z";
  const FLAP = `${RAY};${RAY_DOWN};${RAY}`;
  const EASE = "0.45 0 0.55 1;0.45 0 0.55 1";

  const SHAFTS = [
    { x: 90, w: 70, d: 7, b: 0 },
    { x: 210, w: 46, d: 9, b: 1.5 },
    { x: 330, w: 90, d: 8, b: 0.8 },
    { x: 470, w: 55, d: 10, b: 2.2 },
  ];
  // Fixed positions so the scene looks the same every time.
  const PARTICLES = [
    [341, 1, 13, 0.6, 0.7], [106, 1.2, 7, 1.5, 0.7], [454, 1.6, 8, 0.7, 0.35], [70, 2, 8, 5, 0.35],
    [580, 1, 11, 1.6, 0.5], [594, 1.2, 9, 5.2, 0.35], [202, 1.2, 8, 5.7, 0.7], [331, 1.6, 14, 4.2, 0.5],
    [259, 0.8, 11, 5.4, 0.7], [469, 1.2, 8, 4.1, 0.35], [89, 2, 12, 8.3, 0.5], [518, 2, 14, 11.8, 0.35],
    [495, 0.8, 7, 4.9, 0.7], [601, 1.6, 11, 4.2, 0.7], [365, 0.8, 14, 2.4, 0.5], [417, 1.6, 14, 2.3, 0.35],
    [164, 1, 10, 4.8, 0.35], [388, 2, 12, 8.3, 0.35], [24, 1.2, 9, 3, 0.5], [628, 1, 10, 6, 0.5],
  ];
</script>

<svg class="scene" class:still viewBox="0 0 640 360" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
  <defs>
    <linearGradient id="sbg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1fb8ff" /><stop offset="0.35" stop-color="#2a3cf0" /><stop offset="0.7" stop-color="#4a12c8" /><stop offset="1" stop-color="#c21ad6" /></linearGradient>
    <radialGradient id="sshade" cx="0.55" cy="0.45" r="0.75"><stop offset="0" stop-color="#0a0640" stop-opacity="0.12" /><stop offset="1" stop-color="#05032a" stop-opacity="0.8" /></radialGradient>
    <radialGradient id="shalo" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#9d8bff" stop-opacity="0.55" /><stop offset="1" stop-color="#6a4cff" stop-opacity="0" /></radialGradient>
    <linearGradient id="sshaft" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.22" /><stop offset="1" stop-color="#ffffff" stop-opacity="0" /></linearGradient>
    <linearGradient id="sw1" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#3fd0ff" stop-opacity="0" /><stop offset="0.4" stop-color="#4a6bff" stop-opacity="0.6" /><stop offset="1" stop-color="#f03cf0" stop-opacity="0.8" /></linearGradient>
    <linearGradient id="sw2" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#5a3cff" stop-opacity="0.45" /><stop offset="1" stop-color="#ff4fe0" stop-opacity="0.32" /></linearGradient>
    <linearGradient id="sw3" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#21c8ff" stop-opacity="0.35" /><stop offset="1" stop-color="#7a3cff" stop-opacity="0.15" /></linearGradient>
    <linearGradient id="sray" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffffff" /><stop offset="0.6" stop-color="#e6e0ff" /><stop offset="1" stop-color="#b9a8ff" /></linearGradient>
    <linearGradient id="strail" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ffffff" stop-opacity="0.9" /><stop offset="0.5" stop-color="#c58bff" stop-opacity="0.55" /><stop offset="1" stop-color="#ff5ae6" stop-opacity="0" /></linearGradient>
    <filter id="sglow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="6" result="b" /><feMerge><feMergeNode in="b" /><feMergeNode in="SourceGraphic" /></feMerge></filter>
    <filter id="sblur"><feGaussianBlur stdDeviation="2.2" /></filter>
  </defs>

  <rect width="640" height="360" fill="url(#sbg)" />
  {#each SHAFTS as s (s.x)}
    <polygon fill="url(#sshaft)" points="{s.x},-10 {s.x + s.w},-10 {s.x + s.w * 2 + 60},360 {s.x + 60},360" opacity="0.5">
      {#if !still}<animate attributeName="opacity" dur="{s.d}s" begin="-{s.b}s" repeatCount="indefinite" values="0.25;0.7;0.25" />{/if}
    </polygon>
  {/each}
  <rect width="640" height="360" fill="url(#sshade)" />

  {#if !still}
    {#each [{ y0: 70, y1: 95, k: 0.22, d: 14, o: 0.22, b: 0 }, { y0: 210, y1: 185, k: 0.16, d: 18, o: 0.16, b: 6 }] as far (far.y0)}
      <g opacity={far.o}>
        <animateTransform attributeName="transform" type="translate" dur="{far.d}s" begin="-{far.b}s" repeatCount="indefinite" values="680 {far.y0};-80 {far.y1}" />
        <g transform="scale({far.k})"><path fill="#cfc4ff" d={RAY}><animate attributeName="d" dur="1.9s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values={FLAP} /></path></g>
      </g>
    {/each}
    {#each PARTICLES as [x, r, d, b, o] (x)}
      <circle cx={x} cy="370" {r} fill="#ffffff" opacity={o}><animate attributeName="cy" dur="{d}s" begin="-{b}s" repeatCount="indefinite" values="370;-10" /></circle>
    {/each}
  {/if}

  <path fill="url(#sw3)" d="M0,300 C140,270 260,320 380,296 C500,272 580,270 640,280 L640,360 L0,360 Z">
    {#if !still}<animate attributeName="d" dur="5s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="M0,300 C140,270 260,320 380,296 C500,272 580,270 640,280 L640,360 L0,360 Z;M0,292 C150,285 260,300 380,286 C500,268 580,292 640,272 L640,360 L0,360 Z;M0,300 C140,270 260,320 380,296 C500,272 580,270 640,280 L640,360 L0,360 Z" />{/if}
  </path>
  <path fill="url(#sw2)" opacity="0.85" d="M0,320 C120,280 230,345 360,320 C470,300 560,285 640,300 L640,360 L0,360 Z">
    {#if !still}<animate attributeName="d" dur="4s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="M0,320 C120,280 230,345 360,320 C470,300 560,285 640,300 L640,360 L0,360 Z;M0,312 C130,295 240,330 360,312 C480,292 560,305 640,288 L640,360 L0,360 Z;M0,320 C120,280 230,345 360,320 C470,300 560,285 640,300 L640,360 L0,360 Z" />{/if}
  </path>
  <path fill="none" stroke="url(#sw1)" stroke-width="3" filter="url(#sglow)" d="M-20,305 C110,255 230,280 340,318 C430,348 540,340 660,318">
    {#if !still}<animate attributeName="d" dur="4s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="M-20,305 C110,255 230,280 340,318 C430,348 540,340 660,318;M-20,295 C120,270 240,272 340,305 C440,340 550,325 660,330;M-20,305 C110,255 230,280 340,318 C430,348 540,340 660,318" />{/if}
  </path>

  <g transform="translate(60 38)">
    {#if !still}<animateTransform attributeName="transform" type="translate" dur="2.6s" fill="freeze" calcMode="spline" keySplines="0.2 0.8 0.2 1" keyTimes="0;1" values="330 170;60 38" />{/if}
    <g>
      {#if !still}<animateTransform attributeName="transform" type="translate" dur="3.2s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="0 0;0 -7;0 0" />{/if}
      <ellipse cx="130" cy="110" rx="150" ry="95" fill="url(#shalo)" />
      {#if !still}
        <path fill="none" stroke="url(#strail)" stroke-width="5" stroke-linecap="round" filter="url(#sblur)"><animate attributeName="d" dur="1.6s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="M226,203 C270,226 320,232 390,252;M226,203 C272,222 318,246 390,240;M226,203 C270,226 320,232 390,252" /></path>
        <path fill="none" stroke="url(#strail)" stroke-width="2.5" stroke-linecap="round" opacity="0.7"><animate attributeName="d" dur="2s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values="M200,180 C250,205 300,220 360,232;M200,180 C252,198 298,230 360,222;M200,180 C250,205 300,220 360,232" /></path>
      {/if}
      <g filter="url(#sglow)" transform="scale(0.95)">
        <path fill="url(#sray)" d={RAY}>
          {#if !still}<animate attributeName="d" dur="1.6s" repeatCount="indefinite" calcMode="spline" keySplines={EASE} values={FLAP} />{/if}
        </path>
      </g>
    </g>
  </g>

  <g class="words">
    <text x="360" y="150" class="name">Stingray</text>
    <text x="362" y="182" class="line">Your files, your clouds, your servers.</text>
    <text x="362" y="203" class="line strong">One window.</text>
  </g>
</svg>

<style>
  @font-face {
    font-family: "Outfit";
    src: url("/fonts/Outfit.ttf") format("truetype");
    font-weight: 100 900;
    font-display: block;
  }
  .scene {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .words {
    font-family: "Outfit", "Ubuntu", "Segoe UI", sans-serif;
    fill: #ffffff;
    opacity: 0;
    animation: words 0.8s ease 1.4s forwards;
  }
  .still .words {
    opacity: 1;
    animation: none;
  }
  .name {
    font-size: 56px;
    font-weight: 700;
    letter-spacing: 1px;
    filter: drop-shadow(0 2px 10px rgba(110, 80, 255, 0.7));
  }
  .line {
    font-size: 15px;
    font-weight: 400;
    fill-opacity: 0.82;
  }
  .line.strong {
    font-weight: 600;
    fill-opacity: 1;
  }
  @keyframes words {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
