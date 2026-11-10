import { Fragment } from "react";
import { useTranslation } from "react-i18next";
import { t } from "../i18n";
import type { WorldGetDto } from "../api.world";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth, Tool } from "../ui/frame/shellState";
import { Kbd } from "../ui/primitives";
import type { SceneHover } from "../workbench/Inspector";
import { usePaint } from "../workbench/paint/PaintContext";
import { PAINT_TOOLS, paintStatusHints } from "../workbench/paintTools";
import { TOOLS } from "../workbench/tools";
import styles from "./ToolHints.module.css";

/**
 * The left of the status line: the active tool and what its keys do („Auswählen · Klick wählt · Ziehen verschiebt“). `placing` names the
 * trainer the figure tool places („In der Welt platzieren“): "Figur · Klick setzt Wanderer Jens · Esc hebt die Bindung auf". While
 * „Gelände malen“ paints a world map (`usePaint().target`) it names the sub-tool and its keys: „Gelände malen · Pinsel · Ziehen malt · …“;
 * Tüfteln adds the tokens of the hovered cell of that map („Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“).
 */
export function ToolHints({ tool, placing = null, hover = null, depth = "bauen" }: { tool: Tool; placing?: string | null; hover?: SceneHover | null; depth?: Depth }) {
  const { t: tr } = useTranslation("app");
  const spec = TOOLS.find((t) => t.id === tool);
  const paint = usePaint();
  if (!spec) return null;
  const sub = tool === "paint" && paint.target !== null ? PAINT_TOOLS.find((t) => t.id === paint.state.sub) : undefined;
  const tokens = sub && depthAtLeast(depth, "tuefteln") && hover?.kind === "tile" && hover.map === paint.target ? cellTokens(paint.world ?? null, paint.state.layer, hover.x, hover.y) : null;
  const hints = sub
    ? paintStatusHints(paint.state)
    : tool === "figure" && placing !== null
      ? [
          { keys: tr("toolHints.click"), does: tr("toolHints.places", { name: placing }) },
          { keys: "Esc", does: tr("toolHints.unbind") },
        ]
      : spec.statusHints;
  return (
    <span className={styles.hints}>
      <strong className={styles.tool}>{spec.label}</strong>
      {sub && (
        <>
          <span aria-hidden="true">·</span>
          <span className={styles.tool}>{sub.label}</span>
        </>
      )}
      {hints.map((hint) => (
        <Fragment key={hint.keys}>
          <span aria-hidden="true">·</span>
          <span className={styles.hint}>
            <Kbd keys={hint.keys} /> {hint.does}
          </span>
        </Fragment>
      ))}
      {tokens && (
        <>
          <span aria-hidden="true">·</span>
          <span className={styles.hint} data-paint-tokens="">
            {tokens}
          </span>
        </>
      )}
    </span>
  );
}

/** World units per height step of the world document. */
const UNITS_PER_STEP = 8;

/** The tokens of cell (`x`, `z`) of `world`: the tile of `layer`, the layer, the height step and its units; null without a world. */
function cellTokens(world: WorldGetDto | null, layer: number, x: number, z: number): string | null {
  const doc = world?.world;
  if (!doc) return null;
  const tile = doc.layers[layer]?.tiles[z]?.split(" ")[x] ?? ".";
  const step = Number.parseInt(doc.layers[0]?.heights?.[z]?.split(" ")[x] ?? "0", 10) || 0;
  return t("app:toolHints.cell", { tile, layer, step, units: step * UNITS_PER_STEP });
}
