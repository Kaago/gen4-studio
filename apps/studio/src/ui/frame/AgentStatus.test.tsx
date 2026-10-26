// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentStatus as AgentStatusDto } from "../../api";
import { pressKey } from "../primitives/testUtils";
import { AgentStatus } from "./AgentStatus";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const status = (clients: string[], readOnly = false): AgentStatusDto => ({
  enabled: true,
  expert: false,
  read_only: readOnly,
  port: 49152,
  server_error: null,
  bridge: false,
  connections: clients.map((client, i) => ({ id: i + 1, client, since_ms: 0 })),
  question: null,
});

describe("AgentStatus", () => {
  let container: HTMLElement;
  let root: Root;
  const onDisconnect = vi.fn();
  const onReadOnly = vi.fn();

  const show = (s: AgentStatusDto | null) => act(() => root.render(<AgentStatus status={s} onDisconnect={onDisconnect} onReadOnly={onReadOnly} />));
  const item = () => container.querySelector<HTMLElement>("[data-agent-status]");
  const trigger = () => item()!.querySelector<HTMLElement>("button")!;
  /** Opens the menu the way a pointer does and returns its items. */
  const openMenu = () => {
    act(() => {
      trigger().dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger().dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger().click();
    });
    return [...document.querySelectorAll<HTMLElement>("[role^=menuitem]")];
  };

  beforeEach(() => {
    onDisconnect.mockReset();
    onReadOnly.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows_nothing_without_a_connection", () => {
    show(null);
    expect(container.innerHTML).toBe("");
    show(status([]));
    expect(container.innerHTML).toBe("");
  });

  it("shows_client_name", () => {
    show(status(["Claude Code"]));
    expect(item()?.textContent).toBe("Agent verbunden: Claude Code");
    // The agent symbol stands before the text.
    expect(trigger().querySelector("svg")).not.toBeNull();
    expect(item()?.hasAttribute("data-agent-read-only")).toBe(false);
  });

  it("two_agents_counted", () => {
    show(status(["Claude Code", "Agent"]));
    expect(item()?.textContent).toBe("2 Agenten verbunden");
  });

  it("read_only_marked", () => {
    show(status(["Claude Code"], true));
    expect(item()?.textContent).toBe("Agent verbunden: Claude Code · Nur lesen");
    expect(item()?.hasAttribute("data-agent-read-only")).toBe(true);
    const [readOnly] = openMenu();
    expect(readOnly?.textContent).toContain("Nur lesen");
    expect(readOnly?.getAttribute("aria-checked")).toBe("true");
    act(() => readOnly!.click());
    expect(onReadOnly).toHaveBeenCalledExactlyOnceWith(false);
  });

  it("read_only_turns_on_from_the_keyboard", () => {
    show(status(["Claude Code"]));
    const [readOnly] = openMenu();
    expect(readOnly?.getAttribute("aria-checked")).toBe("false");
    act(() => void pressKey(readOnly!, "Enter"));
    expect(onReadOnly).toHaveBeenCalledExactlyOnceWith(true);
  });

  it("menu_disconnects", () => {
    show(status(["Claude Code"]));
    const items = openMenu();
    expect(items.map((i) => i.textContent)).toEqual(["Nur lesen", "Trennen"]);
    act(() => items[1]!.click());
    expect(onDisconnect).toHaveBeenCalledOnce();
    expect(onReadOnly).not.toHaveBeenCalled();
  });
});
