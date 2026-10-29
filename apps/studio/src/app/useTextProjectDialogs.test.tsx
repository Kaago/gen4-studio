// @vitest-environment jsdom
import { act } from "react";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { adoptTextProject } from "../api";
import type { AdoptReport, TextProjectStatus } from "../api";
import { AdoptDialog } from "../textproject/AdoptDialog";
import { useTextProjectDialogs } from "./useTextProjectDialogs";
import type { ExternalSyncApi } from "./useExternalSync";
import type { ProjectSession } from "./useProjectSession";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  adoptTextProject: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const report = { export: { files: 6574 }, ms: 170000 } as AdoptReport;
const dspre: TextProjectStatus = { state: "dspre", adoptable: true, reason: null };
const current: TextProjectStatus = { state: "current", schema: 1, last_sync_at: null };

/** The hook and the dialog as App wires them: the status is the test's to flip, the session read after a set-up answers when the test says so. */
describe("useTextProjectDialogs: the dialog of the set-up", () => {
  let container: HTMLElement;
  let root: Root;
  let setStatus: (s: TextProjectStatus) => void;
  let openDialog: () => void;
  let dialogState: string | null;
  let finishReload: () => void;
  const setMessage = vi.fn();
  const info = { root: "/p" };

  const dialog = () => document.querySelector<HTMLElement>("[data-adopt-dialog]");
  const progress = () => document.querySelector("[data-adopt-progress]")?.textContent ?? null;
  const click = (text: string) =>
    act(async () => [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === text)?.click());

  function Harness() {
    const [status, setStatusState] = useState<TextProjectStatus>(dspre);
    setStatus = setStatusState;
    const session = {
      info,
      busy: false,
      setMessage,
      fail: vi.fn(),
      isReopened: () => true,
      // The reload of the set-up's session: the folder reads as a text project now, the call answers later.
      reloadSession: () => {
        setStatusState(current);
        return new Promise<void>((resolve) => (finishReload = resolve));
      },
    } as unknown as ProjectSession;
    const text = useTextProjectDialogs({
      session,
      projectText: status,
      sync: {} as ExternalSyncApi,
      onResolved: () => {},
      reloads: { current: Promise.resolve() },
      resetProject: () => {},
      bumpChanged: () => {},
      idsRevision: 0,
      bumpIds: () => {},
      canOpen: () => true,
      showPublish: () => {},
      onClosed: () => {},
    });
    openDialog = () => text.textProject?.onSetUp?.();
    dialogState = text.dialog;
    return (
      text.dialog === "adopt" && (
        <AdoptDialog
          unsaved={0}
          baseLabel="Platin DE"
          baseReason={null}
          scriptsAdopted={false}
          onClose={text.closeDialog}
          onAdopted={text.onAdopted}
        />
      )
    );
  }

  beforeEach(() => {
    setMessage.mockReset();
    vi.mocked(adoptTextProject).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("own_set_up_keeps_the_dialog_with_its_status_until_onAdopted_has_resolved", async () => {
    let finish: (r: AdoptReport) => void = () => {};
    vi.mocked(adoptTextProject).mockImplementation(() => new Promise((resolve) => (finish = resolve)));
    await act(async () => root.render(<Harness />));
    await act(async () => openDialog());
    expect(dialog()).not.toBeNull();
    await click("Einrichten");
    expect(progress()).toBe("Wird eingerichtet …");

    // The backend is done; the reload reads the folder as a text project, but `onAdopted` has not resolved yet.
    await act(async () => finish(report));
    expect(dialogState).toBe("adopt");
    expect(dialog()).not.toBeNull();
    expect(progress()).toBe("Wird eingerichtet …");
    expect(setMessage).not.toHaveBeenCalled();

    await act(async () => finishReload());
    expect(setMessage).toHaveBeenCalledWith({ kind: "info", text: expect.stringContaining("Textprojekt eingerichtet") });
    expect(dialog()).toBeNull();
    expect(dialogState).toBeNull();
  });

  it("a_dialog_that_does_not_run_closes_when_an_agent_made_the_folder_a_text_project", async () => {
    await act(async () => root.render(<Harness />));
    await act(async () => openDialog());
    expect(dialog()).not.toBeNull();
    await act(async () => setStatus(current));
    expect(dialogState).toBeNull();
    expect(dialog()).toBeNull();
    // A stale „Einrichten…“ opens nothing.
    await act(async () => openDialog());
    expect(dialog()).toBeNull();
  });
});
