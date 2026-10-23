import { useMemo, useState } from "react";
import { Button, Picker, Reference } from "../../primitives";
import type { PickerItem } from "../../primitives";
import styles from "../Gallery.module.css";

const preview = <span className={styles.placeholder} />;

const places: PickerItem[] = [
  { id: 410, label: "Jubelstadt", tech: "T01R0201", area: "welt", group: "Städte", preview, count: "3 Räume" },
  { id: 411, label: "Zweiblattdorf", tech: "T01R0101", area: "welt", group: "Städte", preview, count: "5 Räume" },
  { id: 412, label: "Sandgemme", tech: "T01R0102", area: "welt", group: "Städte", preview, count: "2 Räume" },
  { id: 420, label: "Route 201", tech: "R201", area: "welt", group: "Routen", preview, count: "8 Events" },
  { id: 421, label: "Route 202", tech: "R202", area: "welt", group: "Routen", preview, count: "11 Events" },
  { id: 422, label: "Route 203", tech: "R203", area: "welt", group: "Routen", preview, count: "6 Events" },
  { id: 430, label: "Ewigwald", tech: "D010", area: "welt", group: "Höhlen und Wälder", preview },
  { id: 431, label: "Kraterberg", tech: "D011", area: "welt", group: "Höhlen und Wälder", preview },
];

const texts: PickerItem[] = [
  { id: 412, label: "Zweiblattdorf", tech: "Textarchiv 0412", area: "texte" },
  { id: 413, label: "Sandgemme", tech: "Textarchiv 0413", area: "texte" },
  { id: 427, label: "Route 201", tech: "Textarchiv 0427", area: "texte" },
];

const commands: PickerItem[] = [
  { id: "cmd-open", label: "Projekt öffnen", group: "Befehle", icon: "open", shortcut: "⌘O" },
  { id: "cmd-save", label: "Speichern", group: "Befehle", icon: "save", shortcut: "⌘S", description: "Schreibt alle Änderungen ins Projekt" },
  { id: "cmd-build", label: "Bauen", group: "Befehle", icon: "build", shortcut: "⌘B" },
  { id: "cmd-play", label: "Hier spielen", group: "Befehle", icon: "play", shortcut: "⌘R" },
  { id: "cmd-theme", label: "Thema wechseln", group: "Befehle", icon: "moon" },
  { id: "cmd-density", label: "Dichte wechseln", group: "Befehle", icon: "settings" },
];

const paletteItems: PickerItem[] = [
  ...commands,
  ...places.map((p) => ({ ...p, preview: undefined, count: undefined })),
  ...texts.map((t) => ({ ...t, id: `text-${t.id}`, group: "Textarchive" })),
];

const kinds = [
  { value: "alle", label: "Alle" },
  { value: "Städte", label: "Städte" },
  { value: "Routen", label: "Routen" },
];

export function PickerPage() {
  const [which, setWhich] = useState<"closed" | "places" | "texts" | "empty" | "palette">("closed");
  const [filter, setFilter] = useState("alle");
  const [picked, setPicked] = useState<PickerItem | null>(null);
  const shown = useMemo(() => (filter === "alle" ? places : places.filter((p) => p.group === filter)), [filter]);
  const close = (open: boolean) => !open && setWhich("closed");

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Picker</h2>
        <p className={styles.note}>
          Tippen filtert sofort über Name und Kennung, eine Zahl findet auch „0411“ als 411. Der erste Treffer ist markiert, Pfeiltasten bewegen, Enter wählt,
          Escape schließt ohne zu wählen. Die Befehlspalette ist die Variante „palette“: oben verankert, höchstens drei Treffer je Gruppe, ohne Eingabe nur
          Befehle, der Suchtext ist im Treffer markiert.
        </p>
        <div className={styles.row}>
          <Button kind="secondary" onClick={() => setWhich("places")}>Header wählen …</Button>
          <Button kind="secondary" onClick={() => setWhich("texts")}>Textarchiv wählen …</Button>
          <Button kind="secondary" onClick={() => setWhich("empty")}>Leere Liste …</Button>
          <Button kind="secondary" shortcut="⌘K" onClick={() => setWhich("palette")}>Befehlspalette</Button>
        </div>
        <div className={styles.row}>
          <span className={styles.note}>Gewählt:</span>
          {picked?.area ? (
            <Reference area={picked.area} label={picked.label} tech={picked.tech} onOpen={() => {}} />
          ) : (
            <span className={styles.note}>{picked ? picked.label : "Noch nichts."}</span>
          )}
        </div>
      </section>

      <Picker
        open={which === "places"}
        onOpenChange={close}
        title="Header wählen"
        items={shown}
        filters={kinds}
        filter={filter}
        onFilter={setFilter}
        onPick={setPicked}
        placeholder="Name oder Kennung …"
        footer="Vorschauen sind Platzhalter; in der App kommen sie aus dem geöffneten Projekt."
      />
      <Picker
        open={which === "palette"}
        onOpenChange={close}
        title="Befehlspalette"
        variant="palette"
        items={paletteItems}
        limitPerGroup={3}
        groupOrder={["Befehle", "Städte", "Routen", "Höhlen und Wälder", "Textarchive"]}
        emptyQueryGroups={["Befehle"]}
        onPick={setPicked}
        placeholder="Suchen oder Befehl eingeben …"
      />
      <Picker open={which === "texts"} onOpenChange={close} title="Textarchiv wählen" items={texts} onPick={setPicked} />
      <Picker
        open={which === "empty"}
        onOpenChange={close}
        title="Gegenstand wählen"
        items={[]}
        onPick={setPicked}
        emptyText="Dieses Projekt hat noch keine Gegenstände. Lege einen im Gegenstand-Editor an."
      />
    </div>
  );
}
