export interface MessageAction {
  label: string;
  run: () => void;
}

/**
 * What the session says about its work: shown as a toast (see `useToastMirror`). `key` (an error): the toast open with that key is
 * replaced in place instead of another one stacking (the conflict message of the sync, say).
 */
export type Message = { kind: "info" | "error"; text: string; actions?: MessageAction[]; key?: string };
