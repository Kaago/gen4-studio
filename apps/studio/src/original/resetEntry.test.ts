import { describe, expect, it, vi } from "vitest";
import type { OriginalStatus, ResetTarget } from "../api";
import { resetEntry, resetLabel } from "./resetEntry";

const ready: OriginalStatus = { state: "ready", label: "Platin DE (Testprojekt)", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", counts: { map: 4, text_archive: 10, header: 20 } };

describe("resetEntry", () => {
  it("entry_is_disabled_without_a_base_and_says_why", () => {
    const open = vi.fn();
    const target: ResetTarget = { kind: "map", index: 3 };
    const missing = resetEntry(target, { state: "unavailable", reason: "Basis-ROM nicht gefunden." }, open);
    expect(missing).toMatchObject({ label: "Auf Original zurücksetzen…", icon: "undo", disabled: true, detail: "Basis-ROM fehlt" });
    // While the status is read the entry waits.
    expect(resetEntry(target, null, open)).toMatchObject({ disabled: true, detail: "Wird gelesen …" });
    // A ready base enables it, without a detail.
    const usable = resetEntry(target, ready, open);
    expect(usable.disabled).toBe(false);
    expect(usable.detail).toBeUndefined();
  });

  it("entry_is_disabled_for_a_new_index_with_kein_original", () => {
    const open = vi.fn();
    expect(resetEntry({ kind: "map", index: 4 }, ready, open)).toMatchObject({ disabled: true, detail: "Kein Original" });
    expect(resetEntry({ kind: "header", id: 19 }, ready, open).disabled).toBe(false);
    expect(resetEntry({ kind: "header", id: 20 }, ready, open)).toMatchObject({ disabled: true, detail: "Kein Original" });
    // A message counts with its archive; the message itself is checked by the plan.
    expect(resetEntry({ kind: "text_message", archive: 9, message: 500 }, ready, open).disabled).toBe(false);
    expect(resetEntry({ kind: "text_message", archive: 10, message: 0 }, ready, open)).toMatchObject({ disabled: true, detail: "Kein Original" });
    // A kind the base has no archive for has no original at all.
    expect(resetEntry({ kind: "script", file: 0 }, ready, open)).toMatchObject({ disabled: true, detail: "Kein Original" });
  });

  it("entry_opens_the_dialog_with_its_target", () => {
    const open = vi.fn();
    const target: ResetTarget = { kind: "text_message", archive: 2, message: 7 };
    const entry = resetEntry(target, ready, open, "Nachricht auf Original zurücksetzen…");
    expect(entry.label).toBe("Nachricht auf Original zurücksetzen…");
    entry.onSelect();
    expect(open).toHaveBeenCalledWith(target);
    expect(resetLabel()).toBe("Auf Original zurücksetzen…");
  });
});
