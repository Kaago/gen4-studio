import { useMemo, useRef } from "react";
import type { BreakpointDto, CoordDto, EventKind, EventRef as LiveEventRef, ObjectDto } from "../../api";
import type { Live } from "../../app/useLive";
import type { EventData, EventRef } from "../../world/events";
import { t } from "../../i18n";

/** What the event inspector needs of play mode: the row „Haltepunkt“ shows while `active`. */
export interface EventLive {
  /** Play mode as the hinge shows it (not while off or unavailable: then the hinge stays dark and nothing could stop it). */
  active: boolean;
  breakpoints: readonly BreakpointDto[];
  /** Sets a breakpoint at the event, or clears the one there. */
  onToggle: (event: LiveEventRef) => void;
}

/** Why a figure takes no breakpoint (the backend's sentence). */
export const noFlag = (): string => t("workbench:eventBreakpoint.noFlag");
/** Why a trigger takes no breakpoint (the backend's sentence). */
export const noVar = (): string => t("workbench:eventBreakpoint.noVar");
/** Why a door or a sign takes no breakpoint (the backend's sentence). */
export const figuresAndTriggers = (): string => t("workbench:eventBreakpoint.figuresAndTriggers");

/** The event as play mode names it (`event_file`), or null for a class that takes no breakpoint (a sign). */
export function liveEventRef(ref: EventRef): LiveEventRef | null {
  if (ref.kind === "bg") return null;
  return { event_file: ref.file, kind: ref.kind, index: ref.index };
}

/** Why `event` of `kind` takes no breakpoint, or null: a figure watches its visibility flag, a trigger its variable; 0 is none. */
export function breakpointLock(kind: EventKind, event: EventData): string | null {
  if (kind === "object") return (event as ObjectDto).hidden_flag === 0 ? noFlag() : null;
  if (kind === "coord") return (event as CoordDto).var === 0 ? noVar() : null;
  return figuresAndTriggers();
}

/** Whether `live` has a breakpoint at the event `ref`. */
export function hasBreakpoint(live: EventLive, ref: EventRef): boolean {
  const at = liveEventRef(ref);
  return at !== null && live.breakpoints.some((b) => b.event.event_file === at.event_file && b.event.kind === at.kind && b.event.index === at.index);
}

/** Sets or clears the breakpoint at the event `ref` (holding `event`); nothing for an event that takes none. */
export function toggleBreakpoint(live: EventLive, ref: EventRef, event: EventData): void {
  const at = liveEventRef(ref);
  if (at !== null && breakpointLock(ref.kind, event) === null) live.onToggle(at);
}

const NONE: readonly BreakpointDto[] = [];

/** Whether two lists hold the same breakpoints (an id names one breakpoint at one event). */
const sameBreakpoints = (a: readonly BreakpointDto[], b: readonly BreakpointDto[]) => a.length === b.length && a.every((bp, i) => bp.id === b[i]?.id);

/**
 * The part of play mode the event inspector reads, with an identity that changes only when `active` or the breakpoints do: every new
 * snapshot brings a new status (and a new breakpoint list), and the inspector need not render for a moved value.
 */
export function useEventLive(liveMode: Live | undefined): EventLive | undefined {
  // The hinge's condition (ruling R19): `Live.active` also holds while melonDS is unavailable.
  const active = (liveMode?.hinge ?? null) !== null;
  const next = liveMode?.status.breakpoints ?? NONE;
  const kept = useRef(next);
  if (!sameBreakpoints(kept.current, next)) kept.current = next;
  const breakpoints = kept.current;
  const onToggle = liveMode?.toggleBreakpoint;
  return useMemo(() => (onToggle === undefined ? undefined : { active, breakpoints, onToggle: (event: LiveEventRef) => void onToggle(event) }), [active, breakpoints, onToggle]);
}
