import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import type { FlagArea, FlagKind, FlagListDto, NameReport, ProjectInfo } from "../api";
import type { Live } from "../app/useLive";
import { useStableCallback } from "../app/useStable";
import { t as translate } from "../i18n";
import { useTextProject } from "../textproject/TextProjectContext";
import type { Depth } from "../ui/frame/shellState";
import { Checkbox, EmptyState, FilterChips, Notice, TextField } from "../ui/primitives";
import type { FilterChipOption } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { FlagDetail } from "./FlagDetail";
import type { FlagLive } from "./FlagDetail";
import { DEFAULT_FILTER, areasOf, filterEntries } from "./flagFilter";
import type { FlagFilter } from "./flagFilter";
import { FlagList } from "./FlagList";
import { areaWord, displayName } from "./flagWords";
import { NameFlagDialog, namedText } from "./NameFlagDialog";
import { invalidateFlagLists, useFlagList } from "./useFlagList";
import { useFlagUsage } from "./useFlagUsage";
import styles from "./FlagsSurface.module.css";

/** What the surface and the entry of Geschichten read under: one read of the flags for both (the name stayed from Technik, 3d.1). */
export const FLAGS_SOURCE = "technik";

/** The line under „Flags und Variablen“ in the list of Geschichten: „4.097 Flags · 135 frei“, „Wird gelesen …“ while the list is read. */
export function flagsSubtitle(list: FlagListDto | null): string {
  if (list === null) return translate("flags:flagsSurface.reading");
  const count = list.entries.length;
  return list.kind === "flag"
    ? translate("flags:flagsSurface.flagsCount", { count, free: list.pool_free })
    : translate("flags:flagsSurface.varsCount", { count, free: list.pool_free });
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
  /** Play mode (`useLive`): with a snapshot the list's column „Live“ and the detail's section „Live“ show and write its values. */
  live?: Live;
};

/** The naming dialog the detail opened: naming a number or describing its own name. */
type Naming = { kind: FlagKind; number: number; name: string | null; current: { id: string | null; description: string | null } };

/**
 * Geschichten › Flags und Variablen (moved from Technik with 3d.6): which flags and variables the game knows and where scripts, objects,
 * triggers, level scripts and trainers set or check them. Above the list the filters (kind, area, unused, free, own name, a search); the list and the detail of the chosen
 * number side by side when the surface has room (≥ 760 px), else the detail below. In the DSPRE layout a notice names where own names come
 * from; while the scripts are not adopted, one says that „frei“ is only a hint. A link (`selected` from outside) shows its kind and lifts a
 * filter that would hide it.
 */
export function FlagsSurface({ project, revision, selected, onSelect, goTo, depth, readOnly, onError, onNamed, onOpenProject, live }: FlagsSurfaceProps): JSX.Element {
  const { t } = useTranslation("flags");
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
  const kinds: readonly FilterChipOption[] = [
    { value: "flag", label: t("flagsSurface.kindFlags") },
    { value: "var", label: t("flagsSurface.kindVars") },
  ];
  const areas: FilterChipOption[] = [
    { value: "all", label: t("flagsSurface.all") },
    ...areasOf(kind)
      .filter((a) => a !== "temp" || list?.entries.some((e) => e.area === "temp"))
      .map((a) => ({ value: a, label: areaWord(a) })),
  ];
  const entry = number === null ? null : (list?.entries.find((e) => e.number === number) ?? null);
  const names = usage ?? entry;

  // Play mode as the hinge shows it (ruling R19: not while melonDS is unavailable): the values are the snapshot's (never what was asked
  // for), a write goes straight to the game and is no step in the history. Before the first snapshot the cells wait with the hinge's text.
  const hinge = live?.hinge ?? null;
  const values = hinge !== null ? (live?.values ?? null) : null;
  const livePending = hinge !== null && values === null ? hinge.text : undefined;
  // The number last written (its read-back is the status's) and how many writes are under way.
  const [written, setWritten] = useState<{ kind: FlagKind; number: number } | null>(null);
  const [writing, setWriting] = useState(0);
  const writeLive = useStableCallback(async (k: FlagKind, n: number, value: number) => {
    if (live === undefined) return;
    setWritten({ kind: k, number: n });
    setWriting((w) => w + 1);
    try {
      await (k === "flag" ? live.setFlag(n, value !== 0) : live.setVar(n, value));
    } finally {
      setWriting((w) => w - 1);
    }
  });
  const onLiveToggle = useStableCallback((n: number) => {
    const set = values?.flag(n) ?? null;
    if (set !== null) void writeLive("flag", n, set ? 0 : 1);
  });
  // A write the game undid (ruling R20): the detail warns for the chosen number, the surface names any other one.
  const notTaken = hinge !== null && written !== null && live?.status.read_back === false;
  const notTakenElsewhere = notTaken && !(written.kind === kind && written.number === number);
  const writtenEntry = written !== null && list !== null && list.kind === written.kind ? list.entries.find((e) => e.number === written.number) : undefined;
  const writtenName = written === null ? "" : displayName(writtenEntry ?? { name: null, own_id: null }, written.kind, written.number);
  let flagLive: FlagLive | undefined;
  if (values !== null && live !== undefined && number !== null) {
    const set = kind === "flag" ? values.flag(number) : null;
    flagLive = {
      value: kind === "flag" ? (set === null ? null : set ? 1 : 0) : values.var(number),
      busy: writing > 0,
      readBack: !(notTaken && written.kind === kind && written.number === number),
      onSet: (value) => void writeLive(kind, number, value),
    };
  }
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
          {t("flagsSurface.title")}
        </h1>
        <p className={styles.text}>{t("flagsSurface.intro")}</p>
      </header>
      {project === null ? (
        <EmptyState
          what={t("flagsSurface.noProject")}
          why={t("flagsSurface.noProjectWhy")}
          action={onOpenProject && { label: t("flagsSurface.openProject"), onClick: onOpenProject }}
        />
      ) : (
        <>
          {list !== null && !list.text_project && <Notice kind="info">{t("flagsSurface.ownNamesNote")}</Notice>}
          {list !== null && !list.scripts_indexed && <Notice kind="info">{t("flagsSurface.scriptsNotAdopted")}</Notice>}
          <div className={styles.filters} data-flag-filter="">
            <FilterChips
              single
              ariaLabel={t("flagsSurface.kindLabel")}
              options={kinds}
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
              ariaLabel={t("flagsSurface.rangeLabel")}
              options={areas}
              visible={areas.length}
              value={new Set([filter.area])}
              onChange={(next) => change({ area: ([...next][0] ?? "all") as FlagArea | "all" })}
            />
            <div className={styles.boxes}>
              <Checkbox label={t("flagsSurface.unusedOnly")} checked={filter.unusedOnly} onChange={(on) => change({ unusedOnly: on })} />
              <Checkbox label={t("flagsSurface.freeOnly")} checked={filter.freeOnly} onChange={(on) => change({ freeOnly: on })} />
              <Checkbox label={t("flagsSurface.ownOnly")} checked={filter.ownOnly} onChange={(on) => change({ ownOnly: on })} />
            </div>
            <div className={styles.search}>
              <TextField
                type="search"
                label={t("flagsSurface.search")}
                placeholder={t("flagsSurface.search")}
                value={filter.query}
                onChange={(query) => change({ query })}
                onCommit={(query) => change({ query })}
              />
            </div>
          </div>
          {error !== null && <Notice kind="error">{error}</Notice>}
          {notTakenElsewhere && (
            <div data-flag-live-warning="">
              <Notice kind="warning">{t("flagsSurface.notTakenFor", { name: writtenName, reason: t("flagDetail.notTaken") })}</Notice>
            </div>
          )}
          {list === null ? (
            error === null && <p className={styles.note}>{t("flagsSurface.reading")}</p>
          ) : (
            <div className={styles.split} data-flag-split="">
              <div className={styles.columns}>
                <FlagList list={list} entries={entries} selected={number} onSelect={select} live={values} livePending={livePending} onLiveToggle={onLiveToggle} />
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
                  live={flagLive}
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
