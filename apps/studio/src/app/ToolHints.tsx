import { Fragment } from "react";
import type { Tool } from "../ui/frame/shellState";
import { Kbd } from "../ui/primitives";
import { TOOLS } from "../workbench/tools";
import styles from "./ToolHints.module.css";

/**
 * The left of the status line: the active tool and what its keys do ("Auswählen · Klick wählt · Ziehen verschiebt"). `placing` names the
 * trainer the figure tool places („In der Welt platzieren“): "Figur · Klick setzt Wanderer Jens · Esc hebt die Bindung auf".
 */
export function ToolHints({ tool, placing = null }: { tool: Tool; placing?: string | null }) {
  const spec = TOOLS.find((t) => t.id === tool);
  if (!spec) return null;
  const hints =
    tool === "figure" && placing !== null
      ? [
          { keys: "Klick", does: `setzt ${placing}` },
          { keys: "Esc", does: "hebt die Bindung auf" },
        ]
      : spec.statusHints;
  return (
    <span className={styles.hints}>
      <strong className={styles.tool}>{spec.label}</strong>
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
