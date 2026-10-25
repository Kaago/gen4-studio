import { useId, useRef } from "react";
import type { ReactNode } from "react";
import { Badge, Button, DSScreen, Icon, Menu, useRovingList } from "../primitives";
import { ago, useNow } from "./timeAgo";
import type { RecentProject } from "./shellState";
import styles from "./StartPane.module.css";

export type StartPaneProps = {
  recent: readonly RecentProject[];
  /** Paths that could not be opened: their cards say so and offer to leave the list. */
  missing: ReadonlySet<string>;
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onForget: (path: string) => void;
  /** "Neues Spiel aus ROM…": with it, that is the primary button and "Projekt öffnen" the secondary one. */
  onNewGame?: () => void;
  /** A message below the actions (a refused drop). */
  notice?: ReactNode;
};

/** The index of the project opened last (the list is kept newest first, but the time decides). */
function newestOf(recent: readonly RecentProject[]): number {
  let newest = 0;
  recent.forEach((p, i) => {
    if (p.openedAt > recent[newest]!.openedAt) newest = i;
  });
  return newest;
}

/**
 * One recent project: a DS screen with its name, its game and when it was opened; a click anywhere on the card opens it, its menu takes it
 * off the list. The DS screen (block content) sits beside the open button, whose hit area is stretched over the whole card; the button is
 * named by the project and described by its game, its time and the plaque.
 */
function ProjectCard({ project, latest, missing, now, onOpen, onForget }: {
  project: RecentProject;
  latest: boolean;
  missing: boolean;
  now: number;
  onOpen: () => void;
  onForget: () => void;
}) {
  const id = useId();
  return (
    <li className={missing ? `${styles.card} ${styles.gone}` : styles.card} data-missing={missing ? "" : undefined}>
      <div className={styles.preview}>
        <DSScreen
          variant="empty"
          pages={[]}
          page={0}
          background={
            // The name below the screen names the card; this one is only the picture.
            <span className={styles.screenName} aria-hidden="true">
              {project.name}
            </span>
          }
        />
      </div>
      <button
        type="button"
        className={styles.open}
        data-roving=""
        data-recent={project.path}
        title={project.path}
        aria-labelledby={`${id}-name`}
        aria-describedby={latest ? `${id}-meta ${id}-latest` : `${id}-meta`}
        onClick={onOpen}
      >
        {latest && (
          <span id={`${id}-latest`} className={styles.latest}>
            <Badge kind="info" icon="history">
              Zuletzt
            </Badge>
          </span>
        )}
        <span id={`${id}-name`} className={styles.name}>
          {project.name}
        </span>
        <span id={`${id}-meta`} className={styles.meta}>
          <Badge kind="neutral" icon="game">
            {project.game}
          </Badge>
          {missing ? <span className={styles.missing}>nicht gefunden</span> : <span className={styles.time}>geöffnet {ago(project.openedAt, now)}</span>}
        </span>
      </button>
      <span className={styles.more}>
        <Menu trigger={<Icon name="more" />} label={`Weitere Aktionen für ${project.name}`} items={[{ label: "Aus der Liste nehmen", onSelect: onForget }]} />
      </span>
      {missing && (
        <span className={styles.footer}>
          <Button kind="secondary" size="s" onClick={onForget}>
            Aus der Liste nehmen
          </Button>
        </span>
      )}
    </li>
  );
}

/** The surface of the area Start: a greeting, the recently opened projects as cards, and the way to open another one. */
export function StartPane({ recent, missing, onOpen, onOpenRecent, onForget, onNewGame, notice }: StartPaneProps) {
  const grid = useRef<HTMLUListElement>(null);
  useRovingList(grid, { orientation: "grid" });
  const now = useNow();
  const newest = newestOf(recent);
  return (
    <section className={styles.pane} aria-labelledby="start-title">
      <h1 id="start-title" className={styles.title}>
        Willkommen zurück
      </h1>
      {recent.length > 0 ? (
        <ul ref={grid} role="list" aria-label="Zuletzt geöffnete Projekte" className={styles.grid}>
          {recent.map((p, i) => (
            <ProjectCard
              key={p.path}
              project={p}
              latest={i === newest}
              missing={missing.has(p.path)}
              now={now}
              onOpen={() => onOpenRecent(p.path)}
              onForget={() => onForget(p.path)}
            />
          ))}
        </ul>
      ) : (
        <p className={styles.hint}>Noch kein Projekt geöffnet.</p>
      )}
      <div className={styles.actions} data-start-actions="">
        {onNewGame && (
          <Button kind="primary" icon="plus" onClick={onNewGame}>
            Neues Spiel aus ROM…
          </Button>
        )}
        <Button kind={onNewGame ? "secondary" : "primary"} icon="open" shortcut="⌘O" onClick={onOpen}>
          Projekt öffnen
        </Button>
      </div>
      {onNewGame && (
        <div className={styles.hints}>
          <p className={styles.hint}>Du brauchst deine eigene ROM-Datei (.nds).</p>
          <p className={styles.hint}>Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM.</p>
        </div>
      )}
      {notice}
      <p className={styles.hint}>Projekte sind entpackte DSPRE-Ordner.</p>
    </section>
  );
}
