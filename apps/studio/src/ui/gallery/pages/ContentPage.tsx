import { lazy, Suspense, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  Badge,
  Button,
  ChangeTable,
  EmptyState,
  IconButton,
  InspectorRow,
  ListRow,
  NumberField,
  Notice,
  PanelHead,
  Reference,
  Section,
  SelectField,
  StoryNode,
  Switch,
  TextField,
  TreeRow,
  useRovingList,
} from "../../primitives";
import type { Area, BadgeKind, MenuEntry, StoryNodeKind } from "../../primitives";
import type { FlagEntryDto, FlagListDto, FlagUsageDto } from "../../../api";
import { FlagDetail } from "../../../flags/FlagDetail";
import { FlagList } from "../../../flags/FlagList";
import { sampleThreadGraph } from "../../../stories/graph/sampleGraph";
import styles from "../Gallery.module.css";

const places = [
  { id: "a", label: "Zweiblattdorf", tech: "0412", icon: "world" as const },
  { id: "b", label: "Route 201", tech: "0413", icon: "outside" as const },
  { id: "c", label: "Haus der Rivalin", tech: "0414", icon: "room" as const },
  { id: "d", label: "Ein sehr langer Ortsname, der abgeschnitten werden muss", tech: "0415", icon: "room" as const },
];

type Node = { id: string; label: string; children?: Node[] };
const tree: Node[] = [
  {
    id: "sinnoh",
    label: "Sinnoh",
    children: [
      { id: "zweiblatt", label: "Zweiblattdorf", children: [{ id: "haus", label: "Haus der Rivalin" }, { id: "labor", label: "Labor" }] },
      { id: "r201", label: "Route 201" },
    ],
  },
  { id: "kanto", label: "Kanto" },
];

const areas: { area: Area; label: string; tech: string }[] = [
  { area: "welt", label: "Zweiblattdorf", tech: "0412" },
  { area: "geschichten", label: "Rivalin trifft", tech: "script 12" },
  { area: "pokemon", label: "Staralili", tech: "Nummer 0396" },
  { area: "gegenstaende", label: "Trank", tech: "Nummer 0017" },
  { area: "texte", label: "Textarchiv 0412", tech: "msg 412" },
  { area: "technik", label: "Header-Patch", tech: "arm9" },
];

/** The context menu of a place row: the last place is new, so it has no original. */
const placeMenu = (id: string): MenuEntry[] => [
  { label: "Auf Original zurücksetzen…", icon: "undo", onSelect: () => {}, ...(id === "d" && { disabled: true, detail: "Kein Original" }) },
];
/** The context menu of a tree row: two targets side by side, each with its own label. */
const treeMenu: MenuEntry[] = [
  { label: "Karte auf Original zurücksetzen…", icon: "undo", onSelect: () => {} },
  { label: "Header auf Original zurücksetzen…", icon: "undo", onSelect: () => {} },
  "separator",
  { label: "Eventdatei auf Original zurücksetzen…", icon: "undo", onSelect: () => {}, disabled: true, detail: "Basis-ROM fehlt" },
];

/** One number of Technik › Flags und Variablen: an own name over the pret name, set by a script and checked by an object. */
const flagEntry: FlagEntryDto = {
  number: 0x94,
  name: "FLAG_UNUSED_0x0094",
  aliases: [],
  own_id: "FLAG_HAT_MEDAILLE",
  description: "Die Medaille ist abgeholt.",
  area: "general",
  in_save: true,
  usage: { read: 1, write: 1, unknown: 0, sources: 2 },
  free: false,
  reserved: null,
  places: 0,
};
const flagListExample: FlagListDto = {
  kind: "flag",
  entries: [flagEntry],
  pool_free: 135,
  pool_total: 152,
  counts: { script_local: 1234, dynamic: 17 },
  scripts_indexed: true,
  text_project: true,
  family: "Pt",
  ms: 4,
};
const flagUsageExample: FlagUsageDto = {
  kind: "flag",
  number: 0x94,
  name: flagEntry.name,
  own_id: flagEntry.own_id,
  description: flagEntry.description,
  area: "general",
  in_save: true,
  usage: flagEntry.usage,
  reserved: null,
  refs: [
    {
      source: { kind: "Script", index: 412 },
      field: "SetFlag",
      slot: 1,
      access: "write",
      label: "Script 1 · SetFlag",
      source_name: "Scriptdatei 0412",
      nav: { kind: "script", file: 412, line: 4 },
      header: 412,
      header_name: "Zweiblattdorf",
      value: null,
    },
    {
      source: { kind: "Event", index: 412 },
      field: "hidden_flag",
      slot: 3,
      access: "read",
      label: "Objekt 3 · Sichtbarkeits-Flag",
      source_name: "Eventdatei 0412",
      nav: { kind: "event", eventFile: 412, eventKind: "object", index: 3, header: 412 },
      header: 412,
      header_name: "Zweiblattdorf",
      value: null,
    },
  ],
  ms: 2,
};

/** Technik › Flags und Variablen: a row of the list and the detail of the chosen number with its two ways. */
function FlagsExample() {
  const [selected, setSelected] = useState<number | null>(0x94);
  return (
    <div className={styles.columns}>
      <FlagList list={flagListExample} entries={flagListExample.entries} selected={selected} onSelect={setSelected} />
      <FlagDetail
        kind="flag"
        number={selected}
        entry={flagEntry}
        usage={flagUsageExample}
        busy={false}
        error={null}
        textProject
        readOnly={false}
        depth="bauen"
        goTo={() => {}}
        onRename={() => {}}
        onName={() => {}}
        onDescribe={() => {}}
      />
    </div>
  );
}

/** The seven kinds of a story's step with a name each, as the graph of a story shows them on their values. */
const storySteps: { kind: StoryNodeKind; title: string }[] = [
  { kind: "start", title: "Aufbruch" },
  { kind: "dialog", title: "Rivalin trifft" },
  { kind: "condition", title: "Hat Pokédex" },
  { kind: "battle", title: "Kampf am See" },
  { kind: "reward", title: "Pokédex bekommen" },
  { kind: "progress", title: "Labor besucht" },
  { kind: "appearance", title: "Rivalin verschwindet" },
];

// React Flow and dagre load with the graph, in their own chunk.
const ThreadGraph = lazy(() => import("../../../stories/graph/ThreadGraph"));

function StoryNodesExample() {
  const [selected, setSelected] = useState<string | null>("p:event:0394");
  return (
    <>
      <h3 className={styles.storyHead}>Ruhe</h3>
      <div className={styles.storyNodes}>
        {storySteps.map((step, i) => (
          <StoryNode key={step.kind} kind={step.kind} title={step.title} subtitle={`Wert ${i}`} />
        ))}
      </div>
      <h3 className={styles.storyHead}>Gewählt</h3>
      <div className={styles.storyNodes}>
        {storySteps.map((step, i) => (
          <StoryNode key={step.kind} kind={step.kind} title={step.title} subtitle={`Wert ${i}`} selected />
        ))}
      </div>
      <h3 className={styles.storyHead}>Nur notiert, Wert, Ort, Ort ohne Wert, Ort als Taste</h3>
      <div className={styles.storyNodes}>
        <StoryNode kind="reward" title="Pokédex bekommen" subtitle="Wert 7" notedOnly />
        <StoryNode kind="value" title="4" />
        <StoryNode kind="place" title="Route 201" subtitle="Eventdatei 0394" />
        <StoryNode kind="place" title="Scriptdatei 0200" loose />
        <StoryNode kind="place" title="Zweiblattdorf" subtitle="Scriptdatei 0101" onOpen={() => {}} />
      </div>
      <h3 className={styles.storyHead}>Graph eines Fadens</h3>
      <div className={styles.graphBox}>
        <Suspense fallback={<p className={styles.note}>Graph wird geladen…</p>}>
          <ThreadGraph graph={sampleThreadGraph} selected={selected} onSelect={setSelected} onOpenPlace={() => {}} />
        </Suspense>
      </div>
    </>
  );
}

const badgeKinds: BadgeKind[] = ["unsaved", "irreversible", "locked", "changedOnDisk", "ptHgssOnly", "error", "live", "info"];

function TreeList() {
  const ref = useRef<HTMLDivElement>(null);
  useRovingList(ref);
  const [open, setOpen] = useState<Record<string, boolean>>({ sinnoh: true, zweiblatt: false });
  const [selected, setSelected] = useState("zweiblatt");
  const rows: ReactNode[] = [];
  const walk = (nodes: Node[], depth: number) => {
    for (const node of nodes) {
      const isOpen = node.children ? Boolean(open[node.id]) : undefined;
      rows.push(
        <TreeRow
          key={node.id}
          label={node.label}
          depth={depth}
          expanded={isOpen}
          onToggle={() => setOpen((o) => ({ ...o, [node.id]: !o[node.id] }))}
          selected={selected === node.id}
          onSelect={() => setSelected(node.id)}
          contextMenu={node.children ? undefined : treeMenu}
        />,
      );
      if (node.children && isOpen) walk(node.children, depth + 1);
    }
  };
  walk(tree, 0);
  return (
    <div ref={ref} role="tree" aria-label="Orte als Baum" className={styles.panelBox}>
      {rows}
    </div>
  );
}

function PlaceList() {
  const ref = useRef<HTMLDivElement>(null);
  useRovingList(ref);
  const [selected, setSelected] = useState("b");
  return (
    <div ref={ref} role="listbox" aria-label="Orte als Liste" className={styles.panelBox}>
      {places.map((p) => (
        <ListRow
          key={p.id}
          label={p.label}
          tech={p.tech}
          icon={p.icon}
          selected={selected === p.id}
          onSelect={() => setSelected(p.id)}
          trailing={p.id === "a" ? <Badge kind="unsaved">3</Badge> : undefined}
          contextMenu={placeMenu(p.id)}
        />
      ))}
    </div>
  );
}

export function ContentPage() {
  const [sections, setSections] = useState({ basis: true, ereignisse: true, leer: false });
  const toggle = (key: keyof typeof sections) => setSections((s) => ({ ...s, [key]: !s[key] }));
  const [model, setModel] = useState(100);
  const [name, setName] = useState("Zweiblattdorf");
  const [kind, setKind] = useState<number | string>(1);
  const [visible, setVisible] = useState(true);

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Liste und Baum</h2>
        <p className={styles.note}>
          Tab landet auf genau einer Zeile; Pfeil hoch und runter bewegen sie, Pos1 und Ende springen an die Enden. Im Baum öffnet Pfeil
          rechts und schließt Pfeil links einen Ast. Rechtsklick, Umschalt+F10 oder die Kontextmenü-Taste öffnen das Kontextmenü einer
          Zeile (in der Liste jede Zeile, im Baum die Blätter).
        </p>
        <div className={styles.columns}>
          <PlaceList />
          <TreeList />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Panelkopf, Abschnitte und Inspector-Zeilen</h2>
        <div className={styles.panelBox}>
          <PanelHead
            title="Ort"
            subtitle="Zweiblattdorf · 0412"
            actions={
              <>
                <IconButton label="Aktualisieren" icon="redo" onClick={() => {}} />
                <IconButton label="Mehr" icon="list" onClick={() => {}} />
              </>
            }
          />
          <Section title="Grunddaten" open={sections.basis} onToggle={() => toggle("basis")}>
            <InspectorRow label="Name">
              <TextField label="Name" value={name} onCommit={setName} />
            </InspectorRow>
            <InspectorRow label="Modell" deviates hint="Weicht vom Original (100) ab.">
              <NumberField label="Modell" value={model} onCommit={setModel} bits={16} />
            </InspectorRow>
            <InspectorRow label="Gelände">
              <SelectField
                label="Gelände"
                value={kind}
                options={[
                  { value: 0, label: "Nichts" },
                  { value: 1, label: "Hohes Gras" },
                ]}
                onChange={setKind}
              />
            </InspectorRow>
            <InspectorRow label="Sichtbar">
              <Switch label="Sichtbar" checked={visible} onChange={setVisible} />
            </InspectorRow>
          </Section>
          <Section title="Ereignisse" count={4} open={sections.ereignisse} onToggle={() => toggle("ereignisse")}>
            <InspectorRow label="Figur">
              <Reference area="geschichten" label="Rivalin" tech="script 12" onOpen={() => {}} />
            </InspectorRow>
          </Section>
          <Section title="Zugeklappt" count={0} open={sections.leer} onToggle={() => toggle("leer")}>
            <InspectorRow label="Nichts">–</InspectorRow>
          </Section>
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Inspector-Zeilen, schmal</h2>
        <p className={styles.note}>So schmal wie der Inspector der Werkbank bei 1100 × 720 (280 px): die Beschriftung steht über dem Feld, ein Verweis behält seinen Namen und „Ändern…“ bleibt daneben.</p>
        <div className={styles.inspectorNarrow} data-gallery-state="inspector-schmal">
          <InspectorRow label="Ort">
            <span className={styles.inlineRow}>
              <Reference area="welt" label="Zweiblattdorf" tech="0412" onOpen={() => {}} />
              <Button kind="quiet" size="s" onClick={() => {}}>
                Ändern…
              </Button>
            </span>
          </InspectorRow>
          <InspectorRow label="Modell" deviates hint="Weicht vom Original (100) ab.">
            <NumberField label="Modell" value={model} onCommit={setModel} bits={16} />
          </InspectorRow>
          <InspectorRow label="Sichtbar">
            <Switch label="Sichtbar" checked={visible} onChange={setVisible} />
          </InspectorRow>
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Leerer Zustand</h2>
        <div className={styles.panelBox}>
          <EmptyState what="Noch kein Ort gewählt" why="Wähle links einen Ort, dann zeigt dieses Panel seine Daten." action={{ label: "Ersten Ort wählen", onClick: () => {} }} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Hinweise</h2>
        <div className={styles.fields}>
          <Notice kind="info" title="Zur Info">Projekte sind entpackte DSPRE-Ordner.</Notice>
          <Notice kind="success">Alles stimmig, gebaut vor 4 Minuten.</Notice>
          <Notice kind="warning" title="Nicht rückgängig">Das Anwenden des Patches lässt sich nicht zurücknehmen.</Notice>
          <Notice
            kind="error"
            title="Die Tür führt ins Leere"
            actions={[
              { label: "Ziel wählen", onClick: () => {} },
              { label: "Tür löschen", onClick: () => {} },
            ]}
          >
            Wähle ein Ziel oder lösche die Tür.
          </Notice>
        </div>
      </section>

      <section className={styles.block} data-gallery-state="change-table">
        <h2 className={styles.blockTitle}>Änderungen</h2>
        <p className={styles.note}>Feld: alt → neu. Entfernte Wörter durchgestrichen, neue unterstrichen; eine fehlende Seite liest sich „fehlt“.</p>
        <ChangeTable
          columns={["Feld", "Commit", "Projekt"]}
          ariaLabel="Trainer 0001 gegen den letzten Commit"
          rows={[
            { key: "level", field: "data.party[0].level", old: "5", new: "7" },
            { key: "weather", field: "Wetter", title: "weather_id", old: "0", new: "3" },
            { key: "item", field: "data.items[1]", old: null, new: "ITEM_TOP_TRANK" },
            {
              key: "text",
              field: "2",
              old: [
                { op: "same", text: "Willkommen in " },
                { op: "removed", text: "Zweiblattdorf" },
                { op: "same", text: "!" },
              ],
              new: [
                { op: "same", text: "Willkommen in " },
                { op: "added", text: "Sandgemme" },
                { op: "same", text: "!" },
              ],
            },
          ]}
          more="und 3 weitere Felder"
        />
      </section>

      <section className={styles.block} data-gallery-state="flags">
        <h2 className={styles.blockTitle}>Flags und Variablen</h2>
        <p className={styles.note}>Eine Zeile der Liste (eigener Name fett über dem pret-Namen, rechts die leere Spalte „Live“) und das Detail mit „Wo gesetzt“ und „Wo geprüft“.</p>
        <FlagsExample />
      </section>

      <section className={styles.block} data-gallery-state="story-nodes">
        <h2 className={styles.blockTitle}>Story-Knoten</h2>
        <p className={styles.note}>
          Die sieben Arten eines Schritts, je mit Symbol und Wort (nie Farbe allein), in Ruhe und gewählt; ein Wert, den nur die Geschichte
          nennt, ist gestrichelt und trägt „nur notiert“. Darunter ein Faden als Graph: links die Orte, die einen Wert setzen, rechts die,
          die ihn prüfen; ein gewählter Ort öffnet sich mit „Öffnen“.
        </p>
        <StoryNodesExample />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Plaketten</h2>
        <div className={styles.row}>
          {badgeKinds.map((kindName) => (
            <Badge key={kindName} kind={kindName} />
          ))}
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Verweise</h2>
        <p className={styles.note}>Klick springt zum Ziel, Rechtsklick listet, wo es verwendet wird.</p>
        <div className={styles.row}>
          {areas.map((a, index) => (
            <Reference
              key={a.area}
              area={a.area}
              label={a.label}
              tech={a.tech}
              onOpen={() => {}}
              usedIn={index % 2 === 0 ? [{ label: "Zweiblattdorf", onOpen: () => {} }, { label: "Route 201", onOpen: () => {} }] : []}
            />
          ))}
        </div>
      </section>
    </div>
  );
}
