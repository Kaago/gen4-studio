import { Fragment, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { HistoryDto } from "../../api";
import { inTauri } from "../../platform/transport";
import type { Crumb } from "../../workbench/breadcrumb";
import { countsText } from "../../workbench/problems/problems";
import type { ProblemCounts } from "../../workbench/problems/problems";
import { Button, Icon, IconButton, Menu, Segmented } from "../primitives";
import type { MenuEntry } from "../primitives";
import { clampHand } from "./shellState";
import type { Depth } from "./shellState";
import styles from "./Hinge.module.css";

export type HingeProps = {
  crumbs: Crumb[];
  depth: Depth;
  onDepth: (depth: Depth) => void;
  problems: ProblemCounts;
  /** While a pass of the checks runs: how far it got („Prüft … Texte 312/724“); the light turns. Null or absent: none runs. */
  checking?: string | null;
  /**
   * The checks have no result for the open project and none runs (the first run failed or answered without a report): the light says
   * „Nicht geprüft“ where it would say „Alles stimmig“, since nothing is known.
   */
  unchecked?: boolean;
  /** Brings the problems list to the front. */
  onProblems: () => void;
  history: HistoryDto;
  /** Goes back this many steps (1 is the newest). */
  onUndoTo: (stepsBack: number) => void;
  onRedo: () => void;
  onPlay: () => void;
  /** The build (and run) is under way. */
  playing: boolean;
  canPlay: boolean;
  /** The new height of the hand while the grip is dragged, already inside 200..560. */
  onDragHeight: (height: number) => void;
  onToggleCollapse: () => void;
  /**
   * The hinge where wizards show (the world's): it keeps a slot for the wizard's step bar (`data-wizard-bar`, filled through a portal).
   * True while a wizard is open: the slot replaces the content, the grip stays but does not collapse the hand. Undefined: no slot.
   */
  wizard?: boolean;
};

/** The menu lists this many of the newest undo steps. */
const MENU_STEPS = 10;
const DEPTHS = [
  { value: "bauen", label: "Bauen", icon: "build" },
  { value: "tuefteln", label: "Tüfteln", icon: "tinker" },
] as const;

/** The light's colour: the worst the checks and the other problems found, or sound; unchecked: nothing is known (no green). */
type LightKind = "error" | "warning" | "info" | "sound" | "unchecked";

/** The three counters of the light, each with its symbol (never colour alone). */
const COUNTERS = [
  { key: "errors", icon: "error" },
  { key: "warnings", icon: "warning" },
  { key: "infos", icon: "info" },
] as const;

/** Whether a press started on a control (a crumb, a button, the depth switch): it is theirs, not the start of a drag. */
const onControl = (target: EventTarget | null) => target instanceof Element && target.closest("button, [role='group']") !== null && target.closest("[data-hinge-grip]") === null;

/**
 * The 44 px bar between the view and the hand: where you are (breadcrumb), how deep you work, whether the project is sound
 * (problem light: errors, warnings and infos counted apart, a spinner with the progress while the checks run), what you did (history, redo)
 * and "Hier spielen". Dragging it (or its grip) sets the height of the hand; a double
 * click on the grip collapses the hand to the bar. It holds no state of its own beyond the drag; everything comes in as props.
 */
export function Hinge({ crumbs, depth, onDepth, problems, checking, unchecked = false, onProblems, history, onUndoTo, onRedo, onPlay, playing, canPlay, onDragHeight, onToggleCollapse, wizard }: HingeProps) {
  const drag = useRef<{ pointer: number; startY: number; startHeight: number; captured: Element } | null>(null);

  function onPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    const bar = e.currentTarget;
    // React delivers the events of a portal (the history menu) to its parent in the tree: those are not presses on the bar.
    if (e.button !== 0 || onControl(e.target) || !bar.contains(e.target as Node)) return;
    // The element that was pressed keeps the pointer (not the bar): a click is delivered to the capturing element, and the grip's
    // double click must still reach the grip.
    const captured = e.target instanceof Element ? e.target : bar;
    // The hand reaches from the bar's lower edge to the bottom of the deck that holds both.
    const deckBottom = bar.parentElement?.getBoundingClientRect().bottom ?? 0;
    drag.current = { pointer: e.pointerId, startY: e.clientY, startHeight: deckBottom - bar.getBoundingClientRect().bottom, captured };
    captured.setPointerCapture?.(e.pointerId);
  }
  function onPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    onDragHeight(clampHand(d.startHeight + d.startY - e.clientY));
  }
  function endDrag(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    drag.current = null;
    d.captured.releasePointerCapture?.(e.pointerId);
  }

  const undoSteps = history.undo.length;
  const recent = history.undo.slice(-MENU_STEPS).reverse();
  // Who made each of them, aligned with `recent` (ignored unless aligned with `undo`): an agent's step shows its symbol and the client.
  const agents = history.undo_agents?.length === undoSteps ? history.undo_agents.slice(-MENU_STEPS).reverse() : [];
  const historyItems: MenuEntry[] =
    recent.length === 0
      ? [{ label: "Noch nichts zurückzunehmen", disabled: true, onSelect: () => {} }]
      : recent.map((label, i) => {
          const step = { label: label === "" ? "Änderung" : label, onSelect: () => onUndoTo(i + 1) };
          const agent = agents[i] ?? null;
          return agent === null ? step : { ...step, icon: "agent" as const, detail: agent };
        });
  const counts = countsText(problems);
  const kind: LightKind = problems.errors > 0 ? "error" : problems.warnings > 0 ? "warning" : problems.infos > 0 ? "info" : unchecked ? "unchecked" : "sound";
  const running = checking !== undefined && checking !== null;
  const calm = unchecked ? "Nicht geprüft" : "Alles stimmig";
  const lightLabel = running ? [checking, counts].filter((part) => part !== "").join(", ") : counts === "" ? calm : unchecked ? `${counts}, nicht geprüft` : counts;

  // The hand holds the wizard's step: it does not collapse under it.
  const toggleCollapse = wizard === true ? undefined : onToggleCollapse;

  return (
    <div className={wizard === true ? `${styles.hinge} ${styles.wizardMode}` : styles.hinge} role="toolbar" aria-label="Scharnier" onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={endDrag} onPointerCancel={endDrag}>
      <button
        type="button"
        className={styles.grip}
        data-hinge-grip=""
        aria-label="Hand ein- oder ausklappen (Ziehen ändert die Höhe)"
        onDoubleClick={toggleCollapse}
        // Enter and Space arrive as a click without a click count; a mouse click is the start of a drag or half a double click.
        onClick={(e) => {
          if (e.detail === 0) toggleCollapse?.();
        }}
      >
        <Icon name="grip" />
      </button>
      {wizard !== undefined && <div className={styles.wizard} data-wizard-bar="" hidden={!wizard} />}
      {wizard !== true && (
        <>
          <nav className={styles.crumbs} aria-label="Brotkrume">
            <ol>
              {crumbs.map((crumb, i) => {
                const last = i === crumbs.length - 1;
                const inner = (
                  <>
                    {crumb.label}
                    {crumb.tech !== undefined && <span className={styles.tech}> {crumb.tech}</span>}
                  </>
                );
                return (
                  <Fragment key={`${i}-${crumb.label}`}>
                    <li className={last ? styles.last : styles.crumb} title={crumb.tech === undefined ? crumb.label : `${crumb.label} ${crumb.tech}`} aria-current={last ? "location" : undefined}>
                      {crumb.menu ? (
                        <Menu
                          trigger={
                            <>
                              {inner}
                              <Icon name="caretDown" />
                            </>
                          }
                          items={crumb.menu}
                        />
                      ) : crumb.onClick ? (
                        <button type="button" className={styles.crumbButton} onClick={crumb.onClick}>
                          {inner}
                        </button>
                      ) : (
                        inner
                      )}
                    </li>
                    {!last && (
                      <li className={styles.separator} aria-hidden="true">
                        ›
                      </li>
                    )}
                  </Fragment>
                );
              })}
            </ol>
          </nav>
          <div className={styles.depth}>
            <Segmented options={DEPTHS} value={depth} onChange={onDepth} ariaLabel="Tiefe" />
          </div>
          <div className={styles.right}>
            <button type="button" className={`${styles.light} ${styles[kind]}`} data-light={kind} data-running={running ? "" : undefined} aria-label={lightLabel} onClick={onProblems}>
              <span className={running ? styles.spinner : styles.dot} aria-hidden="true" />
              {running && <span className={styles.progress}>{checking}</span>}
              {counts === "" ? (
                !running && calm
              ) : (
                <>
                  {COUNTERS.map(({ key, icon }) => (
                    <span key={key} className={problems[key] === 0 ? `${styles.count} ${styles.zero}` : styles.count} data-count={icon}>
                      <Icon name={icon} weight="fill" />
                      {problems[key]}
                    </span>
                  ))}
                </>
              )}
            </button>
            <Menu
              label={`Verlauf, ${undoSteps} ${undoSteps === 1 ? "Schritt" : "Schritte"}`}
              trigger={
                <>
                  <Icon name="history" />
                  {undoSteps}
                </>
              }
              items={historyItems}
            />
            <IconButton label="Wiederholen" icon="redo" shortcut="⇧⌘Z" disabled={history.redo.length === 0} onClick={onRedo} />
            <Button kind="play" icon="play" shortcut={inTauri() && !playing ? "⌘R" : undefined} disabled={!canPlay || playing} onClick={onPlay}>
              {playing ? "Baut …" : "Hier spielen"}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
