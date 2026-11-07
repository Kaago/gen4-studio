import { Fragment } from "react";
import type { Tool } from "../ui/frame/shellState";
import { Kbd } from "../ui/primitives";
import { usePaint } from "../workbench/paint/PaintContext";
import { PAINT_TOOLS, paintStatusHints } from "../workbench/paintTools";
import { TOOLS } from "../workbench/tools";
import styles from "./ToolHints.module.css";

/**
 * The left of the status line: the active tool and what its keys do ("Auswählen · Klick wählt · Ziehen verschiebt"). `placing` names the
 * trainer the figure tool places („In der Welt platzieren“): "Figur · Klick setzt Wanderer Jens · Esc hebt die Bindung auf". While
 * „Gelände malen“ paints a world map (`usePaint().target`) it names the sub-tool and its keys: "Gelände malen · Pinsel · Ziehen malt · …".
 */
export function ToolHints({ tool, placing = null }: { tool: Tool; placing?: string | null }) {
  const spec = TOOLS.find((t) => t.id === tool);
  const paint = usePaint();
  if (!spec) return null;
  const sub = tool === "paint" && paint.target !== null ? PAINT_TOOLS.find((t) => t.id === paint.state.sub) : undefined;
  const hints = sub
    ? paintStatusHints(paint.state)
    : tool === "figure" && placing !== null
      ? [
          { keys: "Klick", does: `setzt ${placing}` },
          { keys: "Esc", does: "hebt die Bindung auf" },
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
    </span>
  );
}
