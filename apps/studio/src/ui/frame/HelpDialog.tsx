import { Dialog } from "../primitives";
import { TERMS } from "../../workbench/terms";
import styles from "./HelpDialog.module.css";

export type HelpDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** The help of the app: the glossary, every word of the world explorer with its technical name and explanation. */
export function HelpDialog({ open, onOpenChange }: HelpDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Hilfe" description="Die Begriffe, mit denen das Studio die Welt des Spiels beschreibt.">
      <dl className={styles.terms}>
        {Object.entries(TERMS).map(([key, term]) => (
          <div key={key} className={styles.term}>
            <dt className={styles.name}>
              {term.label}
              {term.tech && <span className={styles.tech}> ({term.tech})</span>}
            </dt>
            <dd className={styles.short}>{term.short}</dd>
          </div>
        ))}
      </dl>
    </Dialog>
  );
}
