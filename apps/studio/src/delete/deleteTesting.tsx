// Helpers for the tests of „Löschen…“: a plan, a delete api with spies, and the places offering the entry under it.
import type { ReactNode } from "react";
import { vi } from "vitest";
import type { Mock } from "vitest";
import type { DeletePlan, DeleteResult, DeleteTarget, DiffSubject, RefDto } from "../api";
import { subjectKey } from "../technik/useChanges";
import type { EntityState } from "../ui/primitives";
import { DeleteApiProvider } from "./DeleteContext";
import type { DeleteApi } from "./DeleteContext";

export const TRAINER_901: DeleteTarget = { kind: "trainer", index: 901 };

/** A new trainer that can be deleted and freed; `over` changes what a test needs. */
export const deletePlan = (over: Partial<DeletePlan> = {}): DeletePlan => ({
  target: TRAINER_901,
  label: "Trainer 0901",
  refusal: null,
  references: [],
  warnings: [],
  original: false,
  deleted: false,
  can_free: null,
  cannot_free: null,
  ...over,
});

/** A reference from figure `slot` of event file 8 (its way leads to the figure). */
export const figureRef = (slot: number): RefDto => ({
  source: { kind: "Event", index: 8 },
  field: "script",
  slot,
  label: `Figur ${slot} in Eventdatei 0008`,
  source_name: "Eventdatei 0008",
  nav: { kind: "event", eventFile: 8, eventKind: "object", index: slot },
});

export const deleteResult = (over: Partial<DeleteResult> = {}): DeleteResult => ({ change: "c1", changed: [{ kind: "Trainer", index: 901 }], freed: null, ms: 3, ...over });

/** A delete api whose `open` and `restore` are spies; `states` gives `stateOf` its answers by `subjectKey` („trainer:12“). */
export function deleteSpy(
  states: Record<string, EntityState> = {},
): DeleteApi & { open: Mock<DeleteApi["open"]>; restore: Mock<DeleteApi["restore"]>; states: Record<string, EntityState> } {
  const api = {
    states,
    open: vi.fn<DeleteApi["open"]>(),
    restore: vi.fn<DeleteApi["restore"]>(),
    stateOf: (subject: DiffSubject) => api.states[subjectKey(subject)] ?? null,
  };
  return api;
}

/** `node` below `api` (a fresh object each time, so changed states reach every `useDelete`). */
export function withDelete(api: DeleteApi, node: ReactNode): ReactNode {
  return <DeleteApiProvider value={{ open: api.open, stateOf: api.stateOf, restore: api.restore }}>{node}</DeleteApiProvider>;
}
