import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const inTauri = vi.hoisted(() => vi.fn(() => true));
vi.mock("./platform/transport", () => ({ invoke, inTauri }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import { changedFromOriginal, errorText, originalStatus, planReset, resetErrorText, resetToOriginal, targetIndex, targetLabel } from "./api";
import type { ChangedReport, ResetTarget } from "./api";

const report: ChangedReport = { entries: [], files: 12, hashed: 3, recorded: 3, unbuilt: [], ms: 40 };

describe("original commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    inTauri.mockReturnValue(true);
  });

  it("original_commands_send_the_documented_arguments", async () => {
    await originalStatus();
    expect(invoke).toHaveBeenLastCalledWith("original_status");
    const map: ResetTarget = { kind: "map", index: 3 };
    await planReset(map);
    expect(invoke).toHaveBeenLastCalledWith("plan_reset", { target: map });
    // Without fields the key is left out: the backend resets every differing field.
    await resetToOriginal(map);
    expect(invoke).toHaveBeenLastCalledWith("reset_to_original", { target: map });
    const header: ResetTarget = { kind: "header", id: 12 };
    await resetToOriginal(header, ["weather_id", "music_day_id"]);
    expect(invoke).toHaveBeenLastCalledWith("reset_to_original", { target: header, fields: ["weather_id", "music_day_id"] });

    // In the app the progress comes over a channel.
    invoke.mockResolvedValueOnce(report);
    const onProgress = vi.fn();
    await expect(changedFromOriginal(onProgress)).resolves.toBe(report);
    const [cmd, args] = invoke.mock.calls[invoke.mock.calls.length - 1]!;
    expect(cmd).toBe("changed_from_original");
    args.progress.onmessage({ done: 1, total: 4 });
    expect(onProgress).toHaveBeenCalledWith({ done: 1, total: 4 });

    // The dev bridge has no channel: one final progress from the report.
    inTauri.mockReturnValue(false);
    invoke.mockReset();
    invoke.mockResolvedValueOnce(report);
    const later = vi.fn();
    await expect(changedFromOriginal(later)).resolves.toBe(report);
    expect(invoke.mock.calls).toEqual([["changed_from_original"]]);
    expect(later.mock.calls).toEqual([[{ done: 3, total: 3 }]]);
  });

  it("target_index_reads_the_number_of_each_kind", () => {
    expect(targetIndex({ kind: "map", index: 3 })).toBe(3);
    expect(targetIndex({ kind: "interior_building_model", index: 9 })).toBe(9);
    expect(targetIndex({ kind: "text_message", archive: 5, message: 2 })).toBe(5);
    expect(targetIndex({ kind: "header", id: 12 })).toBe(12);
    expect(targetIndex({ kind: "script", file: 40 })).toBe(40);
  });

  it("target_label_names_the_entity_like_the_kernel", () => {
    expect(targetLabel({ kind: "map", index: 3 })).toBe("Karte 0003");
    expect(targetLabel({ kind: "event_file", index: 17 })).toBe("Eventdatei 0017");
    expect(targetLabel({ kind: "text_archive", index: 5 })).toBe("Textarchiv 0005");
    expect(targetLabel({ kind: "text_message", archive: 5, message: 2 })).toBe("Nachricht 2 in Textarchiv 0005");
    expect(targetLabel({ kind: "trainer", index: 1 })).toBe("Trainer 0001");
    expect(targetLabel({ kind: "species", index: 396 })).toBe("Pokémon 0396");
    expect(targetLabel({ kind: "encounter", index: 8 })).toBe("Begegnungsdatei 0008");
    expect(targetLabel({ kind: "header", id: 12 })).toBe("Header 0012");
    expect(targetLabel({ kind: "script", file: 40 })).toBe("Scriptdatei 0040");
    expect(targetLabel({ kind: "map_texture", index: 2 })).toBe("Kartentextur 0002");
    expect(targetLabel({ kind: "building_texture", index: 2 })).toBe("Gebäudetextur 0002");
    expect(targetLabel({ kind: "building_model", index: 2 })).toBe("Gebäudemodell 0002");
    expect(targetLabel({ kind: "interior_building_model", index: 2 })).toBe("Innenraum-Gebäudemodell 0002");
  });

  it("errors_of_a_reset_read_as_their_german_sentence", () => {
    // The kernel's sentences come through as they are.
    expect(errorText({ code: "no_original", message: "Neue Nachricht, kein Original.", asset: null, problems: [] })).toBe("Neue Nachricht, kein Original.");
    expect(resetErrorText({ code: "base_missing", message: "Basis-ROM nicht gefunden.", asset: null, problems: [] })).toBe("Basis-ROM nicht gefunden.");
    // A locked asset: only the German reason after the English lead.
    expect(
      resetErrorText({ code: "read_only", message: "Trainer 0001 is read-only: Platzhalter-Trainer des Sicht-Scripts", asset: { kind: "Trainer", index: 1 }, problems: [] }),
    ).toBe("Platzhalter-Trainer des Sicht-Scripts");
    expect(resetErrorText(new Error("kaputt"))).toBe("Error: kaputt");
  });
});
