// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createEditBus } from "../bus/editBus";
import type { EditEvent } from "../bus/editBus";
import { usePlaceChanged } from "./panels";

// The scene needs WebGL and the script editor CodeMirror: neither is under test here.
vi.mock("../world/MatrixScene", () => ({ default: () => null }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("usePlaceChanged", () => {
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

  /** Mounts the hook and returns the callback it gives, with the events the bus saw and the header callback. */
  function mount() {
    const bus = createEditBus();
    const seen: EditEvent[] = [];
    bus.subscribe(() => true, (event) => seen.push(event));
    const onHeaderChanged = vi.fn(() => {
      seen.push({ topic: "edited" });
    });
    let changed: ReturnType<typeof usePlaceChanged> | undefined;
    function Probe() {
      changed = usePlaceChanged(bus, onHeaderChanged);
      return null;
    }
    act(() => root.render(<Probe />));
    return { changed: () => changed!, seen, onHeaderChanged };
  }

  it("tells_the_header_table_and_the_changed_bank_then_reads_the_headers_again", async () => {
    const { changed, seen, onHeaderChanged } = mount();
    await act(async () => changed()(433));
    expect(seen).toEqual([{ topic: "headerTable" }, { topic: "textData", archive: 433 }, { topic: "edited" }]);
    expect(onHeaderChanged).toHaveBeenCalledTimes(1);
  });

  it("leaves_the_text_panel_alone_while_the_bank_is_not_known", async () => {
    const { changed, seen, onHeaderChanged } = mount();
    await act(async () => changed()(null));
    expect(seen.map((e) => e.topic)).toEqual(["headerTable", "edited"]);
    expect(onHeaderChanged).toHaveBeenCalledTimes(1);
  });
});
