import { useEffect, useRef, useState } from "react";
import type { Created } from "../api";
import { Hinge } from "../ui/frame/Hinge";
import { StartPane } from "../ui/frame/StartPane";
import type { StartPaneProps } from "../ui/frame/StartPane";
import { DSScreen, Icon, Notice } from "../ui/primitives";
import { WizardSlotScope } from "../workbench/wizards/WizardFrame";
import { NewProjectWizard, TEMPLATE_NAMES } from "./NewProjectWizard";
import type { WizardChoice } from "./NewProjectWizard";
import { gameName, languageName } from "./projectName";
import { useRomDrop } from "./romDrop";
import type { StartWizardRequest } from "./romDrop";
import styles from "./ProjectStart.module.css";

export type { StartWizardRequest } from "./romDrop";

export type ProjectStartProps = StartPaneProps & {
  /** The open wizard "Neues Spiel aus ROM" (null: none); kept by App, so the palette can open it. */
  wizard: StartWizardRequest | null;
  onWizard: (request: StartWizardRequest | null) => void;
  /** "Öffnen" on the wizard's result. */
  onOpenCreated: (created: Created) => void;
  /** Says something while the wizard is open (the start pane, which shows refusals, is not there then): a drop onto the wizard. */
  onNotice?: (text: string) => void;
};

const noop = () => {};
const NO_HISTORY = { undo: [], redo: [] };
const NO_PROBLEMS = { errors: 0, warnings: 0, infos: 0 };

/** The view above the wizard: the ROM's header title on a DS screen, like the label of a cartridge, and what was chosen so far. */
function StartSchau({ choice }: { choice: WizardChoice | null }) {
  const rom = choice?.rom ?? null;
  return (
    <section className={styles.schau} data-start-schau="" aria-label="Vorschau">
      <div className={styles.preview}>
        <div className={styles.screen}>
          <DSScreen
            variant="empty"
            pages={[]}
            page={0}
            background={<span className={rom ? styles.title : `${styles.title} ${styles.none}`}>{rom ? rom.title : "Noch keine ROM"}</span>}
          />
        </div>
        <dl className={styles.facts}>
          <div>
            <dt>Spiel</dt>
            <dd>{rom ? `${gameName(rom)} (${languageName(rom.language)})` : "–"}</dd>
          </div>
          <div>
            <dt>Vorlage</dt>
            <dd>{choice ? `${TEMPLATE_NAMES[choice.template]}${choice.textProject ? ", als Textprojekt" : ""}` : "–"}</dd>
          </div>
          <div>
            <dt>Ziel</dt>
            <dd className={styles.code}>{choice?.dest ?? "–"}</dd>
          </div>
        </dl>
      </div>
    </section>
  );
}

/** The frame over the start while a ROM is dragged over it: the whole surface is the slot it goes into. */
export function DropHint() {
  return (
    <div className={styles.drop}>
      <span className={styles.dropIcon} aria-hidden="true">
        <Icon name="game" />
      </span>
      <p className={styles.dropTitle}>ROM hier ablegen</p>
      <p className={styles.dropText}>Eine .nds-Datei, deine eigene ROM.</p>
    </div>
  );
}

/**
 * The start's own small deck while the wizard is open (no project needs to be open): the view of the choice, a hinge in wizard mode with
 * the slot of the step bar, and the hand with the slot of the step. The wizard looks for its slots inside this deck only.
 */
function StartDeck({ request, onClose, onOpenCreated }: { request: StartWizardRequest; onClose: () => void; onOpenCreated: (created: Created) => void }) {
  const [scope, setScope] = useState<HTMLDivElement | null>(null);
  const [choice, setChoice] = useState<WizardChoice | null>(null);
  return (
    <div ref={setScope} className={styles.deck} data-start-deck="">
      <StartSchau choice={choice} />
      <Hinge
        crumbs={[]}
        depth="bauen"
        onDepth={noop}
        problems={NO_PROBLEMS}
        onProblems={noop}
        history={NO_HISTORY}
        onUndoTo={noop}
        onRedo={noop}
        onPlay={noop}
        playing={false}
        canPlay={false}
        wizard
      />
      <div className={styles.hand} data-wizard-hand="" />
      {scope && (
        <WizardSlotScope.Provider value={scope}>
          <NewProjectWizard initialRom={request.rom} initialNotice={request.notice} onCancel={onClose} onOpen={onOpenCreated} onChoice={setChoice} />
        </WizardSlotScope.Provider>
      )}
    </div>
  );
}

/**
 * The area Start: the recent projects with "Neues Spiel aus ROM…" as the main action, and a surface that takes a dropped `.nds`; with the
 * wizard open, its own deck. A refused drop says why below the actions.
 */
export function ProjectStart({ wizard, onWizard, onOpenCreated, onNotice, ...pane }: ProjectStartProps) {
  const [refused, setRefused] = useState<string | null>(null);
  const open = (request: StartWizardRequest) => {
    setRefused(null);
    onWizard(request);
  };
  // With the wizard open a drop is refused through the toast (the refusal below the actions is not on screen then).
  const { over, handlers } = useRomDrop(wizard === null, open, (text) => (wizard === null ? setRefused(text) : onNotice?.(text)));
  // The button that opened the wizard went with the start pane: when the wizard closes, the focus comes back to the main action.
  const surface = useRef<HTMLDivElement>(null);
  const wasOpen = useRef(wizard !== null);
  useEffect(() => {
    if (wasOpen.current && wizard === null) surface.current?.querySelector<HTMLElement>('[data-start-actions] button[data-kind="primary"]')?.focus();
    wasOpen.current = wizard !== null;
  }, [wizard]);
  return (
    <div ref={surface} className={styles.surface} data-drop={over ? "over" : "idle"} {...handlers}>
      {wizard === null ? (
        <>
          <StartPane {...pane} onNewGame={() => open({})} notice={refused !== null && <Notice kind="error">{refused}</Notice>} />
          {over && <DropHint />}
        </>
      ) : (
        <StartDeck request={wizard} onClose={() => onWizard(null)} onOpenCreated={onOpenCreated} />
      )}
    </div>
  );
}
