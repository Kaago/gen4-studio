import * as THREE from "three";
import type { TileBehavior } from "../../api";

/** The patterns of the layer „Gelände“ (spec §6, rule „Gelände-Muster“ of 6.4b). */
export type Pattern = "none" | "hatch" | "waves" | "dots" | "arrow" | "crosshatch";

/** The number the shader reads for a pattern (first float of `aPattern`). */
export const PATTERN_CODE: Record<Pattern, number> = { none: 0, hatch: 1, waves: 2, dots: 3, arrow: 4, crosshatch: 5 };

/** The pattern pixels take the cell colour times this (in the linear working space). */
export const PATTERN_SHADE = 0.55;

const BLOCKED = 0x80;
const DIRECTIONS = ["NORTH", "EAST", "SOUTH", "WEST"] as const;
const BY_CLASS: Record<string, Pattern> = { water: "waves", grass: "dots", ice: "crosshatch" };

/**
 * The pattern of a permission cell (rule „Gelände-Muster“); dir 0 N, 1 E, 2 S, 3 W (arrows only). A ledge points where its
 * behaviour name says it jumps (the first of NORTH/EAST/SOUTH/WEST in it, else north), even with the collision bit set; then the
 * collision bit hatches; then water, grass and ice by their class; everything else has none.
 */
export function patternOf(_kind: number, collision: number, behavior: TileBehavior | undefined): { pattern: Pattern; dir: number } {
  const cls = behavior?.class ?? null;
  if (cls === "ledge") {
    const match = /NORTH|EAST|SOUTH|WEST/.exec(behavior?.name ?? "");
    const dir = match ? DIRECTIONS.indexOf(match[0] as (typeof DIRECTIONS)[number]) : 0;
    return { pattern: "arrow", dir };
  }
  if ((collision & BLOCKED) !== 0) return { pattern: "hatch", dir: 0 };
  return { pattern: (cls !== null && BY_CLASS[cls]) || "none", dir: 0 };
}

const VERTEX_PARS = /* glsl */ `
attribute vec2 aPattern;
varying vec2 vPattern;
varying vec2 vCellUv;
`;

/**
 * Whether the pattern `code` marks the point `uv` of a cell (u east, v north on the overlay plate). The arrow is drawn pointing
 * north in a frame turned clockwise by `dir` quarter turns.
 */
const FRAGMENT_PARS = /* glsl */ `
varying vec2 vPattern;
varying vec2 vCellUv;

bool g4Hatch(vec2 uv) {
  return fract((uv.x + uv.y) * 4.0) < 0.22;
}

bool g4PatternMark(int code, float dir, vec2 uv) {
  if (code == 1) return g4Hatch(uv);
  if (code == 2) return abs(fract(uv.y * 3.0 + 0.15 * sin(uv.x * 6.2832 * 2.0)) - 0.5) < 0.09;
  if (code == 3) return length((fract(uv * 3.0) - 0.5) / 3.0) < 0.12;
  if (code == 4) {
    float a = floor(dir + 0.5) * 1.5708;
    vec2 p = uv - 0.5;
    vec2 q = vec2(p.x * cos(a) - p.y * sin(a), p.x * sin(a) + p.y * cos(a));
    bool shaft = abs(q.x) < 0.06 && q.y > -0.34 && q.y < 0.1;
    bool head = q.y >= 0.06 && q.y <= 0.36 && abs(q.x) <= (0.36 - q.y) * 0.75;
    return shaft || head;
  }
  if (code == 5) return g4Hatch(uv) || fract((uv.x - uv.y) * 4.0) < 0.22;
  return false;
}
`;

const FRAGMENT_MAIN = /* glsl */ `
if (g4PatternMark(int(vPattern.x + 0.5), vPattern.y, vCellUv)) {
  diffuseColor.rgb *= ${PATTERN_SHADE.toFixed(2)};
  diffuseColor.a = min(1.0, 2.0 * opacity);
}
`;

/**
 * Adds the instanced attribute `aPattern` (2 floats per instance: code, direction; all 0 = none) to `mesh` and the pattern to its
 * material: after the colour chunk the shader draws the pattern pixels in the cell colour × 0.55 at twice the opacity (at most 1);
 * the other pixels stay as they were.
 */
export function withPatterns(mesh: THREE.InstancedMesh): void {
  const attr = new THREE.InstancedBufferAttribute(new Float32Array(mesh.count * 2), 2);
  mesh.geometry.setAttribute("aPattern", attr);
  const material = mesh.material as THREE.Material;
  material.onBeforeCompile = (shader) => {
    shader.vertexShader = VERTEX_PARS + shader.vertexShader.replace("#include <begin_vertex>", "#include <begin_vertex>\nvPattern = aPattern;\nvCellUv = uv;");
    shader.fragmentShader = FRAGMENT_PARS + shader.fragmentShader.replace("#include <color_fragment>", `#include <color_fragment>\n${FRAGMENT_MAIN}`);
  };
  material.customProgramCacheKey = () => "g4-terrain-patterns";
}

/** Sets the pattern of instance `i` (the caller flags the attribute for upload, like `setColorAt`). */
export function setPatternAt(mesh: THREE.InstancedMesh, i: number, p: { pattern: Pattern; dir: number }): void {
  const attr = mesh.geometry.getAttribute("aPattern") as THREE.InstancedBufferAttribute;
  attr.setXY(i, PATTERN_CODE[p.pattern], p.dir);
}

/** How many instances of `mesh` carry a pattern. */
export function countPatterns(mesh: THREE.InstancedMesh): number {
  const attr = mesh.geometry.getAttribute("aPattern") as THREE.InstancedBufferAttribute | undefined;
  if (!attr) return 0;
  let n = 0;
  for (let i = 0; i < attr.count; i++) if (attr.getX(i) !== 0) n++;
  return n;
}

/** `#RRGGBB` of the pattern lines on a cell of colour `hex` (`#RRGGBB`): × 0.55 in the linear working space, like the shader. */
export function patternShade(hex: string): string {
  return `#${new THREE.Color(hex).multiplyScalar(PATTERN_SHADE).getHexString().toUpperCase()}`;
}
