import { useEffect, useId, useRef, useState } from "react";
import type { JSX } from "react";
import { errorText, storyList } from "../api";
import type { ProjectInfo, StoryListDto } from "../api";
import { FLAGS_SOURCE, FlagsSurface, flagsSubtitle } from "../flags/FlagsSurface";
import type { FlagsSurfaceProps } from "../flags/FlagsSurface";
import { useFlagList } from "../flags/useFlagList";
import { ListPane } from "../ui/frame/ListPane";
import type { Depth } from "../ui/frame/shellState";
import { EmptyState, ListRow, Notice, useRovingList } from "../ui/primitives";
import type { IconName } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { ThreadsSurface, threadsCount } from "./ThreadsSurface";
import { useThreads } from "./useThreads";
import styles from "./GeschichtenArea.module.css";

/** The entries of the list, each with its surface. */
export type GeschichtenEntry = "threads" | "stories" | "flags";

/** What the flags surface needs from the app (3d.1, unchanged since Technik held it): the open project, the chosen number, the reports. */
export type GeschichtenFlagsProps = Pick<FlagsSurfaceProps, "project" | "revision" | "selected" | "onSelect" | "readOnly" | "onError" | "onNamed">;

export interface GeschichtenAreaProps {
  /** The open project's folder; null without one. */
  project: string | null;
  entry: GeschichtenEntry;
  onEntry: (e: GeschichtenEntry) => void;
  /** Whether the list shows or is folded to its grip. */
  listOpen: boolean;
  onToggleList: () => void;
  /** The area's view depth: Erzählen names the threads only, Bauen and Tüfteln their numbers too. */
  depth: Depth;
  /** Moves whenever the project may differ (an edit, a save, an undo, an own name): the lists are read again. */
  revision: number;
  goTo: (t: NavTarget) => void;
  /** The chosen thread's variable, held by the app (a link to a thread sets it). */
  selectedThread: number | null;
  onSelectThread: (v: number | null) => void;
  /** The chosen story's number (as `NavTarget.story`), held by the app. */
  selectedStory: number | null;
  onSelectStory: (n: number | null) => void;
  /** Flags und Variablen; without it no project (its `project` is the open project for every read of the area). */
  flags?: GeschichtenFlagsProps;
  /** The project is a text project of the current schema (stories are created there only). */
  textProject: boolean;
  /** Technik › Textprojekt: where scripts are adopted and a DSPRE folder is set up. */
  onOpenTextProject: () => void;
  /** What failed (a write's refusal or warning). */
  onError: (text: string) => void;
}

const ENTRIES: readonly { id: GeschichtenEntry; label: string; icon: IconName }[] = [
  { id: "threads", label: "Fäden", icon: "thread" },
  { id: "stories", label: "Geschichten", icon: "story" },
  { id: "flags", label: "Flags und Variablen", icon: "flag" },
];

/** „2 Geschichten“, „1 Geschichte“, „Keine Geschichten“. */
function storiesCount(n: number): string {
  return n === 0 ? "Keine Geschichten" : n === 1 ? "1 Geschichte" : `${n.toLocaleString("de-DE")} Geschichten`;
}

/** The line under an entry: no project, still reading, not readable, or the count. */
function subtitleOf<T>(info: ProjectInfo | null, read: { list: T | null; error: string | null }, count: (list: T) => string): string {
  if (info === null) return "Kein Projekt";
  if (read.error !== null) return "Nicht lesbar";
  return read.list === null ? "Wird gelesen …" : count(read.list);
}

/**
 * The area Geschichten (3d.6, a surface without a deck like Technik): the list „Geschichten“ with Fäden, Geschichten and Flags und Variablen
 * (moved here from Technik), each with its count, and the surface of the chosen entry beside it. The threads and the stories are read once
 * here, for the list and the surfaces together; the flags as the flags surface reads them.
 */
export function GeschichtenArea(props: GeschichtenAreaProps): JSX.Element {
  const { entry, onEntry, listOpen, onToggleList, depth, revision, goTo } = props;
  const info = props.flags?.project ?? null;
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const threads = useThreads({ project: info, revision });
  const stories = useStoryList(info, revision);
  const flagList = useFlagList("flag", { project: info, revision: props.flags?.revision ?? 0, source: FLAGS_SOURCE });
  const subtitles: Record<GeschichtenEntry, string> = {
    threads: subtitleOf(info, threads, (l) => threadsCount(l.threads.length)),
    stories: subtitleOf(info, stories, (l) => storiesCount(l.stories.length)),
    flags: subtitleOf(info, flagList, flagsSubtitle),
  };
  return (
    <div className={styles.area} data-geschichten={entry}>
      <ListPane title="Geschichten" open={listOpen} onToggle={onToggleList}>
        <div ref={list} role="listbox" aria-label="Geschichten" className={styles.list}>
          {ENTRIES.map((e) => (
            <ListRow key={e.id} label={e.label} icon={e.icon} subtitle={subtitles[e.id]} selected={e.id === entry} onSelect={() => onEntry(e.id)} />
          ))}
        </div>
      </ListPane>
      {entry === "threads" ? (
        <ThreadsSurface
          threads={info === null ? null : threads.list}
          error={threads.error}
          depth={depth}
          selected={props.selectedThread}
          onSelect={props.onSelectThread}
          goTo={goTo}
          project={info}
          revision={revision}
          onOpenTextProject={props.onOpenTextProject}
          stories={stories.list?.stories ?? null}
        />
      ) : entry === "stories" ? (
        <StoriesPlaceholder project={info} stories={stories.list} error={stories.error} />
      ) : (
        <FlagsSurface
          project={info}
          revision={props.flags?.revision ?? 0}
          selected={props.flags?.selected ?? null}
          onSelect={props.flags?.onSelect ?? noSelect}
          goTo={goTo}
          depth={depth}
          readOnly={props.flags?.readOnly ?? false}
          onError={props.flags?.onError ?? noSelect}
          onNamed={props.flags?.onNamed}
        />
      )}
    </div>
  );
}

/** Without the app's selection the flags surface chooses nothing (`flags` not given: no project either). */
const noSelect = () => {};

type StoriesRead = { project: ProjectInfo; list: StoryListDto | null; error: string | null };

/**
 * The stories of the project (`story_list`) for the entry's count and the threads' story chips, read when there is a project and again for
 * every new `revision`; an answer a newer read overtook never shows, nor the list of another project.
 */
function useStoryList(project: ProjectInfo | null, revision: number): { list: StoryListDto | null; error: string | null } {
  const [read, setRead] = useState<StoriesRead | null>(null);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    storyList().then(
      (list) => live && setRead({ project, list, error: null }),
      (e: unknown) => live && setRead({ project, list: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, revision]);
  const current = read !== null && project !== null && read.project === project ? read : null;
  return { list: current?.list ?? null, error: current?.error ?? null };
}

/** Geschichten › Geschichten until its list and detail come (3d.6 Task 8): how many there are, and that agents create them meanwhile. */
function StoriesPlaceholder({ project, stories, error }: { project: ProjectInfo | null; stories: StoryListDto | null; error: string | null }) {
  const titleId = useId();
  return (
    <section className={styles.surface} data-geschichten-surface="stories" aria-labelledby={titleId}>
      <header className={styles.head}>
        <h1 id={titleId} className={styles.title}>
          Geschichten
        </h1>
        <p className={styles.text}>Geschichten erzählen mit einer Variable oder einem Flag, Schritt für Schritt, was im Spiel geschieht.</p>
      </header>
      {project === null ? (
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann stehen hier seine Geschichten." />
      ) : error !== null ? (
        <Notice kind="error">{error}</Notice>
      ) : stories === null ? (
        <p className={styles.note}>Wird gelesen …</p>
      ) : (
        <EmptyState what={`${storiesCount(stories.stories.length)}.`} why="Die Liste und das Detail der Geschichten folgen; bis dahin legen Agenten sie über MCP an (story_create)." />
      )}
    </section>
  );
}
