import { useEffect, useMemo, useRef, useState } from "react";
import { acceptBaseline, applyFix, checksStatus, ignoreFinding, planFix, runChecks, setCheckEnabled, unignoreFinding } from "../api";
import type { AssetId, CheckGroup, CheckProblemDto, CheckScope, ChecksProgressDto, ChecksReportDto, FixId, FixPlanDto, FixResultDto, ProjectInfo } from "../api";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";
import { CHECK_CODES, groupOf } from "./messages";

/** The cheap checks run this long after the last edit (spec 3b.4 §5). */
export const CHECKS_DEBOUNCE_MS = 300;
/** How often a running pass is asked how far it got (`checks_status`). */
export const CHECKS_POLL_MS = 250;

type Skipped = ChecksReportDto["skipped"][number];

/** What the light, the Problems panel and Technik show of the checks besides the findings (those are in the `ProblemsStore`, source `check`). */
export interface ChecksView {
  /** A report of this project arrived (before: the findings say nothing yet). */
  reported: boolean;
  /** Ignored warnings and infos („12 ausgeblendet“). */
  ignored: readonly CheckProblemDto[];
  /** Checks that did not run or left part of their work undone, with the kernel's reason (switched-off ones are in `disabled`). */
  skipped: readonly Skipped[];
  /** Codes switched off. */
  disabled: readonly string[];
  /** A pass runs (one this hook asked for everything, or any the backend reports). */
  running: boolean;
  progress: ChecksProgressDto | null;
  /** Only the cheap checks ran since the last change: the script and text findings are those of the last full run. */
  outdated: boolean;
}

const NOTHING: ChecksView = { reported: false, ignored: [], skipped: [], disabled: [], running: false, progress: null, outdated: false };

/** The `ChecksView` for `useSyncExternalStore`: the same object until something in it changed. */
export class ChecksFeed {
  private view: ChecksView = NOTHING;
  private readonly listeners = new Set<() => void>();

  get = (): ChecksView => this.view;

  update(patch: Partial<ChecksView>): void {
    const next = { ...this.view, ...patch };
    if ((Object.keys(next) as (keyof ChecksView)[]).every((key) => next[key] === this.view[key])) return;
    this.view = next;
    for (const listener of [...this.listeners]) listener();
  }

  /** Forgets everything (another project, or none). */
  reset(): void {
    if (this.view === NOTHING) return;
    this.view = NOTHING;
    for (const listener of [...this.listeners]) listener();
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };
}

/** What the runs found, per group from the newest report that covered it. */
export interface MergedChecks {
  findings: CheckProblemDto[];
  ignored: CheckProblemDto[];
  skipped: Skipped[];
  /** The store generation of the newest report taken. */
  generation: number;
  /** That of the newest report that covered the script and text groups; -1 before any. */
  fullGeneration: number;
}

const position = (code: string) => {
  const at = CHECK_CODES.indexOf(code);
  return at < 0 ? CHECK_CODES.length : at;
};
const byCatalogue = <T extends { code: string }>(rows: T[]) => rows.sort((a, b) => position(a.code) - position(b.code));

/**
 * Takes `report` into what the runs found so far: its findings, ignored findings and skipped checks replace those of the groups it covers
 * (a cheap run says nothing about the script and text checks, which stay as the last full run found them). Null when the report is no result
 * (`groups` empty: there is no report of the current state, which is not "clean") or of an older store generation than the one taken.
 */
export function mergeReport(previous: MergedChecks | null, report: ChecksReportDto): MergedChecks | null {
  if (report.groups.length === 0) return null;
  if (previous !== null && report.generation < previous.generation) return null;
  const covered = new Set<CheckGroup>(report.groups);
  const kept = <T extends { code: string }>(rows: readonly T[] | undefined): T[] =>
    (rows ?? []).filter((row) => {
      const group = groupOf(row.code);
      return group !== null && !covered.has(group);
    });
  const full = covered.has("per_script") && covered.has("per_archive");
  return {
    findings: byCatalogue([...kept(previous?.findings), ...report.findings]),
    ignored: byCatalogue([...kept(previous?.ignored), ...report.ignored]),
    skipped: byCatalogue([...kept(previous?.skipped), ...report.skipped]),
    generation: report.generation,
    fullGeneration: full ? report.generation : (previous?.fullGeneration ?? -1),
  };
}

function viewOf(merged: MergedChecks): Partial<ChecksView> {
  return {
    reported: true,
    ignored: merged.ignored,
    skipped: merged.skipped.filter((s) => s.why !== "disabled"),
    disabled: merged.skipped.filter((s) => s.why === "disabled").map((s) => s.code),
    outdated: merged.fullGeneration < merged.generation,
  };
}

/** The checks of the open project as the panels use them. A stable object. */
export interface ChecksApi {
  feed: ChecksFeed;
  /** „Erneut prüfen“: every group; resolves when the queued runs are done. */
  recheck: () => Promise<void>;
  /** „Ausblenden“ (a warning or info); a refusal goes to the app's error message. */
  ignore: (finding: CheckProblemDto) => Promise<void>;
  /** „Wieder melden“. */
  unignore: (finding: CheckProblemDto) => Promise<void>;
  /** „Ausgangslage übernehmen“: ignores every warning and info of the current state. */
  acceptBaseline: () => Promise<void>;
  setEnabled: (code: string, enabled: boolean) => Promise<void>;
  /** What `fix` would change; rejects as the command does (code "finding_stale" when the finding is no longer current). */
  planFix: (finding: CheckProblemDto, fix: FixId) => Promise<FixPlanDto>;
  /** Applies `fix`, reloads what it changed (`onFixed`) and checks everything again; rejects as the command does. */
  applyFix: (finding: CheckProblemDto, fix: FixId) => Promise<FixResultDto>;
}

export interface UseChecksParams {
  /** Its topic `edited` (every write the app counts) starts the cheap run after `CHECKS_DEBOUNCE_MS`. */
  bus: EditBus;
  /** Gets the findings as its source `check`. */
  problems: ProblemsStore;
  /** The open project (a new object for every opening): it is checked as a whole when it opens; null: nothing is checked. */
  project: ProjectInfo | null;
  onError: (e: unknown) => void;
  /** A fix changed these assets: the editors and the scene read them again. */
  onFixed?: (changed: AssetId[]) => void | Promise<void>;
}

/**
 * The project checks in the frontend (spec 3b.4 §5, ruling R4): everything when a project opens and on `recheck`, the cheap group
 * `CHECKS_DEBOUNCE_MS` after the last `edited`, the same in the app and in the browser mode. One run at a time: a request while one runs
 * waits for it (a full one wins over a cheap one). While a run is under way `checks_status` is asked every `CHECKS_POLL_MS` for its progress.
 * Answers of another project, without a report (`groups` empty) or of an older store generation never replace what is shown.
 */
export function useChecks(params: UseChecksParams): ChecksApi {
  const { bus, project } = params;
  const [feed] = useState(() => new ChecksFeed());
  const latest = useRef(params);
  latest.current = params;
  const state = useRef({
    project: null as ProjectInfo | null,
    merged: null as MergedChecks | null,
    /** The widest scope asked for and not started yet. */
    pending: null as CheckScope | null,
    /** The runs being worked off, one after the other. */
    loop: null as Promise<void> | null,
    /** A run of this hook is waiting for its answer. */
    inFlight: false,
    /** That run checks everything: the light turns until it ends. */
    ownFull: false,
    /** What the backend said last: a pass runs, and how far it got. */
    backendRunning: false,
    progress: null as ChecksProgressDto | null,
    poll: null as ReturnType<typeof setTimeout> | null,
    debounce: null as ReturnType<typeof setTimeout> | null,
  });
  const s = state.current;

  // The commands, and the hook's own handles on the queue (`request`) and the progress (`poll`); made once.
  const engine = useMemo(() => {
    const show = () => {
      const running = s.ownFull || s.backendRunning;
      feed.update({ running, progress: running ? s.progress : null });
    };
    const stopPoll = () => {
      if (s.poll !== null) clearTimeout(s.poll);
      s.poll = null;
    };
    /** Takes an answer of `project` (a run, a status, an ignore, ...). */
    const take = (report: ChecksReportDto, of: ProjectInfo) => {
      if (s.project !== of) return;
      s.backendRunning = report.running;
      s.progress = report.progress ?? null;
      const merged = mergeReport(s.merged, report);
      if (merged !== null) {
        s.merged = merged;
        latest.current.problems.replaceChecks(merged.findings);
        feed.update(viewOf(merged));
      }
      show();
    };
    const poll = (of: ProjectInfo) => {
      if (s.poll !== null) return;
      s.poll = setTimeout(async () => {
        s.poll = null;
        if (s.project !== of) return;
        try {
          take(await checksStatus(), of);
        } catch {
          /* the next look asks again */
        }
        if (s.project === of && (s.inFlight || s.backendRunning)) poll(of);
      }, CHECKS_POLL_MS);
    };
    const work = async () => {
      while (s.pending !== null && s.project !== null) {
        const scope = s.pending;
        const of = s.project;
        s.pending = null;
        s.inFlight = true;
        s.ownFull = scope === "all";
        show();
        poll(of);
        try {
          take(await runChecks(scope), of);
        } catch (e) {
          if (s.project === of) latest.current.onError(e);
        } finally {
          s.inFlight = false;
          s.ownFull = false;
          if (!s.backendRunning) stopPoll();
          if (s.project === of) show();
        }
      }
    };
    const request = (scope: CheckScope): Promise<void> => {
      s.pending = s.pending === "all" || scope === "all" ? "all" : "cheap";
      if (s.loop === null) s.loop = work().finally(() => (s.loop = null));
      return s.loop;
    };
    /** An answer of a command that changes the ignore list; everything runs again when it lacks the group of `code` (or any report). */
    const answer = async (call: () => Promise<ChecksReportDto>, code: string | null) => {
      const of = s.project;
      if (of === null) return;
      try {
        const report = await call();
        if (s.project !== of) return;
        take(report, of);
        const group = code === null ? null : groupOf(code);
        if (report.groups.length === 0 || (group !== null && !report.groups.includes(group))) void request("all");
      } catch (e) {
        if (s.project === of) latest.current.onError(e);
      }
    };
    const api: ChecksApi = {
      feed,
      recheck: () => request("all"),
      ignore: (f) => answer(() => ignoreFinding(f), f.code),
      unignore: (f) => answer(() => unignoreFinding(f), f.code),
      acceptBaseline: () => answer(() => acceptBaseline(), null),
      setEnabled: (code, enabled) => answer(() => setCheckEnabled(code, enabled), code),
      planFix: (f, fix) => planFix(f, fix),
      applyFix: async (f, fix) => {
        const of = s.project;
        const result = await applyFix(f, fix);
        if (s.project === of) {
          try {
            await latest.current.onFixed?.(result.changed);
          } finally {
            void request("all");
          }
        }
        return result;
      },
    };
    return { api, request, poll };
  }, [feed, s]);

  // Another project (or none): everything shown goes; an opened one is checked as a whole.
  useEffect(() => {
    if (s.project === project) return;
    s.project = project;
    s.merged = null;
    s.pending = null;
    s.backendRunning = false;
    s.progress = null;
    if (s.debounce !== null) clearTimeout(s.debounce);
    s.debounce = null;
    latest.current.problems.replaceChecks([]);
    feed.reset();
    if (project !== null) void engine.request("all");
  }, [project, engine, feed, s]);

  // 300 ms after the last edit: the cheap checks.
  useEffect(
    () =>
      bus.subscribe("edited", () => {
        if (s.project === null) return;
        if (s.debounce !== null) clearTimeout(s.debounce);
        s.debounce = setTimeout(() => {
          s.debounce = null;
          void engine.request("cheap");
        }, CHECKS_DEBOUNCE_MS);
      }),
    [bus, engine, s],
  );

  // No timer outlives the component; a run still under way when it comes back (StrictMode) is watched again.
  useEffect(() => {
    if (s.inFlight && s.project !== null) engine.poll(s.project);
    return () => {
      if (s.poll !== null) clearTimeout(s.poll);
      s.poll = null;
      if (s.debounce !== null) clearTimeout(s.debounce);
      s.debounce = null;
    };
  }, [engine, s]);

  return engine.api;
}
