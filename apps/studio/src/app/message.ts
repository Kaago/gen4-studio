export interface MessageAction {
  label: string;
  run: () => void;
}

/** What the session says about its work: shown as a toast (see `useToastMirror`). */
export type Message = { kind: "info" | "error"; text: string; actions?: MessageAction[] };
