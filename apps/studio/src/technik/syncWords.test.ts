import { describe, expect, it } from "vitest";
import { adoptedText, appliedText, brokenCountText, brokenText, conflictCountText, conflictsText, moreText, problemLine, syncProgressText } from "./syncWords";

/** The no-break space between a count and what it counts (UI test 4.3b U7). */
const NB = "\u00a0";

describe("syncWords", () => {
  it("counts_have_singular_and_plural_bound_to_their_count", () => {
    expect(adoptedText(1)).toBe(`1${NB}Änderung aus DSPRE übernommen`);
    expect(adoptedText(3)).toBe(`3${NB}Änderungen aus DSPRE übernommen`);
    expect(appliedText(1)).toBe(`1${NB}geänderte Projektdatei übernommen`);
    expect(appliedText(12)).toBe(`12${NB}geänderte Projektdateien übernommen`);
    expect(conflictsText(1)).toBe(`1${NB}Konflikt zwischen Studio und DSPRE`);
    expect(conflictsText(2)).toBe(`2${NB}Konflikte zwischen Studio und DSPRE`);
    expect(brokenText(1)).toBe(`1${NB}Projektdatei ist fehlerhaft`);
    expect(brokenText(4)).toBe(`4${NB}Projektdateien sind fehlerhaft`);
    expect(conflictCountText(1)).toBe(`1${NB}Konflikt`);
    expect(conflictCountText(5)).toBe(`5${NB}Konflikte`);
    expect(moreText(1)).toBe(`und 1${NB}weitere Abweichung`);
    expect(moreText(7)).toBe(`und 7${NB}weitere Abweichungen`);
  });

  it("counts_broken_files_for_the_list_of_technik", () => {
    expect(brokenCountText(1)).toBe(`1${NB}fehlerhafte Datei`);
    expect(brokenCountText(3)).toBe(`3${NB}fehlerhafte Dateien`);
  });

  it("words_the_progress_and_a_problem_with_its_place", () => {
    expect(syncProgressText({ done: 120, total: 6500 })).toBe(`Gleiche mit DSPRE ab … 120 von 6500${NB}Dateien`);
    expect(problemLine({ severity: "error", file: "data/moves/0001.json", line: 3, col: 12, message: "Unbekannter Schlüssel „pwr“." })).toBe(
      "data/moves/0001.json:3:12: Unbekannter Schlüssel „pwr“.",
    );
  });

  it("a_problem_line_leaves_out_the_kernels_check_code", () => {
    // UI test 4.3b U6: the kernel's message starts with its check code; the line shows the sentence only.
    const broken = { severity: "error" as const, file: "data/moves/0039.json", line: 7, col: 5, message: "textproject.schema: kein gültiges JSON: `,` oder `}` wird erwartet." };
    expect(problemLine(broken)).toBe("data/moves/0039.json:7:5: kein gültiges JSON: `,` oder `}` wird erwartet.");
    expect(problemLine({ ...broken, message: "textproject.unknown-id: personal.type1: unbekannte ID TYPE_FOO." })).toBe(
      "data/moves/0039.json:7:5: personal.type1: unbekannte ID TYPE_FOO.",
    );
    // A sentence that only has a colon keeps it.
    expect(problemLine({ ...broken, message: "Achtung: die Datei fehlt." })).toBe("data/moves/0039.json:7:5: Achtung: die Datei fehlt.");
  });
});
