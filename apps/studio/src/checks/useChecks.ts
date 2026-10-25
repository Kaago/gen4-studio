import { useEffect, useMemo, useRef, useState } from "react";
import { acceptBaseline, applyFix, checksStatus, ignoreFinding, planFix, runChecks, setCheckEnabled, unignoreFinding } from "../api";
import type { AssetId, CheckGroup, CheckProblemDto, ChecksProgressDto, ChecksReportDto, FixId, FixPlanDto, FixResultDto, ProjectInfo } from "../api";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";

/** The checks run this long after the last edit (spec 3b.4 §5). */
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
  /** A pass runs (one this hook asked for, or any the backend reports). */
  running: boolean;
  progress: ChecksProgressDto | null;
}

const NOTHING: ChecksView = { reported: false, ignored: [], skipped: [], disabled: [], running: false, progress: null };

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

/**
 * Whether `report` replaces what is shown (the report of store generation `shown`; null: none yet). Only a report of every group does
 * (spec 3b.4 §5: the source `check` is replaced as a whole per run), and only one of the shown generation or a newer one: an answer
 * without groups (there is no report of the current state, which is not "clean"), a report of some groups (a cheap run says nothing about
 * the script and text checks) and one of an older state are no result.
 */
export function replacesShown(shown: number | null, report: ChecksReportDto): boolean {
  if (!ALL_GROUPS.every((group) => report.groups.includes(group))) return false;
  return shown === null || report.generation >= shown;
}

const ALL_GROUPS: readonly CheckGroup[] = ["cheap", "per_script", "per_archive"];

function viewOf(report: ChecksReportDto): Partial<ChecksView> {
  return {
    reported: true,
    ignored: report.ignored,
    skipped: report.skipped.filter((s) => s.why !== "disabled"),
    disabled: report.skipped.filter((s) => s.why === "disabled").map((s) => s.code),
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
  /** Its topic `edited` (every write the app counts) starts a run after `CHECKS_DEBOUNCE_MS`. */
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
 * The project checks in the frontend (spec 3b.4 §5, rulings R4 and R12): everything when a project opens, on `recheck` and
 * `CHECKS_DEBOUNCE_MS` after the last `edited` (the kernel caches the text and script checks per file, so a warm run reads only what
 * changed), the same in the app and in the browser mode. One run at a time: a request while one runs waits for it. While a run is under way
 * the light turns and `checks_status` is asked every `CHECKS_POLL_MS` for its progress. Each report replaces the findings as a whole;
 * answers of another project, without a report (`groups` empty), of some groups only or of an older store generation never replace what is
 * shown (`replacesShown`).
 */
export function useChecks(params: UseChecksParams): ChecksApi {
  const { bus, project } = params;
  const [feed] = useState(() => new ChecksFeed());
  const latest = useRef(params);
  latest.current = params;
  const state = useRef({
    project: null as ProjectInfo | null,
    /** The store generation of the report shown; null: none yet. */
    shown: null as number | null,
    /** A run is asked for and not started yet. */
    pending: false,
    /** The runs being worked off, one after the other. */
    loop: null as Promise<void> | null,
    /** A run of this hook is waiting for its answer. */
    inFlight: false,
    /** The light turns until that run ends. */
    ownRun: false,
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
      const running = s.ownRun || s.backendRunning;
      feed.update({ running, progress: running ? s.progress : null });
    };
    const stopPoll = () => {
      if (s.poll !== null) clearTimeout(s.poll);
      s.poll = null;
    };
    /** Takes an answer of `project` (a run, a status, an ignore, ...); whether it replaced what is shown. */
    const take = (report: ChecksReportDto, of: ProjectInfo): boolean => {
      if (s.project !== of) return false;
      s.backendRunning = report.running;
      s.progress = report.progress ?? null;
      const replaces = replacesShown(s.shown, report);
      if (replaces) {
        s.shown = report.generation;
        latest.current.problems.replaceChecks(report.findings);
        feed.update(viewOf(report));
      }
      show();
      return replaces;
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
      while (s.pending && s.project !== null) {
        const of = s.project;
        s.pending = false;
        s.inFlight = true;
        s.ownRun = true;
        show();
        poll(of);
        try {
          take(await runChecks("all"), of);
        } catch (e) {
          if (s.project === of) latest.current.onError(e);
        } finally {
          s.inFlight = false;
          s.ownRun = false;
          if (!s.backendRunning) stopPoll();
          if (s.project === of) show();
        }
      }
    };
    /** Asks for a run of every group; it waits for the one under way (several requests meanwhile make one run). */
    const request = (): Promise<void> => {
      s.pending = true;
      if (s.loop === null) s.loop = work().finally(() => (s.loop = null));
      return s.loop;
    };
    /** An answer of a command that changes the ignore list; everything runs again when the answer replaced nothing (see `replacesShown`). */
    const answer = async (call: () => Promise<ChecksReportDto>) => {
      const of = s.project;
      if (of === null) return;
      try {
        const report = await call();
        if (s.project !== of) return;
        if (!take(report, of)) void request();
      } catch (e) {
        if (s.project === of) latest.current.onError(e);
      }
    };
    const api: ChecksApi = {
      feed,
      recheck: () => request(),
      ignore: (f) => answer(() => ignoreFinding(f)),
      unignore: (f) => answer(() => unignoreFinding(f)),
      acceptBaseline: () => answer(() => acceptBaseline()),
      setEnabled: (code, enabled) => answer(() => setCheckEnabled(code, enabled)),
      planFix: (f, fix) => planFix(f, fix),
      applyFix: async (f, fix) => {
        const of = s.project;
        const result = await applyFix(f, fix);
        if (s.project === of) {
          try {
            await latest.current.onFixed?.(result.changed);
          } finally {
            void request();
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
    s.shown = null;
    s.pending = false;
    s.backendRunning = false;
    s.progress = null;
    if (s.debounce !== null) clearTimeout(s.debounce);
    s.debounce = null;
    latest.current.problems.replaceChecks([]);
    feed.reset();
    if (project !== null) void engine.request();
  }, [project, engine, feed, s]);

  // 300 ms after the last edit: everything again (R12).
  useEffect(
    () =>
      bus.subscribe("edited", () => {
        if (s.project === null) return;
        if (s.debounce !== null) clearTimeout(s.debounce);
        s.debounce = setTimeout(() => {
          s.debounce = null;
          void engine.request();
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
