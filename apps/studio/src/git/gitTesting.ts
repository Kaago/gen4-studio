import type { GitStatus, InitReport } from "../api";

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
  head: { id: "4c1d2e3f".padEnd(40, "0"), short: "4c1d2e3", summary: "Ausgangsstand", time: 1_761_700_000 },
  files: 18449,
  studio_identity: false,
  ms: 9100,
  ...over,
});
