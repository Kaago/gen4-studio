// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { deleteEntity, planDelete } from "../api";
import type { DeletePlan, DeleteResult, StoryMention } from "../api";
import deDelete from "../i18n/locales/de/delete.json";
import enDelete from "../i18n/locales/en/delete.json";
import { kernelSentence } from "../testing/kernelCatalog";
import { pressKey } from "../ui/primitives/testUtils";
import { DeleteDialog } from "./DeleteDialog";
import type { DeleteDialogProps } from "./DeleteDialog";
import { TRAINER_901, deletePlan, deleteResult, figureRef } from "./deleteTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  planDelete: vi.fn(),
  deleteEntity: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ORIGINAL_WARNING = "Original aus der Basis-ROM: der Eintrag bleibt im ROM; „Auf Original zurücksetzen“ holt ihn zurück.";
const KEEPS = "Inhalt und Member bleiben; ⌘Z holt den Eintrag zurück.";
const FREE = "Nummer freigeben (nicht rückgängig)";
const MENTIONS_HINT = "Geschichten sperren nicht; nach dem Speichern meldet die Prüfung des Textprojekts jeden Verweis, der nichts mehr nennt, als offen.";

/** A promise and the function that settles it, for a step the test lets go on. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  let reject: (e: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("DeleteDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onDeleted = vi.fn();
  const goTo = vi.fn();

  const show = (over: Partial<DeleteDialogProps> = {}) =>
    act(async () => root.render(<DeleteDialog target={TRAINER_901} onClose={onClose} onDeleted={onDeleted} goTo={goTo} {...over} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.textContent !== "");
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const notices = (role: "status" | "alert") => [...(dialog()?.querySelectorAll<HTMLElement>(`[role=${role}]`) ?? [])].map((n) => n.textContent);
  const box = (label: string) => [...(dialog()?.querySelectorAll<HTMLElement>("[role=checkbox]") ?? [])].find((b) => b.closest("label")?.textContent === label);
  const references = () => [...(dialog()?.querySelectorAll<HTMLElement>("[data-reference]") ?? [])];

  beforeEach(() => {
    onClose.mockReset();
    onDeleted.mockReset();
    goTo.mockReset();
    vi.mocked(planDelete).mockReset().mockResolvedValue(deletePlan());
    vi.mocked(deleteEntity).mockReset().mockResolvedValue(deleteResult());
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows_the_references_and_locks_delete", async () => {
    const planned = deferred<DeletePlan>();
    vi.mocked(planDelete).mockReturnValueOnce(planned.promise);
    await show();
    expect(planDelete).toHaveBeenCalledWith(TRAINER_901);
    expect(dialog()?.hasAttribute("data-delete-dialog")).toBe(true);
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Trainer 0901 löschen");
    expect(notices("status")).toEqual(["Prüfe Verweise …"]);
    expect(button("Löschen")?.disabled).toBe(true);

    await act(async () => planned.resolve(deletePlan({ references: [figureRef(2), figureRef(5)], can_free: { count: 1, files: ["data/trainers/0901.json"] } })));
    expect(notices("alert")).toEqual(["2 Verweise: zuerst umhängen."]);
    // One row per reference with its label and source; a click leads there and closes the dialog.
    expect(references().map((r) => r.textContent)).toEqual(["Figur 2 in Eventdatei 0008Eventdatei 0008", "Figur 5 in Eventdatei 0008Eventdatei 0008"]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Abbrechen", "Löschen"]);
    expect(button("Löschen")?.disabled).toBe(true);
    // Nothing to free while references lock it.
    expect(box(FREE)).toBeUndefined();
    await act(async () => references()[1]!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "event", eventFile: 8, eventKind: "object", index: 5 });
    expect(onClose).toHaveBeenCalledOnce();
    expect(deleteEntity).not.toHaveBeenCalled();
  });

  it("warns_for_an_original_and_deletes", async () => {
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ target: { kind: "trainer", index: 12 }, label: "Trainer 0012", original: true, cannot_free: "Original-Einträge lassen sich nicht freigeben." }));
    await show({ target: { kind: "trainer", index: 12 } });
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Trainer 0012 löschen");
    expect(notices("status")).toEqual([ORIGINAL_WARNING, KEEPS]);
    // The number cannot be freed: said quietly, no checkbox.
    expect(box(FREE)).toBeUndefined();
    expect(dialog()?.querySelector("[data-cannot-free]")?.textContent).toBe("Original-Einträge lassen sich nicht freigeben.");
    expect(buttons().map((b) => b.textContent)).toEqual(["Abbrechen", "Löschen"]);
    expect(button("Löschen")?.dataset.kind).toBe("primary");
    expect(buttons().filter((b) => b.dataset.kind === "primary")).toHaveLength(1);

    const running = deferred<DeleteResult>();
    vi.mocked(deleteEntity).mockReturnValueOnce(running.promise);
    await act(async () => button("Löschen")!.click());
    expect(deleteEntity).toHaveBeenCalledWith({ kind: "trainer", index: 12 }, false);
    expect(notices("status")).toContain("Lösche …");
    expect(button("Löschen")?.disabled).toBe(true);
    expect(button("Abbrechen")?.disabled).toBe(true);
    // Escape does not leave a delete that runs.
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();

    const result = deleteResult({ changed: [{ kind: "Trainer", index: 12 }] });
    await act(async () => running.resolve(result));
    expect(onDeleted).toHaveBeenCalledWith(result, false, "Trainer 0012");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("offers_free_number_only_when_the_plan_allows", async () => {
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ can_free: { count: 1, files: ["data/trainers/0901.json", "unpacked/trainerProperties/0901"] } }));
    await show();
    expect(notices("status")).toEqual([KEEPS]);
    const free = box(FREE);
    expect(free?.getAttribute("aria-checked")).toBe("false");
    // The badge „Nicht rückgängig“ beside it, and what goes.
    const option = dialog()?.querySelector<HTMLElement>("[data-free-number]");
    expect(option?.textContent).toContain("Nicht rückgängig");
    expect(option?.textContent).toContain("Entfernt 2 Dateien: data/trainers/0901.json, unpacked/trainerProperties/0901");
    expect(button("Löschen")?.disabled).toBe(false);
    await act(async () => free!.click());
    expect(box(FREE)?.getAttribute("aria-checked")).toBe("true");
    expect(buttons().map((b) => b.textContent)).toEqual(["Abbrechen", "Löschen und Nummer freigeben"]);
    // Gate 3d.4 B11: ticked, nothing is kept and ⌘Z brings nothing back: only what goes is said.
    expect(notices("status")).toEqual([]);
    expect(dialog()?.textContent).not.toContain("⌘Z");
    expect(option?.textContent).toContain("Entfernt 2 Dateien");
    await act(async () => box(FREE)!.click());
    expect(notices("status")).toEqual([KEEPS]);

    // A later entry lives: the kernel's sentence, quietly, and no checkbox.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ cannot_free: "Nummer 901 bleibt belegt, weil 2 spätere Einträge leben." }));
    await show();
    expect(box(FREE)).toBeUndefined();
    expect(dialog()?.querySelector("[data-free-number]")).toBeNull();
    expect(dialog()?.querySelector("[data-cannot-free]")?.textContent).toBe("Nummer 901 bleibt belegt, weil 2 spätere Einträge leben.");
  });

  it("a_refused_plan_shows_the_reason_only", async () => {
    const refusal = "Original-Header lassen sich nicht löschen: der Spielcode verweist unsichtbar darauf (ARM9, Overlays).";
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ target: { kind: "header", id: 0 }, label: "Header 0000", refusal, original: true, cannot_free: "Original-Einträge lassen sich nicht freigeben." }));
    await show({ target: { kind: "header", id: 0 } });
    expect(notices("alert")).toEqual([refusal]);
    expect(notices("status")).toEqual([]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Schließen"]);
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalledOnce();

    // A plan that fails (no base ROM) the same: its reason and „Schließen“.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockRejectedValue({ code: "base_missing", message: "Diesem Projekt ist keine Basis-ROM zugeordnet.", asset: null, problems: [] });
    await show();
    expect(notices("alert")).toEqual(["Diesem Projekt ist keine Basis-ROM zugeordnet."]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Schließen"]);
  });

  it("the_dspre_layout_names_the_clearing_path", async () => {
    const refusal = "Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).";
    const hint = "Leeren geht als gewöhnliche Bearbeitung: Eventdatei leeren im Inspector, Begegnungen auf 0.";
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ refusal }));
    await show();
    expect(notices("alert")).toEqual([refusal]);
    expect(notices("status")).toEqual([hint]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Schließen"]);

    // A new entry at the end can still be freed there: the reason as a warning, the hint beside it.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ refusal, can_free: { count: 1, files: ["unpacked/trainerProperties/0901"] } }));
    await show();
    expect(notices("status")).toEqual([refusal, hint]);
    expect(box(FREE)).toBeDefined();

    // Any other refusal goes without it.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ refusal: "Original-Header lassen sich nicht löschen." }));
    await show();
    expect(notices("status")).toEqual([]);
  });

  it("free_number_calls_delete_with_the_flag", async () => {
    // A script file has no tombstone: only freeing it is possible, so „Löschen“ waits for the checkbox.
    const refusal = "Scriptdateien und Textarchive haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen.";
    const target = { kind: "script", file: 1200 } as const;
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ target, label: "Scriptdatei 1200", refusal, can_free: { count: 1, files: ["scripts/1200.rotom"] } }));
    await show({ target });
    expect(notices("status")).toEqual([refusal]);
    expect(button("Löschen")?.disabled).toBe(true);
    await act(async () => box(FREE)!.click());
    expect(button("Löschen und Nummer freigeben")?.disabled).toBe(false);
    const freed = deleteResult({ change: null, changed: [{ kind: "Script", index: 1200 }], freed: { count: 1, files: ["scripts/1200.rotom"] } });
    vi.mocked(deleteEntity).mockResolvedValueOnce(freed);
    await act(async () => button("Löschen und Nummer freigeben")!.click());
    expect(deleteEntity).toHaveBeenCalledWith(target, true);
    expect(onDeleted).toHaveBeenCalledWith(freed, true, "Scriptdatei 1200");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("mentions_are_listed_without_locking", async () => {
    const mention = (over: Partial<StoryMention> = {}): StoryMention => ({
      story_id: "STORY_TESTFADEN",
      story_name: "Testfaden",
      number: 3,
      step_key: "s1",
      step_name: "Kampf gegen Kai",
      file: "data/stories/0003.json",
      ...over,
    });
    const mentions = () => dialog()?.querySelector<HTMLElement>("[data-delete-mentions]") ?? null;
    const chips = () => [...(mentions()?.querySelectorAll<HTMLButtonElement>("[data-delete-mention]") ?? [])];
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ mentions: [mention(), mention({ step_key: null, step_name: null })] }));
    await show();
    // „Erwähnt in Geschichten“: one chip per story and step, and that they do not lock.
    expect(mentions()?.querySelector("h3")?.textContent).toBe("Erwähnt in Geschichten");
    expect(chips().map((c) => c.textContent)).toEqual(["Testfaden › Kampf gegen Kai", "Testfaden"]);
    expect(chips().map((c) => c.dataset.deleteMention)).toEqual(["STORY_TESTFADEN", "STORY_TESTFADEN"]);
    expect(mentions()?.textContent).toContain(MENTIONS_HINT);
    expect(notices("alert")).toEqual([]);
    expect(notices("status")).toEqual([KEEPS]);
    expect(button("Löschen")?.disabled).toBe(false);
    // A chip opens the story and closes the dialog.
    await act(async () => chips()[0]!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "story", number: 3 });
    expect(onClose).toHaveBeenCalledOnce();

    // „Löschen“ goes with the stories beside it.
    act(() => root.unmount());
    root = createRoot(container);
    const result = deleteResult({ mentions: [mention()] });
    vi.mocked(deleteEntity).mockResolvedValueOnce(result);
    await show();
    await act(async () => button("Löschen")!.click());
    expect(deleteEntity).toHaveBeenCalledWith(TRAINER_901, false);
    expect(onDeleted).toHaveBeenCalledWith(result, false, "Trainer 0901");

    // A hard reference still locks; the stories stand beside it.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockResolvedValue(deletePlan({ references: [figureRef(2)], mentions: [mention()] }));
    await show();
    expect(notices("alert")).toEqual(["1 Verweis: zuerst umhängen."]);
    expect(chips().map((c) => c.textContent)).toEqual(["Testfaden › Kampf gegen Kai"]);
    expect(button("Löschen")?.disabled).toBe(true);

    // No story: no section.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planDelete).mockResolvedValue(deletePlan());
    await show();
    expect(mentions()).toBeNull();
  });

  it("a_failed_delete_stays_open_with_the_reason_and_the_references_it_names", async () => {
    vi.mocked(deleteEntity).mockRejectedValueOnce({ code: "error", message: "Das Projekt wurde gewechselt; nichts gelöscht.", asset: null, problems: [] });
    await show();
    await act(async () => button("Löschen")!.click());
    expect(notices("alert")).toEqual(["Das Projekt wurde gewechselt; nichts gelöscht."]);
    expect(onDeleted).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(button("Löschen")?.disabled).toBe(false);

    // A reference that came meanwhile: the kernel's list, and „Löschen“ locked.
    vi.mocked(deleteEntity).mockRejectedValueOnce({ code: "referenced", message: "Trainer 0901 wird verwendet.", asset: null, problems: [], references: [figureRef(3)] });
    await act(async () => button("Löschen")!.click());
    expect(notices("alert")).toEqual(["1 Verweis: zuerst umhängen."]);
    expect(references().map((r) => r.textContent)).toEqual(["Figur 3 in Eventdatei 0008Eventdatei 0008"]);
    expect(button("Löschen")?.disabled).toBe(true);
  });
});

describe("the kernel's refusal in the DSPRE layout", () => {
  it("not_a_text_project_mirrors_the_kernel_sentence", () => {
    // The dialog knows the kernel's refusal by its words in either language: a change in the kernel must show here.
    // The kernel sentence lives in its catalogs as `delete.not_a_text_project`.
    expect(kernelSentence("de", "delete", "not_a_text_project")).toBe(deDelete.deleteDialog.notATextProject);
    expect(kernelSentence("en", "delete", "not_a_text_project")).toBe(enDelete.deleteDialog.notATextProject);
  });
});
