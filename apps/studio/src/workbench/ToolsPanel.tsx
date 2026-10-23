import { Fragment, useId, useRef } from "react";
import type { Tool } from "../ui/frame/shellState";
import type { Density } from "../ui/tokens/theme";
import { Icon, Kbd, PanelHead, Tooltip, useRovingList } from "../ui/primitives";
import { TOOLS, isEventTool } from "./tools";
import styles from "./ToolsPanel.module.css";

export interface ToolsPanelProps {
  tool: Tool;
  onTool: (tool: Tool) => void;
  /** Why the four event tools cannot be used (the locked file's reason, or that no place is shown); null: they can. */
  eventsBlocked: string | null;
  /** In "kompakt" the buttons show their symbol only (the name stays the accessible name and the tooltip). */
  density: Density;
}

/** The palette of the hand: one button per tool with symbol, name and letter, exactly one pressed, and under it what the active tool does. */
export function ToolsPanel({ tool, onTool, eventsBlocked, density }: ToolsPanelProps) {
  const palette = useRef<HTMLDivElement>(null);
  useRovingList(palette, { orientation: "horizontal" });
  const uid = useId();
  const compact = density === "kompakt";
  const active = TOOLS.find((t) => t.id === tool);
  return (
    <section className={styles.panel} aria-label="Werkzeuge">
      <PanelHead title="Werkzeuge" />
      <div className={styles.body}>
        <div ref={palette} role="toolbar" aria-label="Werkzeugleiste" className={styles.palette} data-dichte={density}>
          {TOOLS.map((t) => {
            // A locked tool stays in the tab order and says why (as tooltip and as description), but does nothing.
            const blocked = isEventTool(t.id) ? eventsBlocked : null;
            const reasonId = `${uid}-${t.id}`;
            const button = (
              <button
                type="button"
                className={styles.tool}
                data-roving=""
                data-tool={t.id}
                aria-label={t.label}
                aria-pressed={tool === t.id}
                aria-disabled={blocked ? true : undefined}
                aria-describedby={blocked ? reasonId : undefined}
                aria-keyshortcuts={t.key}
                onClick={blocked ? undefined : () => onTool(t.id)}
              >
                <Icon name={t.icon} weight={tool === t.id ? "fill" : "regular"} />
                {!compact && <span className={styles.name}>{t.label}</span>}
                <span className={styles.key} aria-hidden="true">
                  {t.key}
                </span>
              </button>
            );
            return (
              <Fragment key={t.id}>
                <Tooltip
                  content={
                    blocked ?? (
                      <>
                        {t.label} <Kbd keys={t.key} />
                      </>
                    )
                  }
                >
                  {button}
                </Tooltip>
                {blocked && (
                  <span id={reasonId} hidden>
                    {blocked}
                  </span>
                )}
              </Fragment>
            );
          })}
        </div>
        {active && <p className={styles.hint}>{active.hint}</p>}
      </div>
    </section>
  );
}
