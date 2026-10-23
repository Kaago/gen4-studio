import type { ReactNode } from "react";
import { ToastHost } from "./ToastHost";
import styles from "./Shell.module.css";

export type ShellProps = {
  title: ReactNode;
  atlas: ReactNode;
  /** The list column of the area (left of the deck). */
  list?: ReactNode;
  /** The deck of the area. */
  deck?: ReactNode;
  /**
   * A surface across list and deck (Start, Technik, Einstellungen). It replaces them on screen; they stay mounted
   * (hidden) so a dock and its scene survive a visit to another area.
   */
  pane?: ReactNode;
  status: ReactNode;
};

/** The grid of the app: title bar on top, the atlas on the left, list and deck (or one pane) in the middle, the status line below. */
export function Shell({ title, atlas, list, deck, pane, status }: ShellProps) {
  const covered = pane !== undefined;
  return (
    <div className={styles.shell}>
      <header className={styles.title}>{title}</header>
      <div className={styles.atlas}>{atlas}</div>
      {list !== undefined && (
        <div className={styles.list} hidden={covered}>
          {list}
        </div>
      )}
      {deck !== undefined && (
        <main className={styles.deck} hidden={covered}>
          {deck}
        </main>
      )}
      {covered && <main className={styles.pane}>{pane}</main>}
      <footer className={styles.status}>{status}</footer>
      <ToastHost />
    </div>
  );
}
