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

/** Why a recipe of the bar is locked: it comes with phase 3d.5 (the kernel's word for every recipe it does not know yet). */
const COMES_WITH_3D5 = "Kommt mit 3d.5";

/** The recipes of Erzählen: area and room open their wizards, the others wait for 3d.5. */
export const RECIPE_BAR: readonly RecipeEntry[] = [
  { id: "create_area", label: "Gebiet anlegen", icon: "matrix", area: "welt", hint: "Kopiert Karten zu einem neuen Gebiet mit eigenem Ort." },
  { id: "create_room", label: "Raum anlegen", icon: "room", area: "welt", hint: "Kopiert einen Raum und verbindet ihn mit einer Tür nach draußen." },
  { id: "new_figure", label: "Neue Figur", icon: "figure", area: "welt", hint: "Stellt eine Figur mit Aussehen und Text an eine Stelle.", reason: COMES_WITH_3D5 },
  { id: "connect_doors", label: "Tür verbinden", icon: "door", area: "welt", hint: "Verbindet zwei Türen in beide Richtungen.", reason: COMES_WITH_3D5 },
  { id: "new_trainer_here", label: "Neuer Trainer hier", icon: "pokemon", area: "pokemon", hint: "Stellt einen Trainer mit Team an eine Stelle.", reason: COMES_WITH_3D5 },
];

/** The steps after each recipe, as the kernel's `next_steps` names them (`crates/g4-kernel/src/recipes/{area,room,extend,empty_region}.rs`). */
const NEXT: Record<RecipeId, [id: string, label: string][]> = {
  create_area: [
    ["connect_doors", "Tür verbinden"],
    ["new_figure", "Neue Figur hier"],
  ],
  create_room: [["new_figure", "Neue Figur hier"]],
  extend_area: [["adjust_terrain", "Gelände anpassen"]],
  empty_region: [["create_area", "Gebiet anlegen"]],
};

/** The recipes the kernel has (`recipes::list()`): a step to one of them runs. */
const KNOWN: readonly string[] = ["create_area", "create_room", "extend_area", "empty_region"] satisfies RecipeId[];

/** The next steps a wizard shows while it still calls the old command (the kernel's table). */
export function nextStepsFor(recipe: RecipeId): NextStep[] {
  return NEXT[recipe].map(([id, label]) => {
    const available = KNOWN.includes(id);
    return { id, label, available, reason: available ? null : id === "adjust_terrain" ? "Kommt mit Phase 6" : COMES_WITH_3D5 };
  });
}

/**
 * The wizard a recipe (from the bar or a result's next step) opens; null: it has none yet. „Raum anlegen“ puts the door outside into
 * `header` (the selected area or room, or the one a wizard just made).
 */
export function recipeRequest(id: string, header: number | null): WizardRequest | null {
  if (id === "create_area") return { kind: "area" };
  if (id === "create_room") return header === null ? { kind: "room" } : { kind: "room", prefill: { outsideHeader: header } };
  return null;
}
