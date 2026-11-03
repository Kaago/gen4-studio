import type { EntityDiffReport } from "../api";

/** Test data for the detail of Technik › Änderungen: the diff of Trainer 0001 against the last commit, one field changed unless given. */
export const diffReport = (over: Partial<EntityDiffReport> = {}): EntityDiffReport => ({
  subject: { kind: "trainer", index: 1 },
  against: "head",
  label: "Trainer 0001",
  old_label: "Commit 4c1d2e3 „Ausgangsstand“",
  new_label: "Projekt (gespeichert)",
  state: "changed",
  diff: { shape: "fields", changes: [{ path: "data.party[0].level", old: "5", new: "6" }], more: 0 },
  reset: { kind: "trainer", index: 1 },
  tombstone: { old: false, new: false },
  ms: 4,
  ...over,
});
