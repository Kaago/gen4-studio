import { Fragment } from "react";
import type { Tool } from "../ui/frame/shellState";
import { Kbd } from "../ui/primitives";
import { TOOLS } from "../workbench/tools";
import styles from "./ToolHints.module.css";

/** The left of the status line: the active tool and what its keys do ("Auswählen · Klick wählt · Ziehen verschiebt"). */
export function ToolHints({ tool }: { tool: Tool }) {
  const spec = TOOLS.find((t) => t.id === tool);
  if (!spec) return null;
  return (
    <span className={styles.hints}>
      <strong className={styles.tool}>{spec.label}</strong>
      {spec.statusHints.map((hint) => (
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
