import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listMatrices, listWarps } from "../api";
import type { MatrixSummary, MemberCounts, WarpLink } from "../api";
import type { EditBus } from "../bus/editBus";
import { useBusEvent } from "../bus/useBusEvent";
import { TERMS, id4 } from "../workbench/terms";

export interface Overview {
  matrices: MatrixSummary[];
  links: WarpLink[];
  /** Reads both lists again; resolves when they are read (and any read queued behind them). */
  refresh: () => Promise<void>;
}

type Kind = "matrices" | "warps";

const REPORTED_MEMBERS = 3;

/** One error per list: "Raster: 3 nicht lesbar (0007: reason; ...)", the list named with the glossary's label. */
function unreadableError(kind: Kind, unreadable: [number, string][]): Error {
  const label = kind === "matrices" ? TERMS.matrix.label : TERMS.eventFile.label;
  const shown = unreadable.slice(0, REPORTED_MEMBERS).map(([id, reason]) => `${id4(id)}: ${reason}`);
  const more = unreadable.length > REPORTED_MEMBERS ? `; und ${unreadable.length - REPORTED_MEMBERS} weitere` : "";
  return new Error(`${label}: ${unreadable.length} nicht lesbar (${shown.join("; ")}${more})`);
}

interface Slot {
  busy: boolean;
  /** A read was asked for while one was running: one more follows. */
  queued: boolean;
  waiters: (() => void)[];
}

/**
 * The matrix and warp overviews of the open project. The first read per project walks every
 * member under the session lock, so a list is never read twice at once: a request during a read
 * is remembered and served by one more read after it. Results that arrive for an older project
 * or after the unmount are dropped. Unreadable members are reported once per project and list.
 */
export function useOverview(params: { bus: EditBus; projectRoot: string | null; counts: MemberCounts; onError: (e: unknown) => void }): Overview {
  const { bus, projectRoot, counts, onError } = params;
  const [matrices, setMatrices] = useState<MatrixSummary[]>([]);
  const [links, setLinks] = useState<WarpLink[]>([]);
  const slots = useRef<Record<Kind, Slot>>({
    matrices: { busy: false, queued: false, waiters: [] },
    warps: { busy: false, queued: false, waiters: [] },
  });
  /** Bumped when the project changes (or the host unmounts): a read started under an older one is dropped. */
  const generation = useRef(0);
  const currentRoot = useRef(projectRoot);
  const mounted = useRef(false);
  const reported = useRef(new Set<Kind>());
  const latestOnError = useRef(onError);
  latestOnError.current = onError;

  const fetchOnce = useCallback(async (kind: Kind) => {
    const gen = generation.current;
    try {
      let unreadable: [number, string][];
      if (kind === "matrices") {
        const listing = await listMatrices();
        if (gen !== generation.current || !mounted.current) return;
        setMatrices(listing.matrices);
        unreadable = listing.unreadable;
      } else {
        const listing = await listWarps();
        if (gen !== generation.current || !mounted.current) return;
        setLinks(listing.links);
        unreadable = listing.unreadable;
      }
      if (unreadable.length > 0 && !reported.current.has(kind)) {
        reported.current.add(kind);
        latestOnError.current(unreadableError(kind, unreadable));
      }
    } catch (e) {
      if (gen === generation.current && mounted.current) latestOnError.current(e);
    }
  }, []);

  const read = useCallback(
    (kind: Kind): Promise<void> => {
      const slot = slots.current[kind];
      const done = new Promise<void>((resolve) => slot.waiters.push(resolve));
      if (slot.busy) {
        slot.queued = true;
        return done;
      }
      slot.busy = true;
      void (async () => {
        try {
          do {
            slot.queued = false;
            if (currentRoot.current !== null && mounted.current) await fetchOnce(kind);
          } while (slot.queued && mounted.current);
        } finally {
          slot.busy = false;
          const waiters = slot.waiters;
          slot.waiters = [];
          for (const resolve of waiters) resolve();
        }
      })();
      return done;
    },
    [fetchOnce],
  );

  // Declared before the read effect, so `mounted` is already true when it starts the first reads.
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current++;
    };
  }, []);

  useEffect(() => {
    if (currentRoot.current !== projectRoot) {
      currentRoot.current = projectRoot;
      generation.current++;
      reported.current.clear();
      setMatrices([]);
      setLinks([]);
    }
    void read("matrices");
    void read("warps");
  }, [projectRoot, counts, read]);

  useBusEvent(bus, "matrixData", () => void read("matrices"));
  useBusEvent(bus, ["eventData", "eventsRefetch"], () => void read("warps"));

  const refresh = useCallback(async () => {
    await Promise.all([read("matrices"), read("warps")]);
  }, [read]);

  return useMemo(() => ({ matrices, links, refresh }), [matrices, links, refresh]);
}
