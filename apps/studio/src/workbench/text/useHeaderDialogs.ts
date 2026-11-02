import { createContext, useCallback, useContext, useEffect, useSyncExternalStore } from "react";
import { dialogsOfHeader, errorText } from "../../api";
import type { HeaderDialogs } from "../../api";

/** What a header's figures came to as last read: at `revision`, the dialogs or why they could not be read. */
interface Read {
  revision: number;
  dialogs: HeaderDialogs | null;
  error: string | null;
}

/**
 * The figures of the headers as `dialogsOfHeader` read them, per header and data revision: each header is read once per revision, however
 * many rows and views ask (the list's rows, the view, a link). A read for a newer revision overtakes an older one. Another project gets a
 * new cache (the text area makes one per project).
 */
export class HeaderDialogsCache {
  private readonly reads = new Map<number, Read>();
  /** The read running per header: its revision and its answer. */
  private readonly running = new Map<number, { revision: number; answer: Promise<HeaderDialogs | null> }>();
  private readonly listeners = new Set<() => void>();

  /** The last read of `header` (at whatever revision); undefined before the first. */
  get(header: number): Read | undefined {
    return this.reads.get(header);
  }

  /** The dialogs of `header` at `revision`, from the cache or read now; null when they cannot be read. */
  load(header: number, revision: number): Promise<HeaderDialogs | null> {
    const known = this.reads.get(header);
    if (known?.revision === revision) return Promise.resolve(known.dialogs);
    const running = this.running.get(header);
    if (running?.revision === revision) return running.answer;
    const settle = (dialogs: HeaderDialogs | null, error: string | null) => {
      if (this.running.get(header)?.revision !== revision) return;
      this.running.delete(header);
      this.reads.set(header, { revision, dialogs, error });
      for (const listener of [...this.listeners]) listener();
    };
    const answer = dialogsOfHeader(header).then(
      (dialogs) => {
        settle(dialogs, null);
        return dialogs;
      },
      (e) => {
        settle(null, errorText(e));
        return null;
      },
    );
    this.running.set(header, { revision, answer });
    return answer;
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };
}

/** The cache the hook reads; the text area puts its own around its parts (a new one per project). */
export const HeaderDialogsCacheContext = createContext(new HeaderDialogsCache());

/** The figures of a header as `useHeaderDialogs` gives them. */
export interface FoundHeaderDialogs {
  /** The last answer (also while a newer revision is read); null before it, without a header and when it could not be read. */
  dialogs: HeaderDialogs | null;
  error: string | null;
  /** A read for `revision` is still on its way. */
  loading: boolean;
}

/** The figures of `header` with their dialogs, read when it is first asked for and again when `revision` changes (cached per header and revision). */
export function useHeaderDialogs(header: number | null, revision: number): FoundHeaderDialogs {
  return useCachedHeaderDialogs(useContext(HeaderDialogsCacheContext), header, revision);
}

/** `useHeaderDialogs` with the cache given (the text area, which puts the cache around its parts, reads its own). */
export function useCachedHeaderDialogs(cache: HeaderDialogsCache, header: number | null, revision: number): FoundHeaderDialogs {
  const read = useSyncExternalStore(
    cache.subscribe,
    useCallback(() => (header === null ? undefined : cache.get(header)), [cache, header]),
  );
  useEffect(() => {
    if (header !== null) void cache.load(header, revision);
  }, [cache, header, revision]);
  if (header === null) return { dialogs: null, error: null, loading: false };
  return { dialogs: read?.dialogs ?? null, error: read?.error ?? null, loading: read?.revision !== revision };
}
