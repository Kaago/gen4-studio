import type { IconName } from "../ui/primitives";
import { flipTile, turnRamp, turnTile } from "../world/paint/ops";
import type { PaintOptions, PaintSub, RampDir } from "../world/paint/ops";

/** One sub-tool of „Gelände malen“ on a world map: its name, its letter (Tiled's), its symbol, its tooltip and its status line. */
export interface PaintToolSpec {
  id: PaintSub;
  label: string;
  key: string;
  icon: IconName;
  /** The second line of the tooltip: what a stroke with it does. */
  hint: string;
  /** Key and effect pairs for the status line; the ramp's first pair follows the state (`paintStatusHints`). */
  statusHints: { keys: string; does: string }[];
}

const PICK = { keys: "Rechtsklick", does: "nimmt auf" };
const TURN = { keys: "Z", does: "dreht" };
const FLIP = { keys: "X", does: "spiegelt" };

export const PAINT_TOOLS: PaintToolSpec[] = [
  {
    id: "brush",
    label: "Pinsel",
    key: "B",
    icon: "paint",
    hint: "Ziehen malt die gewählte Kachel auf die Zellen unter dem Zeiger.",
    statusHints: [{ keys: "Ziehen", does: "malt" }, PICK, TURN, FLIP],
  },
  {
    id: "rect",
    label: "Rechteck",
    key: "P",
    icon: "rectangle",
    hint: "Ziehen spannt ein Rechteck auf und füllt es mit der gewählten Kachel.",
    statusHints: [{ keys: "Ziehen", does: "füllt ein Rechteck" }, PICK, TURN, FLIP],
  },
  {
    id: "fill",
    label: "Füllen",
    key: "G",
    icon: "bucket",
    hint: "Klick füllt die zusammenhängende Fläche gleicher Kacheln mit der gewählten Kachel.",
    statusHints: [{ keys: "Klick", does: "füllt die Fläche" }, PICK, TURN, FLIP],
  },
  {
    id: "erase",
    label: "Radierer",
    key: "E",
    icon: "eraser",
    hint: "Ziehen leert die Zellen der aktiven Ebene.",
    statusHints: [{ keys: "Ziehen", does: "radiert" }, PICK],
  },
  {
    id: "pick",
    label: "Pipette",
    key: "I",
    icon: "eyedropper",
    hint: "Klick nimmt die Kachel der Zelle als gewählte Kachel auf.",
    statusHints: [{ keys: "Klick", does: "nimmt die Kachel auf" }],
  },
  {
    id: "height",
    label: "Höhe",
    key: "H",
    icon: "heights",
    hint: "Klick hebt die Zelle um eine Stufe, Umschalt-Klick senkt sie.",
    statusHints: [
      { keys: "Klick", does: "hebt" },
      { keys: "⇧Klick", does: "senkt" },
      { keys: "Ziehen", does: "über mehrere" },
    ],
  },
  {
    id: "ramp",
    label: "Rampe",
    key: "R",
    icon: "ramp",
    hint: "Klick setzt eine Rampe in die gewählte Richtung, Umschalt-Klick entfernt sie.",
    statusHints: [{ keys: "Klick", does: "setzt Rampe" }, TURN, { keys: "1/2", does: "Stufen" }, { keys: "⇧Klick", does: "entfernt" }],
  },
];

/** What a key does while painting a world map (see `paintKey`). */
export type PaintKeyAction = { sub: PaintSub } | { turn: true } | { flip: true } | { steps: 1 | 2 };

/** What a plain key does while painting a world map: a sub-tool, turn (Z), flip (X), ramp steps (1, 2); null: not ours. */
export function paintKey(key: string): PaintKeyAction | null {
  const upper = key.toUpperCase();
  const tool = PAINT_TOOLS.find((t) => t.key === upper);
  if (tool) return { sub: tool.id };
  if (upper === "Z") return { turn: true };
  if (upper === "X") return { flip: true };
  if (key === "1" || key === "2") return { steps: key === "1" ? 1 : 2 };
  return null;
}

/** The change a paint key makes to the state: Z turns the ramp while the ramp is the sub-tool, else the tile. */
export function paintPatch(state: PaintOptions & { sub: PaintSub }, action: PaintKeyAction): Partial<PaintOptions & { sub: PaintSub }> {
  if ("sub" in action) return { sub: action.sub };
  if ("turn" in action) return state.sub === "ramp" ? { ramp: { ...state.ramp, dir: turnRamp(state.ramp.dir) } } : { tile: turnTile(state.tile) };
  if ("flip" in action) return { tile: flipTile(state.tile) };
  return { ramp: { ...state.ramp, steps: action.steps } };
}

const DIR_WORDS: Record<RampDir, string> = { n: "Norden", e: "Osten", s: "Süden", w: "Westen" };

/** The status line of the active sub-tool; the ramp names its direction and steps („Klick setzt Rampe nach Osten, 1 Stufe“). */
export function paintStatusHints(state: PaintOptions & { sub: PaintSub }): { keys: string; does: string }[] {
  const spec = PAINT_TOOLS.find((t) => t.id === state.sub) ?? PAINT_TOOLS[0]!;
  if (spec.id !== "ramp") return spec.statusHints;
  const steps = state.ramp.steps === 1 ? "1 Stufe" : `${state.ramp.steps} Stufen`;
  return [{ keys: "Klick", does: `setzt Rampe nach ${DIR_WORDS[state.ramp.dir]}, ${steps}` }, ...spec.statusHints.slice(1)];
}
