import { Fragment, useLayoutEffect, useRef, useState } from "react";
import type { HistoryDto } from "../../api";
import type { LiveHingeInfo } from "../../api.live";
import { inTauri } from "../../platform/transport";
import type { Crumb } from "../../workbench/breadcrumb";
import { countsText } from "../../workbench/problems/problems";
import type { ProblemCounts } from "../../workbench/problems/problems";
import { Badge, Button, Icon, IconButton, Menu, Segmented } from "../primitives";
import type { MenuEntry } from "../primitives";
import type { Depth } from "./shellState";
import styles from "./Hinge.module.css";

/** Play mode in the hinge: what `useLive` shows (`LiveHingeInfo`) and what its buttons do. */
export interface HingeLive extends LiveHingeInfo {
  /** „Weiter“ after a halt. */
  onResume: () => void;
  /** „Neu laden“: stops, builds and starts again (as „Hier spielen“). */
  onReload: () => void;
  /** „Stoppen“: disconnects, ends our melonDS, back to editing. */
  onStop: () => void;
}

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
  /** Whether the view of the dock below is maximized („Schau maximieren“ is pressed then). */
  maximized?: boolean;
  /** Maximizes the view of the dock below, or restores it. Absent: the hinge has no dock with a view, and no button for it. */
  onMaximize?: () => void;
  /**
   * The hinge where wizards show (the world's): it keeps a slot for the wizard's step bar (`data-wizard-bar`, filled through a portal).
   * True while a wizard is open: the slot replaces the content. Undefined: no slot.
   */
  wizard?: boolean;
  /**
   * Play mode („Hier spielen“ in melonDS): in any state but `off` the bar turns amber and shows the state and „Weiter“ (while halted),
   * „Neu laden“ and „Stoppen“ instead of „Hier spielen“. Null or absent: no play mode, the bar stays dark.
   */
  live?: HingeLive | null;
};

/** The menu lists this many of the newest undo steps. */
const MENU_STEPS = 10;
const DEPTHS_OPTIONS = [
  { value: "erzaehlen", label: "Erzählen", icon: "narrate" },
  { value: "bauen", label: "Bauen", icon: "build" },
  { value: "tuefteln", label: "Tüfteln", icon: "tinker" },
] as const;

/**
 * Play mode in a narrow hinge (B1): „Weiter“, „Neu laden“, „Stoppen“ and the badge never shrink. The state text gives way first (ellipsis,
 * the full text as its title); when it would fall below `TEXT_ROOM` or the bar still overflows, the other parts yield one step at a time:
 * 1 the depth shows its symbols (the names stay as accessible names and tooltips), 2 the problem light shows its dot (the counts stay in
 * its name and title), 3 „Schau maximieren“ steps aside (unless it is pressed; the palette, a double click on a tab and Esc remain).
 * History and redo always stay. The hinge measures itself (jsdom lays nothing out, so there it stays at 0).
 */
const SQUEEZE_MAX = 3;
/** Below this many pixels of a longer state text the next part yields (about two words: „Gehalten: Auslö…“). */
const TEXT_ROOM = 96;

/** The light's colour: the worst the checks and the other problems found, or sound; unchecked: nothing is known (no green). */
type LightKind = "error" | "warning" | "info" | "sound" | "unchecked";

/** The three counters of the light, each with its symbol (never colour alone). */
const COUNTERS = [
  { key: "errors", icon: "error" },
  { key: "warnings", icon: "warning" },
  { key: "infos", icon: "info" },
] as const;

/**
 * The 44 px bar above the view: where you are (breadcrumb), how deep you work, whether the project is sound
 * (problem light: errors, warnings and infos counted apart, a spinner with the progress while the checks run), what you did (history, redo),
 * „Schau maximieren“ (pressed while the view is maximized; Esc restores it too) and "Hier spielen". In play mode the bar is amber and says how
 * the game in melonDS stands, with „Weiter“, „Neu laden“ and „Stoppen“ in place of "Hier spielen". It holds no state of its own; everything
 * comes in as props. The sizes of the dock's groups are dockview's (its sashes).
 */
export function Hinge({ crumbs, depth, onDepth, problems, checking, unchecked = false, onProblems, history, onUndoTo, onRedo, onPlay, playing, canPlay, maximized = false, onMaximize, wizard, live }: HingeProps) {
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
  const play = live !== undefined && live !== null && live.state !== "off" ? live : null;
  const barRef = useRef<HTMLDivElement>(null);
  const [squeeze, setSqueeze] = useState(0);
  const [barSize, setBarSize] = useState("");
  const inPlay = play !== null;
  // The bar's own size (its parent sets it; the density changes its height): a new size starts the fit again from the full bar.
  useLayoutEffect(() => {
    const el = barRef.current;
    if (!inPlay || el === null || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => setBarSize(`${el.clientWidth}x${el.clientHeight}`));
    observer.observe(el);
    return () => observer.disconnect();
  }, [inPlay]);
  // What can change a width (the chosen depth cannot: its toggles keep their size).
  const fitKey = play ? [barSize, play.state, play.text, lightLabel, running, undoSteps, maximized, onMaximize !== undefined].join("|") : "";
  const fitted = useRef("");
  // After every render in play mode: start from the full bar when what it holds changed, else take one more step while it does not fit.
  // The steps render synchronously before the paint, so the bar never shows overflowing.
  useLayoutEffect(() => {
    if (fitted.current !== fitKey) {
      fitted.current = fitKey;
      if (squeeze !== 0) {
        setSqueeze(0);
        return;
      }
    }
    const el = barRef.current;
    if (fitKey === "" || el === null || squeeze >= SQUEEZE_MAX) return;
    const text = el.querySelector<HTMLElement>("[data-live-state]");
    const textCut = text !== null && text.clientWidth < Math.min(text.scrollWidth, TEXT_ROOM);
    if (el.scrollWidth > el.clientWidth || textCut) setSqueeze(squeeze + 1);
  });
  const squeezed = play ? squeeze : 0;
  // „Schau maximieren“ steps aside once it is no longer pressed (step 3): the focus it had goes to redo, else to the history.
  const refocus = useRef(false);
  const maximizeAside = onMaximize !== undefined && squeezed >= 3 && !maximized;
  useLayoutEffect(() => {
    if (!refocus.current) return;
    const el = barRef.current;
    if (!maximizeAside) {
      if (!maximized) refocus.current = false;
      return;
    }
    refocus.current = false;
    if (el === null || (document.activeElement !== null && document.activeElement !== document.body)) return;
    const next = el.querySelector<HTMLButtonElement>('button[aria-label="Wiederholen"]:not(:disabled)') ?? el.querySelector<HTMLButtonElement>('button[aria-label^="Verlauf"]');
    next?.focus();
  });
  const toggleMaximize = () => {
    refocus.current = squeezed >= 3 && maximized;
    onMaximize?.();
  };
  const lightCompact = squeezed >= 2;
  const classes = [styles.hinge, wizard === true ? styles.wizardMode : null, play ? styles.playMode : null].filter((c) => c !== null).join(" ");

  return (
    <div className={classes} role="toolbar" aria-label="Scharnier" data-hinge-mode={play ? "play" : undefined} data-hinge-squeeze={play ? squeezed : undefined} ref={barRef}>
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
            <Segmented options={DEPTHS_OPTIONS} value={depth} onChange={onDepth} ariaLabel="Tiefe" iconOnly={squeezed >= 1} />
          </div>
          <div className={styles.right}>
            <button type="button" className={`${styles.light} ${styles[kind]}`} data-light={kind} data-running={running ? "" : undefined} aria-label={lightLabel} title={lightCompact ? lightLabel : undefined} onClick={onProblems}>
              <span className={running ? styles.spinner : styles.dot} aria-hidden="true" />
              {running && !lightCompact && <span className={styles.progress}>{checking}</span>}
              {lightCompact ? null : counts === "" ? (
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
            {onMaximize && !maximizeAside && <IconButton label="Schau maximieren" icon="maximize" pressed={maximized} onClick={toggleMaximize} />}
            {play ? (
              <div className={styles.liveActions}>
                <span className={styles.liveBadge} data-live-badge="">
                  <Badge kind="live">Spielt in melonDS</Badge>
                </span>
                <span className={styles.liveText} data-live-state={play.state} title={play.text} aria-live="polite">
                  {play.text}
                </span>
                <div className={styles.liveButtons} data-live-buttons="">
                  {play.state === "halted" ? (
                    <Button kind="primary" data-live-resume="" onClick={play.onResume}>
                      Weiter
                    </Button>
                  ) : null}
                  {/* „Neu laden“ builds again: it waits with aria-disabled (keeps the focus) while a build runs. */}
                  <Button kind="secondary" data-live-reload="" aria-disabled={playing || undefined} onClick={() => !playing && play.onReload()}>
                    Neu laden
                  </Button>
                  <Button kind="secondary" data-live-stop="" onClick={play.onStop}>
                    Stoppen
                  </Button>
                </div>
              </div>
            ) : (
              /* Playing blocks with aria-disabled, not `disabled`: the button keeps keyboard focus while it builds (and gets it back from a dialog). */
              <Button kind="play" icon="play" shortcut={inTauri() && !playing ? "⌘R" : undefined} disabled={!canPlay} aria-disabled={playing || undefined} onClick={() => !playing && onPlay()}>
                {playing ? "Baut …" : "Hier spielen"}
              </Button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
