import { useCallback, useEffect, useMemo, useRef } from "react";
import { liveBreakpointClear, liveBreakpointSet, liveResume, liveSet, liveStop } from "../api.live";
import type { BreakpointDto, EventRef, LiveHingeInfo, LiveState, LiveStatusDto } from "../api.live";
import { subscribeLive } from "../platform/liveEvents";
import { liveStore, liveValuesOf, useLiveStatus } from "./liveStore";
import type { LiveValues } from "./liveStore";
import type { Message } from "./message";
import { useStableCallback } from "./useStable";

/** What the hinge, the flag list and detail and the event inspector read and do in play mode (`useLive`). */
export interface Live {
  status: LiveStatusDto;
  /** The snapshot's values; `null` unless live or halted. */
  values: LiveValues | null;
  /** Play mode is on: any state but `off`. */
  active: boolean;
  /** What the hinge shows; `null` while off or unavailable. */
  hinge: LiveHingeInfo | null;
  /** Writes a flag; the status stays until the next one brings `read_back`. */
  setFlag(number: number, value: boolean): Promise<void>;
  /** Writes a variable (0…65535); the status stays until the next one brings `read_back`. */
  setVar(number: number, value: number): Promise<void>;
  /** „Weiter“ after a halt. */
  resume(): Promise<void>;
  /** „Stoppen“: disconnects and ends our melonDS. */
  stop(): Promise<void>;
  /** The breakpoint at `event`, else `null`. */
  breakpointOf(event: EventRef): BreakpointDto | null;
  /** Sets a breakpoint at `event`, or clears the one there. */
  toggleBreakpoint(event: EventRef): Promise<void>;
}

const sameEvent = (a: EventRef, b: EventRef) => a.event_file === b.event_file && a.kind === b.kind && a.index === b.index;

/** „1 Haltepunkt“, „2 Haltepunkte“. */
const breakpointCount = (n: number) => (n === 1 ? "1 Haltepunkt" : `${n} Haltepunkte`);

/** What the hinge shows for `s`; `null` while off or unavailable (the hinge stays dark then). */
export function liveHingeOf(s: LiveStatusDto): LiveHingeInfo | null {
  const breakpoints = s.breakpoints.length;
  const info = (text: string): LiveHingeInfo => ({ state: s.state, text, breakpoints });
  switch (s.state) {
    case "off":
    case "unavailable":
      return null;
    case "starting":
      return info("Startet melonDS …");
    case "connecting":
      return info("Verbindet …");
    case "waiting_for_save":
      return info("Wartet auf den Spielstand");
    case "live":
      return info(breakpoints > 0 ? `Live · ${breakpointCount(breakpoints)}` : "Live");
    case "halted":
      return info(s.halt ? `Gehalten: ${s.halt.watch.label}: ${s.halt.before} → ${s.halt.after}` : "Gehalten");
    case "lost":
      return info(s.reason ? `Verbindung verloren: ${s.reason}` : "Verbindung verloren");
  }
}

/** The states play mode is under way in (the hinge shows them). */
const UNDER_WAY: readonly LiveState[] = ["starting", "connecting", "waiting_for_save", "live", "halted", "lost"];

/**
 * What to say when play mode ends by itself: from a state under way to `unavailable` (an error) or `off` (info) with a reason, the
 * reason; `null` otherwise (a user „Stoppen“ has no reason, and a start refused from `off` is said by the answer of „Hier spielen“).
 */
export function liveEndNoticeOf(was: LiveState, now: LiveStatusDto): Message | null {
  if (!UNDER_WAY.includes(was) || !now.reason) return null;
  if (now.state === "unavailable") return { kind: "error", text: now.reason };
  if (now.state === "off") return { kind: "info", text: now.reason };
  return null;
}

/**
 * Follows play mode for as long as the caller is mounted (once, in the App) and keeps the latest status in `liveStore`. `resume` and
 * `stop` set the status they answer at once; `setFlag` and `setVar` keep the last one (the next status brings `read_back`);
 * `toggleBreakpoint` adds or removes the breakpoint in the status before the next one arrives. A refusal goes to `onError` (its latest
 * value) and the action resolves. When play mode ends by itself (`liveEndNoticeOf`), `onNotice` gets why: the hinge goes dark then.
 */
export function useLive(onError: (e: unknown) => void, onNotice: (message: Message) => void = () => {}): Live {
  const status = useLiveStatus();
  const fail = useStableCallback(onError);
  const notice = useStableCallback(onNotice);

  useEffect(() => subscribeLive((next) => liveStore.set(next)), []);

  const previous = useRef<LiveState>(status.state);
  useEffect(() => {
    const was = previous.current;
    previous.current = status.state;
    const said = liveEndNoticeOf(was, status);
    if (said) notice(said);
  }, [status, notice]);

  const values = useMemo(() => liveValuesOf(status), [status]);
  const hinge = useMemo(() => liveHingeOf(status), [status]);

  const setFlag = useCallback(
    async (number: number, value: boolean) => {
      try {
        await liveSet("flag", number, value ? 1 : 0);
      } catch (e) {
        fail(e);
      }
    },
    [fail],
  );
  const setVar = useCallback(
    async (number: number, value: number) => {
      try {
        await liveSet("var", number, value);
      } catch (e) {
        fail(e);
      }
    },
    [fail],
  );
  const resume = useCallback(async () => {
    try {
      liveStore.set(await liveResume());
    } catch (e) {
      fail(e);
    }
  }, [fail]);
  const stop = useCallback(async () => {
    try {
      liveStore.set(await liveStop());
    } catch (e) {
      fail(e);
    }
  }, [fail]);

  const breakpointOf = useCallback((event: EventRef) => status.breakpoints.find((b) => sameEvent(b.event, event)) ?? null, [status]);
  const toggleBreakpoint = useCallback(
    async (event: EventRef) => {
      const found = liveStore.get().breakpoints.find((b) => sameEvent(b.event, event));
      try {
        if (found) {
          await liveBreakpointClear(found.id);
          const now = liveStore.get();
          liveStore.set({ ...now, breakpoints: now.breakpoints.filter((b) => b.id !== found.id) });
        } else {
          const added = await liveBreakpointSet(event);
          const now = liveStore.get();
          if (!now.breakpoints.some((b) => b.id === added.id)) liveStore.set({ ...now, breakpoints: [...now.breakpoints, added] });
        }
      } catch (e) {
        fail(e);
      }
    },
    [fail],
  );

  return useMemo(
    () => ({ status, values, active: status.state !== "off", hinge, setFlag, setVar, resume, stop, breakpointOf, toggleBreakpoint }),
    [status, values, hinge, setFlag, setVar, resume, stop, breakpointOf, toggleBreakpoint],
  );
}
