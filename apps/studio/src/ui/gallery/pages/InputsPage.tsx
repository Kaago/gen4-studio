import { useState } from "react";
import { Checkbox, NumberField, Segmented, SelectField, Switch, TextField } from "../../primitives";
import styles from "../Gallery.module.css";

const terrain = [
  { value: 0, label: "Nichts" },
  { value: 1, label: "Hohes Gras", icon: "pin" as const },
  { value: 2, label: "Wasser", icon: "world" as const },
];

const views = [
  { value: "baum", label: "Baum", icon: "tree" as const },
  { value: "liste", label: "Liste", icon: "list" as const },
];

export function InputsPage() {
  const [name, setName] = useState("Route 201");
  const [search, setSearch] = useState("");
  const [note, setNote] = useState("Hier wohnt die Rivalin.");
  const [model, setModel] = useState(100);
  const [flags, setFlags] = useState(0x1f);
  const [hexOnly, setHexOnly] = useState(255);
  const [kind, setKind] = useState<number | string>(1);
  const [unknown, setUnknown] = useState<number | string>(7);
  const [on, setOn] = useState(true);
  const [group, setGroup] = useState<boolean | "mixed">("mixed");
  const [view, setView] = useState("baum");

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Textfeld</h2>
        <p className={styles.note}>Schreibt bei Blur oder Enter, nie je Taste. Escape nimmt die Eingabe zurück. Tab zeigt den Fokusring.</p>
        <div className={styles.fields}>
          <TextField label="Ruhe" showLabel value={name} onCommit={setName} hint="Wird beim Verlassen des Feldes übernommen." />
          <TextField label="Fehler" showLabel value="Ein sehr langer Ortsname" onCommit={() => {}} error="Der Name ist zu lang. Kürze ihn auf 12 Zeichen." />
          <TextField label="Gesperrt" showLabel value="Nur lesbar" onCommit={() => {}} disabled />
          <TextField label="Ort, Raum oder Kürzel suchen" type="search" placeholder="Ort, Raum oder Kürzel suchen" value={search} onCommit={setSearch} />
          <TextField label="Notiz" showLabel multiline value={note} onCommit={setNote} hint="⌘/Strg+Enter übernimmt, Enter bricht die Zeile um." />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Zahlfeld</h2>
        <p className={styles.note}>
          Zieh an der Bezeichnung (oder am Griff) nach links und rechts: 1 je 4 px, mit Umschalt ×10. Pfeiltasten stufen. Eine zu große
          oder unlesbare Eingabe wird nicht übernommen und nie gekürzt.
        </p>
        <div className={styles.fields}>
          <NumberField label="Modell (16 Bit)" showLabel value={model} onCommit={setModel} bits={16} hint="Tippe 70000 und drücke Enter, um die Meldung zu sehen." />
          <NumberField label="Flags (Hex, mit Umschalter)" showLabel value={flags} onCommit={setFlags} bits={16} showBase />
          <NumberField label="Kennung (nur Hex)" showLabel value={hexOnly} onCommit={setHexOnly} base="hex" bits={8} />
          <NumberField label="Ohne Bezeichnung" value={model} onCommit={setModel} bits={16} />
          <NumberField label="Gesperrt" showLabel value={42} onCommit={() => {}} disabled />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Auswahl</h2>
        <div className={styles.fields}>
          <SelectField label="Gelände" showLabel value={kind} options={terrain} onChange={setKind} />
          <SelectField label="Unbekannter Wert" showLabel value={unknown} options={terrain} onChange={setUnknown} />
          <SelectField label="Gesperrt" showLabel value={0} options={terrain} onChange={() => {}} disabled />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Schalter, Kästchen, Segmente</h2>
        <div className={styles.row}>
          <Switch label="Gelände zeigen" checked={on} onChange={setOn} />
          <Switch label="Aus" checked={false} onChange={() => {}} />
          <Switch label="Gesperrt" checked disabled onChange={() => {}} />
        </div>
        <div className={styles.row}>
          <Checkbox label="Alle Ebenen" checked={group} onChange={(next) => setGroup(next)} />
          <Checkbox label="Gewählt" checked onChange={() => {}} />
          <Checkbox label="Gesperrt" checked={false} disabled onChange={() => {}} />
        </div>
        <div className={styles.row}>
          <Segmented ariaLabel="Ansicht" options={views} value={view} onChange={setView} />
          <Segmented ariaLabel="Ansicht klein" size="s" options={views} value={view} onChange={setView} />
        </div>
      </section>
    </div>
  );
}
