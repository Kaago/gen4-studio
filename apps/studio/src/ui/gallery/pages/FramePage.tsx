import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import type { IDockviewPanel } from "dockview-react";
import type { EventsDto, HeaderSummary, MatrixDto } from "../../../api";
import { InspectorHead } from "../../../workbench/inspector/InspectorHead";
import type { InspectorHeadProps } from "../../../workbench/inspector/InspectorHead";
import { SceneHierarchy } from "../../../workbench/SceneHierarchy";
import type { SceneRef } from "../../../workbench/sceneModel";
import { RECIPE_BAR } from "../../../workbench/recipes";
import { TOOLS } from "../../../workbench/tools";
import { WizardBar } from "../../../workbench/wizards/WizardBar";
import deckStyles from "../../frame/Deck.module.css";
import { tabMenuItems } from "../../frame/dockMenu";
import { FloatingToolbar } from "../../frame/FloatingToolbar";
import { Hinge } from "../../frame/Hinge";
import { Palette } from "../../frame/Palette";
import { RecipeBar } from "../../frame/RecipeBar";
import { ShellProvider } from "../../frame/ShellContext";
import type { Tool, ToolbarAnchor } from "../../frame/shellState";
import { StartPane } from "../../frame/StartPane";
import { Button, Kbd, RecipeButton, Splitter, Tabs } from "../../primitives";
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

/** The hinge on top of an area's dock, here with a view group and an edit group below it (dockview's sash sizes them in the app). */
function HingeOnTop() {
  const [maximized, setMaximized] = useState(false);
  return (
    <div className={styles.deckFrame} data-frame="scharnier-oben">
      <Hinge
        crumbs={[{ label: "Zweiblattdorf" }, { label: "Gebiet 0411" }]}
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
        maximized={maximized}
        onMaximize={() => setMaximized((m) => !m)}
      />
      <div className={styles.deckDock} data-dock="welt">
        <div className={styles.deckView} data-dock-group="schau" data-dock-group-kind="schau">
          <span className={styles.note}>Schau</span>
        </div>
        <div className={styles.deckHand} data-dock-group="bearbeiten" hidden={maximized}>
          <span className={styles.note}>Script, Probleme</span>
        </div>
      </div>
    </div>
  );
}

/** A tab of a dock group: its id (the panel), its title, whether it carries the unsaved dot. */
type DemoTab = { id: string; title: string; unsaved?: boolean };

/**
 * A dock group as dockview builds it, with its classes (the theme styles them as in the app): the tab strip with the deck's own tab inside
 * each tab (`Deck.module.css`: the title, then the unsaved dot), the visible tab first-class, then the content. `focused`: the group has
 * the focus (dockview's active group), so its visible tab is on `flaeche/schau`.
 */
function DockGroupDemo({ name, tabs, visible, focused, className, children }: { name: string; tabs: DemoTab[]; visible: string; focused: boolean; className?: string; children?: ReactNode }) {
  return (
    <div className={`dv-groupview ${focused ? "dv-active-group" : "dv-inactive-group"} ${className ?? ""}`} data-dock-group={name}>
      <div className="dv-tabs-and-actions-container">
        <div className="dv-tabs-container dv-horizontal" role="tablist" aria-label={`Reiter ${name}`}>
          {tabs.map((tab) => (
            <div key={tab.id} className={`dv-tab ${tab.id === visible ? "dv-active-tab" : "dv-inactive-tab"}`} role="tab" aria-selected={tab.id === visible} tabIndex={tab.id === visible ? 0 : -1}>
              <span className={deckStyles.tab} data-dock-tab={tab.id}>
                <div className="dv-default-tab">
                  <div className="dv-default-tab-content">{tab.title}</div>
                </div>
                {tab.unsaved && <span className={deckStyles.dot} role="img" aria-label="ungespeichert" />}
              </span>
            </div>
          ))}
        </div>
      </div>
      <div className={`dv-content-container ${styles.dockGroupContent}`}>{children}</div>
    </div>
  );
}

/** A panel as the tab menu sees it: docked in the grid or floating, not maximized. */
const demoPanel = (id: string, location: "grid" | "floating") =>
  ({ id, api: { isMaximized: () => false, location: { type: location }, maximize: noop, exitMaximized: noop } }) as unknown as IDockviewPanel;

/**
 * The tab menu of `panel` as the app builds it (`tabMenuItems`, with the world's „Als Vorschaubild“, locked on the world map as without a
 * chosen map), shown open: dockview's menu classes, a disabled line dimmed, its reason as the tooltip.
 */
function TabMenuDemo({ title, panel }: { title: string; panel: IDockviewPanel }) {
  const items = tabMenuItems({ panel }, { float: noop, dock: noop, reset: noop, setPreview: noop, previewLocked: (id) => (id === "worldmap" ? "Erst eine Karte wählen" : null) });
  return (
    <ul className={`dv-context-menu ${styles.dockMenu}`} aria-label={`Reitermenü ${title}`} data-tab-menu={panel.id}>
      {items.map((item, i) =>
        item === "separator" ? (
          <li key={`separator-${i}`} className="dv-context-menu-separator" aria-hidden="true" />
        ) : (
          <li key={item.label} className={item.disabled ? "dv-context-menu-item dv-context-menu-item--disabled" : "dv-context-menu-item"} aria-disabled={item.disabled || undefined} title={item.hint}>
            {item.label}
          </li>
        ),
      )}
    </ul>
  );
}

const VIEW_TABS: DemoTab[] = [
  { id: "world", title: "3D" },
  { id: "matrix", title: "Raster" },
  { id: "worldmap", title: "Weltkarte" },
];
const BELOW_TABS: DemoTab[] = [
  { id: "script", title: "Script", unsaved: true },
  { id: "problems", title: "Probleme" },
];

/** The dock tabs in their states: the view group with the focus (3D visible, the others at rest), the group below with the unsaved script; each with its open tab menu. */
function DockTabs() {
  return (
    <div className={`dockview-theme-g4 ${styles.dockDemos}`} data-frame="dock-reiter">
      <div className={styles.dockDemo}>
        <DockGroupDemo name="schau" tabs={VIEW_TABS} visible="world" focused className={styles.dockGroup}>
          <span className={styles.note}>Aktiv: der sichtbare Reiter der Gruppe mit Fokus. Ruhe: Raster, Weltkarte.</span>
        </DockGroupDemo>
        <TabMenuDemo title="3D" panel={demoPanel("world", "grid")} />
        <TabMenuDemo title="Weltkarte" panel={demoPanel("worldmap", "grid")} />
      </div>
      <div className={styles.dockDemo}>
        <DockGroupDemo name="unten" tabs={BELOW_TABS} visible="script" focused={false} className={styles.dockGroup}>
          <span className={styles.note}>Sichtbar in einer Gruppe ohne Fokus, mit dem Punkt „ungespeichert“.</span>
        </DockGroupDemo>
        <TabMenuDemo title="Script" panel={demoPanel("script", "grid")} />
      </div>
    </div>
  );
}

/** The inspector floating at the right edge of the view: the title bar is the grip, the group a sheet with a window's corners and shadow. */
function FloatingGroupDemo() {
  return (
    <div className={`dockview-theme-g4 ${styles.dockDemos}`} data-frame="schwebende-gruppe">
      <div className={styles.floatStage}>
        <DockGroupDemo name="schau" tabs={VIEW_TABS} visible="world" focused={false} className={styles.dockGroup}>
          <span className={styles.note}>Die Schau bleibt angedockt und nimmt die Breite, die der Inspector frei gab.</span>
        </DockGroupDemo>
        <div className={`dv-resize-container dv-resize-container-with-titlebar ${styles.floatGroup}`}>
          <div className="dv-floating-titlebar" aria-hidden="true" />
          <div className="dv-grid-view">
            <DockGroupDemo name="schwebend" tabs={[{ id: "inspector", title: "Inspector" }]} visible="inspector" focused>
              <InspectorHead kind="figure" title="Objekt 2 · ID 2" ident="Objekt 2 · Eventdatei 0390" actions={[{ label: "Löschen", detail: <Kbd keys="Entf" />, onSelect: noop }]} depth="bauen" />
            </DockGroupDemo>
          </div>
        </div>
      </div>
      <TabMenuDemo title="Inspector" panel={demoPanel("inspector", "floating")} />
    </div>
  );
}

/** The two arrangements as plans: where the groups lie in Werkbank and Doppeldeck. */
function Arrangements() {
  return (
    <div className={styles.dockDemos} data-frame="anordnung">
      <figure className={styles.dockDemo} data-arrangement="werkbank">
        <div className={`${styles.arrangement} ${styles.arrangementWerkbank}`}>
          <span className={styles.arrangementCell} data-cell="schau">
            Schau: 3D, Raster, Weltkarte
          </span>
          <span className={styles.arrangementCell} data-cell="side">
            Inspector 340, volle Höhe
          </span>
          <span className={styles.arrangementCell} data-cell="below">
            Script, Probleme 240
          </span>
        </div>
        <figcaption className={styles.note}>Werkbank: der Standard der Welt</figcaption>
      </figure>
      <figure className={styles.dockDemo} data-arrangement="doppeldeck">
        <div className={`${styles.arrangement} ${styles.arrangementDoppeldeck}`}>
          <span className={styles.arrangementCell} data-cell="schau">
            Schau
          </span>
          <span className={styles.arrangementCell} data-cell="below">
            Bearbeiten 286 (Welt: Inspector, Script, Probleme)
          </span>
        </div>
        <figcaption className={styles.note}>Doppeldeck: Texte, Pokémon, Gegenstände; in der Welt wählbar</figcaption>
      </figure>
    </div>
  );
}

/** A view with the floating toolbar at one anchor; `blocked` locks the four event tools with that reason, as a view without a shown place does. */
function ToolbarView({ anchor, blocked = null }: { anchor: ToolbarAnchor; blocked?: string | null }) {
  const [tool, setTool] = useState<Tool>("select");
  const [at, setAt] = useState(anchor);
  return (
    <div className={at === "left" ? styles.toolbarView : `${styles.toolbarView} ${styles.toolbarViewFlat}`}>
      <FloatingToolbar tools={TOOLS} tool={tool} onTool={setTool} eventsBlocked={blocked} density="komfort" anchor={at} chosen={at} onAnchor={setAt} />
    </div>
  );
}

/** A splitter between two stand-in panes, sizing the first: the horizontal one grows downwards, the vertical one to the right. */
function SplitterDemo({ orientation }: { orientation: "horizontal" | "vertical" }) {
  const [size, setSize] = useState(120);
  const horizontal = orientation === "horizontal";
  const first = horizontal ? { height: size } : { width: size };
  return (
    <div className={horizontal ? styles.splitColumn : styles.splitRow} data-frame={horizontal ? "teiler-waagrecht" : "teiler-senkrecht"}>
      <div className={styles.splitPane} style={first /* dyn */}>
        <span className={styles.note}>{`${size} px`}</span>
      </div>
      {horizontal ? (
        <Splitter orientation="horizontal" grows="down" label="Höhe des oberen Teils" value={size} min={60} max={180} onValue={setSize} measure={(e) => e.clientY} />
      ) : (
        <Splitter orientation="vertical" grows="right" label="Breite des linken Teils" value={size} min={80} max={320} onValue={setSize} measure={(e) => e.clientX} />
      )}
      <div className={styles.splitPane} data-rest="">
        <span className={styles.note}>Rest</span>
      </div>
    </div>
  );
}

/** The heads of the inspector in the states it has: a figure with unsaved changes, an area in Bauen, the same header in Tüfteln, a door of a locked event file. */
const HEADS: { name: string; head: InspectorHeadProps }[] = [
  {
    name: "Figur, ungespeichert",
    head: { kind: "figure", title: "Objekt 2 · ID 3", ident: "Objekt 2 · Eventdatei 0390", unsaved: true, actions: [{ label: "Löschen", detail: <Kbd keys="Entf" />, onSelect: noop }], depth: "bauen" },
  },
  {
    name: "Gebiet in Bauen",
    head: { kind: "area", title: "Zweiblattdorf", ident: "Zweiblattdorf · T01", actions: [{ label: "Raum hier anbinden…", onSelect: noop }], depth: "bauen" },
  },
  {
    name: "Header in Tüfteln",
    head: { kind: "area", title: "Zweiblattdorf", ident: "Header 0411 · T01", actions: [{ label: "Raum hier anbinden…", onSelect: noop }], depth: "tuefteln" },
  },
  {
    name: "Tür, Eventdatei gesperrt",
    head: { kind: "door", title: "Warp 1 → Haus 1", ident: "Warp 1 · Eventdatei 0390", locked: true, actions: [{ label: "Löschen", detail: <Kbd keys="Entf" />, disabled: true, onSelect: noop }], depth: "bauen" },
  },
  {
    name: "Karte mit Menü",
    head: {
      kind: "map",
      title: "Karte 0411",
      ident: "Sinnoh · Zelle 3, 4",
      irreversible: true,
      actions: [{ label: "Teile importieren…", onSelect: noop }, { label: "Teile exportieren…", onSelect: noop }, { label: "Duplizieren", onSelect: noop }],
      depth: "bauen",
    },
  },
];

/** The heads as the inspector shows them, one box each. */
function InspectorHeads() {
  return (
    <div className={styles.columns} data-frame="inspector-kopf">
      {HEADS.map(({ name, head }) => (
        <div key={name} className={styles.panelBox} aria-label={name} role="group">
          <InspectorHead {...head} />
        </div>
      ))}
    </div>
  );
}

// A scene as „In dieser Szene“ lists it; the numbers are made up, the gallery never ships data from a game.
const header = (id: number, internal_name: string, location_name: string, matrix_id: number): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  location_index: id,
  kanto: false,
  matrix_id,
  area_data_id: 0,
  event_file_id: 390,
  text_archive_id: 0,
  script_file_id: 412,
  level_script_id: 9,
});
const TOWN = header(411, "T01", "Zweiblattdorf", 0);
const HOUSE = header(414, "T01R0101", "Haus 1", 200);
const SCENE_MATRIX: MatrixDto = { index: 0, name: "Sinnoh", width: 2, height: 1, maps: [411, 412], headers: [411, 411] };
const SCENE_EVENTS: EventsDto = {
  event_file_id: 390,
  locked: null,
  objects: [
    { index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, trainer_type: 0, sight: 0, x: 10, z: 20, y: 0 },
    { index: 1, local_id: 2, graphics_id: 4, movement_type: 0, script: 3, dir: 0, trainer_type: 0, sight: 0, x: 11, z: 21, y: 0 },
  ],
  warps: [
    { index: 0, x: 8, z: 9, dest_header_id: 414, dest_warp_id: 0, height: 0 },
    { index: 1, x: 9, z: 9, dest_header_id: 999, dest_warp_id: 0, height: 0 },
  ],
  coords: [{ index: 0, script: 5, x: 1, z: 2, width: 1, length: 1 }],
  bg: [{ index: 0, script: 12, event_type: 0, x: 4, z: 5, y: 0 }],
};
const SCENE_HEADERS = [TOWN, HOUSE];
const SCENE_BUILDINGS = [{ map: 411, models: [12, 13] }];
const SCENE_MARKS: ReadonlyMap<number, "warning"> = new Map([[411, "warning"]]);
const SCENE_FAULTY = [{ file: 390, kind: "warp" as const, index: 1 }];
const identify = (h: HeaderSummary) => (h.id === HOUSE.id ? { kind: "room" as const, name: "Haus 1" } : { kind: "area" as const, name: h.location_name });

/** The hierarchy of a shown area with sample events: a selection that follows the clicks, a door with a target and one without, a finding at the root and at a door. */
function SceneDemo({ locked = null }: { locked?: string | null }) {
  const [selected, setSelected] = useState<SceneRef | null>({ kind: "event", file: 390, eventKind: "object", index: 1 });
  const [said, setSaid] = useState("Nichts gewählt.");
  return (
    <div className={styles.sceneColumn}>
      <div className={styles.sceneFrame} data-frame={locked === null ? "hierarchie" : "hierarchie-gesperrt"}>
        <SceneHierarchy
          header={TOWN}
          headers={SCENE_HEADERS}
          matrix={SCENE_MATRIX}
          events={SCENE_EVENTS}
          buildings={SCENE_BUILDINGS}
          depth="bauen"
          identify={identify}
          selected={selected}
          marks={SCENE_MARKS}
          faultyEvents={SCENE_FAULTY}
          locked={locked}
          loading={false}
          toolsBlocked={locked === null ? null : "Eventdatei gesperrt."}
          canBack
          canForward={false}
          onSelect={(ref) => {
            setSelected(ref);
            setSaid(`Gewählt: ${ref.kind === "event" ? `${ref.eventKind} ${ref.index}` : ref.kind}`);
          }}
          onFollow={(h) => setSaid(`Folgt der Tür nach ${h}`)}
          onOpenScript={(file) => setSaid(`Öffnet Scriptdatei ${file}`)}
          onTool={(tool) => setSaid(`Werkzeug ${tool}`)}
          onConnectRoom={() => setSaid("Raum hier anbinden")}
          onNewPlace={() => setSaid("Neuer Ort")}
          onBack={() => setSaid("Zurück")}
          onForward={() => setSaid("Vor")}
        />
      </div>
      <span className={styles.note}>{said}</span>
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

/** The frame's own parts on the primitives: the hinge on top of the dock, the dock tabs, a floating group and the arrangements, the floating toolbar, the hinge in wizard mode and its problem light, the start cards, the palette and tabs. */
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
          Das Scharnier steht über dem Dock des Bereichs, 44 px, ohne Griff und nicht ziehbar, als sein Kopf. Im Dock liegen die Schau und die Gruppen zum Bearbeiten; ihre Größe ändern Dockviews
          Sashes zwischen den Gruppen. „Schau maximieren“ ist gedrückt, solange die Schau das Dock füllt.
        </p>
        <ShellProvider>
          <HingeOnTop />
        </ShellProvider>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Dock-Reiter</h2>
        <p className={styles.note}>
          Jede Gruppe zeigt ihre Reiter, „dichte/reiter“ hoch. Der sichtbare Reiter trägt die 2-px-Kobaltkante, in der Gruppe mit Fokus auf „flaeche/schau“; ein Punkt sagt „ungespeichert“, ein Schließen gibt es
          nicht. Doppelklick maximiert die Gruppe. Das Menü (Rechtsklick, Umschalt+F10) eines Schau-Reiters hat nur „Maximieren“.
        </p>
        <DockTabs />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Schwebende Gruppe</h2>
        <p className={styles.note}>
          Ein schwebendes Panel ist ein Blatt mit Rand, „radius/fenster“ und „schatten/schwebend“; die Titelleiste ist der Griff. „Schweben lassen“ legt es an den rechten Rand der Schau, „Zurück ins
          Dock“ an seinen Platz der Anordnung. Die Schau selbst schwebt nie.
        </p>
        <FloatingGroupDemo />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Anordnung</h2>
        <p className={styles.note}>Die zwei Vorlagen eines Docks, über die Palette „Anordnung: Werkbank“ und „Anordnung: Doppeldeck“; „Layout zurücksetzen“ baut die gewählte neu.</p>
        <Arrangements />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Teiler waagrecht und senkrecht</h2>
        <p className={styles.note}>
          Derselbe Teiler (8 px, Linie „linie/leise“) zwischen zwei Teilen, in der App zwischen „Orte“ und „In dieser Szene“: Ziehen beginnt nach 4 px, die Pfeiltasten (±16 px) ändern die Größe des ersten Teils in der Richtung, in der er wächst, Pos1 und
          Ende setzen Höchst- und Mindestwert. Waagrecht wächst der obere Teil nach unten, senkrecht der linke nach rechts.
        </p>
        <div className={styles.toolbarViews}>
          <SplitterDemo orientation="horizontal" />
          <SplitterDemo orientation="vertical" />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Inspector-Kopf</h2>
        <p className={styles.note}>
          Das Typ-Wort steht als Augenbraue mit Symbol (in Tüfteln „Header“ statt „Gebiet“), darunter der Titel in Zen Maru Gothic und grau in Mono die Kennung. Rechts stehen die Zustände (Ungespeichert,
          Gesperrt, Nicht rückgängig) und das Menü „Aktionen“ mit dem, was sich mit dem Datensatz tun lässt.
        </p>
        <InspectorHeads />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Hierarchie „In dieser Szene“</h2>
        <p className={styles.note}>
          Was das gezeigte Gebiet oder der Raum enthält, als Baum: oben das Gebiet selbst, darunter Gruppen mit Zähler (Karten mit ihren Gebäuden, Figuren, Türen, Auslöser, Schilder, Scripts). Eine Tür mit Ziel
          nennt es in der zweiten Zeile, Doppelklick oder Enter folgt ihr; ein Befund steht als Zeichen am Ende der Zeile, die gewählte Zeile ist kobaltfarben. „Zurück“ und „Vor“ gehen durch die gezeigten
          Szenen, „+“ wählt ein Werkzeug zum Hinzufügen. Rechts dieselbe Szene mit gesperrter Eventdatei.
        </p>
        <div className={styles.toolbarViews}>
          <SceneDemo />
          <SceneDemo locked="Die Datei ist auf der Platte schreibgeschützt." />
        </div>
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

      <section className={styles.block} data-gallery="rezept-taste">
        <h2 className={styles.blockTitle}>Rezept-Taste</h2>
        <p className={styles.note}>
          Ein Rezept in Erzählen: das Symbol auf einer Kachel in der Farbe seines Bereichs, daneben der Name. Ruhe, Hover (Zeiger darauf: Fläche wie das Deck), Aktiv (Kobalt-Kante) und Gesperrt
          (gedämpft, mit Schloss, der Tooltip nennt den Grund). Nur mit Symbol zeigt die Rezept-Leiste eine gesperrte Taste: Name und Grund stehen im Tooltip und bleiben Name und
          Beschreibung der Taste. Die kleine Größe steht im Ergebnis eines Assistenten.
        </p>
        <div className={styles.row}>
          <RecipeButton id="create_area" label="Gebiet anlegen" icon="matrix" area="welt" hint="Kopiert Karten zu einem neuen Gebiet mit eigenem Ort." onClick={noop} />
          <RecipeButton id="create_room" label="Raum anlegen" icon="room" area="welt" hint="Zeiger darauf zeigt den Hover." onClick={noop} />
          <RecipeButton id="create_room" label="Raum anlegen" icon="room" area="welt" pressed onClick={noop} />
          <RecipeButton id="new_figure" label="Neue Figur" icon="figure" area="welt" reason="Kommt mit 3d.5" onClick={noop} />
          <RecipeButton id="connect_doors" label="Tür verbinden" icon="door" area="welt" reason="Kommt mit 3d.5" iconOnly onClick={noop} />
        </div>
        <div className={styles.row}>
          <RecipeButton id="new_trainer_here" label="Neuer Trainer hier" icon="pokemon" area="pokemon" onClick={noop} size="s" />
          <RecipeButton id="story" label="Neue Geschichte" icon="stories" area="geschichten" onClick={noop} size="s" />
          <RecipeButton id="text" label="Neuer Text" icon="texts" area="texte" onClick={noop} size="s" />
          <RecipeButton id="adjust_terrain" label="Gelände anpassen" icon="paint" area="welt" reason="Kommt mit Phase 6" onClick={noop} size="s" />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Rezept-Leiste</h2>
        <p className={styles.note}>
          In Erzählen steht sie, wo Bauen die Werkzeugleiste hat, am selben Anker und wie sie in einer Reihe (oder Spalte), nie umgebrochen. Gebiet und Raum öffnen ihre Assistenten und zeigen
          Symbol und Namen; die anderen Rezepte kommen mit 3d.5 und zeigen nur ihr Symbol mit dem Schloss, Name und Grund im Tooltip.
        </p>
        <div className={styles.toolbarViews}>
          <div className={styles.toolbarView}>
            <RecipeBar entries={RECIPE_BAR} onRecipe={noop} density="komfort" anchor="left" />
          </div>
          <div className={`${styles.toolbarView} ${styles.toolbarViewFlat} ${styles.recipeViewFlat}`}>
            <RecipeBar entries={RECIPE_BAR} onRecipe={noop} density="komfort" anchor="top" />
          </div>
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
            previews={new Map()}
            onChoosePreview={noop}
            onRemovePreview={noop}
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
