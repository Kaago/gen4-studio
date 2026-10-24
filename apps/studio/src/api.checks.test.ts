import { beforeEach, describe, expect, expectTypeOf, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { acceptBaseline, checksStatus, ignoreFinding, runChecks, setCheckEnabled, unignoreFinding } from "./api";
import type { CheckProblemDto, ChecksReportDto, NavTargetDto } from "./api";
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
    expectTypeOf<NavTargetDto>().toEqualTypeOf<NavTarget>();
  });
});
