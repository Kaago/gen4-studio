import { listen } from "@tauri-apps/api/event";
import { agentPoll, agentStatus } from "../api.agents";
import type { AgentEvent, AgentStatus } from "../api.agents";
import { inTauri } from "./transport";

/** How often the browser mode asks the hub for new events (the local bridge; it has no pushed events). */
export const AGENT_POLL_MS = 500;

/**
 * Follows the agent hub: `onEvents` gets new events with the status when it was read along (else `null`). The status is there soon after
 * subscribing (with no events): what happened before is past. Errors are silent (the status line shows no agent then). Returns the
 * unsubscribe; nothing arrives after it, not even the answer of a read under way.
 *
 * Inside the app Tauri emits each event as `agent-event`; each one arrives alone, a `status` one after the status was read again. Events
 * may arrive out of `seq` order and are passed on as they come. The browser mode polls `agentPoll` from the last `seq`, each poll
 * `AGENT_POLL_MS` after the previous one answered.
 */
export function subscribeAgentEvents(onEvents: (events: AgentEvent[], status: AgentStatus | null) => void): () => void {
  return inTauri() ? listenToTauri(onEvents) : pollTheHub(onEvents);
}

function listenToTauri(onEvents: (events: AgentEvent[], status: AgentStatus | null) => void): () => void {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  // Only the newest status read is passed on: an older one answering late must not overwrite it.
  let reads = 0;
  const readStatus = (events: AgentEvent[]) => {
    const read = ++reads;
    agentStatus().then(
      (status) => {
        if (!disposed) onEvents(events, read === reads ? status : null);
      },
      () => {
        if (!disposed && events.length > 0) onEvents(events, null);
      },
    );
  };
  listen<AgentEvent>("agent-event", (e) => {
    if (disposed) return;
    const event = e.payload;
    if (event.kind === "status") readStatus([event]);
    else onEvents([event], null);
  })
    .then((fn) => {
      if (disposed) {
        fn();
        return;
      }
      unlisten = fn;
      // Listening now: the status as it is, so a connection made before is shown.
      readStatus([]);
    })
    .catch(() => {
      /* not running inside a Tauri window */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

function pollTheHub(onEvents: (events: AgentEvent[], status: AgentStatus | null) => void): () => void {
  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let since = 0;
  let first = true;
  const poll = async () => {
    try {
      const answer = await agentPoll(since);
      if (disposed) return;
      // The hub's latest seq, even when it went back (a restarted bridge counts from 1 again).
      const wentBack = answer.seq < since;
      since = answer.seq;
      if (first) {
        first = false;
        // The events kept from before subscribing are past; the status counts.
        onEvents([], answer.status);
      } else if (wentBack) {
        // A restarted bridge: its events are not ours to replay, but its status is.
        onEvents([], answer.status);
      } else if (answer.events.length > 0) {
        onEvents(answer.events, answer.status);
      }
    } catch {
      // Silent: the bridge may be restarting; the next poll tries again.
    }
    if (!disposed) timer = setTimeout(() => void poll(), AGENT_POLL_MS);
  };
  void poll();
  return () => {
    disposed = true;
    clearTimeout(timer);
  };
}
