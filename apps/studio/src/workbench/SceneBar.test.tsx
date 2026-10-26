// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { visibleWords } from "../ui/visibleText";
import { SceneBar } from "./SceneBar";
import type { SceneBarProps } from "./SceneBar";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("SceneBar", () => {
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

  const show = (props: Partial<SceneBarProps> = {}) =>
    act(() => root.render(<SceneBar showPermissions showEvents={false} topDown={false} onShowPermissions={() => {}} onShowEvents={() => {}} onTopDown={() => {}} {...props} />));
  const toggle = (name: string) => container.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;

  it("is_a_toolbar_with_one_toggle_per_layer_that_reports_its_state", () => {
    show();
    expect(container.querySelector("[role=toolbar]")?.getAttribute("aria-label")).toBe("Ebenen der Schau");
    expect(toggle("Gelände").getAttribute("aria-pressed")).toBe("true");
    expect(toggle("Ereignisse").getAttribute("aria-pressed")).toBe("false");
  });

  it("toggles_the_top_down_view_with_aria_pressed", () => {
    const onTopDown = vi.fn();
    show({ onTopDown });
    expect(toggle("Draufsicht").getAttribute("aria-pressed")).toBe("false");
    // A group of its own beside the layers.
    expect(toggle("Draufsicht").parentElement).not.toBe(toggle("Gelände").parentElement);
    act(() => toggle("Draufsicht").click());
    expect(onTopDown).toHaveBeenCalledWith(true);
    show({ topDown: true, onTopDown });
    expect(toggle("Draufsicht").getAttribute("aria-pressed")).toBe("true");
    act(() => toggle("Draufsicht").click());
    expect(onTopDown).toHaveBeenLastCalledWith(false);
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    show();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("asks_for_the_opposite_state_when_a_toggle_is_pressed", () => {
    const onShowPermissions = vi.fn();
    const onShowEvents = vi.fn();
    show({ onShowPermissions, onShowEvents });
    act(() => toggle("Gelände").click());
    act(() => toggle("Ereignisse").click());
    expect(onShowPermissions).toHaveBeenCalledWith(false);
    expect(onShowEvents).toHaveBeenCalledWith(true);
  });
});
