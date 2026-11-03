import type { NextStep, RecipeId } from "../api";
import type { Area, IconName } from "../ui/primitives";
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

/** Why a next step is locked, as the kernel's `next_steps` says it: playing comes with 3d.9, any other step without a recipe later. */
const COMES_LATER = "Kommt später";
const COMES_WITH_3D9 = "Kommt mit 3d.9";

/** The recipes of Erzählen: each opens its wizard (since 3d.5 none is locked). */
export const RECIPE_BAR: readonly RecipeEntry[] = [
  { id: "create_area", label: "Gebiet anlegen", icon: "matrix", area: "welt", hint: "Kopiert Karten zu einem neuen Gebiet mit eigenem Ort." },
  { id: "create_room", label: "Raum anlegen", icon: "room", area: "welt", hint: "Kopiert einen Raum und verbindet ihn mit einer Tür nach draußen." },
  { id: "new_figure", label: "Neue Figur", icon: "figure", area: "welt", hint: "Stellt eine Figur mit Aussehen und Text an eine Stelle." },
  { id: "connect_doors", label: "Tür verbinden", icon: "door", area: "welt", hint: "Verbindet zwei Türen in beide Richtungen." },
  { id: "new_trainer_here", label: "Neuer Trainer hier", icon: "pokemon", area: "pokemon", hint: "Stellt einen Trainer mit Team an eine Stelle." },
];

/**
 * The steps after each recipe, as the kernel's `next_steps` names them (`crates/g4-kernel/src/recipes/{area,room,extend,empty_region,
 * new_figure,connect_doors,new_trainer_here,new_story,talk_advances,figure_appears,scene_on_enter,reward_once}.rs`). The wizards of 3d.5 show the kernel's own `outcome.next`; this table is for the wizards
 * that still call the old commands and for the tests that hold both together.
 */
const NEXT: Partial<Record<RecipeId, [id: string, label: string][]>> = {
  create_area: [
    ["connect_doors", "Tür verbinden"],
    ["new_figure", "Neue Figur hier"],
  ],
  create_room: [["new_figure", "Neue Figur hier"]],
  extend_area: [["adjust_terrain", "Gelände anpassen"]],
  empty_region: [["create_area", "Gebiet anlegen"]],
  new_figure: [
    ["edit_dialog", "Dialog bearbeiten"],
    ["story_join", "Zu einer Geschichte"],
  ],
  connect_doors: [["play_here", "Hier spielen"]],
  new_trainer_here: [["play_here", "Hier spielen"]],
  new_story: [
    ["talk_advances", "Gespräch schaltet weiter"],
    ["figure_appears", "Figur erscheint ab Schritt"],
  ],
  talk_advances: [
    ["figure_appears", "Figur erscheint ab Schritt"],
    ["scene_on_enter", "Szene beim Betreten"],
  ],
  figure_appears: [["play_here", "Hier spielen"]],
  scene_on_enter: [["play_here", "Hier spielen"]],
  reward_once: [["play_here", "Hier spielen"]],
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
] satisfies RecipeId[];

/** Steps that are no recipe but always there (the app runs them): the dialog editor of the figure just made, and „Zu einer Geschichte“
 * („Gespräch schaltet weiter“ with that figure). */
const ALWAYS: readonly string[] = ["edit_dialog", "story_join"];

/** Why a step the kernel does not run yet is locked (`types.rs` `next_steps`). */
function lockReason(id: string): string {
  if (id === "adjust_terrain") return "Kommt mit Phase 6";
  return id === "play_here" ? COMES_WITH_3D9 : COMES_LATER;
}

/** The next steps a wizard shows while it still calls the old command (the kernel's table). */
export function nextStepsFor(recipe: RecipeId): NextStep[] {
  return (NEXT[recipe] ?? []).map(([id, label]) => {
    const available = KNOWN.includes(id) || ALWAYS.includes(id);
    return { id, label, available, reason: available ? null : lockReason(id) };
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
