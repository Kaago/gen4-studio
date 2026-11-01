// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogOf } from "../../api";
import type { Dialog } from "../../api";
import type { DialogTarget } from "./dialogModel";
import { useDialog } from "./useDialog";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), dialogOf: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const said = (text: string): Dialog => ({ source: { kind: "local", file: 1, slot: 1, line: 1 }, messages: [{ archive: 9, index: 0, text, command: "Message", line: 2, col: 13, branch: null, shared_with: 0 }], followed: [] });
const figure = (index: number): DialogTarget => ({ header: 3, eventFile: 8, kind: "object", index, title: `Objekt ${index}` });

describe("useDialog", () => {
  let container: HTMLElement;
  let root: Root;
  let seen: ReturnType<typeof useDialog> | null = null;
  function Probe({ target, revision }: { target: DialogTarget | null; revision: number }) {
    seen = useDialog(target, revision);
    return null;
  }
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    seen = null;
    vi.mocked(dialogOf).mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("reads_on_target_and_revision_and_drops_an_older_answer", async () => {
    const answers: ((d: Dialog) => void)[] = [];
    vi.mocked(dialogOf).mockImplementation(() => new Promise<Dialog>((resolve) => answers.push(resolve)));
    await act(async () => root.render(<Probe target={figure(2)} revision={0} />));
    expect(dialogOf).toHaveBeenLastCalledWith(3, 8, "object", 2);
    expect(seen!.dialog).toBeNull();
    // Another revision reads again; the first answer comes last and is dropped.
    await act(async () => root.render(<Probe target={figure(2)} revision={1} />));
    expect(dialogOf).toHaveBeenCalledTimes(2);
    await act(async () => answers[1]!(said("neu")));
    await act(async () => answers[0]!(said("alt")));
    expect(seen!.dialog?.messages[0]?.text).toBe("neu");
    // `reload` reads again; another figure never shows the old one's dialog.
    await act(async () => seen!.reload());
    expect(dialogOf).toHaveBeenCalledTimes(3);
    await act(async () => root.render(<Probe target={figure(5)} revision={1} />));
    expect(dialogOf).toHaveBeenLastCalledWith(3, 8, "object", 5);
    expect(seen!.dialog).toBeNull();
    // A refused read is the error.
    await act(async () => answers[3]!(said("fünf")));
    expect(seen!.dialog?.messages[0]?.text).toBe("fünf");
    vi.mocked(dialogOf).mockRejectedValueOnce({ code: "error", message: "kaputt", asset: null });
    await act(async () => root.render(<Probe target={figure(6)} revision={1} />));
    expect(seen!.error).toBe("kaputt");
    // No target: nothing is read.
    const calls = vi.mocked(dialogOf).mock.calls.length;
    await act(async () => root.render(<Probe target={null} revision={2} />));
    expect(dialogOf).toHaveBeenCalledTimes(calls);
    expect(seen!.dialog).toBeNull();
  });
});
