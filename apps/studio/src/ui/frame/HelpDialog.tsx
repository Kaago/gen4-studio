import { Dialog } from "../primitives";
import { TERMS, termLabel, termShort } from "../../workbench/terms";
import type { TermKey } from "../../workbench/terms";
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
        {(Object.keys(TERMS) as TermKey[]).map((key) => (
          <div key={key} className={styles.term}>
            <dt className={styles.name}>
              {termLabel(key)}
              {TERMS[key].tech && <span className={styles.tech}> ({TERMS[key].tech})</span>}
            </dt>
            <dd className={styles.short}>{termShort(key)}</dd>
          </div>
        ))}
      </dl>
    </Dialog>
  );
}
