import { describe, expect, it } from "vitest";
import type { DestCheck } from "../api";
import { destProblem, megabytes, startMessage } from "./startMessages";

const check = (change: Partial<DestCheck> = {}): DestCheck => ({ state: "missing", free_bytes: 10_000_000_000, needed_bytes: 335_544_320, ok: true, ...change });

describe("megabytes", () => {
  it("rounds_to_whole_megabytes_in_german_format", () => {
    expect(megabytes(320_400_000)).toBe("320 MB");
    expect(megabytes(335_544_320)).toBe("336 MB");
    expect(megabytes(12_345_600_000)).toBe("12.346 MB");
  });
});

describe("destProblem", () => {
  it("is_null_for_a_free_destination", () => {
    expect(destProblem(check())).toBeNull();
    expect(destProblem(check({ state: "empty", free_bytes: null }))).toBeNull();
  });

  it("names_each_refusal", () => {
    expect(destProblem(check({ state: "not_empty", ok: false }))).toBe("Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.");
    expect(destProblem(check({ state: "not_a_directory", ok: false }))).toBe("Dort liegt schon eine Datei.");
    expect(destProblem(check({ state: "unreadable", ok: false }))).toBe("Den Zielordner kann Gen4 Studio nicht lesen.");
    expect(destProblem(check({ free_bytes: 100_000_000, needed_bytes: 320_000_000, ok: false }))).toBe("Zu wenig Platz: 100 MB frei, gebraucht etwa 320 MB.");
  });
});

describe("startMessage", () => {
  it("translates_the_known_refusals_of_create_project", () => {
    expect(startMessage("staging folder /p/.Neu.g4-new is in the way; remove it or choose another name")).toBe("Ein alter Zwischenordner liegt im Weg. Lösche ihn oder wähle einen anderen Namen.");
    expect(startMessage("cannot read destination /p/Neu")).toBe("Den Zielordner kann Gen4 Studio nicht lesen.");
    expect(startMessage("not enough free space: 335 MB needed, 1200 MB free")).toBe("Zu wenig Platz: 1.200 MB frei, gebraucht etwa 335 MB.");
    expect(startMessage("the ROM's ARM9 is encrypted; encrypted ROMs are not supported")).toBe("Die ROM ist verschlüsselt. Nutze einen entschlüsselten Dump.");
    expect(startMessage("destination /p/Neu is not empty")).toBe("Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.");
    expect(startMessage("destination /p/Neu exists and is not a folder")).toBe("Dort liegt schon eine Datei.");
  });

  it("keeps_any_other_text", () => {
    expect(startMessage("build: boom")).toBe("build: boom");
    expect(startMessage("Diamant/Perl: ohne Testprojekt nicht freigegeben")).toBe("Diamant/Perl: ohne Testprojekt nicht freigegeben");
  });
});
