// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SHORTCUTS } from "../world/buildings";
import { SettingsArea } from "./SettingsArea";
import type { SettingsAreaProps } from "./SettingsArea";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("SettingsArea", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onTheme: vi.fn(), onDensity: vi.fn(), onPickEmulator: vi.fn() };
  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const render = (props: Partial<SettingsAreaProps> = {}) =>
    act(() => root.render(<SettingsArea theme="system" density="komfort" emulatorPath={undefined} {...handlers} {...props} />));
  const group = (label: string) => container.querySelector<HTMLElement>(`[role=group][aria-label="${label}"]`);
  const option = (groupLabel: string, text: string) => [...(group(groupLabel)?.querySelectorAll("button") ?? [])].find((b) => b.textContent === text);
  const heading = (text: string) => [...container.querySelectorAll("h1, h2")].find((h) => h.textContent === text);

  it("renders_the_columns_darstellung_emulator_and_tastenkuerzel", () => {
    render();
    expect(heading("Einstellungen")).toBeDefined();
    expect(heading("Darstellung")).toBeDefined();
    expect(heading("Emulator")).toBeDefined();
    expect(heading("Tastenkürzel")).toBeDefined();
  });

  it("the_theme_segmented_offers_hell_dunkel_and_wie_system_and_picks_with_set_theme", () => {
    render({ theme: "system" });
    expect([...group("Thema")!.querySelectorAll("button")].map((b) => b.textContent)).toEqual(["Hell", "Dunkel", "Wie System"]);
    expect(option("Thema", "Wie System")?.getAttribute("aria-pressed")).toBe("true");
    // Each theme option carries its symbol.
    expect(group("Thema")!.querySelectorAll("button svg")).toHaveLength(3);
    act(() => option("Thema", "Dunkel")!.click());
    expect(handlers.onTheme).toHaveBeenCalledWith("dunkel");
  });

  it("the_density_segmented_picks_with_set_density_and_explains_kompakt", () => {
    render({ density: "komfort" });
    expect([...group("Dichte")!.querySelectorAll("button")].map((b) => b.textContent)).toEqual(["Komfort", "Kompakt"]);
    expect(container.textContent).toContain("Kompakt: Zeilen 24 statt 32 px hoch");
    act(() => option("Dichte", "Kompakt")!.click());
    expect(handlers.onDensity).toHaveBeenCalledWith("kompakt");
  });

  it("has_the_emulator_row", () => {
    render();
    expect(container.textContent).toContain("Kein Emulator gewählt");
    act(() => [...container.querySelectorAll("button")].find((b) => b.textContent === "Emulator wählen")!.click());
    expect(handlers.onPickEmulator).toHaveBeenCalledOnce();
    render({ emulatorPath: "/Applications/melonDS.app" });
    expect(container.textContent).toContain("/Applications/melonDS.app");
  });

  it("lists_every_app_shortcut_read_only", () => {
    render();
    const list = container.querySelector('[aria-label="Tastenkürzel"]')!;
    const keys = [...list.querySelectorAll("kbd")].map((k) => k.textContent);
    expect(keys).toEqual(Object.values(SHORTCUTS).map((s) => s.keys));
    expect(list.textContent).toContain("Projekt öffnen");
    expect(list.querySelector("button, input")).toBeNull();
  });
});
