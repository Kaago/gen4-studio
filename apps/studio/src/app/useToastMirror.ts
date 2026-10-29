import { useEffect } from "react";
import type { Dispatch, SetStateAction } from "react";
import { INFO_MS, listToasts, subscribeToasts, toast } from "../ui/frame/toasts";
import type { Message } from "./message";

/** The key of the one progress toast that shows the session's info message. */
const SESSION = "session";

/**
 * Shows the session message as a toast, and gives the message up again when its toast is gone:
 * - an info without actions is the progress toast "session" (a new one replaces it, `null` removes it) and clears the message after 5 s;
 * - an error stays until the user closes it, and closing it clears the message (if it is still the shown one);
 * - `setMessage(null)` removes only the progress toast. An error toast (and its actions) outlives the message that made it.
 */
export function useToastMirror(message: Message | null, setMessage: Dispatch<SetStateAction<Message | null>>): void {
  useEffect(() => {
    if (message === null) {
      toast.progress(SESSION, null);
      return;
    }
    const actions = (message.actions ?? []).map((a) => ({ label: a.label, onClick: a.run }));
    const clear = () => setMessage((shown) => (shown === message ? null : shown));
    if (message.kind === "error") {
      toast.progress(SESSION, null);
      const id = toast.error(message.text, { actions, ...(message.key === undefined ? {} : { key: message.key }) });
      return subscribeToasts(() => {
        if (!listToasts().some((t) => t.id === id)) clear();
      });
    }
    if (actions.length > 0) {
      // Buttons need a toast that is not replaced by the next progress line.
      toast.progress(SESSION, null);
      toast.info(message.text, { actions });
    } else {
      toast.progress(SESSION, message.text);
    }
    const timer = setTimeout(clear, INFO_MS);
    return () => clearTimeout(timer);
  }, [message, setMessage]);
}
