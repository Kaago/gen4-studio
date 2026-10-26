import { useCallback, useEffect, useMemo, useState } from "react";
import { agentAnswer, agentDisconnect, agentSetReadOnly, agentStatus } from "../api";
import type { AgentAnswer, AgentChangedEvent, AgentStatus } from "../api";
import { subscribeAgentEvents } from "../platform/agentEvents";
import { useStableCallback } from "./useStable";

/** What the status line, the question dialog and the settings read and do (`useAgents`). */
export interface Agents {
  /** The hub's latest status; `null` until it was first read. */
  status: AgentStatus | null;
  /** Answers the open question by id. */
  answer: (question: number, answer: AgentAnswer) => Promise<void>;
  /** „Trennen“: closes every agent session. */
  disconnect: () => Promise<void>;
  /** „Nur lesen“ for all agents. */
  setReadOnly: (on: boolean) => Promise<void>;
  /** Reads the status again (after a change in the settings, say). */
  refresh: () => Promise<void>;
}

/**
 * Follows the agent hub for as long as the app is mounted (a connecting agent shows without a project too): keeps the latest status and
 * calls `onChanged` (its latest value) for each `changed` event, in the order they arrive, then `onGap` when events before them were lost
 * (the browser mode's poll fell behind the hub). The actions read the status afterwards and reject as their command did, so the caller
 * can show the error.
 */
export function useAgents({ onChanged, onGap }: { onChanged: (event: AgentChangedEvent) => void; onGap?: () => void }): Agents {
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const changed = useStableCallback(onChanged);
  const lost = useStableCallback(() => onGap?.());

  useEffect(
    () =>
      subscribeAgentEvents((events, next, gap) => {
        if (next) setStatus(next);
        for (const event of events) if (event.kind === "changed") changed(event);
        if (gap) lost();
      }),
    [changed, lost],
  );

  const refresh = useCallback(async () => setStatus(await agentStatus()), []);
  const answer = useCallback(
    async (question: number, reply: AgentAnswer) => {
      await agentAnswer(question, reply);
      await refresh();
    },
    [refresh],
  );
  const disconnect = useCallback(async () => {
    await agentDisconnect();
    await refresh();
  }, [refresh]);
  const setReadOnly = useCallback(
    async (on: boolean) => {
      await agentSetReadOnly(on);
      await refresh();
    },
    [refresh],
  );

  return useMemo(() => ({ status, answer, disconnect, setReadOnly, refresh }), [status, answer, disconnect, setReadOnly, refresh]);
}
