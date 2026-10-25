// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ChecksFeed } from "../checks/useChecks";
import { ProblemsStore } from "../workbench/problems/problems";
import { HingeHost } from "./HingeHost";
import { HistoryFeed } from "./useHistory";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("HingeHost", () => {
  let container: HTMLElement;
  let root: Root;
  let checks: ChecksFeed;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    checks = new ChecksFeed();
    const noop = vi.fn();
    act(() =>
      root.render(
        <HingeHost
          problems={new ProblemsStore()}
          history={new HistoryFeed()}
          checks={checks}
          crumbs={[]}
          depth="bauen"
          onDepth={noop}
          onProblems={noop}
          onUndoTo={noop}
          onRedo={noop}
          onPlay={noop}
          playing={false}
          canPlay
        />,
      ),
    );
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const light = () => container.querySelector<HTMLButtonElement>("[data-light]")!;

  it("says_Nicht_geprueft_until_a_report_arrived_and_Alles_stimmig_only_after_one", () => {
    expect(light().getAttribute("aria-label")).toBe("Nicht geprüft");
    expect(light().getAttribute("data-light")).toBe("unchecked");
    act(() => checks.update({ running: true }));
    expect(light().getAttribute("aria-label")).toBe("Prüft …");
    // The first run ended without a report (it failed, or answered with none).
    act(() => checks.update({ running: false }));
    expect(light().getAttribute("aria-label")).toBe("Nicht geprüft");
    act(() => checks.update({ reported: true }));
    expect(light().getAttribute("aria-label")).toBe("Alles stimmig");
    expect(light().getAttribute("data-light")).toBe("sound");
  });
});
