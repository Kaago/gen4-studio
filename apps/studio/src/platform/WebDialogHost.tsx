import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { CSSProperties } from "react";
import { escapeCloses, useFocusTrap } from "../workbench/modal";
import { answerRequest, currentRequest, subscribeRequests } from "./dialogs";
import type { WebRequest } from "./dialogs";

const backdrop: CSSProperties = { position: "fixed", inset: 0, background: "rgba(0,0,0,0.35)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 };
const sheet: CSSProperties = { background: "#fff", borderRadius: 8, padding: 16, width: 460, maxWidth: "90vw", fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" };
const buttons: CSSProperties = { display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 12 };

/** Shows the dialogs `platform/dialogs` asks for in a browser, one at a time. Rendered only outside Tauri. */
export function WebDialogHost() {
  const request = useSyncExternalStore(subscribeRequests, currentRequest);
  return request ? <RequestDialog key={requestId(request)} request={request} /> : null;
}

// A new request must not reuse the previous one's state (typed text), also when two of the same kind follow each other.
const ids = new WeakMap<WebRequest, number>();
let nextId = 0;
function requestId(request: WebRequest): number {
  let id = ids.get(request);
  if (id === undefined) {
    id = nextId++;
    ids.set(request, id);
  }
  return id;
}

function RequestDialog({ request }: { request: WebRequest }) {
  const dialog = useRef<HTMLDivElement>(null);
  const [path, setPath] = useState(request.kind === "path" ? request.initial : "");
  const pathRef = useRef(path);
  pathRef.current = path;
  const isAsk = request.kind === "ask";
  useFocusTrap(dialog);
  useEffect(() => {
    dialog.current?.querySelector<HTMLElement>("input, button")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (escapeCloses(e)) answerRequest(isAsk ? false : null);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [isAsk]);

  if (request.kind === "ask") {
    return (
      <div style={backdrop} role="presentation">
        <div ref={dialog} role="dialog" aria-modal="true" aria-label="Rückfrage" style={sheet}>
          <div>{request.message}</div>
          <div style={buttons}>
            <button onClick={() => answerRequest(true)}>Ja</button>
            <button onClick={() => answerRequest(false)}>Nein</button>
          </div>
        </div>
      </div>
    );
  }
  return (
    <div style={backdrop} role="presentation">
      <div ref={dialog} role="dialog" aria-modal="true" aria-label={request.title} style={sheet}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            answerRequest(pathRef.current);
          }}
        >
          <div style={{ fontSize: 14, fontWeight: 600 }}>{request.title}</div>
          <label style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 8 }}>
            Pfad
            {/* `data-draft`: Escape cancels even with text in the field (see `escapeCloses`); the path is never "taken back". */}
            <input data-draft="" value={path} onChange={(e) => setPath(e.target.value)} spellCheck={false} style={{ fontFamily: "ui-monospace, monospace" }} />
          </label>
          <div style={buttons}>
            <button type="submit">OK</button>
            <button type="button" onClick={() => answerRequest(null)}>
              Abbrechen
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
