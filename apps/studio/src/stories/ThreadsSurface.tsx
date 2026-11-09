import { Suspense, lazy, useId, useMemo, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectInfo, StoryRowDto, ThreadDto, ThreadListDto } from "../api";
import { hex4 } from "../flags/flagWords";
import { t as translate } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { EmptyState, Notice, Reference, Table } from "../ui/primitives";
import type { TableColumn, TableRow } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { placesText, threadName, valuesText } from "./storyWords";
import { useThreadGraph } from "./useThreadGraph";
import styles from "./ThreadsSurface.module.css";

// React Flow and dagre load with the graph, in their own chunk.
const ThreadGraph = lazy(() => import("./graph/ThreadGraph"));

export type ThreadsSurfaceProps = {
  /** The threads of the project (`list_threads`); null while they are read or when they cannot be. */
  threads: ThreadListDto | null;
  /** Why the threads cannot be read. */
  error: string | null;
  /** Erzählen shows the names only, Bauen and Tüfteln the variable's number too. */
  depth: Depth;
  /** The chosen thread's variable, held by the app (a link sets it). */
  selected: number | null;
  onSelect: (variable: number | null) => void;
  goTo: (t: NavTarget) => void;
  project: ProjectInfo | null;
  /** Moves whenever the project may differ: the graph is read again. */
  revision: number;
  /** „Scripts übernehmen…“ of the empty state: Technik › Textprojekt. */
  onOpenTextProject: () => void;
  /** „Projekt öffnen“ of the empty state without a project; without it the empty state offers nothing. */
  onOpenProject?: () => void;
  /** The stories of the project, to name a thread's story and open it by its number; without them the chip names the id. */
  stories?: readonly StoryRowDto[] | null;
};

/** „3 Fäden“, „1 Faden“, „Keine Fäden“: the count of the list (and of the entry in the list of Geschichten). */
export function threadsCount(n: number): string {
  return translate("stories:threadsSurface.count", { count: n });
}

/**
 * Geschichten › Fäden (3d.6): the variables the game tells a story with (three values or more at two places or more), as a table with
 * their name (from Bauen with the number), their values, their places and the story kept in them (a chip that opens it); below the table
 * the graph of the chosen thread, which loads in its own chunk. Without adopted scripts the index sees no script: the empty state offers
 * the way to adopt them; without a project it offers to open one.
 */
export function ThreadsSurface({ threads, error, depth, selected, onSelect, goTo, project, revision, onOpenTextProject, onOpenProject, stories }: ThreadsSurfaceProps): JSX.Element {
  const { t } = useTranslation("stories");
  const titleId = useId();
  const columns: readonly TableColumn[] = [
    { key: "name", label: t("threadsSurface.columns.thread") },
    { key: "values", label: t("threadsSurface.columns.values") },
    { key: "places", label: t("threadsSurface.columns.places") },
    { key: "story", label: t("threadsSurface.columns.story"), width: 200 },
  ];
  const tech = depthAtLeast(depth, "bauen");
  const chosen = threads?.threads.find((thread) => thread.var === selected) ?? null;
  const rows = useMemo(
    (): TableRow[] =>
      (threads?.threads ?? []).map((thread) => ({
        key: String(thread.var),
        attrs: { "data-thread-row": String(thread.var) },
        cells: {
          name: (
            <span className={styles.name}>
              <span data-thread-name="">{threadName(thread)}</span>
              {tech && (
                <span className={styles.tech} data-thread-tech="">
                  {hex4(thread.var)}
                </span>
              )}
            </span>
          ),
          values: valuesText(thread.values),
          places: placesText(thread),
          story: <StoryChip thread={thread} stories={stories ?? null} tech={tech} goTo={goTo} />,
        },
      })),
    // `t` changes with the language: the texts of the cells (values, places) are made again then.
    [threads, tech, stories, goTo, t],
  );

  return (
    <section className={styles.surface} data-geschichten-surface="threads" aria-labelledby={titleId}>
      <header className={styles.head}>
        <h1 id={titleId} className={styles.title}>
          {t("threadsSurface.title")}
        </h1>
        <p className={styles.text}>{t("threadsSurface.intro")}</p>
      </header>
      {project === null ? (
        <EmptyState
          what={t("threadsSurface.noProject")}
          why={t("threadsSurface.noProjectWhy")}
          action={onOpenProject && { label: t("threadsSurface.openProject"), onClick: onOpenProject }}
        />
      ) : threads === null ? (
        error === null ? (
          <p className={styles.note}>{t("threadsSurface.reading")}</p>
        ) : (
          <Notice kind="error">{error}</Notice>
        )
      ) : threads.threads.length === 0 ? (
        threads.scripts_adopted ? (
          <EmptyState what={t("threadsSurface.noThreads")} why={t("threadsSurface.noThreadsWhy")} />
        ) : (
          <EmptyState
            what={t("threadsSurface.notAdopted")}
            why={t("threadsSurface.notAdoptedWhy")}
            action={{ label: t("threadsSurface.adoptScripts"), onClick: onOpenTextProject }}
          />
        )
      ) : (
        <>
          <Table
            columns={columns}
            rows={rows}
            ariaLabel={t("threadsSurface.table")}
            selected={selected === null ? undefined : String(selected)}
            onSelect={(key) => onSelect(Number(key))}
            footer={threadsCount(threads.threads.length)}
          />
          {chosen === null ? (
            <p className={styles.note}>{t("threadsSurface.chooseOne")}</p>
          ) : (
            <ThreadGraphBox key={chosen.var} thread={chosen} project={project} revision={revision} goTo={goTo} />
          )}
        </>
      )}
    </section>
  );
}

/**
 * The story kept in a thread: its name as a chip of the area Geschichten that opens it (the id when the stories are not known), else „–“.
 * The column is narrow: the name has it alone, from Bauen the id is the chip's tooltip.
 */
function StoryChip({ thread, stories, tech, goTo }: { thread: ThreadDto; stories: readonly StoryRowDto[] | null; tech: boolean; goTo: (t: NavTarget) => void }) {
  if (thread.story === null) return <span className={styles.none}>–</span>;
  const story = stories?.find((s) => s.id === thread.story) ?? null;
  return (
    <Reference
      area="geschichten"
      label={story?.name || thread.story}
      title={tech && story !== null ? thread.story : undefined}
      onOpen={() => story !== null && goTo({ kind: "story", number: story.number })}
      disabled={story === null}
      dataAttrs={{ "data-thread-story": thread.story }}
    />
  );
}

/** The graph of the chosen thread (with its story's steps), its title above; a node chosen in it is kept while the thread stays. */
function ThreadGraphBox({ thread, project, revision, goTo }: { thread: ThreadDto; project: ProjectInfo | null; revision: number; goTo: (t: NavTarget) => void }) {
  const { t } = useTranslation("stories");
  const headingId = useId();
  const { graph, error } = useThreadGraph(thread.var, thread.story, { project, revision });
  const [node, setNode] = useState<string | null>(null);
  return (
    <section className={styles.graphSection} data-thread-graph-box="" aria-labelledby={headingId}>
      <h2 id={headingId} className={styles.heading}>
        {t("threadsSurface.graphHeading", { name: threadName(thread) })}
      </h2>
      {error !== null ? (
        <Notice kind="error">{error}</Notice>
      ) : graph === null ? (
        <p className={styles.note}>{t("threadsSurface.graphReading")}</p>
      ) : (
        <div className={styles.graphBox}>
          <Suspense fallback={<p className={styles.note}>{t("threadsSurface.graphLoading")}</p>}>
            <ThreadGraph graph={graph} selected={node} onSelect={setNode} onOpenPlace={goTo} />
          </Suspense>
        </div>
      )}
    </section>
  );
}
