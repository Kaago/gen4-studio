import { Fragment, useRef } from "react";
import type { PointerEvent as ReactPointerEvent, ReactNode } from "react";
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
  /** Replaces the content (the wizard's step bar, 3c.2); the grip stays. */
  wizard?: ReactNode;
};

/** The menu lists this many of the newest undo steps. */
const MENU_STEPS = 10;
const DEPTHS = [
  { value: "bauen", label: "Bauen", icon: "build" },
  { value: "tuefteln", label: "Tüfteln", icon: "tinker" },
] as const;

/** Whether a press started on a control (a crumb, a button, the depth switch): it is theirs, not the start of a drag. */
const onControl = (target: EventTarget | null) => target instanceof Element && target.closest("button, [role='group']") !== null && target.closest("[data-hinge-grip]") === null;

/**
 * The 44 px bar between the view and the hand: where you are (breadcrumb), how deep you work, whether the project is sound
 * (problem light), what you did (history, redo) and "Hier spielen". Dragging it (or its grip) sets the height of the hand; a double
 * click on the grip collapses the hand to the bar. It holds no state of its own beyond the drag; everything comes in as props.
 */
export function Hinge({ crumbs, depth, onDepth, problems, onProblems, history, onUndoTo, onRedo, onPlay, playing, canPlay, onDragHeight, onToggleCollapse, wizard }: HingeProps) {
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
  const historyItems: MenuEntry[] =
    recent.length === 0
      ? [{ label: "Noch nichts zurückzunehmen", disabled: true, onSelect: () => {} }]
      : recent.map((label, i) => ({ label: label === "" ? "Änderung" : label, onSelect: () => onUndoTo(i + 1) }));
  const counts = countsText(problems);
  const lightKind = problems.errors > 0 ? styles.error : problems.warnings > 0 ? styles.warning : styles.sound;

  return (
    <div className={styles.hinge} role="toolbar" aria-label="Scharnier" onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={endDrag} onPointerCancel={endDrag}>
      <button
        type="button"
        className={styles.grip}
        data-hinge-grip=""
        aria-label="Hand ein- oder ausklappen (Ziehen ändert die Höhe)"
        onDoubleClick={onToggleCollapse}
        // Enter and Space arrive as a click without a click count; a mouse click is the start of a drag or half a double click.
        onClick={(e) => {
          if (e.detail === 0) onToggleCollapse();
        }}
      >
        <Icon name="grip" />
      </button>
      {wizard !== undefined ? (
        <div className={styles.wizard}>{wizard}</div>
      ) : (
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
                      {crumb.onClick ? (
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
            <button type="button" className={`${styles.light} ${lightKind}`} onClick={onProblems}>
              <span className={styles.dot} aria-hidden="true" />
              {counts === "" ? "Alles stimmig" : counts}
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
