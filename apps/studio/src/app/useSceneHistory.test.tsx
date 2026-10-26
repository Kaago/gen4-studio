// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useSceneHistory } from "./useSceneHistory";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type History = ReturnType<typeof useSceneHistory>;

describe("useSceneHistory", () => {
  let container: HTMLElement;
  let root: Root;
  // What the app does: a click or a jump sets the selected header, and the history sees it on the next render.
  let history: History;
  let shownNow: () => number | null;
  let showScene: (header: number | null) => void;
  let shown: number[];
  let setProject: (project: string | null) => void;

  function Probe() {
    const [selected, setSelected] = useState<number | null>(null);
    const [project, setProjectState] = useState<string | null>("a");
    showScene = setSelected;
    setProject = setProjectState;
    shownNow = () => selected;
    history = useSceneHistory(
      selected,
      (header) => {
        shown.push(header);
        setSelected(header);
      },
      project,
    );
    return null;
  }
  const show = (...headers: number[]) => {
    for (const h of headers) act(() => showScene(h));
  };

  beforeEach(() => {
    shown = [];
    container = document.createElement("div");
    root = createRoot(container);
    act(() => root.render(<Probe />));
  });
  afterEach(() => act(() => root.unmount()));

  it("records_each_shown_scene_once", () => {
    expect(history.canBack).toBe(false);
    expect(history.canForward).toBe(false);
    show(5);
    // One scene alone has nothing to go back to.
    expect(history.canBack).toBe(false);
    // The same header shown again (a click on the shown scene, a link to it) is not a new scene.
    show(5);
    expect(history.canBack).toBe(false);
    show(7);
    expect(history.canBack).toBe(true);
    act(() => history.back());
    expect(shownNow()).toBe(5);
    expect(history.canBack).toBe(false);
  });

  it("goes_back_and_forward_without_recording_its_own_jumps", () => {
    show(1, 2, 3);
    act(() => history.back());
    expect(shownNow()).toBe(2);
    act(() => history.back());
    expect(shownNow()).toBe(1);
    expect(history.canBack).toBe(false);
    expect(history.canForward).toBe(true);
    act(() => history.forward());
    act(() => history.forward());
    expect(shownNow()).toBe(3);
    expect(history.canForward).toBe(false);
    expect(shown).toEqual([2, 1, 2, 3]);
    act(() => history.back());
    expect(shownNow()).toBe(2);
    expect(history.canBack).toBe(true);
  });

  it("does_nothing_at_the_ends", () => {
    show(1, 2);
    act(() => history.forward());
    expect(shown).toEqual([]);
    act(() => history.back());
    act(() => history.back());
    expect(shown).toEqual([1]);
  });

  it("drops_forward_after_a_new_scene", () => {
    show(1, 2, 3);
    act(() => history.back());
    act(() => history.back());
    expect(history.canForward).toBe(true);
    show(9);
    expect(history.canForward).toBe(false);
    act(() => history.back());
    expect(shownNow()).toBe(1);
    act(() => history.forward());
    expect(shownNow()).toBe(9);
  });

  it("shows_the_previous_scene_after_a_creation_showed_a_new_one", () => {
    // A wizard made an area and the app showed it (a goTo like any other): „Zurück“ leads to the scene that was shown before.
    show(4);
    show(61);
    act(() => history.back());
    expect(shownNow()).toBe(4);
  });

  it("keeps_at_most_50", () => {
    for (let h = 1; h <= 60; h++) show(h);
    let steps = 0;
    while (history.canBack) {
      act(() => history.back());
      steps++;
    }
    // 50 scenes: 49 steps back, ending on the 11th.
    expect(steps).toBe(49);
    expect(shownNow()).toBe(11);
  });

  it("starts_over_with_another_project", () => {
    show(1, 2);
    expect(history.canBack).toBe(true);
    act(() => {
      setProject("b");
      showScene(null);
    });
    expect(history.canBack).toBe(false);
    expect(history.canForward).toBe(false);
    show(3);
    expect(history.canBack).toBe(false);
    show(4);
    act(() => history.back());
    expect(shownNow()).toBe(3);
  });
});
