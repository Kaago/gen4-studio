// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TitleBar } from "./TitleBar";
import type { TitleBarProps } from "./TitleBar";

const env = vi.hoisted(() => ({ mac: false }));
vi.mock("../../platform/os", () => ({ isMacTauri: () => env.mac }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TitleBar", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = () => ({ onOpen: vi.fn(), onSave: vi.fn(), onBuild: vi.fn(), onHelp: vi.fn(), onToggleTheme: vi.fn(), onPalette: vi.fn() });
  const base = (over: Partial<TitleBarProps> = {}): TitleBarProps => ({
    projectName: "pt-3c1",
    game: "Platin, DE",
    dirtyCount: 0,
    busy: false,
    theme: "hell",
    ...handlers(),
    ...over,
  });
  const show = (props: TitleBarProps) => act(() => root.render(<TitleBar {...props} />));
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === name || b.textContent === name) as HTMLButtonElement | undefined;
  const header = () => container.firstElementChild as HTMLElement;

  beforeEach(() => {
    env.mac = false;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("shows_the_project_name_and_the_game", () => {
    show(base());
    expect(container.textContent).toContain("pt-3c1");
    expect(container.textContent).toContain("Platin, DE");
  });

  it("says_that_no_project_is_open_and_shows_neither_game_nor_save_state", () => {
    show(base({ projectName: null, game: null }));
    expect(container.textContent).toContain("Kein Projekt geöffnet");
    expect(container.textContent).not.toContain("gespeichert");
  });

  it("counts_the_unsaved_changes_or_says_everything_is_saved", () => {
    show(base({ dirtyCount: 3 }));
    expect(container.textContent).toContain("3 Änderungen nicht gespeichert");
    show(base({ dirtyCount: 1 }));
    expect(container.textContent).toContain("1 Änderung nicht gespeichert");
    expect(container.textContent).not.toContain("1 Änderungen");
    show(base({ dirtyCount: 0 }));
    expect(container.textContent).toContain("Alles gespeichert");
    expect(container.textContent).not.toContain("nicht gespeichert");
  });

  it("indents_for_the_traffic_lights_only_on_a_mac_in_tauri", () => {
    show(base());
    expect(header().style.paddingLeft).toBe("");
    env.mac = true;
    show(base());
    expect(header().style.paddingLeft).toBe("78px");
  });

  it("is_a_drag_region_and_its_controls_are_not", () => {
    show(base());
    expect(header().hasAttribute("data-tauri-drag-region")).toBe(true);
    for (const b of container.querySelectorAll("button")) expect(b.hasAttribute("data-tauri-drag-region")).toBe(false);
  });

  it("calls_the_handlers_of_its_buttons_by_name", () => {
    const props = base();
    show(props);
    act(() => button("Projekt öffnen")!.click());
    act(() => button("Speichern")!.click());
    act(() => button("Bauen")!.click());
    act(() => button("Thema wechseln")!.click());
    act(() => button("Hilfe")!.click());
    expect(props.onOpen).toHaveBeenCalledOnce();
    expect(props.onSave).toHaveBeenCalledOnce();
    expect(props.onBuild).toHaveBeenCalledOnce();
    expect(props.onToggleTheme).toHaveBeenCalledOnce();
    expect(props.onHelp).toHaveBeenCalledOnce();
  });

  it("disables_save_and_build_without_a_project_and_while_busy", () => {
    show(base({ projectName: null, game: null }));
    expect(button("Speichern")!.disabled).toBe(true);
    expect(button("Bauen")!.disabled).toBe(true);
    expect(button("Projekt öffnen")!.disabled).toBe(false);
    show(base({ busy: true }));
    expect(button("Speichern")!.disabled).toBe(true);
    expect(button("Bauen")!.disabled).toBe(true);
    expect(button("Projekt öffnen")!.disabled).toBe(true);
  });

  it("opens_the_palette_from_its_search_button_and_shows_the_shortcut", () => {
    const props = base();
    show(props);
    const palette = [...container.querySelectorAll("button")].find((b) => b.textContent?.includes("Suchen oder Befehl eingeben"))!;
    expect(palette.textContent).toContain("⌘K");
    expect(palette.disabled).toBe(false);
    expect(palette.hasAttribute("aria-disabled")).toBe(false);
    act(() => palette.click());
    expect(props.onPalette).toHaveBeenCalledOnce();
  });

  it("locks_the_palette_button_when_asked_also_without_a_project", () => {
    const props = base({ paletteDisabled: true });
    show(props);
    const palette = () => [...container.querySelectorAll("button")].find((b) => b.textContent?.includes("Suchen oder Befehl eingeben"))!;
    expect(palette().disabled).toBe(true);
    act(() => palette().click());
    expect(props.onPalette).not.toHaveBeenCalled();
    // Without a project the palette still opens: its commands open one.
    show(base({ projectName: null, game: null }));
    expect(palette().disabled).toBe(false);
  });
});
