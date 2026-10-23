import styles from "./Kbd.module.css";

export type KbdProps = {
  /** The key combination as shown, e.g. "⌘K". */
  keys: string;
  /** Variant for keys shown on the dark hinge bar ("Auf Scharnier"); default is the light variant. */
  onHinge?: boolean;
};

export function Kbd({ keys, onHinge = false }: KbdProps) {
  return <kbd className={onHinge ? `${styles.kbd} ${styles.onHinge}` : styles.kbd}>{keys}</kbd>;
}
