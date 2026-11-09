import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { t } from "../i18n";
import { deleteTargetLabel, restoreEntity } from "../api";
import type { DeleteResult, DeleteTarget, DiffSubject } from "../api";
import type { EntityState, MenuItemSpec } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { DeleteDialog } from "./DeleteDialog";

export interface DeleteApi {
  /** Opens „{label} löschen“ for `target`. */
  open: (target: DeleteTarget) => void;
  /** How the entity stands in the saved project against the base ROM (Technik › Änderungen); null when unchanged or not compared yet. */
  stateOf: (subject: DiffSubject) => EntityState | null;
  /** „Wiederherstellen“: takes the tombstone of `target` back as one undo step (the app says so and reads the entity again). */
  restore: (target: DeleteTarget) => void;
}

const NONE: DeleteApi = { open: () => {}, stateOf: () => null, restore: () => {} };
const DeleteContext = createContext<DeleteApi>(NONE);

/** What every place offering „Löschen…“ or showing the state badge needs. */
export function useDelete(): DeleteApi {
  return useContext(DeleteContext);
}

/**
 * The state badge of the entity `subject` in a head: „gelöscht“ when the live DTO says so (`dto.deleted`, unsaved tombstones too), else the
 * state of the saved project against the base. A saved tombstone the live DTO no longer has (restored, not saved yet) shows nothing.
 */
export function useEntityState(subject: DiffSubject | null, dto?: { deleted?: boolean } | null): EntityState | null {
  const { stateOf } = useDelete();
  if (subject === null) return null;
  if (dto?.deleted === true) return "deleted";
  const saved = stateOf(subject);
  return saved === "deleted" && dto != null ? null : saved;
}

/** The label of the entry that takes a tombstone back (a row's context menu, Technik › Änderungen). */
export function restoreLabel(): string {
  return t("delete:deleteContext.restore");
}

/** The context menu entry „Wiederherstellen“ of a deleted row. */
export function restoreEntry(target: DeleteTarget, restore: DeleteApi["restore"]): MenuItemSpec {
  return { label: restoreLabel(), onSelect: () => restore(target) };
}

/** Gives `value` to every `useDelete` below: a part shown on its own (the gallery, a test) without the app's dialog. */
export function DeleteApiProvider({ value, children }: { value: DeleteApi; children: ReactNode }) {
  return <DeleteContext.Provider value={value}>{children}</DeleteContext.Provider>;
}

export type DeleteProviderProps = {
  children: ReactNode;
  /** The state of an entity in the saved project (`stateOf` of `useChanges`' report). */
  stateOf: (subject: DiffSubject) => EntityState | null;
  /** Where a reference of the dialog leads. */
  goTo: (target: NavTarget) => void;
  /** The delete went in (the dialog has closed): its result, whether the number was freed, the entity's name („Trainer 0901“) and target. */
  onDeleted: (result: DeleteResult, freed: boolean, label: string, target: DeleteTarget) => void | Promise<void>;
  /** „Wiederherstellen“ went in: its result, the entity's name („Trainer 0930“) and target. */
  onRestored: (result: DeleteResult, label: string, target: DeleteTarget) => void | Promise<void>;
  /** „Wiederherstellen“ was refused („Trainer 0930 ist nicht gelöscht.“) or failed. */
  onRestoreFailed: (e: unknown) => void;
};

/** One `open`; `serial` tells a dialog's close from that of the one before. */
type Request = { target: DeleteTarget; serial: number };

/**
 * Holds the one „Löschen…“ dialog: `useDelete().open(target)` shows it; after a delete `onDeleted` gets the result. `useDelete().restore`
 * asks the backend at once (no dialog: one undo step) and hands the result to `onRestored`, a refusal to `onRestoreFailed`.
 */
export function DeleteProvider({ children, stateOf, goTo, onDeleted, onRestored, onRestoreFailed }: DeleteProviderProps) {
  const [request, setRequest] = useState<Request | null>(null);
  const serials = useRef(0);
  const open = useCallback((target: DeleteTarget) => {
    serials.current += 1;
    setRequest({ target, serial: serials.current });
  }, []);
  // The callbacks of the latest render, so `restore` keeps its identity (rows are memoised on it).
  const restored = useRef({ onRestored, onRestoreFailed });
  restored.current = { onRestored, onRestoreFailed };
  const restore = useCallback((target: DeleteTarget) => {
    restoreEntity(target).then(
      (result) => restored.current.onRestored(result, deleteTargetLabel(target), target),
      (e: unknown) => restored.current.onRestoreFailed(e),
    );
  }, []);
  const api = useMemo<DeleteApi>(() => ({ open, stateOf, restore }), [open, stateOf, restore]);
  return (
    <DeleteContext.Provider value={api}>
      {children}
      {request !== null && (
        <DeleteDialog
          key={request.serial}
          target={request.target}
          goTo={goTo}
          onClose={() => setRequest((shown) => (shown?.serial === request.serial ? null : shown))}
          onDeleted={(result, freed, label) => void onDeleted(result, freed, label, request.target)}
        />
      )}
    </DeleteContext.Provider>
  );
}
