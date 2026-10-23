// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { INFO_MS, listToasts, resetToasts, toast } from "../ui/frame/toasts";
import type { Message } from "./message";
import { useToastMirror } from "./useToastMirror";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useToastMirror", () => {
  let container: HTMLElement;
  let root: Root;
  let current: Message | null;
  let set: (m: Message | null) => void;

  function Host() {
    const [message, setMessage] = useState<Message | null>(null);
    current = message;
    set = setMessage;
    useToastMirror(message, setMessage);
    return null;
  }
  const texts = () => listToasts().map((t) => `${t.kind}:${t.text}`);
  const say = (m: Message | null) => act(() => set(m));

  beforeEach(() => {
    vi.useFakeTimers();
    resetToasts();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<Host />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
    vi.useRealTimers();
  });

  it("shows_an_info_as_one_progress_toast_that_the_next_message_replaces", () => {
    say({ kind: "info", text: "Modelle 1/900" });
    say({ kind: "info", text: "Modelle 2/900" });
    expect(texts()).toEqual(["progress:Modelle 2/900"]);
  });

  it("clears_an_info_after_five_seconds_and_the_message_with_it", () => {
    say({ kind: "info", text: "3 Datei(en) gespeichert" });
    act(() => {
      vi.advanceTimersByTime(INFO_MS - 1);
    });
    expect(texts()).toEqual(["progress:3 Datei(en) gespeichert"]);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(texts()).toEqual([]);
    expect(current).toBeNull();
  });

  it("restarts_the_five_seconds_with_every_new_message", () => {
    say({ kind: "info", text: "a" });
    act(() => {
      vi.advanceTimersByTime(4000);
    });
    say({ kind: "info", text: "b" });
    act(() => {
      vi.advanceTimersByTime(4000);
    });
    expect(texts()).toEqual(["progress:b"]);
  });

  it("keeps_an_error_with_actions_through_five_seconds_and_through_setMessage_null", () => {
    const run = vi.fn();
    say({ kind: "error", text: "Patch fehlerhaft", actions: [{ label: "Patch reparieren", run }] });
    expect(listToasts()[0]?.actions.map((a) => a.label)).toEqual(["Patch reparieren"]);
    say({ kind: "info", text: "Lade …" });
    expect(texts()).toEqual(["error:Patch fehlerhaft", "progress:Lade …"]);
    say(null);
    // Only the session's own toast is gone.
    expect(texts()).toEqual(["error:Patch fehlerhaft"]);
    act(() => {
      vi.advanceTimersByTime(INFO_MS * 3);
    });
    expect(texts()).toEqual(["error:Patch fehlerhaft"]);
  });

  it("an_error_replaces_the_progress_toast_of_the_session", () => {
    say({ kind: "info", text: "Baue ROM ..." });
    say({ kind: "error", text: "Build abgebrochen" });
    expect(texts()).toEqual(["error:Build abgebrochen"]);
  });

  it("gives_the_message_up_when_the_user_closes_its_error_toast", () => {
    say({ kind: "error", text: "Kaputt" });
    expect(current?.text).toBe("Kaputt");
    act(() => toast.dismiss(listToasts()[0]!.id));
    expect(current).toBeNull();
  });

  it("reuses_the_open_toast_of_an_identical_error_and_closing_it_clears_the_message", () => {
    say({ kind: "error", text: "Kaputt" });
    say(null);
    say({ kind: "error", text: "Kaputt" });
    expect(texts()).toEqual(["error:Kaputt"]);
    expect(current?.text).toBe("Kaputt");
    act(() => toast.dismiss(listToasts()[0]!.id));
    expect(current).toBeNull();
  });

  it("does_not_clear_a_newer_message_when_an_older_error_toast_is_closed", () => {
    say({ kind: "error", text: "Alt" });
    const oldId = listToasts()[0]!.id;
    say({ kind: "info", text: "Neu" });
    act(() => toast.dismiss(oldId));
    expect(current?.text).toBe("Neu");
  });

  it("runs_the_action_of_the_error_toast", () => {
    const run = vi.fn();
    say({ kind: "error", text: "Fehler", actions: [{ label: "Probleme zeigen", run }] });
    act(() => listToasts()[0]!.actions[0]!.onClick());
    expect(run).toHaveBeenCalledOnce();
  });

  it("an_info_with_actions_gets_a_toast_of_its_own", () => {
    const run = vi.fn();
    say({ kind: "info", text: "Fertig", actions: [{ label: "Zeigen", run }] });
    say({ kind: "info", text: "Lade …" });
    expect(texts()).toEqual(["info:Fertig", "progress:Lade …"]);
  });
});
