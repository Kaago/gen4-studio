import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { TOOLS } from "../../../workbench/tools";
import { WizardBar } from "../../../workbench/wizards/WizardBar";
import { FloatingToolbar } from "../../frame/FloatingToolbar";
import { HandSplitter } from "../../frame/HandSplitter";
import { Hinge } from "../../frame/Hinge";
import { Palette } from "../../frame/Palette";
import { ShellProvider, useShell } from "../../frame/ShellContext";
import type { Tool, ToolbarAnchor } from "../../frame/shellState";
import { StartPane } from "../../frame/StartPane";
import { Button, Tabs } from "../../primitives";
import type { PickerItem, TabItem } from "../../primitives";
import styles from "../Gallery.module.css";

const noop = () => {};
const HOUR = 60 * 60 * 1000;
const STEPS = ["Größe", "Lage", "Name"] as const;

/** The hinge in wizard mode: its slot holds the step bar, as `WizardFrame` fills it through a portal. */
function WizardHinge() {
  const box = useRef<HTMLDivElement>(null);
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const [step, setStep] = useState(0);
  const [done, setDone] = useState(false);
  useEffect(() => setSlot(box.current?.querySelector<HTMLElement>("[data-wizard-bar]") ?? null), []);
  const last = step === STEPS.length - 1;
  return (
    <div ref={box} className={styles.hingeFrame} data-frame="assistent">
      <Hinge
        crumbs={[]}
        depth="bauen"
        onDepth={noop}
        problems={{ errors: 0, warnings: 0, infos: 0 }}
        onProblems={noop}
        history={{ undo: [], redo: [] }}
        onUndoTo={noop}
        onRedo={noop}
        onPlay={noop}
        playing={false}
        canPlay={false}
        wizard
      />
      {slot &&
        createPortal(
          <WizardBar
            titleId="gallery-wizard-title"
            title="Gebiet anlegen"
            icon="world"
            steps={STEPS}
            step={step}
            done={done}
            busy={false}
            canNext
            submitLabel="Gebiet anlegen"
            onCancel={() => {
              setStep(0);
              setDone(false);
            }}
            onBack={() => setStep((s) => Math.max(0, s - 1))}
            onNext={() => (last ? setDone(true) : setStep((s) => s + 1))}
          />,
          slot,
        )}
    </div>
  );
}

/** The hinge in its usual mode, to show the light: three counters, or the spinner with the progress of the checks. */
function LightHinge({ problems, checking = null, unchecked = false }: { problems: { errors: number; warnings: number; infos: number }; checking?: string | null; unchecked?: boolean }) {
  return (
    <div className={styles.hingeFrame}>
      <Hinge
        crumbs={[{ label: "Zweiblattdorf" }]}
        depth="bauen"
        onDepth={noop}
        problems={problems}
        checking={checking}
        unchecked={unchecked}
        onProblems={noop}
        history={{ undo: [], redo: [] }}
        onUndoTo={noop}
        onRedo={noop}
        onPlay={noop}
        playing={false}
        canPlay
      />
    </div>
  );
}

/** The hinge on top of a view and a hand with the splitter between them, as the deck of an area builds them; the splitter sizes the hand here too. */
function HingeOnTop() {
  const { state } = useShell();
  const hand = state.handHeight.welt;
  return (
    <div className={styles.deckFrame} data-frame="scharnier-oben">
      <Hinge
        crumbs={[{ label: "Zweiblattdorf" }, { label: "Außen 0411" }]}
        depth="bauen"
        onDepth={noop}
        problems={{ errors: 0, warnings: 1, infos: 2 }}
        onProblems={noop}
        history={{ undo: [], redo: [] }}
        onUndoTo={noop}
        onRedo={noop}
        onPlay={noop}
        playing={false}
        canPlay
      />
      <div className={styles.deckView} data-dock="schau">
        <span className={styles.note}>Schau</span>
      </div>
      <HandSplitter area="welt" controls="gallery-deck-hand" />
      <div className={styles.deckHand} id="gallery-deck-hand" data-dock="hand" hidden={state.handCollapsed.welt} style={{ height: hand } /* dyn */}>
        <span className={styles.note}>Hand</span>
      </div>
    </div>
  );
}

/** A view with the floating toolbar at one anchor; `blocked` locks the four event tools with that reason, as a view without a shown place does. */
function ToolbarView({ anchor, blocked = null }: { anchor: ToolbarAnchor; blocked?: string | null }) {
  const [tool, setTool] = useState<Tool>("select");
  const [at, setAt] = useState(anchor);
  return (
    <div className={at === "left" ? styles.toolbarView : `${styles.toolbarView} ${styles.toolbarViewFlat}`}>
      <FloatingToolbar tools={TOOLS} tool={tool} onTool={setTool} eventsBlocked={blocked} density="komfort" anchor={at} onAnchor={setAt} />
    </div>
  );
}

const PALETTE_ITEMS: PickerItem[] = [
  { id: "cmd-open", label: "Projekt öffnen", icon: "open", shortcut: "⌘O", group: "Befehle" },
  { id: "cmd-save", label: "Speichern", icon: "save", shortcut: "⌘S", group: "Befehle", disabled: true, description: "Erst ein Projekt öffnen" },
  { id: "cmd-theme", label: "Thema wechseln", icon: "sun", group: "Befehle" },
  { id: "place-1", label: "Zweiblattdorf", area: "welt", description: "2 Header", group: "Orte" },
  { id: "header-411", label: "Zweiblattdorf", tech: "0411", description: "T01", area: "welt", group: "Header" },
  { id: "archive-411", label: "Zweiblattdorf", tech: "0411", description: "Textarchiv, verwendet von Header 0411", area: "texte", group: "Textarchive" },
];

const TAB_ITEMS: TabItem[] = [
  { id: "a", label: "Scriptdatei 0412", unsaved: true, closeLabel: "Scriptdatei 0412 schließen" },
  { id: "b", label: "Level-Script 0009", closeLabel: "Level-Script 0009 schließen" },
  { id: "c", label: "Probleme", count: 3 },
];

/** The frame's own parts on the primitives: the hinge on top of the view with its splitter, the floating toolbar, the hinge in wizard mode and its problem light, the start cards, the palette and tabs. */
export function FramePage() {
  const [palette, setPalette] = useState(false);
  const [ran, setRan] = useState("Noch nichts ausgeführt.");
  const [tabs, setTabs] = useState(TAB_ITEMS);
  const [tab, setTab] = useState("a");
  const [now] = useState(() => Date.now());

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Scharnier oben</h2>
        <p className={styles.note}>
          Das Scharnier heißt über der Schau, 44 px, ohne Griff und nicht ziehbar. Unter der Schau trennt der Teiler (8 px) die Hand: Ziehen, Pfeiltasten (±16 px), Pos1 und Ende ändern ihre Höhe (560 und 200
          px), Enter oder Doppelklick klappt sie ein und aus.
        </p>
        <ShellProvider>
          <HingeOnTop />
        </ShellProvider>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Werkzeugleiste schwebend</h2>
        <p className={styles.note}>
          Die Leiste schwebt über der 3D-Ansicht: senkrecht am linken Rand, waagrecht oben oder unten. Der Griff zieht sie an den nächsten Anker, das Menü neben ihm wählt die Lage mit der Tastatur. Ohne
          gezeigten Ort sind die vier Ereigniswerkzeuge gesperrt und nennen den Grund.
        </p>
        <div className={styles.toolbarViews}>
          <ToolbarView anchor="left" />
          <ToolbarView anchor="top" />
          <ToolbarView anchor="left" blocked="Erst einen Ort wählen." />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Scharnier im Assistenten-Modus</h2>
        <p className={styles.note}>Der Assistent zeigt seine Schritte im Scharnier; Weiter, Zurück und Abbrechen stehen rechts.</p>
        <WizardHinge />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Prüflicht</h2>
        <p className={styles.note}>
          Drei Zähler mit Symbol (Fehler, Warnungen, Hinweise), der Punkt in der Farbe des Schlimmsten; während einer Prüfung dreht sich ein Ring und sagt, wie weit sie ist. Ohne Ergebnis der Prüfungen ein
          leerer Ring und „Nicht geprüft“, nie „Alles stimmig“.
        </p>
        <LightHinge problems={{ errors: 2, warnings: 1, infos: 3 }} />
        <LightHinge problems={{ errors: 0, warnings: 0, infos: 10 }} checking="Prüft … Texte 312/724" />
        <LightHinge problems={{ errors: 0, warnings: 0, infos: 0 }} />
        <LightHinge problems={{ errors: 0, warnings: 0, infos: 0 }} unchecked />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Startkarten</h2>
        <div className={styles.startFrame}>
          <StartPane
            recent={[
              { path: "/Projekte/platin-hack", name: "platin-hack", game: "Platin, DE", openedAt: now - HOUR },
              { path: "/Projekte/hgss-test", name: "hgss-test", game: "HeartGold, DE", openedAt: now - 26 * HOUR },
            ]}
            missing={new Set(["/Projekte/hgss-test"])}
            onOpen={noop}
            onOpenRecent={noop}
            onForget={noop}
          />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Befehlspalette</h2>
        <div className={styles.row}>
          <Button kind="secondary" icon="search" shortcut="⌘K" onClick={() => setPalette(true)}>
            Palette öffnen
          </Button>
          <span className={styles.note}>{ran}</span>
        </div>
        <Palette open={palette} onOpenChange={setPalette} items={PALETTE_ITEMS} onRun={(item) => setRan(`Ausgeführt: ${item.label}`)} />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Reiter</h2>
        <div className={styles.panelBox}>
          <Tabs
            items={tabs}
            value={tab}
            onChange={setTab}
            onClose={(id) => {
              const rest = tabs.filter((t) => t.id !== id);
              setTabs(rest.length > 0 ? rest : TAB_ITEMS);
              if (id === tab) setTab((rest[0] ?? TAB_ITEMS[0]!).id);
            }}
            ariaLabel="Offene Dateien"
          >
            <p className={styles.note}>{`Inhalt von ${tabs.find((t) => t.id === tab)?.label ?? ""}`}</p>
          </Tabs>
        </div>
      </section>
    </div>
  );
}
