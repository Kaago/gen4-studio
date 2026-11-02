import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import type { FlagArea, FlagKind, FlagListDto, NameReport, ProjectInfo } from "../api";
import { useTextProject } from "../textproject/TextProjectContext";
import type { Depth } from "../ui/frame/shellState";
import { Checkbox, EmptyState, FilterChips, Notice, TextField } from "../ui/primitives";
import type { FilterChipOption } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { FlagDetail } from "./FlagDetail";
import { DEFAULT_FILTER, areasOf, filterEntries } from "./flagFilter";
import type { FlagFilter } from "./flagFilter";
import { FlagList } from "./FlagList";
import { areaWord } from "./flagWords";
import { NameFlagDialog, namedText } from "./NameFlagDialog";
import { invalidateFlagLists, useFlagList } from "./useFlagList";
import { useFlagUsage } from "./useFlagUsage";
import styles from "./FlagsSurface.module.css";

/** What the surface and the entry of Geschichten read under: one read of the flags for both (the name stayed from Technik, 3d.1). */
export const FLAGS_SOURCE = "technik";

/** Why the list's „frei“ is only a hint while the scripts are not adopted (ruling R15). */
export const SCRIPTS_NOT_ADOPTED_NOTE = "Scripts sind noch nicht übernommen: Script-Verweise fehlen im Index, „frei“ ist darum nur ein Hinweis.";

/** Where own names come from in the DSPRE layout. */
export const OWN_NAMES_NOTE = "Eigene Namen gibt es im Textprojekt: Technik › Textprojekt › „Als Textprojekt einrichten…“.";

const de = (n: number) => n.toLocaleString("de-DE");

/** The line under „Flags und Variablen“ in the list of Geschichten: „4.097 Flags · 135 frei“, „Wird gelesen …“ while the list is read. */
export function flagsSubtitle(list: FlagListDto | null): string {
  if (list === null) return "Wird gelesen …";
  const n = list.entries.length;
  const what = list.kind === "flag" ? (n === 1 ? "Flag" : "Flags") : n === 1 ? "Variable" : "Variablen";
  return `${de(n)} ${what} · ${de(list.pool_free)} frei`;
}

export type FlagsSurfaceProps = {
  project: ProjectInfo | null;
  /** Moves whenever the project may differ (an edit, a save, an undo): list and usage are read again. */
  revision: number;
  /** The chosen number, held by the app (a link from a picker sets it). */
  selected: { kind: FlagKind; number: number } | null;
  onSelect: (sel: { kind: FlagKind; number: number } | null) => void;
  goTo: (t: NavTarget) => void;
  depth: Depth;
  /** The project takes no writes now (a save or a build runs): naming waits. */
  readOnly: boolean;
  /** What failed after a naming went in (the report's warnings). */
  onError: (e: unknown) => void;
  /** A naming or a description went in: the report and its sentence („FLAG_X benannt (ein Schritt im Verlauf).“) for the app's message. */
  onNamed?: (report: NameReport, said: string) => void;
  /** „Projekt öffnen“ of the empty state; without it the empty state offers nothing. */
  onOpenProject?: () => void;
};

const KINDS: readonly FilterChipOption[] = [
  { value: "flag", label: "Flags" },
  { value: "var", label: "Variablen" },
];

/** The naming dialog the detail opened: naming a number or describing its own name. */
type Naming = { kind: FlagKind; number: number; name: string | null; current: { id: string | null; description: string | null } };

/**
 * Geschichten › Flags und Variablen (moved from Technik with 3d.6): which flags and variables the game knows and where scripts, objects,
 * triggers, level scripts and trainers set or check them. Above the list the filters (kind, area, unused, free, own name, a search); the list and the detail of the chosen
 * number side by side when the surface has room (≥ 760 px), else the detail below. In the DSPRE layout a notice names where own names come
 * from; while the scripts are not adopted, one says that „frei“ is only a hint. A link (`selected` from outside) shows its kind and lifts a
 * filter that would hide it.
 */
export function FlagsSurface({ project, revision, selected, onSelect, goTo, depth, readOnly, onError, onNamed, onOpenProject }: FlagsSurfaceProps): JSX.Element {
  const titleId = useId();
  const text = useTextProject();
  const [filter, setFilter] = useState<FlagFilter>(() => ({ ...DEFAULT_FILTER, kind: selected?.kind ?? DEFAULT_FILTER.kind }));
  const { list, error } = useFlagList(filter.kind, { project, revision, source: FLAGS_SOURCE });
  const entries = useMemo(() => (list === null ? [] : filterEntries(list.entries, filter)), [list, filter]);

  // A selection from outside (a link) of another kind, or one the filter hides: its kind, without any other filter.
  const key = selected === null ? null : `${selected.kind}:${selected.number}`;
  const [seen, setSeen] = useState(key);
  if (seen !== key) {
    setSeen(key);
    if (selected !== null && (selected.kind !== filter.kind || (list !== null && !entries.some((e) => e.number === selected.number)))) {
      setFilter({ ...DEFAULT_FILTER, kind: selected.kind });
    }
  }
  const number = selected !== null && selected.kind === filter.kind ? selected.number : null;
  const { usage, error: usageError, busy } = useFlagUsage(filter.kind, number, { project, revision });

  // The chosen row comes into view when it is chosen (a link) and when the list first shows (a click chose a row in view already); a list
  // read again after an edit leaves the scroll where it is.
  const surface = useRef<HTMLElement>(null);
  const listed = list !== null;
  useEffect(() => {
    if (number === null || !listed) return;
    surface.current?.querySelector(`[data-flag-row="${number}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [number, listed]);

  const kind = filter.kind;
  const select = useCallback((n: number) => onSelect({ kind, number: n }), [onSelect, kind]);
  const [naming, setNaming] = useState<Naming | null>(null);
  const change = (next: Partial<FlagFilter>) => setFilter((f) => ({ ...f, ...next }));
  const areas: FilterChipOption[] = [
    { value: "all", label: "Alle" },
    ...areasOf(kind)
      .filter((a) => a !== "temp" || list?.entries.some((e) => e.area === "temp"))
      .map((a) => ({ value: a, label: areaWord(a) })),
  ];
  const entry = number === null ? null : (list?.entries.find((e) => e.number === number) ?? null);
  const names = usage ?? entry;
  const onDone = (report: NameReport) => {
    // The lists on screen (the chips of the inspector too) and this usage show the new name.
    invalidateFlagLists();
    onNamed?.(report, namedText(report));
    for (const warning of report.warnings) onError(warning);
  };

  return (
    <section ref={surface} className={styles.surface} data-technik="flags" aria-labelledby={titleId}>
      <header className={styles.head}>
        <h1 id={titleId} className={styles.title}>
          Flags und Variablen
        </h1>
        <p className={styles.text}>Welche Flags und Variablen das Spiel kennt und wo Scripts, Figuren, Auslöser, Level-Scripts und Trainer sie setzen oder prüfen.</p>
      </header>
      {project === null ? (
        <EmptyState
          what="Kein Projekt geöffnet."
          why="Öffne ein Projekt, dann stehen hier seine Flags und Variablen."
          action={onOpenProject && { label: "Projekt öffnen", onClick: onOpenProject }}
        />
      ) : (
        <>
          {list !== null && !list.text_project && <Notice kind="info">{OWN_NAMES_NOTE}</Notice>}
          {list !== null && !list.scripts_indexed && <Notice kind="info">{SCRIPTS_NOT_ADOPTED_NOTE}</Notice>}
          <div className={styles.filters} data-flag-filter="">
            <FilterChips
              single
              ariaLabel="Art"
              options={KINDS}
              value={new Set([kind])}
              onChange={(next) => {
                const picked = [...next][0] as FlagKind | undefined;
                if (picked === undefined || picked === kind) return;
                setFilter((f) => ({ ...f, kind: picked, area: "all" }));
                onSelect(null);
              }}
            />
            <FilterChips
              single
              ariaLabel="Bereich"
              options={areas}
              visible={areas.length}
              value={new Set([filter.area])}
              onChange={(next) => change({ area: ([...next][0] ?? "all") as FlagArea | "all" })}
            />
            <div className={styles.boxes}>
              <Checkbox label="nur unbenutzt" checked={filter.unusedOnly} onChange={(on) => change({ unusedOnly: on })} />
              <Checkbox label="nur frei" checked={filter.freeOnly} onChange={(on) => change({ freeOnly: on })} />
              <Checkbox label="nur mit eigenem Namen" checked={filter.ownOnly} onChange={(on) => change({ ownOnly: on })} />
            </div>
            <div className={styles.search}>
              <TextField
                type="search"
                label="Name oder Nummer suchen"
                placeholder="Name oder Nummer suchen"
                value={filter.query}
                onChange={(query) => change({ query })}
                onCommit={(query) => change({ query })}
              />
            </div>
          </div>
          {error !== null && <Notice kind="error">{error}</Notice>}
          {list === null ? (
            error === null && <p className={styles.note}>Wird gelesen …</p>
          ) : (
            <div className={styles.split} data-flag-split="">
              <div className={styles.columns}>
                <FlagList list={list} entries={entries} selected={number} onSelect={select} />
                <FlagDetail
                  kind={kind}
                  number={number}
                  entry={entry}
                  usage={usage}
                  busy={busy}
                  error={usageError}
                  textProject={list.text_project}
                  readOnly={readOnly}
                  depth={depth}
                  goTo={goTo}
                  onRename={text.renameId}
                  onName={() => number !== null && setNaming({ kind, number, name: names?.name ?? null, current: { id: null, description: null } })}
                  onDescribe={() =>
                    number !== null && setNaming({ kind, number, name: names?.name ?? null, current: { id: names?.own_id ?? null, description: names?.description ?? null } })
                  }
                />
              </div>
            </div>
          )}
        </>
      )}
      {naming !== null && (
        <NameFlagDialog
          key={`${naming.kind}:${naming.number}:${naming.current.id ?? ""}`}
          kind={naming.kind}
          number={naming.number}
          name={naming.name}
          current={naming.current}
          onClose={() => setNaming(null)}
          onDone={onDone}
        />
      )}
    </section>
  );
}
