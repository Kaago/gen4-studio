import type { AssetId } from "./api";
import { invoke } from "./platform/transport";

/**
 * Local agents (MCP clients working in the user's session), mirroring `apps/studio/src-tauri/src/commands/agents.rs` and
 * `src/mcp/hub.rs`. `api.ts` re-exports everything.
 *
 * These commands are the UI's only: an agent cannot reach them (`Unavailable("Nur in der App")`), so it cannot unlock itself. The hub
 * reports what happens as `AgentEvent`s: inside the app Tauri emits each one as `agent-event`, the browser mode polls `agentPoll` (see
 * `platform/agentEvents.ts`).
 */

/** How far agents may go in one project: not at all, read, or read and edit. */
export type GrantLevel = "none" | "read" | "write";

/** The grant of one project (`agents.json`); `base_id` is `<sha1>/<created>` of its base mark, `null` without one. */
export interface Grant {
  path: string;
  base_id: string | null;
  level: GrantLevel;
}

/** The content of `agents.json`: „Lokale Agenten zulassen“ (`enabled`), „Expertenwerkzeuge (g4_call)“ (`expert`) and the grants. */
export interface AgentSettings {
  version: number;
  enabled: boolean;
  expert: boolean;
  grants: Grant[];
}

/** The grant of the open project; `level` is `null` while it has none. */
export interface GrantView {
  path: string;
  level: GrantLevel | null;
}

/** `agent_settings`: the settings, the line that sets up Claude Code and the open project (`null`: none open). */
export interface AgentSettingsDto {
  settings: AgentSettings;
  setup_line: string;
  open_project: GrantView | null;
}

/** One connected client (an MCP session): `client` is its display name („Claude Code“, „Agent“), `since_ms` milliseconds since the epoch. */
export interface ConnectionDto {
  id: number;
  client: string;
  since_ms: number;
}

/** The question the UI shows: `connect` names the project (its folder name), `confirm` the irreversible step (`what`). */
export interface QuestionDto {
  id: number;
  kind: "connect" | "confirm";
  client: string;
  project: string | null;
  what: string | null;
}

/**
 * What the hub shows. `bridge`: the dev bridge serves `/mcp` itself (no `port` of its own); `read_only` is the runtime switch „Nur lesen“;
 * `server_error`: why the server did not start the last time it was switched on (`null` once it runs or is switched off).
 */
export interface AgentStatus {
  enabled: boolean;
  expert: boolean;
  read_only: boolean;
  port: number | null;
  server_error: string | null;
  bridge: boolean;
  connections: ConnectionDto[];
  question: QuestionDto | null;
}

/**
 * An agent changed `assets` with `tool` (one undoable step `label`, `null` for undo and redo); the UI reloads them as after undo. A write
 * that changed no value (a save, a build) names no asset: the UI still reads its unsaved count again.
 */
export interface AgentChangedEvent {
  kind: "changed";
  seq: number;
  client: string;
  tool: string;
  label: string | null;
  assets: AssetId[];
}

/** Something the hub shows changed (a connection, the switch, a question, the settings): read the status again. */
export interface AgentStatusEvent {
  kind: "status";
  seq: number;
}

/**
 * An agent's `tool` (an adoption, revert or lift of the text project) opened the session of `root` anew: the store, its undo history and
 * the language server of the old session are gone. The UI reads the project again as after opening it.
 */
export interface AgentReopenedEvent {
  kind: "reopened";
  seq: number;
  client: string;
  tool: string;
  /** The project folder (absolute), as `ProjectInfo.root` names it. */
  root: string;
}

/**
 * An agent's long `tool` (setting the project up as text project, about a minute) on the folder `root` started (`running`) or ended; the
 * end follows the `reopened` of the same call. The UI shows it meanwhile and offers no set-up of its own.
 */
export interface AgentBusyEvent {
  kind: "busy";
  seq: number;
  client: string;
  tool: string;
  /** The project folder (absolute), as `ProjectInfo.root` names it. */
  root: string;
  running: boolean;
}

/** What the UI follows; `seq` counts up from 1 (Tauri may deliver events out of order). */
export type AgentEvent = AgentChangedEvent | AgentStatusEvent | AgentReopenedEvent | AgentBusyEvent;

/** `agent_poll`: the events after `since` (the hub keeps the last 64), the latest `seq` and the status. */
export interface AgentPoll {
  seq: number;
  events: AgentEvent[];
  status: AgentStatus;
}

/**
 * The answer to a question: `connect` takes "deny", "read" or "write" (saved as the project's grant), `confirm` "deny" or "allow". Both
 * take "dismiss": the dialog closed without a button, a no to this call that saves nothing.
 */
export type AgentAnswer = "deny" | "read" | "write" | "allow" | "dismiss";

export const agentStatus = () => invoke<AgentStatus>("agent_status");

/** The events after `since` (0: all kept) and the status, for the browser mode, which has no pushed events. */
export const agentPoll = (since: number) => invoke<AgentPoll>("agent_poll", { since });

export const agentSettings = () => invoke<AgentSettingsDto>("agent_settings");

/** „Lokale Agenten zulassen“: starts or stops the app's MCP server and remembers the switch. */
export const agentSetEnabled = (on: boolean) => invoke<void>("agent_set_enabled", { on });

/** „Expertenwerkzeuge (g4_call)“. */
export const agentSetExpert = (on: boolean) => invoke<void>("agent_set_expert", { on });

/** The grant of the open project (rejects without one). */
export const agentSetGrant = (level: GrantLevel) => invoke<void>("agent_set_grant", { level });

/** Removes the grant of the project at `path`. */
export const agentRemoveGrant = (path: string) => invoke<void>("agent_remove_grant", { path });

/** Answers the open question `question` (its id); rejects when it is no longer open. */
export const agentAnswer = (question: number, answer: AgentAnswer) => invoke<void>("agent_answer", { question, answer });

/** „Trennen“: closes every agent session; a client's next tool call asks again. */
export const agentDisconnect = () => invoke<void>("agent_disconnect");

/** „Nur lesen“: a runtime switch (not saved) for all agents. */
export const agentSetReadOnly = (on: boolean) => invoke<void>("agent_set_read_only", { on });
