/**
 * The splash stingray: a low-poly 3D ray drawn live as SVG paths. Each frame turns the model,
 * ripples the wings in a travelling wave, whips the tail, lights each facet, sorts them back to
 * front and trails four glowing ribbons. Pure maths, no dependencies; the splash calls
 * `renderFrame` from requestAnimationFrame.
 */

type Vec3 = [number, number, number];

const VERTICES: Record<string, Vec3> = {
  snout: [210, 4, 0], headTop: [145, 34, 0], eyeL: [160, 22, 52], eyeR: [160, 22, -52],
  browL: [150, 34, 45], browR: [150, 34, -45],
  eyePupilL1: [168, 20, 55], eyePupilL2: [152, 25, 57], eyePupilL3: [154, 15, 56],
  eyePupilR1: [168, 20, -55], eyePupilR2: [152, 25, -57], eyePupilR3: [154, 15, -56],
  spineMid: [35, 44, 0], spineBack: [-75, 26, 0], tailBase: [-145, 10, 0], tailMid: [-285, 4, 0], tailTip: [-455, 0, 0],
  wingFrontL: [115, 8, 155], wingTipL: [5, 0, 310], wingMidL: [20, 24, 135], wingRearL: [-95, 8, 165],
  pelvicL: [-155, 3, 68], pelvicInnerL: [-135, 6, 25],
  wingFrontR: [115, 8, -155], wingTipR: [5, 0, -310], wingMidR: [20, 24, -135], wingRearR: [-95, 8, -165],
  pelvicR: [-155, 3, -68], pelvicInnerR: [-135, 6, -25],
  bellyFront: [130, -18, 0], bellyMid: [15, -28, 0], bellyL: [35, -16, 95], bellyR: [35, -16, -95],
};

/** Facets: vertex names, base colour, and kind (d = back, v = belly, t = tail, e = eye). */
const FACETS: [string, string, string][] = [
  ["snout browL headTop", "#8b3dff", "d"], ["snout headTop browR", "#9d4edd", "d"],
  ["snout eyeL browL", "#7b2cbf", "d"], ["snout browR eyeR", "#9333ea", "d"],
  ["eyePupilL1 eyePupilL2 eyePupilL3", "#190d26", "e"], ["eyePupilR1 eyePupilR3 eyePupilR2", "#190d26", "e"],
  ["headTop browL wingMidL spineMid", "#9d4edd", "d"], ["headTop spineMid wingMidR browR", "#b5179e", "d"],
  ["spineMid wingMidL wingRearL spineBack", "#8b3dff", "d"], ["spineMid spineBack wingRearR wingMidR", "#c71f9b", "d"],
  ["snout wingFrontL eyeL", "#7209b7", "d"], ["eyeL wingFrontL wingMidL browL", "#8338ec", "d"],
  ["wingFrontL wingTipL wingMidL", "#6c2bd9", "d"], ["wingMidL wingTipL wingRearL", "#7c3aed", "d"],
  ["spineBack wingRearL pelvicL tailBase", "#8b5cf6", "d"], ["tailBase pelvicL pelvicInnerL", "#a855f7", "d"],
  ["snout eyeR wingFrontR", "#b5179e", "d"], ["eyeR browR wingMidR wingFrontR", "#c026d3", "d"],
  ["wingFrontR wingMidR wingTipR", "#d946ef", "d"], ["wingMidR wingRearR wingTipR", "#e0249a", "d"],
  ["spineBack tailBase pelvicR wingRearR", "#d61f89", "d"], ["tailBase pelvicInnerR pelvicR", "#f72585", "d"],
  ["snout bellyFront bellyL wingFrontL", "#5b21b6", "v"], ["snout wingFrontR bellyR bellyFront", "#86198f", "v"],
  ["bellyFront bellyMid bellyL", "#4c1d95", "v"], ["bellyFront bellyR bellyMid", "#701a75", "v"],
  ["bellyL bellyMid tailBase wingRearL", "#581c87", "v"], ["bellyR wingRearR tailBase bellyMid", "#831843", "v"],
  ["wingFrontL bellyL wingRearL wingTipL", "#6d28d9", "v"], ["wingFrontR wingTipR wingRearR bellyR", "#a21caf", "v"],
  ["spineBack tailBase tailMid", "#d946ef", "t"], ["tailBase pelvicInnerL tailMid", "#c026d3", "t"],
  ["tailBase tailMid pelvicInnerR", "#e81cff", "t"], ["tailMid pelvicInnerL tailTip", "#d946ef", "t"],
  ["tailMid tailTip pelvicInnerR", "#f72585", "t"],
];

export const FACET_COUNT = FACETS.length;

export interface Frame {
  facets: { d: string; fill: string; stroke: string }[];
  ribbons: string[];
}

function shade(hex: string, delta: number): string {
  const n = parseInt(hex.slice(1), 16);
  const c = (v: number) => Math.round(Math.min(255, Math.max(0, v + delta)));
  return `rgb(${c((n >> 16) & 255)},${c((n >> 8) & 255)},${c(n & 255)})`;
}

/**
 * One frame at `t` seconds. `settle` (0..1) calms the wings toward the end of the intro, so the ray
 * comes to rest in the logo pose as the splash fades.
 */
export function renderFrame(t: number, settle = 0): Frame {
  const tau = Math.PI * 2;
  const deg = Math.PI / 180;
  // Camera looks down on the ray at the logo angle and drifts gently.
  const pitch = (58 + 6 * Math.sin(t * 0.6)) * deg;
  const yaw = (-42 + 14 * Math.sin(t * 0.35)) * deg;
  const roll = (-18 + 8 * Math.sin(t * 0.5 + 1)) * deg;
  const cP = Math.cos(pitch), sP = Math.sin(pitch), cY = Math.cos(yaw), sY = Math.sin(yaw);
  const cR = Math.cos(roll), sR = Math.sin(roll);
  const omega = t * 0.7 * tau;
  const amp = 1 - 0.85 * Math.min(1, Math.max(0, settle));

  function project(x0: number, y0: number, z0: number) {
    const span = Math.min(1, Math.abs(z0) / 310);
    const wave = omega - (x0 / 170) * 1.35 - span * 0.85;
    const stroke = Math.sin(wave) * Math.pow(span, 1.35) * 110 * amp;
    const spine = Math.cos(omega - x0 / 190) * 14 * amp;
    let tailY = 0;
    let tailZ = 0;
    if (x0 < -100) {
      const d = (-100 - x0) / 355;
      tailY = Math.sin(omega - 1.4 - d * 2.6) * d * d * 95 * amp;
      tailZ = Math.cos(omega - 1.1 - d * 2.2) * d * 28 * amp;
    }
    const contract = 1 - 0.08 * (0.5 * (1 - Math.cos(2 * wave))) * span * amp;
    const x = x0, y = y0 + stroke + spine + tailY, z = z0 * contract + tailZ;
    const x1 = x * cY + z * sY, z1 = -x * sY + z * cY;
    const y2 = y * cP - z1 * sP, z2 = y * sP + z1 * cP;
    const x3 = x1 * cR - y2 * sR, y3 = x1 * sR + y2 * cR;
    const s = 950 / Math.max(250, 820 - z2);
    return { x: x3, y: y3, z: z2, sx: 500 + x3 * s, sy: 500 - y3 * s };
  }

  const p: Record<string, ReturnType<typeof project>> = {};
  for (const [k, v] of Object.entries(VERTICES)) p[k] = project(v[0], v[1], v[2]);

  const light = { x: 0.35, y: 0.78, z: 0.52 };
  const facets = FACETS.map(([names, colour, kind]) => {
    const q = names.split(" ").map((n) => p[n]);
    const z = q.reduce((sum, v) => sum + v.z, 0) / q.length;
    const ux = q[1].x - q[0].x, uy = q[1].y - q[0].y, uz = q[1].z - q[0].z;
    const vx = q[2].x - q[0].x, vy = q[2].y - q[0].y, vz = q[2].z - q[0].z;
    const nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
    const len = Math.hypot(nx, ny, nz) || 1;
    const dot = (nx * light.x + ny * light.y + Math.abs(nz) * light.z) / len;
    const fill = shade(colour, kind === "e" ? 0 : dot * 34 - 8);
    const d = "M " + q.map((v) => `${v.sx.toFixed(1)} ${v.sy.toFixed(1)}`).join(" L ") + " Z";
    return { d, fill, stroke: kind === "e" ? "#090410" : fill, z };
  }).sort((a, b) => a.z - b.z);

  function ribbon(seedZ: number, yOffset: number, phase: number, radius: number): string {
    const pts = [];
    for (let i = 0; i < 9; i++) {
      const lag = i * 0.16;
      pts.push(project(90 - i * 72, yOffset + Math.sin(omega - lag * 1.4 + phase) * 45 * radius * amp,
        seedZ * (1 + 0.18 * Math.sin(omega - lag + phase))));
    }
    let d = `M ${pts[0].sx.toFixed(1)} ${pts[0].sy.toFixed(1)}`;
    for (let i = 1; i < pts.length - 1; i++) {
      const xc = (pts[i].sx + pts[i + 1].sx) / 2, yc = (pts[i].sy + pts[i + 1].sy) / 2;
      d += ` Q ${pts[i].sx.toFixed(1)} ${pts[i].sy.toFixed(1)}, ${xc.toFixed(1)} ${yc.toFixed(1)}`;
    }
    return d;
  }

  return {
    facets,
    ribbons: [ribbon(295, 10, 0, 1), ribbon(-295, 10, 0.5, 1), ribbon(140, -18, 1.2, 0.7), ribbon(-140, 22, 2.1, 0.8)],
  };
}
