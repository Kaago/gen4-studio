import type { CommitReport, GitStatus, InitReport } from "../api";

/** Test data for the Git state: an own repository on `main` with a commit and no change unless given. */
export const gitStatusOf = (over: Partial<GitStatus> = {}): GitStatus => ({
  repo: { kind: "own" },
  branch: "main",
  detached: false,
  head: { id: "4c1d2e3f".padEnd(40, "0"), short: "4c1d2e3", summary: "Ausgangsstand", time: 1_761_700_000 },
  changes: 0,
  entries: [],
  cli: { version: "git version 2.47.1" },
  layout: "dspre",
  templates: [".gitignore", ".gitattributes"],
  ms: 60,
  ...over,
});

/** A folder in no repository. */
export const noRepo = (over: Partial<GitStatus> = {}): GitStatus =>
  gitStatusOf({ repo: { kind: "none" }, branch: null, head: null, templates: [], ...over });

/** What „Git einrichten“ answers: the first commit with `files` entries unless given. */
export const initReportOf = (over: Partial<InitReport> = {}): InitReport => ({
  templates: {
    layout: "dspre",
    files: [
      { path: ".gitignore", action: "created" },
      { path: ".gitattributes", action: "created" },
    ],
    ignored_archives: 14,
  },
  committed: true,
  reason: null,
  reason_code: null,
  head: { id: "4c1d2e3f".padEnd(40, "0"), short: "4c1d2e3", summary: "Ausgangsstand", time: 1_761_700_000 },
  files: 18449,
  studio_identity: false,
  ms: 9100,
  ...over,
});

/** What „Stand sichern“ answers: a commit of 3 files with the subject of a plain „Stand …“ unless given. */
export const commitReportOf = (over: Partial<CommitReport> = {}): CommitReport => ({
  committed: true,
  reason: null,
  subject: "Stand 2026-10-31 14:05",
  head: { id: "9a8b7c6d".padEnd(40, "0"), short: "9a8b7c6", summary: "Stand 2026-10-31 14:05", time: 1_761_919_500 },
  files: 3,
  studio_identity: false,
  phase_ms: { templates: 4, add: 120, commit: 60 },
  ms: 190,
  ...over,
});
