import type { AssetId, ProblemDto } from "./api";
import type { NavTargetDto } from "./api.data";
import { invoke } from "./platform/transport";

/**
 * The project checks, mirroring `apps/studio/src-tauri/src/commands/checks.rs` and `g4_kernel::checks`. `api.ts` re-exports
 * everything.
 *
 * A finding is a Problems row (`ProblemDto`) of source `check` with the fields of `CheckFieldsDto` (`CheckProblemDto`). Its `message`
 * is the kernel's English sentence; the UI words its own from `code` and `args` (`checks/messages.ts`). Ignoring writes
 * `<project>/g4-checks.json` at once (no `save`); errors cannot be ignored and checks that report errors cannot be switched off (the
 * commands reject with code "error").
 *
 * A fix (`planFix`, `applyFix`) checks the finding again on the current state first: when it is no longer reported as it was sent
 * (its subject changed), the command rejects with code "finding_stale" and changes nothing. A fix the finding does not offer is
 * rejected with code "error" ("fix wrap is not offered for code warp.dest-warp").
 */

/** All groups (the default; the backend also takes `"full"`) or the cheap group only, the run 300 ms after an edit. */
export type CheckScope = "all" | "cheap";

/** What a run covered: a cheap report says nothing about script and text findings. */
export type CheckGroup = "cheap" | "per_script" | "per_archive";

export type SubjectKind =
  | "header"
  | "eventFile"
  | "scriptFile"
  | "textArchive"
  | "trainer"
  | "encounter"
  | "matrix"
  | "map"
  | "areaData"
  | "project";

/** What a finding is about; the project has index 0. */
export interface SubjectDto {
  kind: SubjectKind;
  index: number;
}

export type FixId = "wrap" | "append_message" | "swap_min_max" | "create_script";

/** A fix of a finding: the German `label` of its button; `irreversible` ones (`create_script`) cannot be undone. */
export interface FixDto {
  id: FixId;
  label: string;
  irreversible: boolean;
}

/**
 * What a finding of the checks adds to its Problems row (`ProblemDto` with `source` "check"). Without a script or text source the row's
 * `file` is empty and `line` and `col` are 0.
 */
export interface CheckFieldsDto {
  /** Catalogue code, e.g. "warp.dest-warp". */
  code: string;
  subject: SubjectDto;
  /** The spot within the subject ("warp:2"); with `code` and `subject` it names the finding. */
  key: string;
  /** German: the subject and the spot, "Eventdatei 0411 · Warp 2". */
  label: string;
  /** The numbers and names for the sentence, in the order the check documents. */
  args: string[];
  /** The field of the subject the finding is about ("dest_warp_id"), if any. */
  field?: string;
  /** Where „Öffnen“ leads; none: no „Öffnen“. */
  target?: NavTargetDto;
  /** Further places the finding touches. */
  also: NavTargetDto[];
  fixes: FixDto[];
  /** 16 hex digits (a number cannot hold the 64 bits); sent back as it came. */
  fingerprint: string;
}

/** A finding of the checks: a `ProblemDto` of source "check", which always carries the check fields (one wire shape, one row type). */
export type CheckProblemDto = ProblemDto & CheckFieldsDto & { source: "check" };

/** How far a running pass got: `done` of `total` checks run (`cheap`) or files read (`load`, `scripts`, `texts`); the UI words it („Prüft … Texte 312/724“). */
export interface ChecksProgressDto {
  label: "cheap" | "load" | "scripts" | "texts";
  done: number;
  total: number;
}

/**
 * A report. `groups` empty: there is no report of the current state (none ran, or the project changed since), and the lists are
 * empty too. `running` and `progress` are those of a pass running now.
 */
export interface ChecksReportDto {
  findings: CheckProblemDto[];
  ignored: CheckProblemDto[];
  /** Checks that did not run or left part of their work undone, with the reason (English; `"disabled"`: switched off). */
  skipped: { code: string; why: string }[];
  groups: CheckGroup[];
  ranMs: number;
  /** The project's change generation the report is of. */
  generation: number;
  running: boolean;
  progress?: ChecksProgressDto;
}

/** What names a finding for ignoring and fixing. */
export type FindingKey = Pick<CheckProblemDto, "code" | "subject" | "key" | "fingerprint">;

/** Runs the checks and resolves with the finished report (it waits for a pass that is already running). */
export const runChecks = (scope: CheckScope = "all") => invoke<ChecksReportDto>("run_checks", { scope });

/** The last report of the current state and the progress of a running pass (polled while `running`). */
export const checksStatus = () => invoke<ChecksReportDto>("checks_status");

/** Ignores a warning or info; resolves with the last report sorted out anew. */
export const ignoreFinding = (finding: FindingKey) =>
  invoke<ChecksReportDto>("ignore_finding", { code: finding.code, subject: finding.subject, key: finding.key, fingerprint: finding.fingerprint });

/** Reports an ignored finding again („Wieder melden“). */
export const unignoreFinding = (finding: FindingKey) =>
  invoke<ChecksReportDto>("unignore_finding", { code: finding.code, subject: finding.subject, key: finding.key, fingerprint: finding.fingerprint });

/** „Ausgangslage übernehmen“: ignores every warning and info of the current state (runs all checks first when needed). */
export const acceptBaseline = () => invoke<ChecksReportDto>("accept_baseline");

/** Switches a check on (the checks run again) or off (its findings leave the report at once). */
export const setCheckEnabled = (code: string, enabled: boolean) => invoke<ChecksReportDto>("set_check_enabled", { code, enabled });

/** What a fix changes in one asset: `label` names it in German ("Textarchiv 0433"); `before` and `after` are shown side by side. */
export interface FixChangeDto {
  asset: AssetId;
  label: string;
  /**
   * `wrap`: the message as pages, the lines of a page joined by "\n", pages by an empty line; `append_message`: the last message and
   * the new empty ones, "2: Sandgem Town" / "3:"; `swap_min_max`: "min 30\nmax 20"; `create_script`: "" and the rotom source added.
   */
  before: string;
  after: string;
}

/** What a fix would change. `irreversible` (`create_script`): written at once, undo cannot take it back. */
export interface FixPlanDto {
  changes: FixChangeDto[];
  irreversible: boolean;
}

/** What a fix changed, and whether its check still reports the finding (same code, subject and key) afterwards: „Nicht behoben“. */
export interface FixResultDto {
  changed: AssetId[];
  remaining: boolean;
}

/** The finding as the backend reads it for a fix. */
const fixArgs = (finding: FindingKey, fix: FixId) => ({
  finding: { code: finding.code, subject: finding.subject, key: finding.key, fingerprint: finding.fingerprint },
  fix,
});

/** What `fix` of `finding` would change; changes nothing. */
export const planFix = (finding: FindingKey, fix: FixId) => invoke<FixPlanDto>("plan_fix", fixArgs(finding, fix));

/** Applies `fix` of `finding` as one undoable step „Behoben: <code>“ (`create_script`: written at once) and checks the subject again. */
export const applyFix = (finding: FindingKey, fix: FixId) => invoke<FixResultDto>("apply_fix", fixArgs(finding, fix));
