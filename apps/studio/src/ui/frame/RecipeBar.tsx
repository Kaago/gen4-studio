import { memo, useRef } from "react";
import type { RecipeEntry } from "../../workbench/recipes";
import { RecipeButton, useRovingList } from "../primitives";
import type { Density } from "../tokens/theme";
import type { ToolbarAnchor } from "./shellState";
import styles from "./RecipeBar.module.css";

export interface RecipeBarProps {
  entries: readonly RecipeEntry[];
  onRecipe: (id: RecipeEntry["id"]) => void;
  density: Density;
  /** Where the bar shows, as the toolbar it stands in for: upright at the left, flat at the top or at the bottom. */
  anchor: ToolbarAnchor;
}

/**
 * The recipes of Erzählen as a bar floating over the 3D view, where Bauen has its tools, in one row (or column) like them. Upright (at the
 * left) a recipe that runs is a key with symbol and name; flat (at the top or the bottom, where it shares the row with the layer bar and
 * five names would run past the view, UI test 3d.5) every key shows only its symbol, the name in the tooltip and as the accessible name.
 * A locked one is its symbol with the lock (name and reason in the tooltip, the accessible name and the description). It takes the
 * toolbar's anchor; moving it is the toolbar's business (in Bauen).
 */
export const RecipeBar = memo(function RecipeBar({ entries, onRecipe, density, anchor }: RecipeBarProps) {
  const bar = useRef<HTMLDivElement>(null);
  const vertical = anchor === "left";
  useRovingList(bar, { orientation: vertical ? "vertical" : "horizontal" });
  return (
    <div ref={bar} role="toolbar" aria-label="Rezepte" aria-orientation={vertical ? "vertical" : "horizontal"} data-anchor={anchor} data-dichte={density} className={styles.bar}>
      {entries.map((r) => (
        <RecipeButton key={r.id} id={r.id} label={r.label} icon={r.icon} area={r.area} hint={r.hint} reason={r.reason ?? null} iconOnly={!vertical || r.reason !== undefined} onClick={() => onRecipe(r.id)} />
      ))}
    </div>
  );
});
