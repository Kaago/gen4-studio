import i18next from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import type { AdoptReport, TextProjectStatus } from "../api";
import { adoptProgressText, adoptedText, isReopen, reopenText, unsavedText } from "./AdoptDialog";
import { liftedText } from "./MigrateDialog";
import { renamedText } from "./RenameIdDialog";
import { revertedText } from "./RevertDialog";
import { hintOf, noDp } from "./textProjectHint";

afterEach(() => i18next.changeLanguage("de"));

const report = { export: { files: 6512 }, ms: 14_200, templates: null, templates_warning: null } as unknown as AdoptReport;

describe("textproject in English", () => {
  it("recognizes_the_reopen_answer_in_either_language", async () => {
    // The refusal has no code of its own: the dialog knows it by its words, in the backend's language whatever the app shows.
    const german = reopenText();
    await i18next.changeLanguage("en");
    expect(isReopen(german)).toBe(true);
    expect(isReopen(reopenText())).toBe(true);
    expect(isReopen("Save first: 1 unsaved change.")).toBe(false);
  });

  it("words_the_messages_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(adoptedText(report)).toBe("Text project set up: 6,512 files in 14 s.");
    expect(unsavedText(1)).toBe("Save first: 1 unsaved change.");
    expect(unsavedText(2)).toBe("Save first: 2 unsaved changes.");
    expect(adoptProgressText({ phase: "entities", done: 3, total: 9 })).toBe("Writing entries… 3 of 9 kinds");
    expect(adoptProgressText({ phase: "scripts", done: 0, total: 0 })).toBe("Adopting scripts…");
    expect(liftedText({ from: 1, to: 2 } as Parameters<typeof liftedText>[0])).toBe("Text project upgraded: schema version 1 → 2.");
    expect(revertedText({ backup: "/p/.g4/backup/x", templates: null, templates_warning: null } as unknown as Parameters<typeof revertedText>[0])).toBe("Text project reverted. Backup: /p/.g4/backup/x");
    expect(renamedText({ old_id: "TRAINER_A", new_id: "TRAINER_B" } as Parameters<typeof renamedText>[0])).toBe("ID renamed: TRAINER_A → TRAINER_B. ⌘Z takes it back.");
  });

  it("words_the_hint_in_English", async () => {
    await i18next.changeLanguage("en");
    const dspre: TextProjectStatus = { state: "dspre", adoptable: true, reason: null };
    const noBase: TextProjectStatus = { state: "dspre", adoptable: false, reason: "x" };
    expect(hintOf(dspre).text).toBe("This project can be set up as a text project: one readable file per entry, ready for Git. DSPRE keeps opening the folder.");
    expect(hintOf(noBase).text).toBe("This project can be set up as a text project: one readable file per entry, ready for Git. It needs the base ROM for that.");
    expect(noDp()).toBe("No text project for Diamond/Pearl in phase 4.");
  });
});
