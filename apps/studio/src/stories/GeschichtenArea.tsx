import { useRef } from "react";
import type { JSX } from "react";
import type { ProjectInfo, StoryReport } from "../api";
import { FLAGS_SOURCE, FlagsSurface, flagsSubtitle } from "../flags/FlagsSurface";
import type { FlagsSurfaceProps } from "../flags/FlagsSurface";
import { useFlagList } from "../flags/useFlagList";
import { ListPane } from "../ui/frame/ListPane";
import type { Depth } from "../ui/frame/shellState";
import { ListRow, useRovingList } from "../ui/primitives";
import type { IconName } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { StoriesSurface } from "./StoriesSurface";
import { storiesCount } from "./storyWords";
import { ThreadsSurface, threadsCount } from "./ThreadsSurface";
import { useStories } from "./useStories";
import { useThreads } from "./useThreads";
import styles from "./GeschichtenArea.module.css";

/** The entries of the list, each with its surface. */
export type GeschichtenEntry = "threads" | "stories" | "flags";

/** What the flags surface needs from the app (3d.1, unchanged since Technik held it): the open project, the chosen number, the reports and play mode (3d.9). */
export type GeschichtenFlagsProps = Pick<FlagsSurfaceProps, "project" | "revision" | "selected" | "onSelect" | "readOnly" | "onError" | "onNamed" | "live">;

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
  /** „Projekt öffnen“ of the surfaces' empty states without a project (as Technik offered it for Flags und Variablen). */
  onOpenProject?: () => void;
  /** What failed (a write's refusal or warning). */
  onError: (text: string) => void;
  /** A story was created (one undo step): the report and its name (the app's toast; the history and the lists read again). */
  onStoryCreated?: (r: StoryReport, name: string) => void;
  /** A story's name or notes were written (one undo step): the history and the lists read again. */
  onStoryUpdated?: (r: StoryReport) => void;
  /** A story was deleted (one undo step): the report and its name (the app's toast; the history and the lists read again). */
  onStoryDeleted?: (r: StoryReport, name: string) => void;
}

const ENTRIES: readonly { id: GeschichtenEntry; label: string; icon: IconName }[] = [
  { id: "threads", label: "Fäden", icon: "thread" },
  { id: "stories", label: "Geschichten", icon: "story" },
  { id: "flags", label: "Flags und Variablen", icon: "flag" },
];

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
  const stories = useStories({ project: info, revision });
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
          onOpenProject={props.onOpenProject}
          stories={stories.list?.stories ?? null}
        />
      ) : entry === "stories" ? (
        <StoriesSurface
          stories={info === null ? null : stories.list}
          error={stories.error}
          selected={props.selectedStory}
          onSelect={props.onSelectStory}
          textProject={props.textProject}
          onOpenTextProject={props.onOpenTextProject}
          depth={depth}
          goTo={goTo}
          project={info}
          revision={revision}
          onCreated={props.onStoryCreated ?? noSelect}
          onUpdated={props.onStoryUpdated}
          onDeleted={props.onStoryDeleted}
          onError={props.onError}
          onOpenProject={props.onOpenProject}
        />
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
          onOpenProject={props.onOpenProject}
          live={props.flags?.live}
        />
      )}
    </div>
  );
}

/** Nothing to tell: without the app's handlers a surface chooses and reports nothing (`flags` not given: no project either). */
const noSelect = () => {};
