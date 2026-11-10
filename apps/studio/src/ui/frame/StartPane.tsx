import { useId, useRef } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { PreviewDto } from "../../api";
import { Badge, Button, Icon, Menu, useRovingList } from "../primitives";
import type { MenuEntry } from "../primitives";
import { ProjectPreview } from "./ProjectPreview";
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
  /** The preview pictures read so far, by path (see `usePreviews`); a card without an entry shows its name on the empty screen. */
  previews: ReadonlyMap<string, PreviewDto>;
  /** „Vorschaubild wählen…“ in a card's menu. */
  onChoosePreview: (path: string) => void;
  /** „Vorschaubild entfernen“ in a card's menu (only with an own picture). */
  onRemovePreview: (path: string) => void;
  /** „Neues Spiel aus ROM…“: with it, that is the primary button and „Projekt öffnen“ the secondary one. */
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
 * One recent project: a DS screen with its preview picture (or its name), its game and when it was opened; a click anywhere on the card
 * opens it, its menu chooses or removes the picture and takes it off the list. The DS screen (block content) sits beside the open button,
 * whose hit area is stretched over the whole card; the button is named by the project and described by its game, its time and the plaque.
 * A card that could not be opened shows no picture and offers only to leave the list.
 */
function ProjectCard({ project, latest, missing, preview, now, onOpen, onForget, onChoosePreview, onRemovePreview }: {
  project: RecentProject;
  latest: boolean;
  missing: boolean;
  preview: PreviewDto | undefined;
  now: number;
  onOpen: () => void;
  onForget: () => void;
  onChoosePreview: () => void;
  onRemovePreview: () => void;
}) {
  const { t } = useTranslation("ui");
  const id = useId();
  const forget: MenuEntry = { label: t("startPane.removeFromList"), onSelect: onForget };
  const items: MenuEntry[] = missing
    ? [forget]
    : [
        { label: t("startPane.choosePreview"), onSelect: onChoosePreview },
        ...(preview?.source === "eigen" ? [{ label: t("startPane.removePreview"), onSelect: onRemovePreview }] : []),
        "separator",
        forget,
      ];
  return (
    <li className={missing ? `${styles.card} ${styles.gone}` : styles.card} data-missing={missing ? "" : undefined}>
      <div className={styles.preview}>
        {/* The name below the screen names the card; this is only the picture. */}
        <ProjectPreview name={project.name} png={missing ? null : (preview?.png ?? null)} />
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
              {t("startPane.latest")}
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
          {missing ? <span className={styles.missing}>{t("startPane.notFound")}</span> : <span className={styles.time}>{t("startPane.opened", { ago: ago(project.openedAt, now) })}</span>}
        </span>
      </button>
      <span className={styles.more}>
        <Menu trigger={<Icon name="more" />} label={t("startPane.moreActions", { name: project.name })} items={items} />
      </span>
      {missing && (
        <span className={styles.footer}>
          <Button kind="secondary" size="s" onClick={onForget}>
            {t("startPane.removeFromList")}
          </Button>
        </span>
      )}
    </li>
  );
}

/** The surface of the area Start: a greeting, the recently opened projects as cards, and the way to open another one. */
export function StartPane({ recent, missing, previews, onOpen, onOpenRecent, onForget, onChoosePreview, onRemovePreview, onNewGame, notice }: StartPaneProps) {
  const { t } = useTranslation("ui");
  const grid = useRef<HTMLUListElement>(null);
  useRovingList(grid, { orientation: "grid" });
  const now = useNow();
  const newest = newestOf(recent);
  return (
    <section className={styles.pane} aria-labelledby="start-title">
      <h1 id="start-title" className={styles.title}>
        {t("startPane.welcome")}
      </h1>
      {recent.length > 0 ? (
        <ul ref={grid} role="list" aria-label={t("startPane.recentLabel")} className={styles.grid}>
          {recent.map((p, i) => (
            <ProjectCard
              key={p.path}
              project={p}
              latest={i === newest}
              missing={missing.has(p.path)}
              preview={previews.get(p.path)}
              now={now}
              onOpen={() => onOpenRecent(p.path)}
              onForget={() => onForget(p.path)}
              onChoosePreview={() => onChoosePreview(p.path)}
              onRemovePreview={() => onRemovePreview(p.path)}
            />
          ))}
        </ul>
      ) : (
        <p className={styles.hint}>{t("startPane.noneYet")}</p>
      )}
      <div className={styles.actions} data-start-actions="">
        {onNewGame && (
          <Button kind="primary" icon="plus" onClick={onNewGame}>
            {t("startPane.newGame")}
          </Button>
        )}
        <Button kind={onNewGame ? "secondary" : "primary"} icon="open" shortcut="⌘O" onClick={onOpen}>
          {t("startPane.openProject")}
        </Button>
      </div>
      {onNewGame && (
        <div className={styles.hints}>
          <p className={styles.hint}>{t("startPane.ownRom")}</p>
          <p className={styles.hint}>{t("startPane.noGameData")}</p>
        </div>
      )}
      {notice}
      <p className={styles.hint}>{t("startPane.dspreFolders")}</p>
    </section>
  );
}
