import type { MapHeader } from "../../api";
import { fieldValue, withField } from "./fields";

export interface CommitBackend {
  getHeader: (id: number) => Promise<MapHeader>;
  setHeader: (id: number, header: MapHeader, label: string) => Promise<void>;
}

export interface CommitHooks {
  /** The header the editor should show now (editor state; never called after `dispose`). */
  onState: (header: MapHeader) => void;
  /** An edit was refused (editor state; never called after `dispose`). */
  onRefused: () => void;
  /** A field was written; the app refreshes its list, scene and dirty count. Also after `dispose`. */
  onChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/**
 * Loads and edits ONE header. A session is bound to its `id` for life: every read and write goes
 * to that header, with a base that only this session's own loads and writes have produced, so
 * one header's values can never be written into another. Loads and commits run one after the
 * other, so a later edit builds on the result of the earlier one (and on an undo's reload).
 *
 * After `dispose` (the editor moved to another header or was replaced) commits already made
 * still reach the backend, but nothing touches the editor's state any more.
 */
export class HeaderSession {
  private base: MapHeader | null = null;
  private queue: Promise<void> = Promise.resolve();
  private disposed = false;

  constructor(
    readonly id: number,
    private readonly backend: CommitBackend,
    private readonly hooks: CommitHooks,
  ) {}

  /** (Re)reads the header; queued behind pending commits so it sees their result. */
  load(): Promise<void> {
    return this.enqueue(async () => {
      try {
        const header = await this.backend.getHeader(this.id);
        this.base = header;
        if (!this.disposed) this.hooks.onState(header);
      } catch (e) {
        this.hooks.onError(e);
      }
    });
  }

  /** Sets one field (one undoable change). A value the header already has records nothing. */
  commit(key: string, label: string, value: number): Promise<void> {
    return this.enqueue(async () => {
      const base = this.base;
      if (!base || fieldValue(base, key) === value) return;
      const next = withField(base, key, value);
      try {
        await this.backend.setHeader(this.id, next, label);
      } catch (e) {
        if (!this.disposed) this.hooks.onRefused();
        this.hooks.onError(e);
        return;
      }
      this.base = next;
      if (!this.disposed) this.hooks.onState(next);
      try {
        await this.hooks.onChanged();
      } catch (e) {
        this.hooks.onError(e);
      }
    });
  }

  dispose() {
    this.disposed = true;
  }

  private enqueue(job: () => Promise<void>): Promise<void> {
    const run = this.queue.then(job);
    this.queue = run;
    return run;
  }
}
