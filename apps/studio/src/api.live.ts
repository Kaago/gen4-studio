import type { FlagKind } from "./api.flags";
import { invoke } from "./platform/transport";

/**
 * Play mode live (3d.9), mirroring `apps/studio/src-tauri/src/live_hub.rs`, `commands/live.rs` and `g4_kernel::live`. `api.ts`
 * re-exports everything. Field names are the Rust ones (snake_case). `liveSet` refuses with `not_live` without a connection and with
 * `no_save` while the game waits for its save; every refusal carries the backend's German sentence. The status itself arrives as the
 * `live-event` (in the app) or by polling `liveStatus` (browser mode): `platform/liveEvents.ts`.
 */

/** Where play mode stands. */
export type LiveState = "off" | "starting" | "connecting" | "waiting_for_save" | "live" | "halted" | "lost" | "unavailable";

/** The flags and variables of the save at one moment: `flags` are bytes (flag `n` is bit `n % 8` of byte `n / 8`), variable `v` is `vars[v - 0x4000]`. */
export interface LiveSnapshotDto {
  revision: number;
  flags: number[];
  vars: number[];
  taken_ms: number;
}

/** A flag or a variable of the save. */
export type LiveTarget = { kind: "flag"; number: number } | { kind: "var"; number: number };

/** A watched value; a breakpoint's watch has the breakpoint's id. */
export interface LiveWatchDto {
  id: number;
  target: LiveTarget;
  label: string;
}

/** The game halted because a watched value changed from `before` to `after`. */
export interface LiveHaltDto {
  watch: LiveWatchDto;
  before: number;
  after: number;
  revision: number;
}

/** An event of the project; the backend refuses breakpoints at a `warp` with a sentence. */
export interface EventRef {
  event_file: number;
  kind: "object" | "coord" | "warp";
  index: number;
}

/** A breakpoint at an event: the game halts when its flag (figure) or variable (trigger) changes. */
export interface BreakpointDto {
  id: number;
  event: EventRef;
  target: LiveTarget;
  label: string;
}

/** The GDB stub settings of melonDS (`melonDS.toml`); `null` when the file was not readable. */
export interface MelonDsConfigDto {
  enabled: boolean;
  break_on_startup: boolean;
  port: number;
  jit: boolean;
}

/** The melonDS of the last „Hier spielen“. */
export interface MelonDsDto {
  bundle: string;
  binary: string;
  version: string;
  config_path: string;
  config: MelonDsConfigDto | null;
}

/** What `live_status` answers and `live-event` carries. */
export interface LiveStatusDto {
  state: LiveState;
  /** Why the state is what it is (`unavailable`, `lost`, `off` after melonDS quit, `waiting_for_save`). */
  reason: string | null;
  game: string | null;
  game_check: "verified" | "unverified" | null;
  port: number | null;
  /** The studio launched melonDS itself (and ends it on stop). */
  own_process: boolean;
  /** The last snapshot while `live` or `halted`. */
  snapshot: LiveSnapshotDto | null;
  halt: LiveHaltDto | null;
  breakpoints: BreakpointDto[];
  /** After `live_set`: `null` until the next snapshot, then whether the value held. */
  read_back: boolean | null;
  melonds: MelonDsDto | null;
}

/** `live_play`: `live: false` says in `reason` why the game runs without the connection. */
export interface LivePlayDto {
  live: boolean;
  reason: string | null;
  pid: number | null;
}

/** `live_set`: what was written; `read_back` is `null` until the next snapshot decided it. */
export interface LiveSetDto {
  target: LiveTarget;
  value: number;
  read_back: boolean | null;
}

/** What the hinge shows in play mode (`useLive` builds it; `null` while off or unavailable, the hinge stays dark then). */
export interface LiveHingeInfo {
  state: LiveState;
  /** „Verbindet …“, „Live · 2 Haltepunkte“, „Gehalten: …“. */
  text: string;
  /** How many breakpoints are set. */
  breakpoints: number;
}

/** Play mode off, nothing known: the status before the first read. */
export const OFF_STATUS: LiveStatusDto = {
  state: "off",
  reason: null,
  game: null,
  game_check: null,
  port: null,
  own_process: false,
  snapshot: null,
  halt: null,
  breakpoints: [],
  read_back: null,
  melonds: null,
};

export const liveStatus = () => invoke<LiveStatusDto>("live_status", {});
/** „Hier spielen“ in play mode (app only; the bridge refuses it like `run_rom`). */
export const livePlay = (emulator: string, rom: string) => invoke<LivePlayDto>("live_play", { emulator, rom });
/** Connects to a running stub (melonDS or `g4emu repl --gdb-port`); `host` defaults to 127.0.0.1. */
export const liveConnect = (port: number, host?: string) => invoke<LiveStatusDto>("live_connect", { port, host: host ?? null });
export const liveStop = () => invoke<LiveStatusDto>("live_stop", {});
export const liveResume = () => invoke<LiveStatusDto>("live_resume", {});
export const liveSet = (kind: FlagKind, number: number, value: number) => invoke<LiveSetDto>("live_set", { kind, number, value });
export const liveBreakpoints = () => invoke<BreakpointDto[]>("live_breakpoints", {});
export const liveBreakpointSet = (event: EventRef) =>
  invoke<BreakpointDto>("live_breakpoint_set", { eventFile: event.event_file, kind: event.kind, index: event.index });
export const liveBreakpointClear = (id: number) => invoke<boolean>("live_breakpoint_clear", { id });
