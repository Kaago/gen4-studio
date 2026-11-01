import { Suspense, lazy, useEffect, useRef, useState } from "react";
import type { JSX, ReactNode } from "react";
import { errorText } from "../api";
import type { EntityKind, FlagKind, FlagListDto, ProjectInfo, Step, StoryDto, StoryUse, storyUpdate } from "../api";
import { FLAGS_SOURCE } from "../flags/FlagsSurface";
import { hex4 } from "../flags/flagWords";
import { useFlagList } from "../flags/useFlagList";
import { useTextProject } from "../textproject/TextProjectContext";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Badge, Notice, Reference, Section, Table, TextField } from "../ui/primitives";
import type { Area, TableColumn, TableRow } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { fallbackNumber, flagNumberOf, progressLabel, stepKindWord, useText } from "./storyWords";
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
  project: ProjectInfo | null;
  /** Moves whenever the project may differ: the graph is read again. */
  revision: number;
};

const COLUMNS: readonly TableColumn[] = [
  { key: "value", label: "Wert", width: 64, align: "end" },
  { key: "kind", label: "Art", width: 200 },
  { key: "name", label: "Name" },
  { key: "uses", label: "Verweise" },
];

/** Said in place of the graph when the game has no value of the progress (a free variable, a flag, an id that names nothing). */
export const NO_VALUES = "Noch keine Werte im Spiel";

/** Said in place of the steps: 3d.6 only reads them, the recipes of 3d.7 create them. */
export const NO_STEPS = "Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7).";

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
 * table (value, kind, name, uses as chips; only read: the recipes of 3d.7 create them), and the graph of the progress variable with the
 * steps on its value nodes. A flag (the graph takes a variable only), an id that names nothing and a variable no script uses have no
 * graph: „Noch keine Werte im Spiel“.
 */
export function StoryDetail({ story, depth, goTo, onUpdate, project, revision }: StoryDetailProps): JSX.Element {
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
  const commit = async (patch: { name?: string; notes?: string }) => {
    if (busy.current) return;
    busy.current = true;
    setSaving(true);
    setRefusal(null);
    setWritten((w) => ({ ...w, ...patch }));
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
  };

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
      kind: (
        <span data-story-kind={s.kind}>
          <Badge kind="neutral">{stepKindWord(s.kind)}</Badge>
        </span>
      ),
      name: s.name,
      uses: <Uses uses={s.uses ?? []} at={lookups} goTo={goTo} />,
    },
  }));
  const [open, setOpen] = useState({ steps: true, graph: true });
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
          <ThreadGraph graph={graph} selected={node} onSelect={setNode} onOpenPlace={goTo} />
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
        {steps.length === 0 ? <p className={styles.note}>{NO_STEPS}</p> : <Table columns={COLUMNS} rows={rows} ariaLabel="Schritte" />}
      </Section>
      <Section title="Graph" open={open.graph} onToggle={() => setOpen((o) => ({ ...o, graph: !o.graph }))}>
        {graphBody}
      </Section>
    </section>
  );
}

function EmptyGraph() {
  return (
    <p className={styles.note} data-story-empty-graph="">
      {NO_VALUES}
    </p>
  );
}
