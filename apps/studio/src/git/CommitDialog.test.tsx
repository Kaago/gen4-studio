// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { gitCommitAll, gitCommitPrepare } from "../api";
import type { CommitPhase, CommitReport, GitStatus } from "../api";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { pressKey } from "../ui/primitives/testUtils";
import { CommitDialog } from "./CommitDialog";
import type { CommitDialogProps } from "./CommitDialog";
import { commitReportOf, gitStatusOf } from "./gitTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  gitCommitAll: vi.fn(),
  gitCommitPrepare: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const DESCRIPTION = "Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.";

describe("CommitDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onDone = vi.fn();
  const onRefresh = vi.fn();

  /** Shows the dialog; unless the test set them, the backend's checks answer the status it is given. */
  const show = (over: Partial<CommitDialogProps> = {}, prepared?: GitStatus) => {
    const status = over.status === undefined ? gitStatusOf({ changes: 3 }) : over.status;
    if (status !== null) vi.mocked(gitCommitPrepare).mockResolvedValue(prepared ?? status);
    return act(async () =>
      root.render(<CommitDialog open status={status} unsaved={false} preset={null} onClose={onClose} onDone={onDone} onRefresh={onRefresh} {...over} />),
    );
  };
  const dialog = () => document.querySelector<HTMLElement>("[data-commit-dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.textContent !== "");
  const labels = () => buttons().map((b) => b.textContent);
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const rows = () => [...(dialog()?.querySelectorAll<HTMLElement>("[data-check]") ?? [])].map((r) => `${r.dataset.state}: ${r.textContent}`);
  const progress = () => dialog()?.querySelector("[data-commit-progress]")?.textContent ?? null;
  const field = (label: string) => dialog()!.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)!;
  /** Types as a user does: focus, the text, leaving the field (which commits it). */
  async function type(input: HTMLInputElement | HTMLTextAreaElement, value: string) {
    const proto = input instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    await act(async () => input.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => input.blur());
  }
  /** A run of „Stand sichern“ that reports and answers when the test says so. */
  function running() {
    let report: (phase: CommitPhase) => void = () => {};
    let finish: (r: CommitReport) => void = () => {};
    vi.mocked(gitCommitAll).mockImplementationOnce((_opts, onProgress) => {
      report = (phase) => onProgress({ phase });
      return new Promise((resolve) => (finish = resolve));
    });
    return { report: (phase: CommitPhase) => act(async () => report(phase)), finish: (r: CommitReport) => act(async () => finish(r)) };
  }

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(2026, 9, 31, 14, 5, 30));
    onClose.mockReset();
    onDone.mockReset();
    onRefresh.mockReset();
    resetToasts();
    vi.mocked(gitCommitAll).mockReset();
    vi.mocked(gitCommitPrepare).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
    vi.useRealTimers();
  });

  it("checklist_fields_then_progress_then_toast", async () => {
    const run = running();
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Stand sichern?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(DESCRIPTION);
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Git gefunden: 2.47.1", "ok: 3 geänderte Dateien seit dem letzten Commit"]);
    // The subject of now, an empty note with its hint.
    expect(field("Betreff").value).toBe("Stand 2026-10-31 14:05");
    expect(field("Notiz").tagName).toBe("TEXTAREA");
    expect(field("Notiz").value).toBe("");
    expect(dialog()?.textContent).toContain("Optional; steht im Commit unter dem Betreff.");
    expect(labels()).toEqual(["Abbrechen", "Sichern"]);
    expect(button("Sichern")?.dataset.kind).toBe("primary");
    expect(button("Abbrechen")?.dataset.kind).not.toBe("primary");
    expect(dialog()?.querySelectorAll("[data-kind=primary]")).toHaveLength(1);

    await type(field("Notiz"), "Route 201 neu");
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).toHaveBeenCalledExactlyOnceWith({ message: "Stand 2026-10-31 14:05", notes: "Route 201 neu" }, expect.any(Function));
    // While it runs there is no button at all, and Escape does not close it.
    expect(labels()).toEqual([]);
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
    expect(dialog()?.querySelector("[data-commit-progress]")?.getAttribute("role")).toBe("status");
    const seen: (string | null)[] = [progress()];
    for (const phase of ["templates", "add", "commit"] as CommitPhase[]) {
      await run.report(phase);
      seen.push(progress());
    }
    expect(seen).toEqual(["Vorlagen prüfen …", "Vorlagen prüfen …", "Dateien aufnehmen …", "Commit …"]);

    const report = commitReportOf();
    await run.finish(report);
    expect(onDone).toHaveBeenCalledExactlyOnceWith(report);
    expect(onClose).toHaveBeenCalledOnce();
    expect(listToasts().map((t) => `${t.kind}: ${t.text}`)).toEqual(["info: Stand gesichert: 9a8b7c6 „Stand 2026-10-31 14:05“ (3 Dateien)."]);
  });

  it("unsaved_or_no_git_or_no_change_locks_sichern", async () => {
    await show({ unsaved: true });
    expect(rows()[0]).toBe("error: Erst speichern");
    expect(button("Sichern")?.disabled).toBe(true);
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).not.toHaveBeenCalled();

    await show({ status: gitStatusOf({ changes: 3, cli: null }) });
    expect(rows()).toEqual(["ok: Alles gespeichert", "error: Git ist nicht installiert", "ok: 3 geänderte Dateien seit dem letzten Commit"]);
    expect(button("Sichern")?.disabled).toBe(true);

    await show({ status: gitStatusOf({ changes: 0 }) });
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Git gefunden: 2.47.1", "warning: Keine Änderung seit dem letzten Commit"]);
    expect(button("Sichern")?.disabled).toBe(true);

    // One change is enough.
    await show({ status: gitStatusOf({ changes: 1 }) });
    expect(rows()[2]).toBe("ok: 1 geänderte Datei seit dem letzten Commit");
    expect(button("Sichern")?.disabled).toBe(false);

    // A refusal of the backend shows its sentence, and the dialog can try again.
    vi.mocked(gitCommitAll).mockRejectedValueOnce({ code: "unsaved", message: "Erst speichern: der Commit nimmt den gespeicherten Stand.", asset: null });
    await act(async () => button("Sichern")!.click());
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("Erst speichern: der Commit nimmt den gespeicherten Stand.");
    expect(labels()).toEqual(["Abbrechen", "Sichern"]);
    expect(button("Sichern")?.disabled).toBe(false);
    // The checks ran again after the refusal, and the app read the status.
    expect(gitCommitPrepare).toHaveBeenCalledTimes(2);
    expect(onRefresh).toHaveBeenCalledTimes(2);
    expect(onDone).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    await act(async () => button("Abbrechen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("the_checks_on_opening_give_the_changes_the_commit_would_take", async () => {
    // A repository of 4.4 after a release: the status counts nothing, the checks refresh the block and count it and the list.
    let answer: (s: GitStatus) => void = () => {};
    vi.mocked(gitCommitPrepare).mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    await act(async () =>
      root.render(<CommitDialog open status={gitStatusOf({ changes: 0 })} unsaved={false} preset={null} onClose={onClose} onDone={onDone} onRefresh={onRefresh} />),
    );
    expect(gitCommitPrepare).toHaveBeenCalledOnce();
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Git gefunden: 2.47.1", "info: Änderungen werden geprüft …"]);
    expect(button("Sichern")?.disabled).toBe(true);
    expect(onRefresh).not.toHaveBeenCalled();
    await act(async () => answer(gitStatusOf({ changes: 2 })));
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Git gefunden: 2.47.1", "ok: 2 geänderte Dateien seit dem letzten Commit"]);
    expect(button("Sichern")?.disabled).toBe(false);
    // The app reads the status again, so the status line follows.
    expect(onRefresh).toHaveBeenCalledOnce();
    vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf({ files: 2 }));
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).toHaveBeenCalledOnce();
  });

  it("the_checks_wait_for_saving_and_git_and_show_their_refusal", async () => {
    // Unsaved or without git a row says why; the checks do not run.
    await show({ unsaved: true });
    await show({ unsaved: true, status: gitStatusOf({ changes: 2, cli: null }) });
    expect(gitCommitPrepare).not.toHaveBeenCalled();
    act(() => root.unmount());
    root = createRoot(container);
    // A refusal of the checks shows its sentence and locks nothing more than the status does.
    vi.mocked(gitCommitPrepare).mockRejectedValueOnce({ code: "enclosing_repo", message: "Das Projekt liegt in einem größeren Repository.", asset: null });
    await act(async () =>
      root.render(<CommitDialog open status={gitStatusOf({ changes: 0 })} unsaved={false} preset={null} onClose={onClose} onDone={onDone} onRefresh={onRefresh} />),
    );
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("Das Projekt liegt in einem größeren Repository.");
    expect(rows()[2]).toBe("warning: Keine Änderung seit dem letzten Commit");
    expect(button("Sichern")?.disabled).toBe(true);
  });

  it("a_preset_fills_subject_and_notes", async () => {
    vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf({ subject: "Veröffentlicht v0.1.0", files: 1, studio_identity: true }));
    await show({ preset: { message: "Veröffentlicht v0.1.0", notes: "Neue Route 201.\nMehr Trainer." } });
    expect(field("Betreff").value).toBe("Veröffentlicht v0.1.0");
    expect(field("Notiz").value).toBe("Neue Route 201.\nMehr Trainer.");
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).toHaveBeenCalledExactlyOnceWith({ message: "Veröffentlicht v0.1.0", notes: "Neue Route 201.\nMehr Trainer." }, expect.any(Function));
    expect(listToasts().map((t) => t.text)).toEqual([
      "Stand gesichert: 9a8b7c6 „Veröffentlicht v0.1.0“ (1 Datei). Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.",
    ]);
  });

  it("an_empty_subject_locks_sichern", async () => {
    await show();
    await type(field("Betreff"), "   ");
    expect(button("Sichern")?.disabled).toBe(true);
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).not.toHaveBeenCalled();
    // A subject of the user's own goes in trimmed.
    await type(field("Betreff"), "  Route 201 fertig ");
    expect(button("Sichern")?.disabled).toBe(false);
    vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf({ subject: "Route 201 fertig" }));
    await act(async () => button("Sichern")!.click());
    expect(gitCommitAll).toHaveBeenCalledExactlyOnceWith({ message: "Route 201 fertig", notes: "" }, expect.any(Function));
  });

  it("a_failed_commit_shows_the_reason_and_keeps_the_dialog", async () => {
    const reason = "Der Commit ist gescheitert: error: gpg failed to sign the data. Das Repository bleibt, wie es war.";
    const report = commitReportOf({ committed: false, reason, head: null, files: 0 });
    vi.mocked(gitCommitAll).mockResolvedValueOnce(report);
    await show();
    await act(async () => button("Sichern")!.click());
    // The app reads the status again, but the dialog stays with Git's reason and only „Schließen“.
    expect(onDone).toHaveBeenCalledWith(report);
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
    const warning = [...(dialog()?.querySelectorAll<HTMLElement>("[role=status]") ?? [])].find((n) => n.textContent === reason);
    expect(warning).toBeDefined();
    expect(warning?.querySelector("svg")).not.toBeNull();
    expect(labels()).toEqual(["Schließen"]);
    expect(listToasts()).toEqual([]);
    expect(progress()).toBeNull();
    // The checks and the fields are gone: nothing to try again here.
    expect(rows()).toEqual([]);
    expect(dialog()?.querySelector('[aria-label="Betreff"]')).toBeNull();
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("runs_while_nothing_else_may_write", async () => {
    let held = 0;
    const exclusive = <T,>(work: () => Promise<T>) => {
      held += 1;
      return work();
    };
    vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf());
    await show({ exclusive });
    await act(async () => button("Sichern")!.click());
    expect(held).toBe(1);
    expect(gitCommitAll).toHaveBeenCalledOnce();
  });
});
