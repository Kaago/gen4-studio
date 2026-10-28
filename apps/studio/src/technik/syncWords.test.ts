import { describe, expect, it } from "vitest";
import { adoptedText, appliedText, brokenText, conflictCountText, conflictsText, moreText, problemLine, syncProgressText } from "./syncWords";

describe("syncWords", () => {
  it("counts_have_singular_and_plural", () => {
    expect(adoptedText(1)).toBe("1 Änderung aus DSPRE übernommen");
    expect(adoptedText(3)).toBe("3 Änderungen aus DSPRE übernommen");
    expect(appliedText(1)).toBe("1 geänderte Projektdatei übernommen");
    expect(appliedText(12)).toBe("12 geänderte Projektdateien übernommen");
    expect(conflictsText(1)).toBe("1 Konflikt zwischen Studio und DSPRE");
    expect(conflictsText(2)).toBe("2 Konflikte zwischen Studio und DSPRE");
    expect(brokenText(1)).toBe("1 Projektdatei ist fehlerhaft");
    expect(brokenText(4)).toBe("4 Projektdateien sind fehlerhaft");
    expect(conflictCountText(1)).toBe("1 Konflikt");
    expect(conflictCountText(5)).toBe("5 Konflikte");
    expect(moreText(1)).toBe("und 1 weitere Abweichung");
    expect(moreText(7)).toBe("und 7 weitere Abweichungen");
  });

  it("words_the_progress_and_a_problem_with_its_place", () => {
    expect(syncProgressText({ done: 120, total: 6500 })).toBe("Gleiche mit DSPRE ab … 120 von 6500 Dateien");
    expect(problemLine({ severity: "error", file: "data/moves/0001.json", line: 3, col: 12, message: "Unbekannter Schlüssel „pwr“." })).toBe(
      "data/moves/0001.json:3:12: Unbekannter Schlüssel „pwr“.",
    );
  });
});
