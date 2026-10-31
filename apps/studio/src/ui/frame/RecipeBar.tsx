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
 * The recipes of Erzählen as a bar floating over the 3D view, where Bauen has its tools: one key per recipe with symbol and name, a locked
 * one says why. It takes the toolbar's anchor; moving it is the toolbar's business (in Bauen).
 */
export const RecipeBar = memo(function RecipeBar({ entries, onRecipe, density, anchor }: RecipeBarProps) {
  const bar = useRef<HTMLDivElement>(null);
  const vertical = anchor === "left";
  useRovingList(bar, { orientation: vertical ? "vertical" : "horizontal" });
  return (
    <div ref={bar} role="toolbar" aria-label="Rezepte" aria-orientation={vertical ? "vertical" : "horizontal"} data-anchor={anchor} data-dichte={density} className={styles.bar}>
      {entries.map((r) => (
        <RecipeButton key={r.id} id={r.id} label={r.label} icon={r.icon} area={r.area} hint={r.hint} reason={r.reason ?? null} onClick={() => onRecipe(r.id)} />
      ))}
    </div>
  );
});
