import type { MapHeader } from "../../api";
import { fieldValue, withField } from "./fields";

/** Reads and writes one kind of record by id (a map header, an area data record, ...). */
export interface RecordBackend<T> {
  get: (id: number) => Promise<T>;
  set: (id: number, record: T, label: string) => Promise<void>;
}

/** How a record's number fields are read and changed. */
export interface RecordFields<T> {
  value: (record: T, key: string) => number;
  with: (record: T, key: string, value: number) => T;
}

export interface RecordHooks<T> {
  /** The record the editor should show now (editor state; never called after `dispose`). */
  onState: (record: T) => void;
  /** An edit was refused (editor state; never called after `dispose`). */
  onRefused: () => void;
  /** A field was written; the app refreshes its list, scene and dirty count. Also after `dispose`. */
  onChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/**
 * Loads and edits ONE record. A session is bound to its `id` for life: every read and write goes
 * to that record, with a base that only this session's own loads and writes have produced, so
 * one record's values can never be written into another. Loads and commits run one after the
 * other, so a later edit builds on the result of the earlier one (and on an undo's reload).
 *
 * After `dispose` (the editor moved to another record or was replaced) commits already made
 * still reach the backend, but nothing touches the editor's state any more.
 */
export class EditSession<T> {
  private base: T | null = null;
  private queue: Promise<void> = Promise.resolve();
  private disposed = false;

  constructor(
    readonly id: number,
    private readonly backend: RecordBackend<T>,
    private readonly fields: RecordFields<T>,
    private readonly hooks: RecordHooks<T>,
  ) {}

  /** (Re)reads the record; queued behind pending commits so it sees their result. */
  load(): Promise<void> {
    return this.enqueue(async () => {
      try {
        const record = await this.backend.get(this.id);
        this.base = record;
        if (!this.disposed) this.hooks.onState(record);
      } catch (e) {
        this.hooks.onError(e);
      }
    });
  }

  /**
   * Sets one field (one undoable change). A value the record already has records nothing. Resolves to whether the record
   * has the value afterwards: false when nothing was loaded yet or the write was refused.
   */
  commit(key: string, label: string, value: number): Promise<boolean> {
    return this.enqueue(async () => {
      const base = this.base;
      if (!base) return false;
      if (this.fields.value(base, key) === value) return true;
      const next = this.fields.with(base, key, value);
      try {
        await this.backend.set(this.id, next, label);
      } catch (e) {
        if (!this.disposed) this.hooks.onRefused();
        this.hooks.onError(e);
        return false;
      }
      this.base = next;
      if (!this.disposed) this.hooks.onState(next);
      try {
        await this.hooks.onChanged();
      } catch (e) {
        this.hooks.onError(e);
      }
      return true;
    });
  }

  dispose() {
    this.disposed = true;
  }

  private enqueue<R>(job: () => Promise<R>): Promise<R> {
    const run = this.queue.then(job);
    this.queue = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }
}

export interface CommitBackend {
  getHeader: (id: number) => Promise<MapHeader>;
  setHeader: (id: number, header: MapHeader, label: string) => Promise<void>;
}

export type CommitHooks = RecordHooks<MapHeader>;

/** An `EditSession` over map headers. */
export class HeaderSession extends EditSession<MapHeader> {
  constructor(id: number, backend: CommitBackend, hooks: CommitHooks) {
    super(id, { get: backend.getHeader, set: backend.setHeader }, { value: fieldValue, with: withField }, hooks);
  }
}
