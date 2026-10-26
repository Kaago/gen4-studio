// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import type { AgentSettingsDto, AgentStatus, Grant, GrantLevel } from "../api";
import { AgentsSettings } from "./AgentsSettings";
import type { AgentsSettingsProps } from "./AgentsSettings";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const SETUP = 'claude mcp add gen4-studio -- "/Applications/Gen4 Studio.app/Contents/MacOS/gen4-studio" mcp';
const OPEN = "/Users/me/Projekte/Platin";

const dto = (over: { enabled?: boolean; expert?: boolean; level?: GrantLevel | null; open?: boolean; others?: [string, GrantLevel][] } = {}): AgentSettingsDto => {
  const { enabled = false, expert = false, level = null, open = true, others = [] } = over;
  const grants: Grant[] = others.map(([path, l]) => ({ path, base_id: null, level: l }));
  if (open && level !== null) grants.unshift({ path: OPEN, base_id: "abc/1", level });
  return {
    settings: { version: 1, enabled, expert, grants },
    setup_line: SETUP,
    open_project: open ? { path: OPEN, level } : null,
  };
};

const status = (over: Partial<AgentStatus> = {}): AgentStatus => ({
  enabled: true,
  expert: false,
  read_only: false,
  port: 49152,
  server_error: null,
  bridge: false,
  connections: [],
  question: null,
  ...over,
});

describe("AgentsSettings", () => {
  let container: HTMLElement;
  let root: Root;
  let current: AgentSettingsDto;
  const onError = vi.fn();

  const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(0)));
  const render = async (props: Partial<AgentsSettingsProps> = {}) => {
    await act(async () => root.render(<AgentsSettings projectOpen bridge={false} status={null} onError={onError} {...props} />));
    await settle();
  };
  const section = () => container.querySelector<HTMLElement>("[data-agents-settings]");
  const switchIn = (attr: string) => container.querySelector<HTMLElement>(`[${attr}] [role=switch]`);
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text || b.getAttribute("aria-label") === text);
  const calls = (cmd: string) => invoke.mock.calls.filter(([c]) => c === cmd);

  beforeEach(() => {
    vi.useFakeTimers();
    current = dto();
    invoke.mockReset();
    invoke.mockImplementation(async (cmd: string) => (cmd === "agent_settings" ? current : undefined));
    onError.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("switch_turns_agents_on", async () => {
    await render();
    const section = container.querySelector("[aria-labelledby=settings-agents]");
    expect(section?.querySelector("#settings-agents")?.textContent).toBe("Agenten");
    expect(container.querySelector("[data-agents-enabled]")?.textContent).toContain("Lokale Agenten zulassen");
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Aus");
    const reads = calls("agent_settings").length;
    current = dto({ enabled: true });
    await act(async () => switchIn("data-agents-enabled")!.click());
    await settle();
    expect(invoke).toHaveBeenCalledWith("agent_set_enabled", { on: true });
    // Reads the settings again after the change.
    expect(calls("agent_settings").length).toBeGreaterThan(reads);
    expect(switchIn("data-agents-enabled")?.getAttribute("aria-checked")).toBe("true");
  });

  it("names_the_server_state_in_the_app_and_in_the_browser_mode", async () => {
    current = dto({ enabled: true });
    await render({ status: status({ port: 49152 }) });
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Bereit auf 127.0.0.1:49152");
    await render({ status: status({ port: null }) });
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Aus");
    await render({ bridge: true, status: status({ port: null, bridge: true }) });
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Über die Brücke (/mcp)");
    current = dto({ enabled: false });
    await render({ bridge: true, status: status({ enabled: false, port: null, bridge: true }) });
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Aus");
  });

  it("shows_why_the_server_did_not_start", async () => {
    current = dto({ enabled: true });
    await render({ status: status({ port: null, server_error: "Port 0 nicht frei: Zugriff verweigert" }) });
    expect(container.querySelector("[data-agents-server]")?.textContent).toBe("Aus");
    const error = container.querySelector("[data-agents-server-error]");
    expect(error?.querySelector("[role=alert]")?.textContent).toContain("Server nicht gestartet");
    expect(error?.textContent).toContain("Port 0 nicht frei: Zugriff verweigert");
    // Gone once the server runs.
    await render({ status: status({ port: 49152, server_error: null }) });
    expect(container.querySelector("[data-agents-server-error]")).toBeNull();
  });

  it("rereads_when_the_status_changes", async () => {
    await render({ status: status({ enabled: false, port: null }) });
    const reads = calls("agent_settings").length;
    current = dto({ enabled: true });
    await render({ status: status({ enabled: true, port: 49152 }) });
    expect(calls("agent_settings").length).toBe(reads + 1);
    expect(switchIn("data-agents-enabled")?.getAttribute("aria-checked")).toBe("true");
    // The same status object reads nothing.
    const same = status({ enabled: true, port: 49152 });
    await render({ status: same });
    const after = calls("agent_settings").length;
    await render({ status: same });
    expect(calls("agent_settings").length).toBe(after);
  });

  it("shows_the_grant_an_answered_question_saved", async () => {
    const asking = status({ connections: [{ id: 1, client: "Claude Code", since_ms: 0 }], question: { id: 3, kind: "connect", client: "Claude Code", project: OPEN, what: null } });
    current = dto({ level: null });
    await render({ status: asking });
    const pressed = () => [...container.querySelectorAll<HTMLElement>('[aria-label="Freigabe für dieses Projekt"] button')].find((b) => b.getAttribute("aria-pressed") === "true")?.textContent;
    expect(pressed()).toBeUndefined();
    // The user answered „Lesen“ in the dialog: the hub saved the grant, the question is gone, switch and port are the same.
    current = dto({ level: "read" });
    await render({ status: { ...asking, question: null } });
    expect(pressed()).toBe("Lesen");
  });

  it("shows_setup_line_and_copies", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await render();
    expect(container.querySelector("[data-agents-setup]")?.textContent).toContain(SETUP);
    await act(async () => button("Einrichtungszeile kopieren")!.click());
    await settle();
    expect(writeText).toHaveBeenCalledWith(SETUP);
    expect(button("Kopiert")).toBeDefined();
    await act(async () => void (await vi.advanceTimersByTimeAsync(2000)));
    expect(button("Einrichtungszeile kopieren")).toBeDefined();
    expect(button("Kopiert")).toBeUndefined();
  });

  it("a_missing_clipboard_says_so", async () => {
    Object.defineProperty(navigator, "clipboard", { value: undefined, configurable: true });
    await render();
    await act(async () => button("Einrichtungszeile kopieren")!.click());
    await settle();
    expect(onError).toHaveBeenCalledOnce();
    expect(String(onError.mock.calls[0]![0])).toContain("Zwischenablage");
    expect(button("Kopiert")).toBeUndefined();
  });

  it("grant_segment_sets_level", async () => {
    current = dto({ level: "read" });
    await render();
    const group = container.querySelector<HTMLElement>('[role=group][aria-label="Freigabe für dieses Projekt"]');
    expect([...group!.querySelectorAll("button")].map((b) => b.textContent)).toEqual(["Nicht", "Lesen", "Lesen und bearbeiten"]);
    const write = [...group!.querySelectorAll("button")].find((b) => b.textContent === "Lesen und bearbeiten")!;
    expect([...group!.querySelectorAll("button")].find((b) => b.textContent === "Lesen")?.getAttribute("aria-pressed")).toBe("true");
    current = dto({ level: "write" });
    await act(async () => write.click());
    await settle();
    expect(invoke).toHaveBeenCalledWith("agent_set_grant", { level: "write" });
    expect(write.getAttribute("aria-pressed")).toBe("true");
  });

  it("an_open_project_without_grant_selects_nothing_and_says_the_app_asks", async () => {
    current = dto({ level: null });
    await render();
    const group = container.querySelector<HTMLElement>('[role=group][aria-label="Freigabe für dieses Projekt"]');
    expect([...group!.querySelectorAll("button")].some((b) => b.getAttribute("aria-pressed") === "true")).toBe(false);
    expect(section()?.textContent).toContain("fragt beim ersten Zugriff");
  });

  it("no_project_says_so", async () => {
    current = dto({ open: false });
    await render({ projectOpen: false });
    expect(container.querySelector('[aria-label="Freigabe für dieses Projekt"]')).toBeNull();
    expect(section()?.textContent).toContain("Kein Projekt offen");
  });

  it("expert_switch_defaults_off", async () => {
    await render();
    expect(container.querySelector("[data-agents-expert]")?.textContent).toContain("Expertenwerkzeuge (g4_call)");
    expect(switchIn("data-agents-expert")?.getAttribute("aria-checked")).toBe("false");
    current = dto({ expert: true });
    await act(async () => switchIn("data-agents-expert")!.click());
    await settle();
    expect(invoke).toHaveBeenCalledWith("agent_set_expert", { on: true });
    expect(switchIn("data-agents-expert")?.getAttribute("aria-checked")).toBe("true");
  });

  it("removes_other_grant", async () => {
    current = dto({ level: "write", others: [["/Users/me/Projekte/HeartGold", "read"]] });
    await render();
    const list = container.querySelector<HTMLElement>("[data-agents-grants]");
    // The open project's grant is the segment above, not a row of the list.
    expect(list?.textContent).toContain("/Users/me/Projekte/HeartGold");
    expect(list?.textContent).toContain("Lesen");
    expect(list?.textContent).not.toContain(OPEN);
    current = dto({ level: "write" });
    const remove = list!.querySelector<HTMLElement>('button[aria-label="Freigabe entfernen"]')!;
    await act(async () => remove.click());
    await settle();
    expect(invoke).toHaveBeenCalledWith("agent_remove_grant", { path: "/Users/me/Projekte/HeartGold" });
    expect(container.querySelector("[data-agents-grants]")).toBeNull();
  });

  it("a_failed_action_goes_to_on_error", async () => {
    const failure = new Error("Port belegt");
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "agent_set_enabled") throw failure;
      return cmd === "agent_settings" ? current : undefined;
    });
    await render();
    await act(async () => switchIn("data-agents-enabled")!.click());
    await settle();
    expect(onError).toHaveBeenCalledWith(failure);
  });
});
