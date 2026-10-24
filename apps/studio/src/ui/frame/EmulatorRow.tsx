import { Button } from "../primitives";
import styles from "./EmulatorRow.module.css";

export type EmulatorRowProps = {
  /** The chosen emulator; null or undefined while none is chosen. */
  path: string | null | undefined;
  /** Asks for an emulator (the system's file dialog). */
  onPick: () => void;
};

/** The emulator the studio starts a built ROM in, and the way to choose another one. Shared by Einstellungen and Technik. */
export function EmulatorRow({ path, onPick }: EmulatorRowProps) {
  const chosen = path !== null && path !== undefined && path !== "";
  return (
    <div className={styles.row}>
      {chosen ? (
        <span className={styles.path} title={path}>
          {path}
        </span>
      ) : (
        <span className={styles.none}>Kein Emulator gewählt</span>
      )}
      <Button kind="secondary" size="s" onClick={onPick}>
        Emulator wählen
      </Button>
    </div>
  );
}
