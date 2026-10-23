import { Fragment } from "react";
import type { ReactNode } from "react";
import { ToastHost } from "./ToastHost";
import { AREAS } from "./shellState";
import type { AreaId } from "./shellState";
import styles from "./Shell.module.css";

/** What an area with a deck brings into the shell: its list column (left of the deck) and its deck. */
export type ShellArea = { list?: ReactNode; deck: ReactNode };

export type ShellProps = {
  title: ReactNode;
  atlas: ReactNode;
  /** The active area: its list and deck show, the other areas' are hidden. */
  area: AreaId;
  /**
   * The areas with a list and a deck. Every area given stays mounted (hidden while another one is active), so its docks, its scene and
   * its editors survive a visit to another area; an area is given from the first time it shows.
   */
  areas: Partial<Record<AreaId, ShellArea>>;
  /** A surface across list and deck (Start, Technik, Einstellungen). It covers every area, which stay mounted (hidden). */
  pane?: ReactNode;
  status: ReactNode;
};

/** The grid of the app: title bar on top, the atlas on the left, list and deck of the active area (or one pane) in the middle, the status line below. */
export function Shell({ title, atlas, area, areas, pane, status }: ShellProps) {
  const covered = pane !== undefined;
  return (
    <div className={styles.shell}>
      <header className={styles.title}>{title}</header>
      <div className={styles.atlas}>{atlas}</div>
      {AREAS.map((id) => {
        const given = areas[id];
        if (given === undefined) return null;
        const hidden = covered || id !== area;
        return (
          <Fragment key={id}>
            {given.list !== undefined && (
              <div className={styles.list} data-area={id} data-part="list" hidden={hidden}>
                {given.list}
              </div>
            )}
            <main className={styles.deck} data-area={id} data-part="deck" hidden={hidden}>
              {given.deck}
            </main>
          </Fragment>
        );
      })}
      {covered && <main className={styles.pane}>{pane}</main>}
      <footer className={styles.status}>{status}</footer>
      <ToastHost />
    </div>
  );
}
