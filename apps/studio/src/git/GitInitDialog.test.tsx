// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { gitInit } from "../api";
import type { InitPhase, InitReport } from "../api";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { pressKey } from "../ui/primitives/testUtils";
import { GitInitDialog } from "./GitInitDialog";
import { notInstalledText } from "./gitWords";
import type { GitInitDialogProps } from "./GitInitDialog";
import { initReportOf, noRepo } from "./gitTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  gitInit: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const DESCRIPTION =
  "Legt in diesem Projektordner ein lokales Git-Repository an: .gitignore und .gitattributes, dann einen ersten Commit „Ausgangsstand“. Gen4 Studio richtet kein Remote ein und lädt nichts hoch.";
const LEGAL = "Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).";
const DSPRE_IGNORED = "Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt";
const TEXT_IGNORED = "Ignoriert: ROMs, Patches, build/, Spielstände und die Spieldateien (sie entstehen aus der Basis-ROM)";

describe("GitInitDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onDone = vi.fn();

  const show = (over: Partial<GitInitDialogProps> = {}) =>
    act(async () => root.render(<GitInitDialog open status={noRepo({ templates: [] })} unsaved={false} onClose={onClose} onDone={onDone} {...over} />));
  const dialog = () => document.querySelector<HTMLElement>("[data-git-init-dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.textContent !== "");
  const labels = () => buttons().map((b) => b.textContent);
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const rows = () => [...(dialog()?.querySelectorAll<HTMLElement>("[data-check]") ?? [])].map((r) => `${r.dataset.state}: ${r.textContent}`);
  const progress = () => dialog()?.querySelector("[data-git-init-progress]")?.textContent ?? null;
  const notices = () => [...(dialog()?.querySelectorAll<HTMLElement>("[role=status],[role=alert]") ?? [])].filter((n) => !n.hasAttribute("data-git-init-progress")).map((n) => n.textContent);
  /** A run of „Git einrichten“ that reports and answers when the test says so. */
  function running() {
    let report: (phase: InitPhase) => void = () => {};
    let finish: (r: InitReport) => void = () => {};
    vi.mocked(gitInit).mockImplementationOnce((_opts, onProgress) => {
      report = (phase) => onProgress({ phase });
      return new Promise((resolve) => (finish = resolve));
    });
    return { report: (phase: InitPhase) => act(async () => report(phase)), finish: (r: InitReport) => act(async () => finish(r)) };
  }

  beforeEach(() => {
    onClose.mockReset();
    onDone.mockReset();
    resetToasts();
    vi.mocked(gitInit).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
  });

  it("checklist_then_progress_in_order_then_toast", async () => {
    const run = running();
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Git einrichten?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(DESCRIPTION);
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Git gefunden: 2.47.1", `info: ${DSPRE_IGNORED}`]);
    expect(labels()).toEqual(["Abbrechen", "Einrichten"]);
    expect(button("Einrichten")?.dataset.kind).toBe("primary");
    expect(button("Abbrechen")?.dataset.kind).not.toBe("primary");

    await act(async () => button("Einrichten")!.click());
    expect(gitInit).toHaveBeenCalledExactlyOnceWith({}, expect.any(Function));
    // While it runs there is no button at all, and Escape does not close it.
    expect(labels()).toEqual([]);
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
    expect(dialog()?.querySelector("[data-git-init-progress]")?.getAttribute("role")).toBe("status");
    const seen: (string | null)[] = [progress()];
    for (const phase of ["unpack", "templates", "init", "add", "commit"] as InitPhase[]) {
      await run.report(phase);
      seen.push(progress());
    }
    expect(seen).toEqual([
      "Archive auspacken …",
      "Archive auspacken …",
      "Vorlagen schreiben …",
      "Repository anlegen …",
      "Dateien aufnehmen … (bei DSPRE-Ordnern rund 10 s)",
      "Erster Commit …",
    ]);

    const report = initReportOf();
    await run.finish(report);
    expect(onDone).toHaveBeenCalledWith(report);
    expect(onClose).toHaveBeenCalled();
    expect(listToasts().map((t) => `${t.kind}: ${t.text}`)).toEqual(["info: Git eingerichtet: 18.449 Dateien im ersten Commit „Ausgangsstand“."]);

    // Without a name in Git the commit is Gen4 Studio's, and the message says why.
    resetToasts();
    vi.mocked(gitInit).mockResolvedValueOnce(initReportOf({ files: 6304, studio_identity: true }));
    await show({ status: noRepo({ layout: "text_project", templates: [] }) });
    expect(rows()[2]).toBe(`info: ${TEXT_IGNORED}`);
    await act(async () => button("Einrichten")!.click());
    expect(listToasts().map((t) => t.text)).toEqual([
      "Git eingerichtet: 6.304 Dateien im ersten Commit „Ausgangsstand“. Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.",
    ]);
  });

  it("unsaved_locks_set_up", async () => {
    await show({ unsaved: true });
    expect(rows()[0]).toBe("error: Erst speichern");
    expect(button("Einrichten")?.disabled).toBe(true);
    await act(async () => button("Einrichten")!.click());
    expect(gitInit).not.toHaveBeenCalled();

    // A refusal of the backend shows its sentence, and the dialog can try again.
    await show();
    vi.mocked(gitInit).mockRejectedValueOnce({ code: "unsaved", message: "Erst speichern: der erste Commit nimmt den gespeicherten Stand.", asset: null });
    await act(async () => button("Einrichten")!.click());
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("Erst speichern: der erste Commit nimmt den gespeicherten Stand.");
    expect(labels()).toEqual(["Abbrechen", "Einrichten"]);
    expect(button("Einrichten")?.disabled).toBe(false);
    expect(onDone).not.toHaveBeenCalled();
  });

  it("without_git_offers_templates_only", async () => {
    // Files of the user's own: the block of Gen4 Studio comes on top.
    await show({ status: noRepo({ cli: null, templates: [".gitignore", ".gitattributes"] }) });
    expect(rows()).toEqual([
      "ok: Alles gespeichert",
      "warning: Git ist nicht installiert: nur die Vorlagen werden geschrieben",
      `info: ${DSPRE_IGNORED}`,
      "info: .gitignore vorhanden: der Block von Gen4 Studio kommt dazu",
      "info: .gitattributes vorhanden: der Block von Gen4 Studio kommt dazu",
    ]);
    expect(labels()).toEqual(["Abbrechen", "Vorlagen schreiben"]);
    expect(button("Vorlagen schreiben")?.dataset.kind).toBe("primary");
    const report = initReportOf({ committed: false, reason: notInstalledText(), head: null, files: 0 });
    vi.mocked(gitInit).mockResolvedValueOnce(report);
    await act(async () => button("Vorlagen schreiben")!.click());
    expect(onDone).toHaveBeenCalledWith(report);
    expect(onClose).toHaveBeenCalled();
    expect(listToasts().map((t) => t.text)).toEqual(["Vorlagen geschrieben. Git ist nicht installiert; das Repository legst du an, sobald Git da ist."]);
  });

  it("a_failed_commit_shows_the_reason_and_keeps_the_dialog", async () => {
    const reason = "Der erste Commit ist gescheitert: error: gpg failed to sign the data. Repository und Vorlagen sind angelegt; committe mit deinem Git-Werkzeug.";
    const report = initReportOf({ committed: false, reason, head: null, files: 0 });
    vi.mocked(gitInit).mockResolvedValueOnce(report);
    await show();
    await act(async () => button("Einrichten")!.click());
    // The repository and the templates are there: the app reads the status again, but the dialog stays with Git's reason.
    expect(onDone).toHaveBeenCalledWith(report);
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
    const warning = [...(dialog()?.querySelectorAll<HTMLElement>("[role=status]") ?? [])].find((n) => n.textContent === reason);
    expect(warning).toBeDefined();
    expect(labels()).toEqual(["Schließen"]);
    expect(listToasts()).toEqual([]);
    expect(progress()).toBeNull();
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("the_legal_note_is_always_shown", async () => {
    await show();
    expect(notices()).toContain(LEGAL);
    await show({ unsaved: true, status: noRepo({ cli: null }) });
    expect(notices()).toContain(LEGAL);
    // Also while it runs.
    running();
    await show();
    await act(async () => button("Einrichten")!.click());
    expect(notices()).toContain(LEGAL);
    // A warning.
    const legal = [...(dialog()?.querySelectorAll<HTMLElement>("[role=status]") ?? [])].find((n) => n.textContent === LEGAL);
    expect(legal?.querySelector("svg")).not.toBeNull();
  });
});
