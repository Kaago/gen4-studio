import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { entityIds } from "../api";
import type { EntityKind, RenameResult } from "../api";
import { RenameIdDialog } from "./RenameIdDialog";

/**
 * The ids of the open project by kind, each read once (`entity_ids`) for every head that shows one. The app makes a new cache whenever
 * they may have changed: another session, a rename, an undo or redo, a sync that took something over.
 */
export class EntityIdCache {
  private readonly reads = new Map<EntityKind, Promise<Record<number, string>>>();

  read(kind: EntityKind): Promise<Record<number, string>> {
    let read = this.reads.get(kind);
    if (read === undefined) {
      read = entityIds(kind);
      this.reads.set(kind, read);
      // A read that failed is asked again by the next head.
      read.catch(() => this.reads.get(kind) === read && this.reads.delete(kind));
    }
    return read;
  }
}

export interface TextProjectApi {
  /** The open project is a text project of the current schema: the heads show ids, trainers and headers offer „ID umbenennen…“. */
  active: boolean;
  ids: EntityIdCache;
  /** Opens „ID umbenennen“ for `id`. */
  renameId: (id: string) => void;
}

const NONE: TextProjectApi = { active: false, ids: new EntityIdCache(), renameId: () => {} };
const TextProjectContext = createContext<TextProjectApi>(NONE);

/** What the heads of trainers, headers and Pokémon need of the text project. */
export function useTextProject(): TextProjectApi {
  return useContext(TextProjectContext);
}

export type TextProjectProviderProps = {
  active: boolean;
  ids: EntityIdCache;
  /** A rename went in (the dialog has closed). */
  onRenamed: (result: RenameResult) => void;
  children: ReactNode;
};

/** Shares the text project with the heads below and holds the one „ID umbenennen“ dialog. */
export function TextProjectProvider({ active, ids, onRenamed, children }: TextProjectProviderProps) {
  const [request, setRequest] = useState<{ id: string; serial: number } | null>(null);
  const serials = useRef(0);
  const renameId = useCallback((id: string) => {
    serials.current += 1;
    setRequest({ id, serial: serials.current });
  }, []);
  const api = useMemo<TextProjectApi>(() => ({ active, ids, renameId }), [active, ids, renameId]);
  return (
    <TextProjectContext.Provider value={api}>
      {children}
      {request !== null && (
        <RenameIdDialog
          key={request.serial}
          id={request.id}
          onClose={() => setRequest((shown) => (shown?.serial === request.serial ? null : shown))}
          onRenamed={onRenamed}
        />
      )}
    </TextProjectContext.Provider>
  );
}
