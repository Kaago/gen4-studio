import { useId, useState } from "react";
import type { JSX, ReactNode } from "react";
import { storyUpdate } from "../api";
import type { FlagKind, ProjectInfo, StoryListDto, StoryReport, StoryRowDto } from "../api";
import { FLAGS_SOURCE } from "../flags/FlagsSurface";
import { NOT_A_TEXT_PROJECT, hex4 } from "../flags/flagWords";
import { useFlagList } from "../flags/useFlagList";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Button, EmptyState, Notice, Reference, Table, Tooltip } from "../ui/primitives";
import type { TableColumn, TableRow } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { NewStoryDialog, createStory } from "./NewStoryDialog";
import type { NewStoryRequest } from "./NewStoryDialog";
import { StoryDetail } from "./StoryDetail";
import type { StoryPatch } from "./StoryDetail";
import type { StoryRecipeRequest } from "./recipes/storyRecipeModel";
import { flagNumberOf, progressLabel, storiesCount } from "./storyWords";
import { useStory } from "./useStory";
import styles from "./StoriesSurface.module.css";

export type StoriesSurfaceProps = {
  /** The stories of the project (`story_list`, read once by the area for its entry and this surface); null while read or unreadable. */
  stories: StoryListDto | null;
  /** Why the stories cannot be read. */
  error: string | null;
  /** The chosen story's number, held by the app (a link sets it). */
  selected: number | null;
  onSelect: (n: number | null) => void;
  /** The project is a text project: only there are stories created (the DSPRE layout locks it with the way there). */
  textProject: boolean;
  /** „Als Textprojekt einrichten…“: Technik › Textprojekt. */
  onOpenTextProject: () => void;
  /** Erzählen names the progress only, from Bauen with its number. */
  depth: Depth;
  goTo: (t: NavTarget) => void;
  project: ProjectInfo | null;
  /** Moves whenever the project may differ: the chosen story and its graph are read again. */
  revision: number;
  /** A story was created (one undo step; by the recipe „Neue Geschichte“ for a variable): the report and its name, for the app's toast
   * and history. */
  onCreated: (r: StoryReport, name: string) => void;
  /** A story's name or notes were written (one undo step). */
  onUpdated?: (r: StoryReport) => void;
  /** A story was deleted (one undo step, ⌘Z brings it back): the report and its name, for the app's toast and history. */
  onDeleted?: (r: StoryReport, name: string) => void;
  /** What failed after a write went in (the report's warnings). */
  onError: (t: string) => void;
  /** „Projekt öffnen“ of the empty state without a project; without it the empty state offers nothing. */
  onOpenProject?: () => void;
  /** „Schritt anlegen“ in the chosen story's detail: a step recipe with that story (the app's dialog); absent: no step bar. */
  onRecipe?: (request: StoryRecipeRequest) => void;
};

const COLUMNS: readonly TableColumn[] = [
  { key: "name", label: "Name" },
  { key: "progress", label: "Fortschritt", width: 280 },
  { key: "steps", label: "Schritte", width: 96, align: "end" },
];

/**
 * Geschichten › Geschichten (3d.6, reading): the stories of the project as a table (name, progress as a chip of the area Geschichten,
 * from Bauen with its number, the count of steps; a file that does not read as a warning in its row), „Neue Geschichte…“ above it
 * (locked in the DSPRE layout, with „Als Textprojekt einrichten…“ below), and the detail of the chosen story under the table.
 */
export function StoriesSurface(props: StoriesSurfaceProps): JSX.Element {
  const { stories, error, selected, onSelect, textProject, onOpenTextProject, depth, goTo, project, revision, onCreated, onUpdated, onDeleted, onError, onOpenProject, onRecipe } = props;
  const titleId = useId();
  const tech = depthAtLeast(depth, "bauen");
  const [creating, setCreating] = useState(false);
  const rowsOf = stories?.stories ?? [];
  // The names of the progress numbers, read only for a kind whose ids the list has by name (a fallback id names its number itself).
  const named = (kind: FlagKind) => rowsOf.some((s) => s.progress !== null && progressLabel(s.progress).kind === kind && flagNumberOf(kind, progressLabel(s.progress).id, null) === null);
  const flags = useFlagList("flag", { project: named("flag") ? project : null, revision, source: FLAGS_SOURCE }).list;
  const vars = useFlagList("var", { project: named("var") ? project : null, revision, source: FLAGS_SOURCE }).list;
  const chosen = rowsOf.find((s) => s.number === selected && s.problem === null) ?? null;
  const { story, error: storyError } = useStory(chosen?.id ?? null, { project, revision });

  const create = async (request: NewStoryRequest) => {
    const { report, name } = await createStory(request);
    onCreated(report, name);
    for (const warning of report.warnings) onError(warning);
    onSelect(report.number);
  };
  const update = async (id: string, patch: StoryPatch) => {
    const report = await storyUpdate(id, patch);
    onUpdated?.(report);
    for (const warning of report.warnings) onError(warning);
  };
  // The deleted story's detail goes: nothing is chosen, the list is read again (the app's revision moves).
  const deleted = (report: StoryReport, name: string) => {
    onSelect(null);
    onDeleted?.(report, name);
    for (const warning of report.warnings) onError(warning);
  };

  const progressChip = (row: StoryRowDto): ReactNode => {
    if (row.progress === null) return <span className={styles.none}>–</span>;
    const { kind, id } = progressLabel(row.progress);
    const number = flagNumberOf(kind, id, kind === "flag" ? flags : vars);
    return (
      <Reference
        area="geschichten"
        label={id}
        tech={tech && number !== null ? hex4(number) : undefined}
        onOpen={() => number !== null && goTo({ kind: "flag", flagKind: kind, number })}
        disabled={number === null}
        dataAttrs={{ "data-story-progress": kind }}
      />
    );
  };
  const rows: TableRow[] = rowsOf.map((s) => ({
    key: String(s.number),
    attrs: { "data-story-row": s.id },
    cells: {
      name:
        s.problem === null ? (
          s.name
        ) : (
          <span className={styles.problem}>
            <span className={styles.tech}>{s.id}</span>
            <Notice kind="warning">{s.problem}</Notice>
          </span>
        ),
      progress: progressChip(s),
      steps: s.problem === null ? String(s.steps) : <span className={styles.none}>–</span>,
    },
  }));

  const locked = !textProject;
  return (
    <section className={styles.surface} data-geschichten-surface="stories" aria-labelledby={titleId}>
      <header className={styles.head}>
        <div className={styles.headText}>
          <h1 id={titleId} className={styles.title}>
            Geschichten
          </h1>
          <p className={styles.text}>Geschichten erzählen mit einer Variable oder einem Flag, Schritt für Schritt, was im Spiel geschieht.</p>
        </div>
        {project !== null && <NewStoryButton locked={locked} onClick={() => setCreating(true)} />}
      </header>
      {project === null ? (
        <EmptyState
          what="Kein Projekt geöffnet."
          why="Öffne ein Projekt, dann stehen hier seine Geschichten."
          action={onOpenProject && { label: "Projekt öffnen", onClick: onOpenProject }}
        />
      ) : stories === null ? (
        error === null ? (
          <p className={styles.note}>Wird gelesen …</p>
        ) : (
          <Notice kind="error">{error}</Notice>
        )
      ) : rowsOf.length === 0 ? (
        locked ? (
          <EmptyState
            what="Kein Textprojekt."
            why="Geschichten liegen als Dateien im Textprojekt; im DSPRE-Layout gibt es keine. Technik › Textprojekt richtet es ein."
            action={{ label: "Als Textprojekt einrichten…", onClick: onOpenTextProject }}
          />
        ) : (
          <EmptyState what="Noch keine Geschichten." why="„Neue Geschichte…“ legt eine an: ein Name und die Variable oder das Flag, das ihren Fortschritt hält." />
        )
      ) : (
        <>
          <Table
            columns={COLUMNS}
            rows={rows}
            ariaLabel="Geschichten"
            selected={selected === null ? undefined : String(selected)}
            onSelect={(key) => onSelect(Number(key))}
            footer={storiesCount(rowsOf.length)}
          />
          {chosen === null ? (
            <p className={styles.note}>Wähle eine Geschichte: darunter erscheinen ihre Notizen, ihre Schritte und ihr Graph.</p>
          ) : storyError !== null ? (
            <Notice kind="error">{storyError}</Notice>
          ) : story === null ? (
            <p className={styles.note}>Geschichte wird gelesen …</p>
          ) : (
            <StoryDetail
              key={chosen.id}
              story={story}
              depth={depth}
              goTo={goTo}
              onUpdate={(patch) => update(chosen.id, patch)}
              onDeleted={deleted}
              project={project}
              revision={revision}
              onRecipe={onRecipe}
              textProject={textProject}
            />
          )}
        </>
      )}
      <NewStoryDialog open={creating} onClose={() => setCreating(false)} onCreate={create} project={project} revision={revision} />
    </section>
  );
}

/** „Neue Geschichte…“: locked in the DSPRE layout but focusable, the kernel's reason as its description and tooltip. */
function NewStoryButton({ locked, onClick }: { locked: boolean; onClick: () => void }) {
  const noteId = useId();
  const button = (
    <Button
      kind="secondary"
      icon="plus"
      data-new-story=""
      aria-disabled={locked || undefined}
      aria-describedby={locked ? noteId : undefined}
      onClick={() => !locked && onClick()}
    >
      Neue Geschichte…
    </Button>
  );
  if (!locked) return button;
  return (
    <>
      <Tooltip content={NOT_A_TEXT_PROJECT}>{button}</Tooltip>
      <span id={noteId} hidden>
        {NOT_A_TEXT_PROJECT}
      </span>
    </>
  );
}
