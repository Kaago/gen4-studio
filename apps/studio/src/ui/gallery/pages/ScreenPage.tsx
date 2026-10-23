import { useMemo, useState } from "react";
import { Badge, DSScreen, Segmented, Tabs } from "../../primitives";
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

export function ScreenPage() {
  const dialog = useMemo(() => textPages(sample), []);
  const sign = useMemo(() => textPages(signSample), []);
  const [page, setPage] = useState("0");
  const [tab, setTab] = useState("a");
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
