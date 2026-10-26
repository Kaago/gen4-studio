// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { QuestionDto } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { AgentQuestion } from "./AgentQuestion";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const connect: QuestionDto = { id: 7, kind: "connect", client: "Claude Code", project: "/Users/me/Roms/_testkopien/platin", what: null };
const confirm: QuestionDto = { id: 9, kind: "confirm", client: "Claude Code", project: null, what: "Scripts übernehmen" };

describe("AgentQuestion", () => {
  let container: HTMLElement;
  let root: Root;
  const onAnswer = vi.fn();

  const show = (question: QuestionDto | null) => act(() => root.render(<AgentQuestion question={question} onAnswer={onAnswer} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.getAttribute("aria-label") !== "Schließen");
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  /** The question's sentence: the dialog's description. */
  const sentence = () => document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent;

  beforeEach(() => {
    onAnswer.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows_nothing_without_a_question", () => {
    show(null);
    expect(dialog()).toBeNull();
  });

  it("connect_answers_write", () => {
    show(connect);
    expect(dialog()?.textContent).toContain("Agent möchte zugreifen");
    // The marker sits on the dialog itself.
    expect(document.querySelector("[data-agent-question]")).toBe(dialog());
    expect(sentence()).toBe("„Claude Code“ möchte auf das Projekt „platin“ zugreifen.");
    expect(buttons().map((b) => b.textContent)).toEqual(["Ablehnen", "Lesen", "Lesen und bearbeiten"]);
    // The safer level is the main button.
    expect(buttons().map((b) => b.dataset.kind)).toEqual(["secondary", "primary", "secondary"]);
    act(() => button("Lesen und bearbeiten")!.click());
    expect(onAnswer).toHaveBeenCalledExactlyOnceWith(7, "write");
  });

  it("connect_answers_read_and_deny", () => {
    show({ ...connect, project: "platin" });
    expect(sentence()).toBe("„Claude Code“ möchte auf das Projekt „platin“ zugreifen.");
    act(() => button("Lesen")!.click());
    show({ ...connect, id: 8, project: "platin" });
    act(() => button("Ablehnen")!.click());
    expect(onAnswer.mock.calls).toEqual([
      [7, "read"],
      [8, "deny"],
    ]);
  });

  it("confirm_answers_allow", () => {
    show(confirm);
    expect(dialog()?.textContent).toContain("Agent möchte etwas Unumkehrbares tun");
    expect(sentence()).toBe("„Claude Code“ möchte: Scripts übernehmen (lässt sich nicht rückgängig machen).");
    expect(buttons().map((b) => b.textContent)).toEqual(["Ablehnen", "Zulassen"]);
    act(() => button("Zulassen")!.click());
    expect(onAnswer).toHaveBeenCalledExactlyOnceWith(9, "allow");
  });

  it("answers_each_question_once", () => {
    show(connect);
    act(() => button("Lesen")!.click());
    act(() => button("Lesen und bearbeiten")!.click());
    act(() => void pressKey((document.activeElement as HTMLElement) ?? dialog()!, "Escape"));
    expect(onAnswer.mock.calls).toEqual([[7, "read"]]);
    // The next question is answered again.
    show({ ...connect, id: 8 });
    act(() => button("Ablehnen")!.click());
    expect(onAnswer.mock.calls).toEqual([
      [7, "read"],
      [8, "deny"],
    ]);
  });

  it("escape_denies", () => {
    show(confirm);
    act(() => void pressKey((document.activeElement as HTMLElement) ?? dialog()!, "Escape"));
    expect(onAnswer).toHaveBeenCalledExactlyOnceWith(9, "deny");
  });
});
