import { useMemo, useState } from "react";
import { Badge, DSBattlePreview, DSPokedexProfile, DSScreen, Segmented, Tabs } from "../../primitives";
import type { BadgeKind } from "../../primitives";
import { pages as textPages } from "../../../workbench/text/textModel";
import styles from "../Gallery.module.css";

const sample = "Hallo {COLOR, 0, 5}Rivalin\\x0123!\\nWillkommen in Zweiblattdorf.\\rKomm bald wieder.";
const signSample = "Zweiblattdorf\\nAn der Küste der Ruhe";

const badgeKinds: BadgeKind[] = ["success", "warning", "code", "neutral"];

const files = [
  { id: "a", label: "Zweiblattdorf", count: 12, closeLabel: "Zweiblattdorf schließen" },
  { id: "b", label: "Route 201", unsaved: true, closeLabel: "Route 201 schließen" },
  { id: "c", label: "Labor", closeLabel: "Labor schließen" },
];

const mapStub = <div className={styles.mapStub} />;

// Placeholders only: the gallery shows no game pictures.
const hpOptions = [
  { value: "1", label: "Voll" },
  { value: "0.4", label: "40 %" },
  { value: "0.1", label: "10 %" },
];

export function ScreenPage() {
  const dialog = useMemo(() => textPages(sample), []);
  const sign = useMemo(() => textPages(signSample), []);
  const [page, setPage] = useState("0");
  const [tab, setTab] = useState("a");
  const [hp, setHp] = useState("0.4");
  const [open, setOpen] = useState(files);
  const options = dialog.map((_, i) => ({ value: String(i), label: `Seite ${i + 1}` }));
  const close = (id: string) => {
    setOpen((list) => list.filter((f) => f.id !== id));
    if (tab === id) setTab(open.find((f) => f.id !== id)?.id ?? "");
  };

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Textbox</h2>
        <p className={styles.note}>Zwei Zeilen wie im Spiel; ▼ zeigt, dass die Box auf eine Taste wartet. Der Maßstab ist ganzzahlig.</p>
        <Segmented options={options} value={page} onChange={setPage} ariaLabel="Seite der Textbox" size="s" />
        <div className={styles.row}>
          <DSScreen variant="textbox" pages={dialog} page={Number(page)} scale={1} background={mapStub} />
          <DSScreen variant="textbox" pages={dialog} page={Number(page)} scale={2} background={mapStub} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Schild und leerer Bildschirm</h2>
        <div className={styles.row}>
          <DSScreen variant="sign" pages={sign} page={0} scale={1} background={mapStub} />
          <DSScreen variant="empty" pages={[]} page={0} scale={1} background={mapStub} />
          <DSScreen variant="sign" pages={sign} page={0} scale={2} background={mapStub} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Pokédex-Profil</h2>
        <p className={styles.note}>
          Bild auf dem Sockel, rechts Nummer, Name, Typen und Daten, unten der Pokédex-Text. Fehlt ein Wert, steht „folgt“; ohne Bild der
          Anfangsbuchstabe.
        </p>
        <div className={styles.row}>
          <DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} />
          <DSPokedexProfile
            number={393}
            name="Plinfa"
            types={[11]}
            spriteUrl={null}
            category="Pinguin"
            height="0,4 m"
            weight="5,2 kg"
            text={"Ein stolzes Pokémon, das\nkeine Hilfe annimmt."}
            scale={2}
          />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Kampfvorschau</h2>
        <p className={styles.note}>
          Gegner oben rechts mit Statuskasten oben links, eigenes Pokémon unten links. Der KP-Balken ist über der Hälfte grün, über einem
          Fünftel gelb, sonst rot. Ohne Level zeigt der Kasten nur den Namen (ein Trainer vor dem Kampf).
        </p>
        <Segmented options={hpOptions} value={hp} onChange={setHp} ariaLabel="KP des eigenen Pokémon" size="s" />
        <div className={styles.row}>
          <DSBattlePreview
            foe={{ name: "Staravia", level: 12, spriteUrl: null }}
            own={{ name: "Panflam", level: 9, spriteUrl: null, hp: Number(hp) }}
            scale={1}
          />
          <DSBattlePreview foe={{ name: "Ass-Trainer Lutz", level: 0, spriteUrl: null }} own={null} scale={1} />
          <DSBattlePreview
            foe={{ name: "Staravia", level: 12, spriteUrl: null }}
            own={{ name: "Panflam", level: 9, spriteUrl: null, hp: Number(hp) }}
            scale={2}
          />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Reiter</h2>
        <p className={styles.note}>Pfeiltasten wechseln den Reiter. Entf oder Befehl+W schließt den fokussierten Reiter.</p>
        <div className={styles.panelBox}>
          <Tabs items={open} value={tab} onChange={setTab} onClose={close} ariaLabel="Offene Texte">
            <p>Inhalt von {open.find((f) => f.id === tab)?.label}</p>
          </Tabs>
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Weitere Plaketten</h2>
        <div className={styles.row}>
          {badgeKinds.map((kind) => (
            <Badge key={kind} kind={kind} />
          ))}
        </div>
      </section>
    </div>
  );
}
