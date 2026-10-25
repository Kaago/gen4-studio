// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FixPlanDto } from "../../api";
import { fakeChecks, finding } from "../../checks/checksTesting";
import type { ChecksApi } from "../../checks/useChecks";
import { FixDialog } from "./FixDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const wide = finding("text.too-wide", "warning", {
  subject: { kind: "textArchive", index: 433 },
  key: "message:1:line:1",
  label: "Textarchiv 0433 · Nachricht 1 · Zeile 1",
  args: ["433", "1", "1", "240", "216"],
  fixes: [{ id: "wrap", label: "Umbrechen", irreversible: false }],
});
const missing = finding("event.script-missing", "error", {
  key: "object:0",
  label: "Eventdatei 0411 · Objekt 0",
  args: ["object", "0", "7", "5", "2"],
  fixes: [{ id: "create_script", label: "Script anlegen", irreversible: true }],
});
const plan: FixPlanDto = {
  changes: [{ asset: { kind: "TextArchive", index: 433 }, label: "Textarchiv 0433", before: "Das ist eine viel zu lange Zeile", after: "Das ist eine viel\nzu lange Zeile" }],
  irreversible: false,
};

describe("FixDialog", () => {
  let container: HTMLElement;
  let root: Root;
  let checks: ChecksApi;
  const onClose = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    checks = fakeChecks();
    onClose.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.body.innerHTML = "";
  });
  const show = (f = wide) => act(async () => root.render(<FixDialog finding={f} checks={checks} onClose={onClose} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (name: string) => [...dialog().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("shows_before_and_after_of_every_change_and_applies_the_fix", async () => {
    vi.mocked(checks.planFix).mockResolvedValue(plan);
    await show();
    expect(checks.planFix).toHaveBeenCalledWith(wide, "wrap");
    expect(dialog().querySelector("h2")?.textContent).toBe("Textarchiv 0433 · Nachricht 1 · Zeile 1");
    expect(dialog().textContent).toContain("Die Zeile ist 240 px breit, das Textfenster nur 216 px.");
    const preview = dialog().querySelector<HTMLElement>("[role=group][aria-label='Textarchiv 0433']")!;
    expect(preview.querySelector("[role=group][aria-label=Vorher]")?.textContent).toContain("Das ist eine viel zu lange Zeile");
    expect(preview.querySelector("[role=group][aria-label=Nachher]")?.textContent).toContain("zu lange Zeile");
    expect(dialog().textContent).not.toContain("Nicht rückgängig");
    await click(button("Umbrechen"));
    expect(checks.applyFix).toHaveBeenCalledWith(wide, "wrap");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("marks_a_fix_undo_cannot_take_back", async () => {
    vi.mocked(checks.planFix).mockResolvedValue({ changes: [{ asset: { kind: "Script", index: 5 }, label: "Scriptdatei 0005", before: "", after: "script script_7 #7:\n\tEnd" }], irreversible: true });
    await show(missing);
    expect(checks.planFix).toHaveBeenCalledWith(missing, "create_script");
    expect(dialog().textContent).toContain("Nicht rückgängig");
    expect(button("Script anlegen")).toBeDefined();
  });

  it("says_nicht_behoben_when_the_finding_is_still_there", async () => {
    vi.mocked(checks.planFix).mockResolvedValue(plan);
    vi.mocked(checks.applyFix).mockResolvedValue({ changed: [{ kind: "TextArchive", index: 433 }], remaining: true });
    await show();
    await click(button("Umbrechen"));
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog().textContent).toContain("Nicht behoben");
    expect(button("Umbrechen")).toBeUndefined();
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("says_the_finding_is_no_longer_current_and_offers_to_check_again", async () => {
    vi.mocked(checks.planFix).mockResolvedValue(plan);
    vi.mocked(checks.applyFix).mockRejectedValue({ code: "finding_stale", message: "finding is no longer current", asset: null });
    await show();
    await click(button("Umbrechen"));
    expect(dialog().textContent).toContain("Befund ist nicht mehr aktuell");
    expect(button("Umbrechen")).toBeUndefined();
    await click(button("Erneut prüfen"));
    expect(checks.recheck).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("asks_to_save_first_when_creating_a_script_meets_an_unsaved_text_archive", async () => {
    vi.mocked(checks.planFix).mockResolvedValue({ changes: [{ asset: { kind: "Script", index: 5 }, label: "Scriptdatei 0005", before: "", after: "script script_7 #7:\n\tEnd" }], irreversible: true });
    vi.mocked(checks.applyFix).mockRejectedValue({
      code: "error",
      message: "text archive 0554 has unsaved changes; save it first (creating a script compiles the scripts, which rewrites text archives)",
      asset: null,
    });
    await show(missing);
    await click(button("Script anlegen"));
    const alert = dialog().querySelector("[role=alert]")!;
    expect(alert.textContent).toContain("Erst speichern");
    expect(alert.textContent).toContain("Textarchiv 0554 hat ungespeicherte Änderungen");
    expect(alert.textContent).toContain("„Speichern und anlegen“");
    expect(button("Script anlegen")).toBeUndefined();
    // The way on: save the project, then create the script, without leaving the dialog (a click on the title bar would close it).
    expect(button("Schließen")).toBeDefined();
    vi.mocked(checks.applyFix).mockReset().mockResolvedValue({ changed: [{ kind: "Script", index: 5 }], remaining: false });
    await click(button("Speichern und anlegen"));
    expect(checks.save).toHaveBeenCalledOnce();
    expect(vi.mocked(checks.save).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(checks.applyFix).mock.invocationCallOrder[0]!);
    expect(checks.applyFix).toHaveBeenCalledWith(missing, "create_script");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("stays_on_erst_speichern_when_the_save_left_the_archive_unsaved", async () => {
    vi.mocked(checks.planFix).mockResolvedValue({ changes: [{ asset: { kind: "Script", index: 5 }, label: "Scriptdatei 0005", before: "", after: "script script_7 #7:\n\tEnd" }], irreversible: true });
    const unsaved = { code: "error", message: "text archive 0554 has unsaved changes; save it first (creating a script compiles the scripts, which rewrites text archives)", asset: null };
    vi.mocked(checks.applyFix).mockRejectedValue(unsaved);
    await show(missing);
    await click(button("Script anlegen"));
    // The save failed (the app says why): the script is asked for again and refused again, nothing more.
    await click(button("Speichern und anlegen"));
    expect(checks.save).toHaveBeenCalledOnce();
    expect(checks.applyFix).toHaveBeenCalledTimes(2);
    expect(dialog().querySelector("[role=alert]")!.textContent).toContain("Erst speichern");
    expect(button("Speichern und anlegen")).toBeDefined();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("says_why_a_preview_cannot_be_made", async () => {
    vi.mocked(checks.planFix).mockRejectedValue({ code: "error", message: "word 'WWW' is 520px wide", asset: null });
    await show();
    expect(dialog().querySelector("[role=alert]")?.textContent).toContain("word 'WWW' is 520px wide");
    expect(button("Umbrechen")).toBeUndefined();
  });
});
