import { beforeEach, describe, expect, expectTypeOf, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { acceptBaseline, applyFix, checksStatus, ignoreFinding, isCmdError, planFix, runChecks, setCheckEnabled, unignoreFinding } from "./api";
import type { CheckProblemDto, ChecksReportDto, FixPlanDto, FixResultDto, NavTargetDto } from "./api";
import type { NavTarget } from "./workbench/navigation";

/** A finding as `run_checks` answers it (see `commands/checks.rs`). */
const finding: CheckProblemDto = {
  severity: "warning",
  file: "",
  line: 0,
  col: 0,
  message: "Script file 0 runs with text archives 0, 433 (headers 0, 1); its message numbers match only one of them.",
  source: "check",
  code: "header.text-split",
  subject: { kind: "scriptFile", index: 0 },
  key: "text_archive",
  label: "Scriptdatei 0000 · Textarchiv",
  args: ["0", "0, 433", "0, 1"],
  target: { kind: "script", file: 0 },
  also: [
    { kind: "header", id: 0 },
    { kind: "textArchive", index: 433 },
  ],
  fixes: [],
  fingerprint: "c0ffee00deadbeef",
};

const report: ChecksReportDto = {
  findings: [finding],
  ignored: [],
  skipped: [{ code: "script.message-missing", why: "scripts not adopted" }],
  groups: ["cheap", "per_script", "per_archive"],
  ranMs: 12,
  generation: 3,
  running: false,
};

describe("checks commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(report);
  });

  it("runs all checks by default and the cheap ones on request", async () => {
    await runChecks();
    expect(invoke).toHaveBeenLastCalledWith("run_checks", { scope: "all" });
    await runChecks("cheap");
    expect(invoke).toHaveBeenLastCalledWith("run_checks", { scope: "cheap" });
    await checksStatus();
    expect(invoke).toHaveBeenLastCalledWith("checks_status");
  });

  it("ignores and reports again by code, subject, key and fingerprint, nothing else", async () => {
    const key = { code: "header.text-split", subject: { kind: "scriptFile", index: 0 }, key: "text_archive", fingerprint: "c0ffee00deadbeef" };
    await ignoreFinding(finding);
    expect(invoke).toHaveBeenLastCalledWith("ignore_finding", key);
    await unignoreFinding(finding);
    expect(invoke).toHaveBeenLastCalledWith("unignore_finding", key);
  });

  it("accepts the baseline and switches a check by code", async () => {
    await acceptBaseline();
    expect(invoke).toHaveBeenLastCalledWith("accept_baseline");
    await setCheckEnabled("warp.never-fires", false);
    expect(invoke).toHaveBeenLastCalledWith("set_check_enabled", { code: "warp.never-fires", enabled: false });
  });

  it("passes the report through, the fingerprint as the string it came as", async () => {
    await expect(runChecks()).resolves.toEqual(report);
    const answer = await ignoreFinding(finding);
    expect(answer.findings[0]!.fingerprint).toBe("c0ffee00deadbeef");
  });

  it("navigates like the workbench: a NavTargetDto is a NavTarget and back", () => {
    // Except the group of the scene: a link of the UI alone (the header editor's doors), the checks never send it.
    expectTypeOf<NavTargetDto>().toEqualTypeOf<Exclude<NavTarget, { kind: "sceneGroup" }>>();
  });
});

describe("fix commands", () => {
  /** A `text.too-wide` finding as `run_checks` answers it. */
  const wide: CheckProblemDto = {
    ...finding,
    code: "text.too-wide",
    subject: { kind: "textArchive", index: 433 },
    key: "message:1:line:1",
    label: "Textarchiv 0433 · Nachricht 1 · Zeile 1",
    args: ["433", "1", "1", "530", "216"],
    target: { kind: "textArchive", index: 433, message: 1 },
    fixes: [{ id: "wrap", label: "Umbrechen", irreversible: false }],
  };
  const sent = { finding: { code: "text.too-wide", subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1", fingerprint: "c0ffee00deadbeef" }, fix: "wrap" };

  beforeEach(() => {
    invoke.mockReset();
  });

  it("plans a fix by the finding's key and the fix id, nothing else, and passes the plan through", async () => {
    const plan: FixPlanDto = {
      changes: [{ asset: { kind: "TextArchive", index: 433 }, label: "Textarchiv 0433", before: "Welcome to Twinleaf Town, where", after: "Welcome to\nTwinleaf Town,\n\nwhere" }],
      irreversible: false,
    };
    invoke.mockResolvedValue(plan);
    await expect(planFix(wide, "wrap")).resolves.toEqual(plan);
    expect(invoke).toHaveBeenLastCalledWith("plan_fix", sent);
  });

  it("applies a fix the same way and resolves with what changed and whether the finding remains", async () => {
    const result: FixResultDto = { changed: [{ kind: "TextArchive", index: 433 }], remaining: false };
    invoke.mockResolvedValue(result);
    await expect(applyFix(wide, "wrap")).resolves.toEqual(result);
    expect(invoke).toHaveBeenLastCalledWith("apply_fix", sent);
  });

  it("rejects with a CmdError of code finding_stale when the finding changed since", async () => {
    invoke.mockRejectedValue({ code: "finding_stale", message: "finding is no longer current", asset: null, problems: [] });
    await expect(applyFix(wide, "wrap")).rejects.toSatisfy((e: unknown) => isCmdError(e) && e.code === "finding_stale");
  });
});
