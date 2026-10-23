import styles from "./Kbd.module.css";

export type KbdProps = {
  /** The key combination as shown, e.g. "⌘K". */
  keys: string;
  /** When given, the key hint is a button that opens the hinge for this shortcut. */
  onHinge?: () => void;
};

export function Kbd({ keys, onHinge }: KbdProps) {
  if (!onHinge) return <kbd className={styles.kbd}>{keys}</kbd>;
  return (
    <button type="button" className={styles.hinge} onClick={onHinge}>
      <kbd className={styles.kbd}>{keys}</kbd>
    </button>
  );
}
