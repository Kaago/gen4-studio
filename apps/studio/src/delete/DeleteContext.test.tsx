// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { restoreEntity } from "../api";
import { DeleteProvider, useDelete } from "./DeleteContext";
import type { DeleteApi } from "./DeleteContext";
import { deleteResult } from "./deleteTesting";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), restoreEntity: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DeleteProvider", () => {
  let container: HTMLElement;
  let root: Root;
  let api: DeleteApi | null = null;
  const onDeleted = vi.fn();
  const onRestored = vi.fn();
  const onRestoreFailed = vi.fn();
  function Probe() {
    api = useDelete();
    return null;
  }
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onRestored.mockReset();
    onRestoreFailed.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("restore_takes_the_tombstone_back_and_names_the_entity", async () => {
    await act(async () =>
      root.render(
        <DeleteProvider stateOf={() => null} goTo={() => {}} onDeleted={onDeleted} onRestored={onRestored} onRestoreFailed={onRestoreFailed}>
          <Probe />
        </DeleteProvider>,
      ),
    );
    const result = deleteResult({ change: "c2", changed: [{ kind: "Trainer", index: 930 }] });
    vi.mocked(restoreEntity).mockResolvedValueOnce(result);
    await act(async () => api!.restore({ kind: "trainer", index: 930 }));
    expect(restoreEntity).toHaveBeenCalledWith({ kind: "trainer", index: 930 });
    expect(onRestored).toHaveBeenCalledExactlyOnceWith(result, "Trainer 0930", { kind: "trainer", index: 930 });
    // A refusal („Trainer 0930 ist nicht gelöscht.“) goes to the app.
    const refusal = { code: "error", message: "Trainer 0930 ist nicht gelöscht." };
    vi.mocked(restoreEntity).mockRejectedValueOnce(refusal);
    await act(async () => api!.restore({ kind: "trainer", index: 930 }));
    expect(onRestoreFailed).toHaveBeenCalledExactlyOnceWith(refusal);
    expect(onRestored).toHaveBeenCalledOnce();
    expect(onDeleted).not.toHaveBeenCalled();
  });
});
