import { Suspense, lazy, useId, useMemo, useState } from "react";
import type { JSX } from "react";
import type { ProjectInfo, StoryRowDto, ThreadDto, ThreadListDto } from "../api";
import { hex4 } from "../flags/flagWords";
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

const COLUMNS: readonly TableColumn[] = [
  { key: "name", label: "Faden" },
  { key: "values", label: "Werte" },
  { key: "places", label: "Orte" },
  { key: "story", label: "Geschichte", width: 200 },
];

/** „3 Fäden“, „1 Faden“, „Keine Fäden“: the count of the list (and of the entry in the list of Geschichten). */
export function threadsCount(n: number): string {
  return n === 0 ? "Keine Fäden" : n === 1 ? "1 Faden" : `${n.toLocaleString("de-DE")} Fäden`;
}

/**
 * Geschichten › Fäden (3d.6): the variables the game tells a story with (three values or more at two places or more), as a table with
 * their name (from Bauen with the number), their values, their places and the story kept in them (a chip that opens it); below the table
 * the graph of the chosen thread, which loads in its own chunk. Without adopted scripts the index sees no script: the empty state offers
 * the way to adopt them; without a project it offers to open one.
 */
export function ThreadsSurface({ threads, error, depth, selected, onSelect, goTo, project, revision, onOpenTextProject, onOpenProject, stories }: ThreadsSurfaceProps): JSX.Element {
  const titleId = useId();
  const tech = depthAtLeast(depth, "bauen");
  const chosen = threads?.threads.find((t) => t.var === selected) ?? null;
  const rows = useMemo(
    (): TableRow[] =>
      (threads?.threads ?? []).map((t) => ({
        key: String(t.var),
        attrs: { "data-thread-row": String(t.var) },
        cells: {
          name: (
            <span className={styles.name}>
              <span data-thread-name="">{threadName(t)}</span>
              {tech && (
                <span className={styles.tech} data-thread-tech="">
                  {hex4(t.var)}
                </span>
              )}
            </span>
          ),
          values: valuesText(t.values),
          places: placesText(t),
          story: <StoryChip thread={t} stories={stories ?? null} tech={tech} goTo={goTo} />,
        },
      })),
    [threads, tech, stories, goTo],
  );

  return (
    <section className={styles.surface} data-geschichten-surface="threads" aria-labelledby={titleId}>
      <header className={styles.head}>
        <h1 id={titleId} className={styles.title}>
          Fäden
        </h1>
        <p className={styles.text}>Variablen, mit denen das Spiel eine Geschichte erzählt: drei Werte oder mehr, gesetzt oder geprüft an zwei Orten oder mehr.</p>
      </header>
      {project === null ? (
        <EmptyState
          what="Kein Projekt geöffnet."
          why="Öffne ein Projekt, dann stehen hier seine Fäden."
          action={onOpenProject && { label: "Projekt öffnen", onClick: onOpenProject }}
        />
      ) : threads === null ? (
        error === null ? (
          <p className={styles.note}>Wird gelesen …</p>
        ) : (
          <Notice kind="error">{error}</Notice>
        )
      ) : threads.threads.length === 0 ? (
        threads.scripts_adopted ? (
          <EmptyState what="Keine Fäden." why="Keine Variable hat drei Werte an zwei Orten." />
        ) : (
          <EmptyState
            what="Keine Fäden: die Scripts sind nicht übernommen."
            why="Ohne Script-Quellen sieht der Index nicht, welche Werte Scripts setzen und prüfen."
            action={{ label: "Scripts übernehmen…", onClick: onOpenTextProject }}
          />
        )
      ) : (
        <>
          <Table
            columns={COLUMNS}
            rows={rows}
            ariaLabel="Fäden"
            selected={selected === null ? undefined : String(selected)}
            onSelect={(key) => onSelect(Number(key))}
            footer={threadsCount(threads.threads.length)}
          />
          {chosen === null ? (
            <p className={styles.note}>Wähle einen Faden: darunter erscheint sein Graph, links die Orte, die einen Wert setzen, rechts die, die ihn prüfen.</p>
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
  const headingId = useId();
  const { graph, error } = useThreadGraph(thread.var, thread.story, { project, revision });
  const [node, setNode] = useState<string | null>(null);
  return (
    <section className={styles.graphSection} data-thread-graph-box="" aria-labelledby={headingId}>
      <h2 id={headingId} className={styles.heading}>
        Graph: {threadName(thread)}
      </h2>
      {error !== null ? (
        <Notice kind="error">{error}</Notice>
      ) : graph === null ? (
        <p className={styles.note}>Graph wird gelesen …</p>
      ) : (
        <div className={styles.graphBox}>
          <Suspense fallback={<p className={styles.note}>Graph wird geladen…</p>}>
            <ThreadGraph graph={graph} selected={node} onSelect={setNode} onOpenPlace={goTo} />
          </Suspense>
        </div>
      )}
    </section>
  );
}
