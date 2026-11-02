import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { isDeleted } from "../api";
import type { DeleteResult, DeleteTarget, DiffSubject } from "../api";
import type { EntityState } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { DeleteDialog } from "./DeleteDialog";

export interface DeleteApi {
  /** Opens „{label} löschen“ for `target`. */
  open: (target: DeleteTarget) => void;
  /** How the entity stands in the saved project against the base ROM (Technik › Änderungen); null when unchanged or not compared yet. */
  stateOf: (subject: DiffSubject) => EntityState | null;
}

const NONE: DeleteApi = { open: () => {}, stateOf: () => null };
const DeleteContext = createContext<DeleteApi>(NONE);

/** What every place offering „Löschen…“ or showing the state badge needs. */
export function useDelete(): DeleteApi {
  return useContext(DeleteContext);
}

/**
 * The state badge of the entity `subject` in a head: „gelöscht“ when the live DTO says so (`dto.deleted`, unsaved tombstones too), else the
 * state of the saved project against the base. A saved tombstone the live DTO no longer has (restored, not saved yet) shows nothing.
 */
export function useEntityState(subject: DiffSubject | null, dto?: object | null): EntityState | null {
  const { stateOf } = useDelete();
  if (subject === null) return null;
  if (isDeleted(dto)) return "deleted";
  const saved = stateOf(subject);
  return saved === "deleted" && dto != null ? null : saved;
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
};

/** One `open`; `serial` tells a dialog's close from that of the one before. */
type Request = { target: DeleteTarget; serial: number };

/** Holds the one „Löschen…“ dialog: `useDelete().open(target)` shows it; after a delete `onDeleted` gets the result. */
export function DeleteProvider({ children, stateOf, goTo, onDeleted }: DeleteProviderProps) {
  const [request, setRequest] = useState<Request | null>(null);
  const serials = useRef(0);
  const open = useCallback((target: DeleteTarget) => {
    serials.current += 1;
    setRequest({ target, serial: serials.current });
  }, []);
  const api = useMemo<DeleteApi>(() => ({ open, stateOf }), [open, stateOf]);
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
