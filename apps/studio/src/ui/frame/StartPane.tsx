import { useRef } from "react";
import { Button, ListRow, useRovingList } from "../primitives";
import { ago, useNow } from "./timeAgo";
import type { RecentProject } from "./shellState";
import styles from "./StartPane.module.css";

export type StartPaneProps = {
  recent: readonly RecentProject[];
  /** Paths that could not be opened: their rows say so and offer to leave the list. */
  missing: ReadonlySet<string>;
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onForget: (path: string) => void;
};

/** The surface of the area Start: a greeting, the recently opened projects, and the way to open another one. */
export function StartPane({ recent, missing, onOpen, onOpenRecent, onForget }: StartPaneProps) {
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const now = useNow();
  return (
    <section className={styles.pane} aria-labelledby="start-title">
      <h1 id="start-title" className={styles.title}>
        Willkommen zurück
      </h1>
      {recent.length > 0 ? (
        <div ref={list} role="listbox" aria-label="Zuletzt geöffnete Projekte" className={styles.list}>
          {recent.map((p) => (
            <ListRow
              key={p.path}
              label={p.name}
              selected={false}
              onSelect={() => onOpenRecent(p.path)}
              dataAttrs={{ "data-recent": p.path }}
              trailing={
                <>
                  <span className={styles.game}>{p.game}</span>
                  {missing.has(p.path) ? (
                    <>
                      <span className={styles.missing}>nicht gefunden</span>
                      <Button
                        kind="secondary"
                        size="s"
                        onClick={(e) => {
                          e.stopPropagation();
                          onForget(p.path);
                        }}
                      >
                        Aus der Liste nehmen
                      </Button>
                    </>
                  ) : (
                    <span className={styles.time} title={p.path}>
                      {ago(p.openedAt, now)}
                    </span>
                  )}
                </>
              }
            />
          ))}
        </div>
      ) : (
        <p className={styles.hint}>Noch kein Projekt geöffnet.</p>
      )}
      <div className={styles.actions}>
        <Button kind="primary" onClick={onOpen}>
          Projekt öffnen
        </Button>
      </div>
      <p className={styles.hint}>Projekte sind entpackte DSPRE-Ordner.</p>
    </section>
  );
}
