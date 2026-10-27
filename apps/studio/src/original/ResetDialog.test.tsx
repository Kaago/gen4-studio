// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { planReset, resetToOriginal } from "../api";
import type { ResetPlan, ResetResult, ResetTarget } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { ResetDialog } from "./ResetDialog";
import type { ResetDialogProps } from "./ResetDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  planReset: vi.fn(),
  resetToOriginal: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MAP: ResetTarget = { kind: "map", index: 3 };
const HEADER: ResetTarget = { kind: "header", id: 12 };
const SCRIPT: ResetTarget = { kind: "script", file: 40 };

const plan = (over: Partial<ResetPlan> = {}): ResetPlan => ({
  target: MAP,
  label: "Karte 0003",
  changes: [{ label: "Karte 0003", before: "Höhe 0\nGebäude 2", after: "Höhe 0\nGebäude 1" }],
  fields: [],
  writes_files: false,
  same: false,
  ...over,
});
const result: ResetResult = { changed: [{ kind: "Map", index: 3 }], recorded: true, ms: 12 };

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

describe("ResetDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onReset = vi.fn();

  const show = (over: Partial<ResetDialogProps> = {}) =>
    act(async () => root.render(<ResetDialog target={MAP} baseLabel="Platin DE (Testprojekt)" family="Pt" onClose={onClose} onReset={onReset} {...over} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.textContent !== "");
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const notices = (role: "status" | "alert") => [...(dialog()?.querySelectorAll<HTMLElement>(`[role=${role}]`) ?? [])].map((n) => n.textContent);
  const box = (label: string) => [...(dialog()?.querySelectorAll<HTMLElement>("[role=checkbox]") ?? [])].find((b) => b.closest("label")?.textContent === label);

  beforeEach(() => {
    onClose.mockReset();
    onReset.mockReset();
    vi.mocked(planReset).mockReset().mockResolvedValue(plan());
    vi.mocked(resetToOriginal).mockReset().mockResolvedValue(result);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows_before_and_after_per_change_then_resets_on_confirm", async () => {
    const planned = deferred<ResetPlan>();
    vi.mocked(planReset).mockReturnValueOnce(planned.promise);
    await show();
    expect(planReset).toHaveBeenCalledWith(MAP);
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Karte 0003 auf Original zurücksetzen?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      "Stellt den Stand der Basis-ROM Platin DE (Testprojekt) wieder her. ⌘Z nimmt es zurück.",
    );
    expect(notices("status")).toEqual(["Lade Vergleich …"]);
    expect(button("Zurücksetzen")?.disabled).toBe(true);

    await act(async () => planned.resolve(plan({ changes: [...plan().changes, { label: "Eventdatei 0003", before: "a", after: "b" }] })));
    expect(notices("status")).toEqual([]);
    const sides = [...(dialog()?.querySelectorAll<HTMLElement>("[role=group][aria-label]") ?? [])].map((g) => g.getAttribute("aria-label"));
    expect(sides.filter((l) => l !== "Vorher" && l !== "Nachher")).toEqual(["Karte 0003", "Eventdatei 0003"]);
    expect(dialog()?.querySelector('[role=group][aria-label="Karte 0003"]')?.textContent).toContain("Gebäude 2");
    // Long lines (a message, an event as JSON, a map's model line) wrap in the dialog instead of scrolling sideways.
    const frames = [...(dialog()?.querySelectorAll<HTMLElement>("[data-change] [role=group][data-wrap]") ?? [])];
    expect(frames.map((f) => f.getAttribute("aria-label"))).toEqual(["Karte 0003", "Eventdatei 0003"]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Abbrechen", "Zurücksetzen"]);
    expect(button("Zurücksetzen")?.dataset.kind).toBe("primary");
    expect(buttons().filter((b) => b.dataset.kind === "primary")).toHaveLength(1);

    const running = deferred<ResetResult>();
    vi.mocked(resetToOriginal).mockReturnValueOnce(running.promise);
    await act(async () => button("Zurücksetzen")!.click());
    expect(resetToOriginal).toHaveBeenCalledWith(MAP);
    expect(vi.mocked(resetToOriginal).mock.calls[0]).toHaveLength(1);
    expect(notices("status")).toContain("Setze zurück …");
    expect(button("Zurücksetzen")?.disabled).toBe(true);
    expect(button("Abbrechen")?.disabled).toBe(true);
    // Escape does not leave a reset that runs.
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();

    await act(async () => running.resolve(result));
    expect(onReset).toHaveBeenCalledWith(result, "Karte 0003");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("names_every_change_visibly_above_its_comparison", async () => {
    vi.mocked(planReset).mockResolvedValue(plan({ changes: [{ label: "Karte 0003: 1 Zelle der Bewegungsrechte anders", before: "a", after: "b" }] }));
    await show();
    const changes = [...(dialog()?.querySelectorAll<HTMLElement>("[data-change]") ?? [])];
    expect(changes.map((c) => c.querySelector("p")?.textContent)).toEqual(["Karte 0003: 1 Zelle der Bewegungsrechte anders"]);
  });

  it("header_fields_can_be_unchecked_and_only_checked_ones_are_sent", async () => {
    vi.mocked(planReset).mockResolvedValue(
      plan({
        target: HEADER,
        label: "Header 0012",
        // The kernel's lines with the format's names: the checkboxes show the fields in German, so they are not shown twice.
        changes: [{ label: "Header 0012", before: "weather_id: 0\nmusic_day_id: 1000", after: "weather_id: 3\nmusic_day_id: 1001" }],
        fields: [
          { field: "weather_id", before: "0", after: "3" },
          { field: "music_day_id", before: "1000", after: "1001" },
          { field: "unknown_x", before: "1", after: "2" },
        ],
      }),
    );
    await show({ target: HEADER });
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Header 0012 auf Original zurücksetzen?");
    // The German name of each field (a field without one keeps its serde name), all checked, the values beside it in mono.
    const rows = [...(dialog()?.querySelectorAll<HTMLElement>("[data-header-field]") ?? [])];
    expect(rows.map((r) => r.dataset.headerField)).toEqual(["weather_id", "music_day_id", "unknown_x"]);
    expect(rows.map((r) => r.querySelector("label")?.textContent)).toEqual(["Wetter", "Musik Tag", "unknown_x"]);
    expect(rows.map((r) => r.querySelector("[data-values]")?.textContent)).toEqual(["0 → 3", "1000 → 1001", "1 → 2"]);
    expect(["Wetter", "Musik Tag", "unknown_x"].map((l) => box(l)?.getAttribute("aria-checked"))).toEqual(["true", "true", "true"]);
    expect(dialog()?.querySelector("[data-change]")).toBeNull();
    expect(dialog()?.textContent).not.toContain("weather_id: 0");

    await act(async () => box("Wetter")!.click());
    await act(async () => box("unknown_x")!.click());
    expect(box("Wetter")?.getAttribute("aria-checked")).toBe("false");
    await act(async () => button("Zurücksetzen")!.click());
    expect(resetToOriginal).toHaveBeenCalledWith(HEADER, ["music_day_id"]);
  });

  it("without_a_checked_header_field_zuruecksetzen_is_disabled", async () => {
    vi.mocked(planReset).mockResolvedValue(plan({ target: HEADER, label: "Header 0012", changes: [], fields: [{ field: "weather_id", before: "0", after: "3" }] }));
    await show({ target: HEADER });
    expect(button("Zurücksetzen")?.disabled).toBe(false);
    await act(async () => box("Wetter")!.click());
    expect(button("Zurücksetzen")?.disabled).toBe(true);
  });

  it("a_file_target_says_it_is_written_at_once", async () => {
    vi.mocked(planReset).mockResolvedValue(plan({ target: SCRIPT, label: "Scriptdatei 0040", changes: [{ label: "Scriptdatei 0040", before: "a", after: "b" }], writes_files: true }));
    await show({ target: SCRIPT });
    expect(notices("status")).toEqual(["Wird sofort geschrieben; ⌘Z nimmt es zurück."]);
    // A store target does not say so.
    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(planReset).mockResolvedValue(plan());
    await show();
    expect(notices("status")).toEqual([]);
  });

  it("an_unsaved_script_warns", async () => {
    vi.mocked(planReset).mockResolvedValue(plan({ target: SCRIPT, label: "Scriptdatei 0040", changes: [{ label: "Scriptdatei 0040", before: "a", after: "b" }], writes_files: true }));
    await show({ target: SCRIPT, scriptDirty: true });
    expect(notices("status")).toEqual(["Ungespeicherte Änderungen an dieser Scriptdatei gehen verloren.", "Wird sofort geschrieben; ⌘Z nimmt es zurück."]);
  });

  it("an_unsaved_asset_says_undo_brings_it_back", async () => {
    await show({ dirty: true });
    expect(notices("status")).toEqual(["Auch ungespeicherte Änderungen werden ersetzt; ⌘Z stellt sie wieder her."]);
  });

  it("same_offers_only_schliessen", async () => {
    vi.mocked(planReset).mockResolvedValue(plan({ changes: [], same: true }));
    await show();
    expect(notices("status")).toEqual(["Schon im Stand der Basis-ROM."]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Schließen"]);
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
    expect(resetToOriginal).not.toHaveBeenCalled();
  });

  it("a_failed_reset_shows_the_reason_and_keeps_the_dialog", async () => {
    // The base ROM was renamed after the dialog opened.
    const reason = "Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).";
    vi.mocked(resetToOriginal).mockRejectedValueOnce({ code: "base_missing", message: reason, asset: null, problems: [] });
    await show();
    await act(async () => button("Zurücksetzen")!.click());
    expect(notices("alert")).toEqual([reason]);
    expect(dialog()).not.toBeNull();
    expect(onReset).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    // It can be tried again, and the error goes when it succeeds.
    expect(button("Zurücksetzen")?.disabled).toBe(false);
    await act(async () => button("Zurücksetzen")!.click());
    expect(onReset).toHaveBeenCalledOnce();
  });

  it("a_refused_plan_shows_the_reason_instead_of_an_empty_dialog", async () => {
    vi.mocked(planReset).mockRejectedValue({ code: "read_only", message: "Trainer 0001 is read-only: Platzhalter-Trainer des Sicht-Scripts", asset: { kind: "Trainer", index: 1 }, problems: [] });
    await show({ target: { kind: "trainer", index: 1 } });
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Trainer 0001 auf Original zurücksetzen?");
    expect(notices("alert")).toEqual(["Platzhalter-Trainer des Sicht-Scripts"]);
    expect(buttons().map((b) => b.textContent)).toEqual(["Schließen"]);
  });

  it("abbrechen_closes_without_resetting", async () => {
    await show();
    await act(async () => button("Abbrechen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
    expect(resetToOriginal).not.toHaveBeenCalled();
  });
});
