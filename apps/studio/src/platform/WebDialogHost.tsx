import { useRef, useState, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, TextField } from "../ui/primitives";
import { answerRequest, currentRequest, subscribeRequests } from "./dialogs";
import type { WebRequest } from "./dialogs";

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

/**
 * One request as a modal `Dialog`: a question with Nein and Ja (or the labels it asked for), or a path typed into a field with Abbrechen and OK. Escape, the close
 * button and a click outside answer no (or cancel); Escape cancels the path dialog also with text in the field (the field follows every
 * keystroke, so there is no draft to take back first). Enter in the field takes the path.
 */
function RequestDialog({ request }: { request: WebRequest }) {
  const { t } = useTranslation("platform");
  const [path, setPath] = useState(request.kind === "path" ? request.initial : "");
  const pathRef = useRef(path);
  pathRef.current = path;
  const cancel = () => answerRequest(request.kind === "ask" ? false : null);
  const onOpenChange = (open: boolean) => {
    if (!open) cancel();
  };

  if (request.kind === "ask") {
    return (
      <Dialog
        open
        onOpenChange={onOpenChange}
        title={request.title ?? t("webDialog.question")}
        description={request.message}
        actions={[
          { label: request.cancelLabel ?? t("webDialog.no"), onClick: () => answerRequest(false) },
          { label: request.okLabel ?? t("webDialog.yes"), kind: request.level === "warning" ? "danger" : "primary", onClick: () => answerRequest(true) },
        ]}
      />
    );
  }
  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={request.title}
      actions={[
        { label: t("webDialog.cancel"), onClick: () => answerRequest(null) },
        { label: t("webDialog.ok"), kind: "primary", onClick: () => answerRequest(pathRef.current) },
      ]}
    >
      <div
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          // Used up here: focus returns to the button that opened the dialog, and the browser would press it again with this Enter.
          e.preventDefault();
          answerRequest(pathRef.current);
        }}
      >
        <TextField label={t("webDialog.path")} showLabel value={path} onChange={setPath} onCommit={setPath} />
      </div>
    </Dialog>
  );
}
