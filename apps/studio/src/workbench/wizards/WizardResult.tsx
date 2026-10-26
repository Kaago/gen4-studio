import type { Depth } from "../../ui/frame/shellState";
import { Notice } from "../../ui/primitives";
import { TERMS } from "../terms";
import styles from "./WizardResult.module.css";

interface DoneProps {
  /** What was made, e.g. "Gebiet angelegt". */
  title: string;
  lines: string[];
  next: string[];
}

/** The result step of a wizard that created everything: what it made and what to do next (the frame adds "Zeigen" and "Hier spielen"). */
export function DoneResult({ title, lines, next }: DoneProps) {
  return (
    <div className={styles.result}>
      <Notice kind="success" title={title}>
        {lines.map((line) => (
          <p key={line} className={styles.line}>
            {line}
          </p>
        ))}
      </Notice>
      <h3 className={styles.heading}>Als Nächstes</h3>
      <ul className={styles.next}>
        {next.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}

/** The result step of a wizard that failed after it had created members: what the backend says stayed behind, and where to finish by hand. */
export function PartialResult({ message, depth = "tuefteln" }: { message: string; depth?: Depth }) {
  return (
    <div className={styles.result}>
      <Notice kind="warning" title="Anlegen unterbrochen">
        <p className={styles.line}>{message}</p>
      </Notice>
      <p className={styles.line}>
        Was schon angelegt ist, bleibt bestehen und lässt sich nicht rückgängig machen.{" "}
        {depth === "bauen" ? "Den Rest in der Welt von Hand fertigstellen." : `Den Rest im ${TERMS.matrix.label}-Tab und im ${TERMS.header.label}-Editor von Hand fertigstellen.`}
      </p>
    </div>
  );
}
