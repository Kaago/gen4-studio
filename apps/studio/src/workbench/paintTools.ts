import type { IconName } from "../ui/primitives";
import { flipTile, turnRamp, turnTile } from "../world/paint/ops";
import type { PaintOptions, PaintSub } from "../world/paint/ops";
import { t } from "../i18n";
import { keyWord } from "./tools";
import type { KeyWord } from "./tools";

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

/** What a status pair of a paint tool does (`paintTools.does.<id>`). */
type Does =
  | "paints"
  | "fillsRect"
  | "fillsArea"
  | "erases"
  | "picksTile"
  | "raises"
  | "lowers"
  | "acrossSeveral"
  | "setsRamp"
  | "steps"
  | "removes"
  | "takesAway"
  | "picks"
  | "turns"
  | "flips";
/** A status pair: a key word (or a key as it is written on the keyboard) and what it does. */
type Pair = readonly [KeyWord | "Z" | "X" | "1/2", Does];

const PICK: Pair = ["rightClick", "picks"];
const TURN: Pair = ["Z", "turns"];
const FLIP: Pair = ["X", "flips"];

/** A paint tool apart from its words; label and hint are `paintTools.<id>.*`, read when asked. */
interface PaintToolDef {
  id: PaintSub;
  key: string;
  icon: IconName;
  pairs: readonly Pair[];
}

const DEFS: readonly PaintToolDef[] = [
  { id: "brush", key: "B", icon: "paint", pairs: [["drag", "paints"], PICK, TURN, FLIP] },
  { id: "rect", key: "P", icon: "rectangle", pairs: [["drag", "fillsRect"], PICK, TURN, FLIP] },
  { id: "fill", key: "G", icon: "bucket", pairs: [["click", "fillsArea"], PICK, TURN, FLIP] },
  { id: "erase", key: "E", icon: "eraser", pairs: [["drag", "erases"], PICK] },
  { id: "pick", key: "I", icon: "eyedropper", pairs: [["click", "picksTile"]] },
  {
    id: "height",
    key: "H",
    icon: "heights",
    pairs: [
      ["click", "raises"],
      ["shiftClick", "lowers"],
      ["drag", "acrossSeveral"],
    ],
  },
  { id: "ramp", key: "R", icon: "ramp", pairs: [["click", "setsRamp"], TURN, ["1/2", "steps"], ["shiftClick", "removes"]] },
  {
    id: "terrain",
    key: "T",
    icon: "terrain",
    pairs: [
      ["drag", "paints"],
      ["shiftDrag", "takesAway"],
    ],
  },
  {
    id: "cliff",
    key: "K",
    icon: "cliff",
    pairs: [
      ["drag", "raises"],
      ["shiftDrag", "lowers"],
    ],
  },
];

const keysOf = (keys: Pair[0]) => (keys === "Z" || keys === "X" || keys === "1/2" ? keys : keyWord(keys));

/** A paint tool whose words are looked up each time they are read: the module loads before i18next has a language. */
function paintToolSpec(def: PaintToolDef): PaintToolSpec {
  return {
    id: def.id,
    key: def.key,
    icon: def.icon,
    get label() {
      return t(`workbench:paintTools.${def.id}.label`);
    },
    get hint() {
      return t(`workbench:paintTools.${def.id}.hint`);
    },
    get statusHints() {
      return def.pairs.map(([keys, does]) => ({ keys: keysOf(keys), does: t(`workbench:paintTools.does.${does}`) }));
    },
  };
}

export const PAINT_TOOLS: PaintToolSpec[] = DEFS.map(paintToolSpec);

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

/**
 * The status line of the active sub-tool; the ramp names its direction and steps („Klick setzt Rampe nach Osten, 1 Stufe“), the terrain
 * and cliff brushes the active edge set and the cliff's steps („Ziehen hebt um 2 Stufen mit dem Rand „Weg““), without a set they ask for one.
 */
export function paintStatusHints(state: PaintOptions & { sub: PaintSub }): { keys: string; does: string }[] {
  const spec = PAINT_TOOLS.find((t) => t.id === state.sub) ?? PAINT_TOOLS[0]!;
  if (spec.id === "terrain" || spec.id === "cliff") {
    const set = state.edgeSet;
    const steps = t("workbench:paintTools.steps", { count: state.cliffRise });
    const does =
      spec.id === "terrain"
        ? set === null
          ? t("workbench:paintTools.status.terrainNoSet")
          : t("workbench:paintTools.status.terrain", { set })
        : set === null
          ? t("workbench:paintTools.status.cliffNoSet", { steps })
          : t("workbench:paintTools.status.cliff", { steps, set });
    return [{ keys: keyWord("drag"), does }, ...spec.statusHints.slice(1)];
  }
  if (spec.id !== "ramp") return spec.statusHints;
  const steps = t("workbench:paintTools.steps", { count: state.ramp.steps });
  return [{ keys: keyWord("click"), does: t("workbench:paintTools.status.ramp", { dir: t(`workbench:paintTools.dir.${state.ramp.dir}`), steps }) }, ...spec.statusHints.slice(1)];
}
