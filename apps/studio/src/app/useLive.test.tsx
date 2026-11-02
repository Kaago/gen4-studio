// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const listen = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import type { BreakpointDto, EventRef, LiveHaltDto, LiveState, LiveStatusDto } from "../api";
import { OFF_STATUS } from "../platform/liveEvents";
import { liveStore, liveValuesOf } from "./liveStore";
import { liveHingeOf, useLive } from "./useLive";
import type { Live } from "./useLive";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const figure: EventRef = { event_file: 12, kind: "object", index: 3 };
const trigger: EventRef = { event_file: 12, kind: "coord", index: 0 };
const breakpoint = (id: number, event: EventRef): BreakpointDto => ({
  id,
  event,
  target: event.kind === "object" ? { kind: "flag", number: 0x173 } : { kind: "var", number: 0x40a5 },
  label: event.kind === "object" ? "Figur 3 · Eventdatei 12" : "Auslöser 0 · Eventdatei 12",
});
const status = (state: LiveState, more: Partial<LiveStatusDto> = {}): LiveStatusDto => ({ ...OFF_STATUS, state, ...more });
// Flag 0x173 = bit 3 of byte 0x2E; variable 0x4001 = vars[1].
const flags = Array.from({ length: 364 }, (_, i) => (i === 0x2e ? 0b0000_1000 : 0));
const live = (more: Partial<LiveStatusDto> = {}): LiveStatusDto =>
  status("live", { game: "CPUD", game_check: "verified", port: 3390, snapshot: { revision: 7, flags, vars: [0, 42, 0], taken_ms: 12 }, ...more });

describe("useLive", () => {
  let container: HTMLElement;
  let root: Root;
  let hook: Live;
  let emit: (s: LiveStatusDto) => void = () => {};
  let current: LiveStatusDto;
  const onError = vi.fn();
  const onNotice = vi.fn();

  function Probe() {
    hook = useLive(onError, onNotice);
    return null;
  }
  const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(0)));
  const render = async () => {
    await act(async () => root.render(<Probe />));
    await settle();
  };
  const send = (s: LiveStatusDto) => act(() => emit(s));

  beforeEach(() => {
    vi.useFakeTimers();
    liveStore.set(OFF_STATUS);
    current = OFF_STATUS;
    invoke.mockReset();
    invoke.mockImplementation(async (cmd: string) => (cmd === "live_status" ? current : null));
    listen.mockReset();
    listen.mockImplementation(async (_name: string, fn: (e: { payload: LiveStatusDto }) => void) => {
      emit = (payload) => fn({ payload });
      return () => {};
    });
    onError.mockReset();
    onNotice.mockReset();
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("values_come_from_the_snapshot_and_are_null_when_off", async () => {
    await render();
    expect(hook.status).toEqual(OFF_STATUS);
    expect(hook.active).toBe(false);
    expect(hook.values).toBeNull();
    await send(live());
    expect(hook.active).toBe(true);
    expect(hook.values?.revision).toBe(7);
    expect(hook.values?.flag(0x173)).toBe(true);
    expect(hook.values?.flag(0x172)).toBe(false);
    // Outside the snapshot: unknown, not 0.
    expect(hook.values?.flag(364 * 8)).toBeNull();
    expect(hook.values?.var(0x4001)).toBe(42);
    expect(hook.values?.var(0x4003)).toBeNull();
    expect(hook.values?.var(0x3fff)).toBeNull();
    // Halted reads the same snapshot.
    await send(live({ state: "halted" }));
    expect(hook.values?.var(0x4001)).toBe(42);
    // Waiting for the save, lost: no values even with an old snapshot.
    await send(live({ state: "waiting_for_save" }));
    expect(hook.values).toBeNull();
    expect(hook.active).toBe(true);
    await send(status("off", { reason: "melonDS wurde beendet" }));
    expect(hook.values).toBeNull();
    expect(hook.active).toBe(false);
    expect(liveValuesOf(live({ snapshot: null }))).toBeNull();
  });

  it("setFlag_calls_live_set_and_keeps_the_last_status", async () => {
    await render();
    await send(live());
    const before = hook.status;
    invoke.mockImplementation(async (cmd: string, args: { kind: string; number: number; value: number }) =>
      cmd === "live_set" ? { target: { kind: args.kind, number: args.number }, value: args.value, read_back: null } : current,
    );
    await act(() => hook.setFlag(0x173, false));
    expect(invoke).toHaveBeenLastCalledWith("live_set", { kind: "flag", number: 0x173, value: 0 });
    await act(() => hook.setVar(0x40a5, 7));
    expect(invoke).toHaveBeenLastCalledWith("live_set", { kind: "var", number: 0x40a5, value: 7 });
    // The answer is no status: the last one stays until the next event brings read_back.
    expect(hook.status).toBe(before);
    await send(live({ read_back: false }));
    expect(hook.status.read_back).toBe(false);
    // A refusal goes to onError and resolves.
    const refusal = { code: "no_save", message: "Wartet auf den Spielstand", asset: null };
    invoke.mockRejectedValueOnce(refusal);
    await act(() => hook.setFlag(0x173, true));
    expect(onError).toHaveBeenCalledWith(refusal);
  });

  it("resume_and_stop_take_the_status_they_answer", async () => {
    await render();
    const halt: LiveHaltDto = { watch: { id: 1, target: { kind: "var", number: 0x40a5 }, label: "Auslöser 0 · Eventdatei 12" }, before: 0, after: 1, revision: 9 };
    await send(live({ state: "halted", halt }));
    invoke.mockImplementation(async (cmd: string) => (cmd === "live_resume" ? live() : cmd === "live_stop" ? OFF_STATUS : current));
    await act(() => hook.resume());
    expect(invoke).toHaveBeenLastCalledWith("live_resume", {});
    expect(hook.status.state).toBe("live");
    await act(() => hook.stop());
    expect(invoke).toHaveBeenLastCalledWith("live_stop", {});
    expect(hook.status).toEqual(OFF_STATUS);
    invoke.mockRejectedValueOnce({ code: "not_live", message: "Nicht live: melonDS ist nicht verbunden", asset: null });
    await act(() => hook.resume());
    expect(onError).toHaveBeenCalledTimes(1);
    expect(hook.status).toEqual(OFF_STATUS);
  });

  it("toggleBreakpoint_sets_and_clears", async () => {
    await render();
    await send(live({ breakpoints: [breakpoint(1, trigger)] }));
    expect(hook.breakpointOf(trigger)?.id).toBe(1);
    expect(hook.breakpointOf(figure)).toBeNull();
    invoke.mockImplementation(async (cmd: string) => (cmd === "live_breakpoint_set" ? breakpoint(2, figure) : cmd === "live_breakpoint_clear" ? true : current));
    // None at the figure: set it; the store shows it before the next status.
    await act(() => hook.toggleBreakpoint(figure));
    expect(invoke).toHaveBeenLastCalledWith("live_breakpoint_set", { eventFile: 12, kind: "object", index: 3 });
    expect(hook.breakpointOf(figure)?.id).toBe(2);
    expect(hook.status.breakpoints).toHaveLength(2);
    // One at the trigger: clear it by id.
    await act(() => hook.toggleBreakpoint(trigger));
    expect(invoke).toHaveBeenLastCalledWith("live_breakpoint_clear", { id: 1 });
    expect(hook.breakpointOf(trigger)).toBeNull();
    expect(hook.status.breakpoints.map((b) => b.id)).toEqual([2]);
    // A refused breakpoint (a door) goes to onError, nothing changes.
    const door: EventRef = { event_file: 12, kind: "warp", index: 0 };
    invoke.mockRejectedValueOnce({ code: "error", message: "Haltepunkte gibt es an Figuren und Auslösern", asset: null });
    await act(() => hook.toggleBreakpoint(door));
    expect(onError).toHaveBeenCalledTimes(1);
    expect(hook.status.breakpoints.map((b) => b.id)).toEqual([2]);
  });

  it("the_hinge_text_follows_the_state", async () => {
    await render();
    expect(hook.hinge).toBeNull();
    await send(status("starting"));
    expect(hook.hinge).toEqual({ state: "starting", text: "Startet melonDS …", breakpoints: 0 });
    const halt: LiveHaltDto = { watch: { id: 1, target: { kind: "flag", number: 0x173 }, label: "Figur 3 · Eventdatei 12" }, before: 1, after: 0, revision: 9 };
    const one = [breakpoint(1, figure)];
    const two = [breakpoint(1, figure), breakpoint(2, trigger)];
    expect(liveHingeOf(status("connecting"))?.text).toBe("Verbindet …");
    expect(liveHingeOf(status("waiting_for_save", { reason: "Wartet auf den Spielstand (Spiel fortsetzen)" }))?.text).toBe("Wartet auf den Spielstand");
    expect(liveHingeOf(live())?.text).toBe("Live");
    expect(liveHingeOf(live({ breakpoints: one }))).toEqual({ state: "live", text: "Live · 1 Haltepunkt", breakpoints: 1 });
    expect(liveHingeOf(live({ breakpoints: two }))?.text).toBe("Live · 2 Haltepunkte");
    expect(liveHingeOf(live({ state: "halted", halt, breakpoints: one }))).toEqual({ state: "halted", text: "Gehalten: Figur 3 · Eventdatei 12: 1 → 0", breakpoints: 1 });
    expect(liveHingeOf(status("lost", { reason: "melonDS antwortet nicht: melonDS neu starten" }))?.text).toBe("Verbindung verloren: melonDS antwortet nicht: melonDS neu starten");
    // Off and unavailable leave the hinge dark.
    expect(liveHingeOf(status("unavailable", { reason: "melonDS 1.1 oder neuer" }))).toBeNull();
    expect(liveHingeOf(status("off", { reason: "melonDS wurde beendet" }))).toBeNull();
  });

  it("says_why_play_mode_ended_or_never_came_up", async () => {
    await render();
    // Connecting ran out of time: unavailable with the reason, said as an error.
    await send(status("connecting", { port: 3333 }));
    await send(status("unavailable", { reason: "Kein GDB-Stub auf Port 3333 (lauscht ein anderes Programm?)" }));
    expect(onNotice).toHaveBeenLastCalledWith({ kind: "error", text: "Kein GDB-Stub auf Port 3333 (lauscht ein anderes Programm?)" });
    // Another game in melonDS.
    await send(status("starting"));
    await send(status("unavailable", { reason: "melonDS spielt IPKE, das Projekt ist CPUD" }));
    expect(onNotice).toHaveBeenLastCalledWith({ kind: "error", text: "melonDS spielt IPKE, das Projekt ist CPUD" });
    // melonDS quit while live: off with the reason, said as info.
    await send(live());
    await send(status("off", { reason: "melonDS wurde beendet" }));
    expect(onNotice).toHaveBeenLastCalledWith({ kind: "info", text: "melonDS wurde beendet" });
    expect(onNotice).toHaveBeenCalledTimes(3);
    // The same status again says nothing new.
    await send(status("off", { reason: "melonDS wurde beendet" }));
    expect(onNotice).toHaveBeenCalledTimes(3);
  });

  it("a_user_stop_or_a_refused_start_says_nothing_here", async () => {
    await render();
    await send(live({ state: "halted" }));
    invoke.mockImplementation(async (cmd: string) => (cmd === "live_stop" ? OFF_STATUS : current));
    await act(() => hook.stop());
    expect(hook.status).toEqual(OFF_STATUS);
    // Lost, then stopped by the user: still nothing.
    await send(status("lost", { reason: "melonDS antwortet nicht: melonDS neu starten" }));
    await send(status("off"));
    // A start refused from off: the answer of „Hier spielen“ says why (useProjectSession), not this.
    await send(status("unavailable", { reason: "melonDS 1.1 oder neuer nötig (gefunden 1.0)" }));
    expect(onNotice).not.toHaveBeenCalled();
  });
});
