import type { NextStep, RecipeId } from "../api";
import { t } from "../i18n";
import type { Tool } from "../ui/frame/shellState";
import type { Area, IconName } from "../ui/primitives";
import type { PaintFrame, PaintPatch } from "./paint/PaintContext";
import type { NavTarget } from "./navigation";
import type { WizardRequest } from "./WorkbenchContext";

/** A recipe of the bar that Erzählen shows instead of the tools. */
export interface RecipeEntry {
  id: "create_area" | "create_room" | "new_figure" | "connect_doors" | "new_trainer_here";
  label: string;
  icon: IconName;
  area: Area;
  hint: string;
  /** Absent: the recipe runs; set: the button is locked with it. */
  reason?: string;
}

/** A recipe of the bar whose label and hint (`recipeBar.<id>.*`) are looked up each time they are read: the module loads before i18next has a language. */
function barEntry(id: RecipeEntry["id"], icon: IconName, area: Area): RecipeEntry {
  return {
    id,
    icon,
    area,
    get label() {
      return t(`workbench:recipeBar.${id}.label`);
    },
    get hint() {
      return t(`workbench:recipeBar.${id}.hint`);
    },
  };
}

/** The recipes of Erzählen: each opens its wizard (since 3d.5 none is locked). */
export const RECIPE_BAR: readonly RecipeEntry[] = [
  barEntry("create_area", "matrix", "welt"),
  barEntry("create_room", "room", "welt"),
  barEntry("new_figure", "figure", "welt"),
  barEntry("connect_doors", "door", "welt"),
  barEntry("new_trainer_here", "pokemon", "pokemon"),
];

/** A next step of the table below; its label is `recipeBar.next.<id>`. */
type StepId = "connect_doors" | "new_figure" | "adjust_terrain" | "create_area" | "edit_dialog" | "story_join" | "play_here" | "talk_advances" | "figure_appears" | "scene_on_enter";

/**
 * The steps after each recipe, as the kernel's `next_steps` names them (`crates/g4-kernel/src/recipes/{area,room,extend,empty_region,
 * new_figure,connect_doors,new_trainer_here,new_story,talk_advances,figure_appears,scene_on_enter,reward_once,new_painted_map}.rs`). The wizards of 3d.5 show the kernel's own `outcome.next`; this table is for the wizards
 * that still call the old commands and for the tests that hold both together.
 */
const NEXT: Partial<Record<RecipeId, StepId[]>> = {
  create_area: ["connect_doors", "new_figure"],
  create_room: ["new_figure"],
  extend_area: ["adjust_terrain"],
  empty_region: ["create_area"],
  new_figure: ["edit_dialog", "story_join"],
  connect_doors: ["play_here"],
  new_trainer_here: ["play_here"],
  new_story: ["talk_advances", "figure_appears"],
  talk_advances: ["figure_appears", "scene_on_enter"],
  figure_appears: ["play_here"],
  scene_on_enter: ["play_here"],
  reward_once: ["play_here"],
  new_painted_map: ["adjust_terrain"],
};

/** The recipes the kernel has (`recipes::list()`): a step to one of them runs. */
const KNOWN: readonly string[] = [
  "create_area",
  "create_room",
  "extend_area",
  "empty_region",
  "new_figure",
  "connect_doors",
  "new_trainer_here",
  "new_story",
  "talk_advances",
  "figure_appears",
  "scene_on_enter",
  "reward_once",
  "new_painted_map",
] satisfies RecipeId[];

/** Steps that are no recipe but always there (the app runs them): the dialog editor of the figure just made, „Zu einer Geschichte“
 * („Gespräch schaltet weiter“ with that figure) and „Gelände anpassen“ (painting the new map, `startPaintingWith`). */
const ALWAYS: readonly string[] = ["edit_dialog", "story_join", "adjust_terrain"];

/** Why a step the kernel does not run yet is locked (`types.rs` `next_steps`): playing comes with 3d.9, any other step without a recipe later. */
function lockReason(id: string): string {
  return id === "play_here" ? t("workbench:recipeBar.comesWith3d9") : t("workbench:recipeBar.comesLater");
}

/** The next steps a wizard shows while it still calls the old command (the kernel's table). */
export function nextStepsFor(recipe: RecipeId): NextStep[] {
  return (NEXT[recipe] ?? []).map((id) => {
    const available = KNOWN.includes(id) || ALWAYS.includes(id);
    return { id, label: t(`workbench:recipeBar.next.${id}`), available, reason: available ? null : lockReason(id) };
  });
}

/**
 * The wizard a recipe (from the bar or a result's next step) opens; null: it has none (`edit_dialog` is the app's, not a wizard).
 * „Raum anlegen“ puts the door outside into `header` (the selected area or room, or the one a wizard just made); „Neue Figur“ and „Neuer
 * Trainer hier“ stand in `header`, door A of „Tür verbinden“ lies there; `tile` (a tile of the header's matrix) is where they start.
 */
export function recipeRequest(id: string, header: number | null, tile?: { x: number; z: number } | null): WizardRequest | null {
  const at = tile ? { tile: { x: tile.x, z: tile.z } } : {};
  if (id === "create_area") return { kind: "area" };
  if (id === "create_room") return header === null ? { kind: "room" } : { kind: "room", prefill: { outsideHeader: header } };
  if (id === "new_figure") return header === null ? { kind: "new_figure" } : { kind: "new_figure", prefill: { header, ...at } };
  if (id === "new_trainer_here") return header === null ? { kind: "new_trainer_here" } : { kind: "new_trainer_here", prefill: { header, ...at } };
  if (id === "connect_doors") return header === null ? { kind: "connect_doors" } : { kind: "connect_doors", prefill: { a: { header, ...at } } };
  return null;
}

/** What the app does to start painting: follow a link, switch the top view, the tool and the paint state, frame the cell. */
export interface PaintingStart {
  goTo: (target: NavTarget) => void;
  setTopDown: (on: boolean) => void;
  onTool: (tool: Tool) => void;
  setPaint: (patch: PaintPatch) => void;
  /** Asks the scene to frame `cell` top-down once its map is the paint target (`PaintModel.requestFrame`). */
  frame: (cell: PaintFrame) => void;
}

/**
 * „Gelände anpassen“ (after „Neue Karte malen“, or as the step of a result): selects the map in `cell` (the matrix of `cell` stays the
 * view: the world map for a map outside, the room plan's own matrix for a room),
 * turns the top view on and frames the cell in it (the scene does so once the refreshed matrix holds the map and the map is the paint
 * target, also when the top view was on already), switches to „Gelände malen“ (which shows the terrain layer) and paints with the brush.
 * The top view stays the user's choice afterwards; choosing the tool alone never switches the camera.
 */
export function startPaintingWith(app: PaintingStart, cell: { matrix: number; x: number; y: number }): void {
  app.goTo({ kind: "matrixCell", matrix: cell.matrix, x: cell.x, y: cell.y, show: "map" });
  app.setTopDown(true);
  app.frame({ matrix: cell.matrix, x: cell.x, y: cell.y });
  app.onTool("paint");
  app.setPaint({ sub: "brush" });
}
