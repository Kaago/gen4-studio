import { Suspense, lazy, useEffect, useRef, useState } from "react";
import type { JSX, ReactNode } from "react";
import { errorText, storyDelete } from "../api";
import type { EntityKind, FlagKind, FlagListDto, Layout, ProjectInfo, Step, StoryDto, StoryReport, StoryUse, storyUpdate } from "../api";
import { useStableCallback } from "../app/useStable";
import { FLAGS_SOURCE } from "../flags/FlagsSurface";
import { hex4 } from "../flags/flagWords";
import { useFlagList } from "../flags/useFlagList";
import { HeadActions } from "../original/HeadActions";
import { useTextProject } from "../textproject/TextProjectContext";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Badge, Dialog, Notice, Reference, Section, Table, TextField, Tooltip } from "../ui/primitives";
import type { Area, TableColumn, TableRow } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { fallbackNumber, flagNumberOf, progressLabel, recipeTitle, stepKindWord, useText } from "./storyWords";
import { StepRecipes } from "./recipes/StepRecipes";
import type { StoryRecipeRequest } from "./recipes/storyRecipeModel";
import { useThreadGraph } from "./useThreadGraph";
import styles from "./StoriesSurface.module.css";

// React Flow and dagre load with the graph, in their own chunk.
const ThreadGraph = lazy(() => import("./graph/ThreadGraph"));

/** The fields `story_update` changes from the detail. */
export type StoryPatch = Parameters<typeof storyUpdate>[1];

export type StoryDetailProps = {
  story: StoryDto;
  /** From Bauen the progress chip names its number. */
  depth: Depth;
  goTo: (t: NavTarget) => void;
  /** Writes the fields given (`story_update`, seconds under load); a refusal rejects with the kernel's sentence. */
  onUpdate: (patch: StoryPatch) => Promise<void>;
  /** „Geschichte löschen…“ went in (`story_delete`, one undo step): the report and the story's name. */
  onDeleted: (r: StoryReport, name: string) => void;
  project: ProjectInfo | null;
  /** Moves whenever the project may differ: the graph is read again. */
  revision: number;
  /** „Schritt anlegen“: a step recipe asked for with this story (the app's dialog); absent: no step bar. */
  onRecipe?: (request: StoryRecipeRequest) => void;
  /** The project is a text project: the step recipes run only there. */
  textProject?: boolean;
};

const COLUMNS: readonly TableColumn[] = [
  { key: "value", label: "Wert", width: 64, align: "end" },
  { key: "kind", label: "Art", width: 200 },
  { key: "name", label: "Name" },
  { key: "uses", label: "Verweise" },
];

/** Said in place of the graph when the game has no value of the progress (a free variable, a flag, an id that names nothing). */
export const NO_VALUES = "Noch keine Werte im Spiel";

/** How long after a drag's end the graph's layout is written (one undo step per drag; a drag within it starts the wait anew). */
const LAYOUT_DELAY_MS = 500;

/** Said in place of the steps: the step recipes above them create them. */
export const NO_STEPS = "Noch keine Schritte. „Schritt anlegen“ legt sie über Rezepte an.";

/**
 * The numbers of the ids of `kind` (trainers, maps by header) from the text project's id cache, read only when `wanted`; null meanwhile
 * or when they cannot be read (a chip then knows a fallback id alone).
 */
function useIdNumbers(kind: EntityKind, wanted: boolean): ReadonlyMap<string, number> | null {
  const { ids } = useTextProject();
  const [read, setRead] = useState<{ kind: EntityKind; numbers: ReadonlyMap<string, number> } | null>(null);
  useEffect(() => {
    if (!wanted) return;
    let live = true;
    ids.read(kind).then(
      (byNumber) => live && setRead({ kind, numbers: new Map(Object.entries(byNumber).map(([n, id]) => [id, Number(n)])) }),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [ids, kind, wanted]);
  return read?.kind === kind ? read.numbers : null;
}

/** What a use leads to: the chip's area and target, or none (shown as text). */
type UseLink = { area: Area; target: NavTarget | null } | null;

type Lookups = { trainers: ReadonlyMap<string, number> | null; maps: ReadonlyMap<string, number> | null; flags: FlagListDto | null; vars: FlagListDto | null };

/** Where a use leads: a trainer to Pokémon › Trainer, a map to its header in Welt, a flag or variable to Geschichten › Flags, a script or level script to its file, a message to its text; an object has no link of its own. */
function linkOf(use: StoryUse, at: Lookups): UseLink {
  const flag = (kind: FlagKind, id: string, list: FlagListDto | null): UseLink => {
    const number = flagNumberOf(kind, id, list);
    return { area: "geschichten", target: number === null ? null : { kind: "flag", flagKind: kind, number } };
  };
  if ("trainer" in use) {
    const index = at.trainers?.get(use.trainer) ?? fallbackNumber("TRAINER", use.trainer);
    return { area: "pokemon", target: index === null ? null : { kind: "trainer", index } };
  }
  if ("map" in use) {
    const id = at.maps?.get(use.map) ?? fallbackNumber("MAP", use.map);
    return { area: "welt", target: id === null ? null : { kind: "header", id } };
  }
  if ("flag" in use) return flag("flag", use.flag, at.flags);
  if ("var" in use) return flag("var", use.var, at.vars);
  if ("script" in use) return { area: "welt", target: { kind: "script", file: use.script.file } };
  if ("levelScript" in use) return { area: "welt", target: { kind: "script", file: use.levelScript.file } };
  if ("message" in use) return { area: "texte", target: { kind: "textArchive", index: use.message.archive, message: use.message.index } };
  return null;
}

/** The uses of a step: a chip each that opens where it leads (locked when the id names nothing known), an object as text. */
function Uses({ uses, at, goTo }: { uses: readonly StoryUse[]; at: Lookups; goTo: (t: NavTarget) => void }) {
  if (uses.length === 0) return <span className={styles.none}>–</span>;
  return (
    <span className={styles.uses}>
      {uses.map((use, i) => {
        const link = linkOf(use, at);
        const text = useText(use);
        if (link === null) {
          return (
            <span key={i} className={styles.useText} data-story-use="">
              {text}
            </span>
          );
        }
        const target = link.target;
        return (
          <Reference
            key={i}
            area={link.area}
            label={text}
            onOpen={() => target !== null && goTo(target)}
            disabled={target === null}
            dataAttrs={{ "data-story-use": "" }}
          />
        );
      })}
    </span>
  );
}

/**
 * The detail of a story (3d.6, reading): its name and its notes as fields that write when left (`story_update`, the fields locked while
 * it runs, a refusal shown below them), its progress as a chip that opens Geschichten › Flags und Variablen on the number, the steps as a
 * table (value, kind, name, uses as chips; the step recipes of „Schritt anlegen“ above it create them, 3d.7), and the graph of the progress variable with the
 * steps on its value nodes. A flag (the graph takes a variable only), an id that names nothing and a variable no script uses have no
 * graph: „Noch keine Werte im Spiel“. The menu „Aktionen“ of the head has „Geschichte löschen…“, which asks first
 * ({@link DeleteStoryDialog}).
 */
export function StoryDetail({ story, depth, goTo, onUpdate, onDeleted, project, revision, onRecipe, textProject = true }: StoryDetailProps): JSX.Element {
  const { id, name, notes = "", progress, steps = [] } = story.story;
  const tech = depthAtLeast(depth, "bauen");
  const progressOf = progressLabel(progress);
  const variable = progressOf.kind === "var" ? story.progress_number : null;
  const { graph, error: graphError } = useThreadGraph(variable, id, { project, revision });

  // What was written last, shown until the story is read back (a field would show the old text meanwhile); a new read clears it.
  const [written, setWritten] = useState<{ name?: string; notes?: string }>({});
  const [seen, setSeen] = useState(story);
  if (seen !== story) {
    setSeen(story);
    setWritten({});
  }
  const [saving, setSaving] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const busy = useRef(false);
  const shownName = written.name ?? name;
  const shownNotes = written.notes ?? notes;
  const commit = async (patch: { name?: string; notes?: string } | { layout: Layout }) => {
    if (busy.current) return false;
    busy.current = true;
    setSaving(true);
    setRefusal(null);
    if (!("layout" in patch)) setWritten((w) => ({ ...w, ...patch }));
    try {
      await onUpdate(patch);
    } catch (e) {
      setRefusal(errorText(e));
      // The field shows what the story holds again.
      setWritten((w) => Object.fromEntries(Object.entries(w).filter(([key]) => !(key in patch))));
    } finally {
      busy.current = false;
      setSaving(false);
    }
    return true;
  };

  // A drag's layout is written 500 ms after its end; a drag meanwhile starts the wait anew (one write, one undo step). While another write
  // runs it waits once more; a layout still waiting when the detail goes is written then.
  const layoutWait = useRef<{ timer: ReturnType<typeof setTimeout>; layout: Layout } | null>(null);
  const writeLayout = useStableCallback(() => {
    const waiting = layoutWait.current;
    if (waiting === null) return;
    layoutWait.current = null;
    void commit({ layout: waiting.layout }).then((went) => {
      if (!went) onLayout(waiting.layout);
    });
  });
  const onLayout = useStableCallback((layout: Layout) => {
    if (layoutWait.current !== null) clearTimeout(layoutWait.current.timer);
    layoutWait.current = { layout, timer: setTimeout(writeLayout, LAYOUT_DELAY_MS) };
  });
  useEffect(
    () => () => {
      if (layoutWait.current === null) return;
      clearTimeout(layoutWait.current.timer);
      writeLayout();
    },
    [writeLayout],
  );

  const uses = steps.flatMap((s) => s.uses ?? []);
  const lookups: Lookups = {
    trainers: useIdNumbers("Trainer", uses.some((u) => "trainer" in u)),
    maps: useIdNumbers("Map", uses.some((u) => "map" in u)),
    flags: useFlagList("flag", { project: uses.some((u) => "flag" in u) ? project : null, revision, source: FLAGS_SOURCE }).list,
    vars: useFlagList("var", { project: uses.some((u) => "var" in u) ? project : null, revision, source: FLAGS_SOURCE }).list,
  };
  const rows: TableRow[] = steps.map((s: Step) => ({
    key: s.key,
    attrs: { "data-story-step": s.key },
    cells: {
      value: String(s.value),
      kind: <StepKindBadge step={s} />,
      name: <StepName step={s} />,
      uses: <Uses uses={s.uses ?? []} at={lookups} goTo={goTo} />,
    },
  }));
  const [open, setOpen] = useState({ steps: true, graph: true });
  const [deleting, setDeleting] = useState(false);
  const [node, setNode] = useState<string | null>(null);
  const number = story.progress_number;

  let graphBody: ReactNode;
  if (variable === null) graphBody = <EmptyGraph />;
  else if (graphError !== null) graphBody = <Notice kind="error">{graphError}</Notice>;
  else if (graph === null) graphBody = <p className={styles.note}>Graph wird gelesen …</p>;
  else if (graph.nodes.length === 0) graphBody = <EmptyGraph />;
  else
    graphBody = (
      <div className={styles.graphBox}>
        <Suspense fallback={<p className={styles.note}>Graph wird geladen…</p>}>
          <ThreadGraph graph={graph} selected={node} onSelect={setNode} onOpenPlace={goTo} onLayout={onLayout} />
        </Suspense>
      </div>
    );

  return (
    <section className={styles.detail} data-story-detail={id} aria-label={`Geschichte ${shownName}`}>
      <div className={styles.detailHead}>
        <div className={styles.nameField}>
          <TextField
            label="Name"
            showLabel
            value={shownName}
            disabled={saving}
            onCommit={(text) => {
              const next = text.trim();
              if (next !== "" && next !== shownName) void commit({ name: next });
            }}
          />
        </div>
        <div className={styles.progressChip}>
          <span className={styles.label}>Fortschritt</span>
          <Reference
            area="geschichten"
            label={progressOf.id}
            tech={tech && number !== null ? hex4(number) : undefined}
            onOpen={() => number !== null && goTo({ kind: "flag", flagKind: progressOf.kind, number })}
            disabled={number === null}
            dataAttrs={{ "data-story-progress": progressOf.kind }}
          />
        </div>
        <HeadActions items={[{ label: "Geschichte löschen…", onSelect: () => setDeleting(true), disabled: saving }]} />
      </div>
      <div data-story-notes="">
        <TextField
          label="Notizen"
          showLabel
          multiline
          value={shownNotes}
          disabled={saving}
          onCommit={(text) => text !== shownNotes && void commit({ notes: text })}
        />
      </div>
      {saving && (
        <p className={styles.note} role="status">
          Wird gespeichert …
        </p>
      )}
      {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      <Section title="Schritte" count={steps.length} open={open.steps} onToggle={() => setOpen((o) => ({ ...o, steps: !o.steps }))}>
        {onRecipe && <StepRecipes story={story} textProject={textProject} onRecipe={onRecipe} />}
        {steps.length === 0 ? <p className={styles.note}>{NO_STEPS}</p> : <Table columns={COLUMNS} rows={rows} ariaLabel="Schritte" />}
      </Section>
      <Section title="Graph" open={open.graph} onToggle={() => setOpen((o) => ({ ...o, graph: !o.graph }))}>
        {graphBody}
      </Section>
      {deleting && <DeleteStoryDialog id={id} name={name} onClose={() => setDeleting(false)} onDeleted={onDeleted} />}
    </section>
  );
}

/** The kind of a step as a badge; a step a recipe made says which in its tooltip („Angelegt von „Neue Geschichte“ (Studio 0.3.0)“). */
function StepKindBadge({ step }: { step: Step }) {
  const badge = (
    <span data-story-kind={step.kind} {...(step.made && { "data-story-made": step.made.recipe })}>
      <Badge kind="neutral">{stepKindWord(step.kind)}</Badge>
    </span>
  );
  if (!step.made) return badge;
  return <Tooltip content={`Angelegt von „${recipeTitle(step.made.recipe)}“ (Studio ${step.made.studio})`}>{badge}</Tooltip>;
}

/** The name of a step, its notes under it. */
function StepName({ step }: { step: Step }) {
  if (!step.notes) return <>{step.name}</>;
  return (
    <span className={styles.stepName}>
      {step.name}
      <span className={styles.stepNotes} data-story-step-notes="">
        {step.notes}
      </span>
    </span>
  );
}

/** What the dialog „Geschichte „X“ löschen?“ says: what goes, and that ⌘Z brings it back. */
export const DELETE_STORY_TEXT = "Die Datei der Geschichte, ihr Layout und ihr Registereintrag gehen. Rückgängig mit ⌘Z.";

/**
 * „Geschichte „X“ löschen?“: no plan to ask (nothing points at a story, it has no tombstone), only „Abbrechen“ and „Löschen“. „Löschen“
 * calls `story_delete` once (both buttons locked while it runs, a second click does nothing), reports and closes; a refusal stays in the
 * dialog with the kernel's sentence and „Löschen“ goes again.
 */
function DeleteStoryDialog({ id, name, onClose, onDeleted }: { id: string; name: string; onClose: () => void; onDeleted: (r: StoryReport, name: string) => void }) {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);
  const remove = async () => {
    if (busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      const report = await storyDelete(id);
      onDeleted(report, name);
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      title={`Geschichte „${name}“ löschen?`}
      description={DELETE_STORY_TEXT}
      data={{ "data-story-delete-dialog": "" }}
      actions={[
        { label: "Abbrechen", onClick: onClose, disabled: running },
        { label: "Löschen", kind: "primary", onClick: () => void remove(), disabled: running },
      ]}
    >
      {running ? (
        <p role="status" className={styles.note}>
          Lösche …
        </p>
      ) : error !== null ? (
        <Notice kind="error">{error}</Notice>
      ) : null}
    </Dialog>
  );
}

function EmptyGraph() {
  return (
    <p className={styles.note} data-story-empty-graph="">
      {NO_VALUES}
    </p>
  );
}
