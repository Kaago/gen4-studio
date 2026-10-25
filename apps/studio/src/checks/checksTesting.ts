// Shared by the checks tests: a finding as `run_checks` answers it, a report, and the checks of a project whose commands are mocks.
import { vi } from "vitest";
import type { CheckProblemDto, ChecksReportDto, FixPlanDto, FixResultDto } from "../api";
import { ChecksFeed } from "./useChecks";
import type { ChecksApi } from "./useChecks";

/** A finding of `code`; by default warp 2 of event file 0411 with no target, fix or arguments. */
export function finding(code: string, severity: CheckProblemDto["severity"] = "error", over: Partial<CheckProblemDto> = {}): CheckProblemDto {
  return {
    severity,
    file: "",
    line: 0,
    col: 0,
    message: `${code} (kernel)`,
    source: "check",
    code,
    subject: { kind: "eventFile", index: 411 },
    key: "warp:2",
    label: "Eventdatei 0411 · Warp 2",
    args: [],
    also: [],
    fixes: [],
    fingerprint: "00000000000000a1",
    ...over,
  };
}

/** A finished report of every group (generation 1) with `over`. */
export function report(over: Partial<ChecksReportDto> = {}): ChecksReportDto {
  return { findings: [], ignored: [], skipped: [], groups: ["cheap", "per_script", "per_archive"], ranMs: 12, generation: 1, running: false, ...over };
}

/** The checks of a project as the panels get them: a feed to fill by hand and mocked commands. */
export function fakeChecks(): ChecksApi {
  return {
    feed: new ChecksFeed(),
    recheck: vi.fn(async () => {}),
    ignore: vi.fn(async () => {}),
    unignore: vi.fn(async () => {}),
    acceptBaseline: vi.fn(async () => {}),
    setEnabled: vi.fn(async () => {}),
    planFix: vi.fn(async (): Promise<FixPlanDto> => ({ changes: [], irreversible: false })),
    applyFix: vi.fn(async (): Promise<FixResultDto> => ({ changed: [], remaining: false })),
    save: vi.fn(async () => {}),
    nextReport: vi.fn(async (): Promise<readonly CheckProblemDto[] | null> => []),
  };
}
