import { useMemo, useSyncExternalStore } from "react";
import { dataList, errorText, isCmdError, itemTable, nameTables } from "../api";
import type { DataKind, DataRowDto, ItemEntryDto, NameTablesDto, ProjectInfo } from "../api";
import type { EditEvent } from "../bus/editBus";
import type { ListKind } from "./dataModel";
import { resetIcons } from "./icons";

/** What the data areas and the palette read: the lists of the open project, as far as they were asked for. */
export interface DataSnapshot {
  /** The project the data belongs to; null before any was asked for. */
  project: ProjectInfo | null;
  /** The name tables; null while they are not read. */
  names: NameTablesDto | null;
  /** The rows of each list read so far. */
  rows: Readonly<Partial<Record<ListKind, DataRowDto[]>>>;
  /** The arm9 item table: undefined while it is not read, null when the game's is not available. */
  itemTable: ItemEntryDto[] | null | undefined;
  /** Why the data cannot be read at all (Diamond and Pearl): the backend's `read_only` message. */
  locked: string | null;
  /** Why a part could not be read, by part. */
  errors: Readonly<Partial<Record<Part, string>>>;
}

/** What the store reads, each on its own. */
type Part = ListKind | "names" | "itemTable";

const EMPTY: DataSnapshot = { project: null, names: null, rows: {}, itemTable: undefined, locked: null, errors: {} };

/** The list a changed member of `kind` shows in; null for kinds no list of the store shows (learnsets, evolutions, …). */
const PART_OF: Partial<Record<DataKind, ListKind>> = { species: "species", move: "move", item: "item" };

/**
 * The data lists of the open project for the data areas and the palette (`useSyncExternalStore`). Nothing is read until a list is
 * needed (`need`, when an area shows or the palette opens); each part is read once, and again when it changed (`onEvent`: a data
 * member, or a text archive for the names). Another project starts over: answers for the old one are dropped, the icons forgotten.
 */
export class DataStore {
  private snapshot: DataSnapshot = EMPTY;
  private readonly listeners = new Set<() => void>();
  /** Bumped for every project: a read started before belongs to the previous one. */
  private generation = 0;
  /** The parts asked for in this project (only those are read again when they change). */
  private readonly wanted = new Set<Part>();
  private readonly reading = new Set<Part>();
  /** Parts that changed while they were being read: read again when that read ends. */
  private readonly stale = new Set<Part>();

  get = (): DataSnapshot => this.snapshot;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  /** Starts over for `project` when it is another one than the data belongs to. */
  setProject(project: ProjectInfo | null): void {
    if (project === this.snapshot.project) return;
    this.generation++;
    this.wanted.clear();
    this.reading.clear();
    this.stale.clear();
    resetIcons();
    this.set({ ...EMPTY, project });
  }

  /** Reads the rows of `kind` and the names (and for items the item table) of `project`, unless they are read or being read. */
  need(project: ProjectInfo, kind: ListKind): void {
    this.setProject(project);
    const parts: Part[] = kind === "item" ? [kind, "names", "itemTable"] : [kind, "names"];
    for (const part of parts) {
      this.wanted.add(part);
      if (!this.has(part) && !this.reading.has(part)) this.read(part);
    }
  }

  /** Every list, for the palette. */
  needAll(project: ProjectInfo): void {
    for (const kind of ["species", "move", "item"] as const) this.need(project, kind);
  }

  /** Reads again what an event says may have changed, when it was asked for. */
  onEvent(event: EditEvent): void {
    if (event.topic === "dataChanged") {
      const part = PART_OF[event.kind];
      if (part !== undefined) this.changed(part);
    } else if (event.topic === "textData") {
      this.changed("names");
    }
  }

  private changed(part: Part): void {
    if (!this.wanted.has(part)) return;
    if (this.reading.has(part)) this.stale.add(part);
    else this.read(part);
  }

  private has(part: Part): boolean {
    const s = this.snapshot;
    if (part === "names") return s.names !== null;
    if (part === "itemTable") return s.itemTable !== undefined;
    return s.rows[part] !== undefined;
  }

  private read(part: Part): void {
    const started = this.generation;
    this.reading.add(part);
    const request: Promise<unknown> = part === "names" ? nameTables() : part === "itemTable" ? itemTable() : dataList(part);
    request
      .then(
        (value) => {
          if (started !== this.generation) return;
          const { errors: before, ...rest } = this.snapshot;
          const errors = { ...before };
          delete errors[part];
          if (part === "names") this.set({ ...rest, errors, names: value as NameTablesDto });
          else if (part === "itemTable") this.set({ ...rest, errors, itemTable: value as ItemEntryDto[] | null });
          else this.set({ ...rest, errors, rows: { ...rest.rows, [part]: value as DataRowDto[] } });
        },
        (e: unknown) => {
          if (started !== this.generation) return;
          // Diamond and Pearl answer every data command so: the areas say why, nothing failed.
          if (isCmdError(e) && e.code === "read_only") this.set({ ...this.snapshot, locked: e.message });
          else this.set({ ...this.snapshot, errors: { ...this.snapshot.errors, [part]: errorText(e) } });
        },
      )
      .finally(() => {
        if (started !== this.generation) return;
        this.reading.delete(part);
        if (this.stale.delete(part)) this.read(part);
      });
  }

  private set(next: DataSnapshot): void {
    this.snapshot = next;
    for (const listener of [...this.listeners]) listener();
  }
}

/** The store's snapshot; renders the caller when it changes. */
export function useDataSnapshot(store: DataStore): DataSnapshot {
  return useSyncExternalStore(store.subscribe, store.get);
}

/** The lists the palette searches: the rows read so far (a list not read yet is empty), and whether items have names and ids. */
export interface DataLists {
  species: readonly DataRowDto[];
  moves: readonly DataRowDto[];
  items: readonly DataRowDto[];
  itemTable: boolean;
}

const NONE: readonly DataRowDto[] = [];

/**
 * The lists of `project` while it is given (the palette is open); null otherwise or while the store holds another project's. The caller
 * renders with the store only while `project` is given: a closed palette does not render App for every list that arrives.
 */
export function useDataLists(store: DataStore, project: ProjectInfo | null): DataLists | null {
  const snapshot = useSyncExternalStore(store.subscribe, () => (project === null ? null : store.get()));
  return useMemo(() => {
    if (snapshot === null || project === null || snapshot.project !== project) return null;
    const { rows, itemTable } = snapshot;
    return { species: rows.species ?? NONE, moves: rows.move ?? NONE, items: rows.item ?? NONE, itemTable: itemTable !== null && itemTable !== undefined };
  }, [snapshot, project]);
}
