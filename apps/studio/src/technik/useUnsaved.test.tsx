// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { unsavedChanges } from "../api";
import type { ChangedEntry } from "../api";
import { entry } from "./changesTesting";
import { useUnsaved } from "./useUnsaved";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  unsavedChanges: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Options = Parameters<typeof useUnsaved>[0];

describe("useUnsaved", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: ReturnType<typeof useUnsaved>;

  function Probe(options: Partial<Options>) {
    shown = useUnsaved({ project: "/p", shown: true, revision: 0, ...options });
    return null;
  }
  const show = (options: Partial<Options> = {}) => act(async () => root.render(<Probe {...options} />));

  beforeEach(() => {
    vi.mocked(unsavedChanges).mockReset().mockResolvedValue([entry({ group: "Trainer", label: "Trainer 0001" })]);
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => act(() => root.unmount()));

  it("reads_when_shown_and_after_a_revision", async () => {
    // Not shown, or no project: nothing read.
    await show({ shown: false });
    await show({ project: null });
    expect(unsavedChanges).not.toHaveBeenCalled();
    expect(shown).toEqual({ entries: null, error: null });
    // Shown: read.
    await show();
    expect(unsavedChanges).toHaveBeenCalledOnce();
    expect(shown.entries?.map((e) => e.label)).toEqual(["Trainer 0001"]);
    // An edit (the revision moves): read again.
    vi.mocked(unsavedChanges).mockResolvedValueOnce([]);
    await show({ revision: 1 });
    expect(unsavedChanges).toHaveBeenCalledTimes(2);
    expect(shown.entries).toEqual([]);
    // Moved while hidden: read when shown again, not before.
    await show({ revision: 2, shown: false });
    expect(unsavedChanges).toHaveBeenCalledTimes(2);
    await show({ revision: 2 });
    expect(unsavedChanges).toHaveBeenCalledTimes(3);
    // A failure: its sentence.
    vi.mocked(unsavedChanges).mockRejectedValueOnce({ code: "error", message: "Kein Projekt geöffnet.", asset: null });
    await show({ revision: 3 });
    expect(shown.error).toBe("Kein Projekt geöffnet.");
    // Another project forgets the list before; an answer for the old one that comes late is dropped.
    let late: (e: ChangedEntry[]) => void = () => {};
    vi.mocked(unsavedChanges).mockImplementationOnce(() => new Promise((resolve) => (late = resolve)));
    await show({ revision: 4 });
    await show({ project: "/q", revision: 4 });
    expect(shown.entries?.map((e) => e.label)).toEqual(["Trainer 0001"]);
    await act(async () => late([entry({ label: "Karte 0009" })]));
    expect(shown.entries?.map((e) => e.label)).toEqual(["Trainer 0001"]);
  });
});
