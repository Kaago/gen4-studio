import { Badge, Button, IconButton, Kbd } from "../../primitives";
import type { BadgeKind } from "../../primitives";
import styles from "../Gallery.module.css";

const kinds = ["primary", "secondary", "quiet", "play", "danger"] as const;
const badgeKinds: BadgeKind[] = ["unsaved", "irreversible", "locked", "changedOnDisk", "ptHgssOnly", "error", "live", "info"];
const labels = {
  primary: "Speichern",
  secondary: "Abbrechen",
  quiet: "Mehr zeigen",
  play: "Im Spiel testen",
  danger: "Ort löschen",
} as const;

export function ButtonsPage() {
  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Tasten</h2>
        <div className={styles.grid}>
          <span className={styles.gridHead}>Ruhe</span>
          <span className={styles.gridHead}>Mit Symbol und Kürzel</span>
          <span className={styles.gridHead}>Klein</span>
          <span className={styles.gridHead}>Gesperrt</span>
          {kinds.map((kind) => (
            <div key={kind} className={styles.row}>
              <Button kind={kind}>{labels[kind]}</Button>
              <Button kind={kind} icon="save" shortcut="⌘S">
                {labels[kind]}
              </Button>
              <Button kind={kind} size="s">
                {labels[kind]}
              </Button>
              <Button kind={kind} disabled>
                {labels[kind]}
              </Button>
            </div>
          ))}
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Symboltasten</h2>
        <div className={styles.row}>
          <IconButton label="Speichern" icon="save" shortcut="⌘S" onClick={() => {}} />
          <IconButton label="Rückgängig" icon="undo" shortcut="⌘Z" onClick={() => {}} />
          <IconButton label="Gelände malen" icon="paint" pressed shortcut="B" onClick={() => {}} />
          <IconButton label="Auswählen" icon="select" pressed={false} shortcut="V" onClick={() => {}} />
          <IconButton label="Gesperrt" icon="lock" disabled onClick={() => {}} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Tastenkürzel</h2>
        <div className={styles.row}>
          <Kbd keys="⌘K" />
          <Kbd keys="⇧⌘P" />
          <Kbd keys="Esc" />
          <Kbd keys="⌘K" onHinge={() => {}} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Plaketten</h2>
        <div className={styles.row}>
          {badgeKinds.map((kind) => (
            <Badge key={kind} kind={kind} />
          ))}
          <Badge kind="info">Eigener Text</Badge>
        </div>
      </section>
    </div>
  );
}
