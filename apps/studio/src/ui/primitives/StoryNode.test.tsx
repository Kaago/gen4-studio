// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { StoryNode, storyKindWord } from "./StoryNode";
import type { StoryNodeKind, StoryNodeProps } from "./StoryNode";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("StoryNode", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (props: StoryNodeProps) => act(() => root.render(<StoryNode {...props} />));
  const node = () => container.querySelector<HTMLElement>("[data-story-node]")!;

  it("seven_kinds_render_their_word", () => {
    const words: [StoryNodeKind, string][] = [
      ["start", "Start"],
      ["dialog", "Dialog"],
      ["condition", "Bedingung"],
      ["battle", "Kampf"],
      ["reward", "Belohnung"],
      ["progress", "Fortschritt"],
      ["appearance", "Erscheinen/Verschwinden"],
      ["value", "Wert"],
      ["place", "Ort"],
    ];
    for (const [kind, word] of words) {
      expect(storyKindWord(kind)).toBe(word);
      show({ kind, title: "Rivalin trifft", subtitle: "Wert 1" });
      expect(node().getAttribute("data-kind")).toBe(kind);
      expect(node().textContent).toContain(word);
      expect(node().textContent).toContain("Rivalin trifft");
      expect(node().textContent).toContain("Wert 1");
      // Never colour alone: every kind has its symbol beside the word.
      expect(node().querySelector("svg")).not.toBeNull();
    }
  });

  it("selected_and_noted_only_set_data_attributes", () => {
    show({ kind: "dialog", title: "Rivalin trifft" });
    expect(node().hasAttribute("data-selected")).toBe(false);
    expect(node().hasAttribute("data-noted-only")).toBe(false);
    expect(node().hasAttribute("data-loose")).toBe(false);
    expect(node().textContent).not.toContain("nur notiert");

    show({ kind: "dialog", title: "Rivalin trifft", selected: true, notedOnly: true, dataAttrs: { "data-node-id": "v:1" } });
    expect(node().getAttribute("data-selected")).toBe("");
    expect(node().getAttribute("data-noted-only")).toBe("");
    expect(node().getAttribute("data-node-id")).toBe("v:1");
    // The dashed edge is not the only sign: the plakette says it.
    expect(node().textContent).toContain("nur notiert");

    show({ kind: "place", title: "Scriptdatei 0101", loose: true });
    expect(node().getAttribute("data-loose")).toBe("");
    expect(node().textContent).toContain("ohne Wert");
  });

  it("a_node_with_onOpen_is_a_button", () => {
    show({ kind: "place", title: "Scriptdatei 0101" });
    expect(node().getAttribute("role")).toBeNull();
    expect(node().hasAttribute("tabindex")).toBe(false);

    const onOpen = vi.fn();
    show({ kind: "place", title: "Scriptdatei 0101", subtitle: "Zweiblattdorf", onOpen });
    expect(node().getAttribute("role")).toBe("button");
    expect(node().getAttribute("tabindex")).toBe("0");
    expect(node().getAttribute("aria-label")).toBe("Ort: Scriptdatei 0101, Zweiblattdorf");
    act(() => node().click());
    expect(onOpen).toHaveBeenCalledTimes(1);
    pressKey(node(), "Enter");
    pressKey(node(), " ");
    expect(onOpen).toHaveBeenCalledTimes(3);

    // A node that is no button can carry a control in its head (the graph's „Öffnen“).
    show({ kind: "place", title: "Scriptdatei 0101", action: <button type="button">Öffnen</button> });
    expect(node().querySelector("button")?.textContent).toBe("Öffnen");
    expect(node().getAttribute("role")).toBeNull();
  });
});
