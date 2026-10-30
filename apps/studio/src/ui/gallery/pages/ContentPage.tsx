import { useRef, useState } from "react";
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
  Switch,
  TextField,
  TreeRow,
  useRovingList,
} from "../../primitives";
import type { Area, BadgeKind, MenuEntry } from "../../primitives";
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
