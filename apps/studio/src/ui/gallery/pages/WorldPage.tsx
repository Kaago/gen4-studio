import { useMemo, useState } from "react";
import type { HeaderPatchDto, MatrixDto } from "../../../api";
import type { LayerOp, TileJson, TileRole, TilesetDto, WorldJson } from "../../../api.world";
import { LayerList } from "../../../workbench/inspector/LayerList";
import { TilePalette } from "../../../workbench/inspector/TilePalette";
import { PAINT_TOOLS } from "../../../workbench/paintTools";
import { toolsFor } from "../../../workbench/tools";
import type { PaintSub } from "../../../world/paint/ops";
import { FloatingToolbar } from "../../frame/FloatingToolbar";
import type { Tool, ToolbarAnchor } from "../../frame/shellState";
import { AreaGrid, cellKey } from "../../../workbench/AreaGrid";
import { InspectorHead } from "../../../workbench/inspector/InspectorHead";
import { placeColor } from "../../../workbench/places";
import { nameWarning, placeKeyOf } from "../../../workbench/places/placeChoices";
import { SHARED_HINT, sharedText } from "../../../workbench/sharedRooms";
import { PatchConsent, patchReady } from "../../../workbench/wizards/PatchConsent";
import { ThumbnailCache } from "../../../world/thumbnails";
import { Badge, Budget, Button, Checkbox, InspectorRow, Notice, PanelHead, TextField, Tooltip } from "../../primitives";
import type { PanelHeadSwitcher } from "../../primitives";
import styles from "../Gallery.module.css";

const noop = () => {};

/** Pictures that never come: the gallery has no project, the cells show their map numbers on the world's surface. */
const NO_PICTURES = new ThumbnailCache({ glb: () => new Promise<ArrayBuffer>(noop) }, async () => "");

/** A room plan of 3 × 3 maps with its own area section; the lower right cell is still empty. */
const PLAN: MatrixDto = {
  index: 7,
  name: "Ewigwald",
  width: 3,
  height: 3,
  maps: [342, 343, 344, 345, 346, 347, 348, 349, null],
  headers: [203, 203, 203, 203, 203, 203, 203, 203, 0],
  altitudes: null,
};
/** The „+“ of the plan: the empty cell beside filled ones, the column on the right and the row below. */
const PLAN_PLUS: ReadonlySet<string> = new Set(["2,2", "3,0", "3,1", "3,2", "0,3", "1,3", "2,3"]);
const PLACE = "Ewigwald";
const planColor = (header: number | null) => (header === 203 ? placeColor(PLACE) : null);

const PT_VIEWS = [
  { id: "w0", label: "Sinnoh" },
  { id: "more", label: "Weitere Gebiete" },
  { id: "plan", label: "Raumplan" },
];
const HG_VIEWS = [
  { id: "w0", label: "Johto" },
  { id: "w0k", label: "Kanto" },
  { id: "w0all", label: "Johto und Kanto" },
  { id: "more", label: "Weitere Gebiete" },
];

const MISSING: HeaderPatchDto = { supported: true, applied: false, repairable: false, narc: null };
const BROKEN: HeaderPatchDto = { supported: true, applied: true, repairable: true, narc: "a/0/5/0" };

/** The heads of the world map: the view switcher of Platin (with the room plan of a selected room) and of HeartGold's two halves. */
function WorldMapHeads() {
  const [pt, setPt] = useState("plan");
  const [hg, setHg] = useState("w0k");
  const ptSwitch: PanelHeadSwitcher = { items: PT_VIEWS, value: pt, onChange: setPt };
  const hgSwitch: PanelHeadSwitcher = { items: HG_VIEWS, value: hg, onChange: setHg };
  return (
    <div className={styles.columns}>
      <div className={styles.worldPanel} data-world-view={pt} data-depth="bauen">
        <PanelHead switcher={ptSwitch} subtitle={pt === "plan" ? "Ewigwald, Ebene 1" : undefined} />
      </div>
      <div className={styles.worldPanel} data-world-view={hg} data-depth="bauen">
        <PanelHead switcher={hgSwitch} />
      </div>
    </div>
  );
}

/** The room plan in 64 px: a click picks the map, „+“ says which map it would add. */
function RoomPlan() {
  const [picked, setPicked] = useState("1,1");
  const [said, setSaid] = useState("Ein Klick wählt die Karte, „+“ legt eine an (nicht rückgängig).");
  const selected = useMemo(() => new Set([picked]), [picked]);
  return (
    <>
      <div className={styles.worldPanel} data-world-view="plan" data-depth="bauen">
        <PanelHead switcher={{ items: PT_VIEWS, value: "plan", onChange: noop }} subtitle="Ewigwald, Ebene 1" />
        <div className={styles.worldSurface}>
          <AreaGrid
            matrix={PLAN}
            headers={[]}
            areaFallback={0}
            cellSize={64}
            selected={selected}
            colorOf={planColor}
            onCellClick={(x, y) => setPicked(cellKey(x, y))}
            onExtend={(x, y) => setSaid(`„+“ bei x ${x}, y ${y}: öffnet „Karte bei x ${x}, y ${y} anlegen?“.`)}
            extendable={PLAN_PLUS}
            label={`Raumplan, ${PLAN.height} Zeilen, ${PLAN.width} Spalten`}
            cache={NO_PICTURES}
            depth="bauen"
          />
        </div>
      </div>
      <p className={styles.note}>{said}</p>
    </>
  );
}

function PlaceButton({ index, name, tech }: { index: number; name: string; tech?: string }) {
  return (
    <Button kind="secondary" size="s" data-place-picker="" title="Ort wählen" onClick={noop}>
      <span className={styles.placeDot} style={{ background: placeColor(placeKeyOf(index, name)) }} /* dyn */ />
      {index === 0 ? "Kein Ort" : name}
      {tech && <span className={styles.placeTech}>{tech}</span>}
    </Button>
  );
}

/** The field „Ort“ of an area in Bauen, „Ortsname“ in Tüfteln, and a room without a place. */
function PlacePickers() {
  return (
    <div className={styles.columns}>
      <div className={styles.panelBox} data-depth="bauen">
        <InspectorRow label="Ort">
          <PlaceButton index={1} name="Zweiblattdorf" />
        </InspectorRow>
        <InspectorRow label="Ort">
          <PlaceButton index={0} name="" />
        </InspectorRow>
      </div>
      <div className={styles.panelBox} data-depth="tuefteln">
        <InspectorRow label="Ortsname">
          <PlaceButton index={1} name="Zweiblattdorf" tech="Nachricht 1 · Archiv 0433" />
        </InspectorRow>
      </div>
    </div>
  );
}

const TAKE_ALONG = [
  { id: 411, label: "Zweiblattdorf", checked: true },
  { id: 414, label: "Haus 1", checked: true },
  { id: 415, label: "Haus 1, 1. Stock", checked: true },
  { id: 416, label: "Haus 2", checked: true },
  { id: 418, label: "Haus 3", checked: false },
];

/** „Neuer Ort“ as a still: the name with the note of a long one, the area and rooms that take it along, the actions. */
function NewPlaceStill() {
  const [name, setName] = useState("Neuheim");
  const [choices, setChoices] = useState(TAKE_ALONG);
  const long = nameWarning(name);
  return (
    <section className={styles.dialogStill} aria-label="Neuer Ort" data-new-place="" data-depth="bauen">
      <h3 className={styles.dialogStillTitle}>Neuer Ort</h3>
      <TextField label="Name des Orts" showLabel value={name} onChange={setName} onCommit={setName} />
      {long && <Notice kind="warning">{long}</Notice>}
      <div className={styles.choiceGroup} role="group" aria-label="Mitnehmen">
        <p className={styles.choiceLegend}>Mitnehmen</p>
        {choices.map((c) => (
          <Checkbox key={c.id} label={c.label} checked={c.checked} onChange={(checked) => setChoices((list) => list.map((d) => (d.id === c.id ? { ...d, checked } : d)))} />
        ))}
      </div>
      <div className={styles.dialogStillActions}>
        <Button kind="secondary" onClick={noop}>
          Abbrechen
        </Button>
        <Button kind="primary" disabled={name.trim() === ""} onClick={noop}>
          Ort anlegen
        </Button>
      </div>
    </section>
  );
}

/** The summary of „Gebiet anlegen“ in Bauen with the patch line: „Anlegen“ is free once the box is ticked. */
function PatchLine() {
  const [consent, setConsent] = useState(false);
  return (
    <div className={styles.columns}>
      <div className={styles.summaryStill} data-depth="bauen">
        <p className={styles.dialogStillText}>
          Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an. Neuer Ort „Neuheim“. Aussehen wie Route 201. Eine neue, leere Liste der Figuren und Türen.
        </p>
        <PatchConsent patch={MISSING} checked={consent} onChange={setConsent} />
        <div className={styles.dialogStillActions}>
          <Button kind="primary" disabled={!patchReady(MISSING, consent)} onClick={noop}>
            Anlegen
          </Button>
        </div>
      </div>
      <div className={styles.summaryStill} data-depth="bauen">
        <PatchConsent patch={BROKEN} checked={false} onChange={noop} />
        <div className={styles.dialogStillActions}>
          <Button kind="primary" disabled onClick={noop}>
            Anlegen
          </Button>
        </div>
      </div>
    </div>
  );
}

/** A room whose map 16 other rooms use: the badge in the inspector head, its menu with „Für diesen Raum abspalten…“, and the dialog. */
function SharedRoom() {
  const badge = (
    <Tooltip content={SHARED_HINT}>
      <span className={styles.focusable} tabIndex={0} data-shared="">
        <Badge kind="info" icon="shared">
          {sharedText(16)}
        </Badge>
      </span>
    </Tooltip>
  );
  return (
    <div className={styles.sharedRow}>
      <div className={styles.inspectorWide} data-depth="bauen">
        <InspectorHead
          kind="room"
          title="Pokémon-Center"
          ident="Jubelstadt · C01PC0101"
          badges={badge}
          actions={[
            { label: "Raum hier anbinden…", onSelect: noop },
            { label: "Ort umbenennen…", onSelect: noop },
            { label: "Für diesen Raum abspalten…", onSelect: noop },
          ]}
          depth="bauen"
        />
      </div>
      <section className={styles.dialogStill} aria-label="Raum abspalten?" data-split-room="" data-depth="bauen">
        <h3 className={styles.dialogStillTitle}>Raum abspalten?</h3>
        <div>
          <Badge kind="irreversible" />
        </div>
        <p className={styles.dialogStillText}>
          Der Raum „Pokémon-Center“ bekommt eine eigene Kopie seiner Karte. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopie lässt sich nicht
          zurücknehmen.
        </p>
        <div className={styles.dialogStillActions}>
          <Button kind="secondary" onClick={noop}>
            Abbrechen
          </Button>
          <Button kind="primary" onClick={noop}>
            Abspalten
          </Button>
        </div>
      </section>
    </div>
  );
}

/** The swatch class of each height step (the tokens `szene/hoehe/0..6`). */
const STEP_CLASSES = [styles.hoehe0, styles.hoehe1, styles.hoehe2, styles.hoehe3, styles.hoehe4, styles.hoehe5, styles.hoehe6];

/** The seven height steps of the layer „Höhen“: a step is half a tile (8 E), step 7 starts over at the first colour. */
function HeightSteps() {
  return (
    <ul className={styles.heightSteps} aria-label="Höhenstufen">
      {STEP_CLASSES.map((swatch, step) => (
        <li key={step} className={styles.heightStep} data-height-step={step}>
          <span className={`${styles.heightSwatch} ${swatch}`} aria-hidden="true" />
          Stufe {step} · {step * 8} E
        </li>
      ))}
    </ul>
  );
}

/** The budget in its three states, as the map inspector shows it. */
function Budgets() {
  return (
    <div className={styles.inspectorWide} data-depth="bauen">
      <Budget label="Terrain (BDHC)" used={300} max={36864} state="calm" unit="B" />
      <Budget label="Modell" used={58000} max={61440} state="tight" unit="B" />
      <Budget label="Gebäude" used={33} max={32} state="full" />
    </div>
  );
}

/** „Gelände malen“ on a world map: the tools with the sub-tools as a second group, the brush pressed. */
function PaintTools() {
  const [tool, setTool] = useState<Tool>("paint");
  const [sub, setSub] = useState<PaintSub>("brush");
  const [at, setAt] = useState<ToolbarAnchor>("top");
  return (
    <div className={`${styles.toolbarView} ${styles.toolbarViewFlat} ${styles.paintToolbarView}`} data-depth="bauen">
      <FloatingToolbar
        tools={toolsFor(true)}
        tool={tool}
        onTool={setTool}
        eventsBlocked={null}
        density="komfort"
        anchor={at}
        chosen={at}
        onAnchor={setAt}
        paint={tool === "paint" ? { tools: PAINT_TOOLS, sub, onSub: setSub } : null}
      />
    </div>
  );
}

const exampleTile = (texture: string, role: TileRole): TileJson => ({
  texture,
  texture_index: 0,
  size: [32, 32],
  role,
  kind: 0,
  collision: 0,
  height: role === "ramp" ? "ramp" : role === "wall" ? "wall" : "flat",
  origin: "synthetic",
});
/** A package of every role (no pictures: the gallery has no project, each tile shows the colour of its role). */
const EXAMPLE_TILESET: TilesetDto = {
  package: 65535,
  stored: false,
  tileset: {
    version: 1,
    package: 65535,
    tiles: [
      exampleTile("weg", "ground"),
      exampleTile("erde", "ground"),
      exampleTile("gras_a", "grass"),
      exampleTile("wasser_a", "water"),
      exampleTile("fels", "wall"),
      exampleTile("treppe", "ramp"),
      exampleTile("baum", "piece"),
    ],
  },
};
const NO_PICTURE = async () => null;

/** The palette of a world map, the second ground tile chosen turned a quarter and mirrored. */
function PaletteStill() {
  const [tile, setTile] = useState("0001r1f");
  return (
    <div className={styles.inspectorWide} data-depth="bauen">
      <TilePalette tileset={EXAMPLE_TILESET} tile={tile} onTile={setTile} depth="bauen" image={NO_PICTURE} />
    </div>
  );
}

const EXAMPLE_WORLD: WorldJson = {
  version: 1,
  tileset: 65535,
  layers: [
    { name: "Boden", role: "ground", tiles: [] },
    { name: "Büsche", role: "overlay", tiles: [] },
  ],
  permissions: { derive: true },
};
/** The gallery writes nothing. */
const NO_EDIT = async (_op: LayerOp) => undefined;

/** The layers of a world map: the ground active, the overlay „Büsche“ to hide and remove. */
function LayersStill() {
  const [active, setActive] = useState(0);
  const [hidden, setHidden] = useState<ReadonlySet<number>>(new Set());
  const toggle = (layer: number) =>
    setHidden((h) => {
      const next = new Set(h);
      if (!next.delete(layer)) next.add(layer);
      return next;
    });
  return (
    <div className={styles.inspectorWide} data-depth="bauen">
      <LayerList map={667} world={EXAMPLE_WORLD} active={active} hidden={hidden} onActive={setActive} onToggle={toggle} onError={noop} edit={NO_EDIT} />
    </div>
  );
}

/**
 * Phase 3f „Weltmodell“: the world map with its views and the room plan, places, the patch line and the shared room, with example data;
 * phase 6.2 adds the budget and the height steps, phase 6.4a the paint tools, the palette and the layers.
 */
export function WorldPage() {
  return (
    <div className={styles.stack}>
      <section className={styles.block} data-gallery-state="umschalter">
        <h2 className={styles.blockTitle}>Umschalter der Weltkarte</h2>
        <p className={styles.note}>
          Der Kopf des Reiters „Weltkarte“: links Platin mit einem gewählten Raum, rechts HeartGold mit seinen zwei Hälften. Die Ansicht folgt der Auswahl, das Menü wählt
          sie bis zur nächsten.
        </p>
        <WorldMapHeads />
      </section>

      <section className={styles.block} data-gallery-state="raumplan">
        <h2 className={styles.blockTitle}>Raumplan</h2>
        <p className={styles.note}>Zellen 64 px in der Ortsfarbe, die gewählte Karte mit Kobalt-Kante; „+“ in der leeren Zelle, rechts und unten.</p>
        <RoomPlan />
      </section>

      <section className={styles.block} data-gallery-state="ort">
        <h2 className={styles.blockTitle}>Picker „Ort“</h2>
        <p className={styles.note}>Die Taste öffnet „Ort wählen“ über alle Namen der Bank, mit „Neuer Ort…“ im Fuß. Tüfteln nennt Nachricht und Archiv dahinter.</p>
        <PlacePickers />
      </section>

      <section className={styles.block} data-gallery-state="neuer-ort">
        <h2 className={styles.blockTitle}>Neuer Ort</h2>
        <p className={styles.note}>Ohne Projekt gezeichnet, nicht als schwebender Dialog. Ab 17 Zeichen warnt die Notiz, „Ort anlegen“ braucht einen Namen.</p>
        <NewPlaceStill />
      </section>

      <section className={styles.block} data-gallery-state="patch">
        <h2 className={styles.blockTitle}>Patch-Zeile</h2>
        <p className={styles.note}>Fehlt der Spielcode für neue Orte, fragt die Zusammenfassung einmal; ist er fehlerhaft, zeigt sie den Weg zur Reparatur.</p>
        <PatchLine />
      </section>

      <section className={styles.block} data-gallery-state="budget">
        <h2 className={styles.blockTitle}>Budget</h2>
        <p className={styles.note}>Grenzen des Spiels, ruhig gezeigt: Ruhig ohne Plakette, ab 90 % „Knapp“, über der Grenze „Voll“. Speichern geht immer.</p>
        <Budgets />
      </section>

      <section className={styles.block} data-gallery-state="hoehen">
        <h2 className={styles.blockTitle}>Höhenstufen</h2>
        <p className={styles.note}>
          Die Schicht „Höhen“ färbt jede Kachel nach ihrer Stufe (eine halbe Kachel, 8 E) in sieben Stufen cividis, danach von vorn; Isolinien in Stufe 0 an jeder Kante
          mit Stufenwechsel, auf der höheren Seite. „Höhen“ und „Gelände“ schließen sich aus.
        </p>
        <HeightSteps />
      </section>

      <section className={styles.block} data-gallery-state="malwerkzeuge">
        <h2 className={styles.blockTitle}>Malwerkzeuge</h2>
        <p className={styles.note}>
          „Gelände malen“ auf einer Welt-Karte: die Unterwerkzeuge als zweite Gruppe der Leiste (Pinsel B, Rechteck P, Füllen G, Radierer E, Pipette I, Höhe H, Rampe R),
          genau eins gedrückt.
        </p>
        <PaintTools />
      </section>

      <section className={styles.block} data-gallery-state="palette">
        <h2 className={styles.blockTitle}>Palette</h2>
        <p className={styles.note}>
          Die Kacheln des Pakets nach Rolle, je eine Taste 32 × 32 mit der ganzen Textur; ohne Bild die Farbe der Rolle (hier überall: die Galerie hat kein Projekt). Die
          gewählte Kachel zeigt Drehung und Spiegelung als kleine Marke.
        </p>
        <PaletteStill />
      </section>

      <section className={styles.block} data-gallery-state="ebenen">
        <h2 className={styles.blockTitle}>Ebenen</h2>
        <p className={styles.note}>
          Je Ebene Name und Rolle, „Aktiv“ für die Ebene, die der Pinsel malt, das Auge nur für die Szene; „Entfernen“ nur bei Überlagerungen, darunter „Ebene
          hinzufügen“.
        </p>
        <LayersStill />
      </section>

      <section className={styles.block} data-gallery-state="geteilt">
        <h2 className={styles.blockTitle}>Plakette „Geteilt“</h2>
        <p className={styles.note}>Der Tooltip erklärt die Plakette; „Aktionen“ › „Für diesen Raum abspalten…“ öffnet den Dialog daneben.</p>
        <SharedRoom />
      </section>
    </div>
  );
}
