import { errorText, isCmdError } from "../api";
import type { CmdError } from "../api";

/** Draws a map's terrain model (GLB bytes) from above and returns the picture as a Data-URL. */
export type ThumbnailRender = (glb: ArrayBuffer) => Promise<string>;

export interface ThumbnailSource {
  glb: (map: number, area: number) => Promise<ArrayBuffer>;
}

export const thumbnailKey = (map: number, area: number) => `${map}:${area}`;

/**
 * A render that failed for a passing reason (the WebGL context was lost): the cache forgets the request instead of
 * remembering a failure, so a cell that still wants the picture asks again.
 */
export class TransientRenderError extends Error {}

interface Job {
  key: string;
  map: number;
  area: number;
}

/**
 * Map pictures keyed by map and tileset. Each picture is produced once (concurrent requests share the work, at most
 * `concurrency` run at a time, in request order); the least recently used are dropped beyond `capacity`; a failure is
 * remembered and never retried until `clear()`, except a `TransientRenderError`, which is forgotten. Whenever an entry disappears (eviction, `clear()`) subscribers are told,
 * so a mounted user can ask again.
 */
export class ThumbnailCache {
  private readonly capacity: number;
  private readonly concurrency: number;
  /** Least recently used first (`get` moves an entry to the end). */
  private readonly done = new Map<string, string>();
  private readonly failures = new Map<string, string>();
  private readonly queue: Job[] = [];
  /** Keys that are queued or running. */
  private readonly open = new Set<string>();
  private running = 0;
  /** Bumped by `clear()`; work started before it is dropped when it finishes. */
  private generation = 0;
  private readonly listeners = new Set<(key: string) => void>();
  private readonly toolListeners = new Set<(e: CmdError) => void>();
  /** Whether a missing conversion tool was reported since the last `clear()` (it is reported once). */
  private toolReported = false;

  constructor(
    private readonly source: ThumbnailSource,
    private readonly render: ThumbnailRender,
    options?: { capacity?: number; concurrency?: number },
  ) {
    this.capacity = options?.capacity ?? 1200;
    this.concurrency = options?.concurrency ?? 2;
  }

  /** Data-URL; null = failed for good; undefined = not requested yet or still on its way. */
  get(map: number, area: number): string | null | undefined {
    const key = thumbnailKey(map, area);
    if (this.failures.has(key)) return null;
    const url = this.done.get(key);
    if (url !== undefined) {
      // Reading refreshes the entry, so eviction drops the least recently used picture.
      this.done.delete(key);
      this.done.set(key, url);
    }
    return url;
  }

  /** Error text of a failed entry (null when the entry has not failed). */
  failure(map: number, area: number): string | null {
    return this.failures.get(thumbnailKey(map, area)) ?? null;
  }

  request(map: number, area: number): void {
    const key = thumbnailKey(map, area);
    if (this.done.has(key) || this.failures.has(key) || this.open.has(key)) return;
    this.open.add(key);
    this.queue.push({ key, map, area });
    this.pump();
  }

  /**
   * Drops a queued request nobody wants any more (its cell went away or out of view). A running job finishes normally.
   * Subscribers of the key are told, so a user that still wants the picture asks again.
   */
  cancel(map: number, area: number): void {
    const key = thumbnailKey(map, area);
    const at = this.queue.findIndex((job) => job.key === key);
    if (at < 0) return;
    this.queue.splice(at, 1);
    this.open.delete(key);
    this.notify(key);
  }

  pending(): number {
    return this.open.size;
  }

  /** `fn` runs after a picture was produced or failed for good. */
  subscribe(fn: (key: string) => void): () => void {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  /** `fn` runs once (until the next `clear()`) when a picture failed because the conversion tool is missing. */
  onToolMissing(fn: (e: CmdError) => void): () => void {
    this.toolListeners.add(fn);
    return () => {
      this.toolListeners.delete(fn);
    };
  }

  /** Forgets every picture (and failure) of `map`, whatever its tileset, after its model changed; subscribers are told so mounted users ask again. Work in flight keeps running. */
  invalidate(map: number): void {
    const prefix = `${map}:`;
    const keys = [...this.done.keys(), ...this.failures.keys()].filter((key) => key.startsWith(prefix));
    for (const key of keys) {
      this.done.delete(key);
      this.failures.delete(key);
    }
    for (const key of keys) this.notify(key);
  }

  clear(): void {
    const keys = [...this.done.keys(), ...this.failures.keys(), ...this.open];
    this.generation++;
    this.toolReported = false;
    this.done.clear();
    this.failures.clear();
    this.queue.length = 0;
    this.open.clear();
    this.running = 0;
    for (const key of keys) this.notify(key);
  }

  private notify(key: string): void {
    for (const fn of [...this.listeners]) fn(key);
  }

  private pump(): void {
    while (this.running < this.concurrency) {
      const job = this.queue.shift();
      if (!job) return;
      this.running++;
      void this.run(job, this.generation);
    }
  }

  private async run(job: Job, generation: number): Promise<void> {
    let url: string | null = null;
    let error = "";
    let transient = false;
    let toolMissing: CmdError | null = null;
    try {
      url = await this.render(await this.source.glb(job.map, job.area));
    } catch (e) {
      error = errorText(e);
      transient = e instanceof TransientRenderError;
      if (isCmdError(e) && e.code === "tool_missing") toolMissing = e;
    }
    if (generation !== this.generation) return;
    this.running--;
    this.open.delete(job.key);
    const evicted: string[] = [];
    if (transient) {
      // Neither done nor failed: the key is free again for a user that still wants it.
    } else if (url !== null) {
      this.done.set(job.key, url);
      while (this.done.size > this.capacity) {
        const oldest = this.done.keys().next().value;
        if (oldest === undefined) break;
        this.done.delete(oldest);
        evicted.push(oldest);
      }
    } else {
      this.failures.set(job.key, error);
    }
    this.pump();
    if (toolMissing && !this.toolReported) {
      this.toolReported = true;
      for (const fn of [...this.toolListeners]) fn(toolMissing);
    }
    this.notify(job.key);
    for (const key of evicted) this.notify(key);
  }
}
