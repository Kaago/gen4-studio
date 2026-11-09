import { describe, expect, it } from "vitest";
import { found, missing } from "../technik/baseTesting";
import { badFile, badVersion, commitSubjectNow, dateText, fileNameFor, fileProblem, filesLine, noRomContent, phaseText, publishLocked, publishedSubject, versionProblem, versionTaken, withXdelta } from "./publishWords";

describe("publishWords", () => {
  it("file_name_follows_stem_and_version", () => {
    expect(fileNameFor("Sinnoh-Remix", "0.1.0")).toBe("Sinnoh-Remix-v0.1.0.xdelta");
    expect(fileNameFor("Mein Hack", "1.0-beta")).toBe("Mein Hack-v1.0-beta.xdelta");
    // The suffix is added once, as the kernel's `file_name` does.
    expect(withXdelta("Hack")).toBe("Hack.xdelta");
    expect(withXdelta("Hack.xdelta")).toBe("Hack.xdelta");
    expect(withXdelta("Hack.v2")).toBe("Hack.v2.xdelta");
  });

  it("version_and_file_problems_match_the_kernel_sentences", () => {
    // The kernel's sentences, word for word (`patch::release::{badVersion(), versionTaken()}`, `patch::publish::badFile()`).
    expect(badVersion()).toBe('Die Version darf nicht leer sein, höchstens 32 Zeichen haben und weder Leerzeichen noch / \\ : * ? " < > | enthalten.');
    expect(versionTaken()).toBe("Diese Version gibt es schon.");
    expect(badFile()).toBe('Der Dateiname darf nicht leer sein und weder / \\ : * ? " < > | noch Steuerzeichen enthalten.');

    for (const ok of ["0.1.0", "1.0-beta", "v2", "x".repeat(32), "1.0.0+ä"]) expect(versionProblem(ok, [])).toBeNull();
    for (const bad of ["", "1 0", "1.0\t", "1/0", "1\\0", "a:b", "a*", "a?", 'a"', "a<", "a>", "a|", "a\u0007", "x".repeat(33)]) expect(versionProblem(bad, []), bad).toBe(badVersion());
    // 32 characters, not bytes.
    expect(versionProblem("ä".repeat(32), [])).toBeNull();
    // Taken comes after the characters, as in `check_version`.
    expect(versionProblem("0.1.0", ["0.1.0"])).toBe(versionTaken());
    expect(versionProblem("0 1", ["0 1"])).toBe(badVersion());

    for (const ok of ["Hack-v0.1.0.xdelta", "Hack", "Mein Hack v1", ".hidden"]) expect(fileProblem(ok), ok).toBeNull();
    // Empty, only spaces and dots (also before the suffix), or a character a file name cannot have; spaces are fine in a file name.
    for (const bad of ["", " ", "..", ".xdelta", " . .xdelta", "a/b", "a\\b", "a:b", "a*", "a?", 'a"', "a<", "a>", "a|", "a\n"]) expect(fileProblem(bad), JSON.stringify(bad)).toBe(badFile());
  });

  it("files_line_counts_and_sums", () => {
    const file = (kind: "patch" | "notes" | "readme", size: number) => ({ kind, path: `/h/${kind}`, size });
    expect(filesLine([file("patch", 4500), file("notes", 600), file("readme", 122)])).toBe("3 Dateien, 5,1 KiB");
    expect(filesLine([file("patch", 553), file("notes", 400)])).toBe("2 Dateien, 953 Byte");
    expect(filesLine([file("patch", 553)])).toBe("1 Datei, 553 Byte");
  });

  it("subjects_for_now_and_for_a_version", () => {
    // Local time, two digits each.
    expect(commitSubjectNow(new Date(2026, 9, 31, 14, 5))).toBe("Stand 2026-10-31 14:05");
    expect(commitSubjectNow(new Date(2027, 0, 2, 9, 7))).toBe("Stand 2027-01-02 09:07");
    expect(publishedSubject("0.1.0")).toBe("Veröffentlicht v0.1.0");
  });

  it("names_the_date_the_phases_and_what_is_handed_on", () => {
    expect(dateText("2026-10-31T10:00:00Z")).toBe("31.10.2026");
    expect(Object.fromEntries((["build", "encode", "verify", "write", "record"] as const).map((phase) => [phase, phaseText(phase)]))).toEqual({
      build: "Speichern und bauen …",
      encode: "Patch kodieren …",
      verify: "Selbstprüfung …",
      write: "Dateien schreiben …",
      record: "Veröffentlichung eintragen …",
    });
    expect(noRomContent()).toBe("Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.");
  });

  it("publishing_waits_for_a_found_base_and_a_readable_list", () => {
    expect(publishLocked(found())).toBeNull();
    expect(publishLocked({ state: "unassigned", suggestion: null })).toBe("Erst eine Basis-ROM zuordnen.");
    expect(publishLocked(missing())).toBe("Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).");
    expect(publishLocked({ state: "unreadable", reason: "Basismarke nicht lesbar." })).toBe("Basismarke nicht lesbar.");
    // A broken list locks too; the base goes first.
    expect(publishLocked(found(), "Die Liste der Veröffentlichungen ist nicht lesbar: x")).toBe("Die Liste der Veröffentlichungen ist nicht lesbar: x");
    expect(publishLocked({ state: "unassigned", suggestion: null }, "kaputt")).toBe("Erst eine Basis-ROM zuordnen.");
    // Not read yet: nothing is known against it (the wizard says why when the release is refused).
    expect(publishLocked(null)).toBeNull();
    expect(publishLocked(null, null)).toBeNull();
  });
});
