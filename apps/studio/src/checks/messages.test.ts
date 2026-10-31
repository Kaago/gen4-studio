import { describe, expect, it } from "vitest";
import { finding } from "./checksTesting";
import { AREA_LABELS, CHECK_CODES, CHECK_MESSAGES, areaOf, checkTitle, findingLabel, findingText, groupOf, progressText, skippedText } from "./messages";

/** The sentences of a text: a statement, then the way out (at least two). */
const sentences = (text: string) => text.split(/(?<=[.!?])\s+/).filter((s) => s.trim() !== "");

describe("check messages", () => {
  // The Rust test `commands::checks::messages_ts` holds this list against the kernel's catalogue (codes, order, group, severity).
  it("has_a_title_and_a_sentence_with_a_way_out_for_every_code", () => {
    expect(CHECK_CODES.length).toBe(45);
    const args = ["0", "1", "2", "3", "4", "5"];
    for (const code of CHECK_CODES) {
      const m = CHECK_MESSAGES[code]!;
      expect(m.title, code).not.toBe("");
      const text = findingText(finding(code, m.severity, { args }), "tuefteln");
      expect(text, code).not.toMatch(/[{}]/);
      expect(sentences(text).length, `${code}: ${text}`).toBeGreaterThanOrEqual(2);
      expect(text.endsWith("."), code).toBe(true);
    }
  });

  it("fills_the_kernel_arguments_into_the_sentence_with_four_digit_numbers_and_german_field_names", () => {
    expect(findingText(finding("warp.dest-warp", "error", { args: ["2", "414", "5", "411", "3"] }), "tuefteln")).toBe(
      "Die Tür führt zu Warp 5 in Header 0414, aber dessen Eventdatei 0411 hat nur 3 Warps. Wähle einen anderen Ziel-Warp oder lege dort einen Warp an.",
    );
    expect(findingText(finding("header.ref-range", "error", { args: ["12", "event_file_id", "900", "620"] }), "tuefteln")).toBe(
      "Header 0012 nennt als Eventdatei 900, aber es gibt nur 620. Wähle im Header einen gültigen Wert.",
    );
    expect(findingText(finding("text.too-wide", "warning", { args: ["433", "1", "1", "240", "216"] }), "tuefteln")).toBe("Die Zeile ist 240 px breit, das Textfenster nur 216 px. Brich sie um oder kürze den Text.");
  });

  it("says_one_file_in_the_singular", () => {
    const changed = (n: string) => findingText(finding("build.unpacked-not-packed", "warning", { args: ["zFoo", n] }), "tuefteln");
    expect(changed("1")).toMatch(/^Im Ordner unpacked\/zFoo hat sich 1 Datei geändert, /);
    expect(changed("3")).toMatch(/^Im Ordner unpacked\/zFoo haben sich 3 Dateien geändert, /);
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    // Every argument a field name the sentence could hold, and a label that names a header and its matrix field.
    const args = ["1", "matrix_id", "2", "3", "4", "5"];
    for (const code of CHECK_CODES) {
      const m = CHECK_MESSAGES[code]!;
      const f = finding(code, m.severity, { args, label: "Header 0012 · Matrix", subject: { kind: "header", index: 12 } });
      const words = `${checkTitle(code, "bauen")} ${findingText(f, "bauen")} ${findingLabel(f, "bauen")}`;
      expect(words, code).not.toMatch(/Raster|Matrix|Header/);
      // The way out stays: a statement and what to do.
      expect(sentences(findingText(f, "bauen")).length, code).toBeGreaterThanOrEqual(2);
    }
    for (const kind of ["matrix", "header table"]) {
      const f = finding("asset.unreadable", "error", { key: "unreadable:x", args: [kind, "3", "truncated"] });
      expect(findingText(f, "bauen"), kind).not.toMatch(/Raster|Matrix|Header/);
    }
    expect(findingText(finding("warp.dest-warp", "error", { args: ["2", "414", "5", "411", "3"] }), "bauen")).toBe(
      "Die Tür führt zu Warp 5 von Ort 0414, aber dessen Eventdatei 0411 hat nur 3 Warps. Wähle einen anderen Ziel-Warp oder lege dort einen Warp an.",
    );
    expect(findingText(finding("header.ref-range", "error", { args: ["12", "matrix_id", "900", "620"] }), "bauen")).toBe(
      "Ort 0012 nennt als Weltkarte 900, aber es gibt nur 620. Wähle beim Ort einen gültigen Wert.",
    );
    expect(findingLabel(finding("header.ref-range", "error", { label: "Header 0012 · Matrix" }), "bauen")).toBe("Ort 0012 · Weltkarte");
    expect(skippedText({ code: "script.message-missing", why: "28 script files that no header uses (common scripts) are not checked: 0101" }, "bauen")).toBe(
      "Nachricht fehlt: 28 Common-Scripts ohne Ort nicht geprüft.",
    );
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    expect(checkTitle("matrix.header-elsewhere", "tuefteln")).toBe("Header einer anderen Matrix");
    expect(findingText(finding("header.ref-range", "error", { args: ["12", "matrix_id", "900", "620"] }), "tuefteln")).toBe(
      "Header 0012 nennt als Matrix 900, aber es gibt nur 620. Wähle im Header einen gültigen Wert.",
    );
    expect(findingLabel(finding("header.ref-range", "error", { label: "Header 0012 · Matrix" }), "tuefteln")).toBe("Header 0012 · Matrix");
    expect(skippedText({ code: "script.message-missing", why: "28 script files that no header uses (common scripts) are not checked: 0101" }, "tuefteln")).toBe(
      "Nachricht fehlt: 28 Common-Scripts ohne Header nicht geprüft.",
    );
  });

  it("names_the_place_name_and_the_place_index_that_go_wrong", () => {
    const long = finding("place.name-too-long", "warning", { subject: { kind: "textArchive", index: 433 }, args: ["17", "Siebzehn Zeichen!", "17"] });
    expect(checkTitle("place.name-too-long", "tuefteln")).toBe("Ortsname zu lang");
    expect(findingText(long, "tuefteln")).toBe("Der Ortsname „Siebzehn Zeichen!“ ist 17 Zeichen lang; das Spiel zeigt höchstens 16. Kürze ihn im Inspector eines Gebiets (Ort umbenennen).");
    expect(findingText(long, "bauen")).toBe(findingText(long, "tuefteln"));
    const past = finding("place.index-out-of-bank", "error", { args: ["12", "140", "125"] });
    expect(checkTitle("place.index-out-of-bank", "tuefteln")).toBe("Ort fehlt in der Ortsnamen-Bank");
    expect(findingText(past, "tuefteln")).toBe("Header 0012 zeigt auf Ortsname 140, die Bank hat nur 125 Namen. Wähle im Inspector einen vorhandenen Ort.");
    expect(findingText(past, "bauen")).toBe("Das Gebiet 0012 zeigt auf einen Ort, den es nicht gibt. Wähle im Inspector einen vorhandenen Ort.");
  });

  it("formats_the_four_build_sentences", () => {
    const text = (code: string, args: string[] = []) => findingText(finding(code, CHECK_MESSAGES[code]!.severity, { subject: { kind: "project", index: 0 }, args }), "tuefteln");
    expect(checkTitle("build.hidden-file", "tuefteln")).toBe("Versteckte Datei im ROM-Ordner");
    expect(text("build.hidden-file", ["files/x/.DS_Store"])).toBe(
      "Die versteckte Datei files/x/.DS_Store liegt unter files/ und käme beim Bauen in die ROM; der Build bricht daran ab. „Entfernen“ verschiebt sie nach .g4/backup.",
    );
    expect(checkTitle("build.member-gap", "tuefteln")).toBe("Lücke im Archivordner");
    expect(text("build.member-gap", ["maps", "0005"])).toBe(
      "Im Ordner unpacked/maps passt 0005 nicht in die Reihe 0000, 0001, …; der Build bricht daran ab. Ergänze die fehlende Datei oder entferne die fremde.",
    );
    expect(checkTitle("build.unpacked-not-packed", "tuefteln")).toBe("Ordner wird nicht gepackt");
    expect(text("build.unpacked-not-packed", ["battleObj", "3"])).toBe(
      "Im Ordner unpacked/battleObj haben sich 3 Dateien geändert, das Studio packt ihn aber nicht; die Änderung fehlt in der gebauten ROM. Packe das Archiv in DSPRE neu oder nimm die Änderung zurück.",
    );
    expect(checkTitle("build.base-missing", "tuefteln")).toBe("Basis-ROM unbekannt");
    expect(text("build.base-missing")).toBe(
      "Das Projekt kennt seine Basis-ROM nicht. „Veröffentlichen“ und „Auf Original zurücksetzen“ brauchen sie. Ordne sie in Technik unter „Veröffentlichen“ zu.",
    );
    // Bauen says the same; the four are listed under the project.
    expect(findingText(finding("build.member-gap", "error", { args: ["maps", "0005"] }), "bauen")).toBe(text("build.member-gap", ["maps", "0005"]));
    expect(areaOf(finding("build.base-missing", "info", { subject: { kind: "project", index: 0 } }))).toBe("projekt");
  });

  it("names_the_asset_that_cannot_be_read", () => {
    expect(findingText(finding("asset.unreadable", "error", { key: "unreadable:event_file", args: ["event file", "411", "truncated"] }), "tuefteln")).toBe(
      "Eventdatei 0411 ist nicht lesbar (truncated). Stelle die Datei wieder her; bis dahin prüft das Studio ohne sie.",
    );
    expect(findingText(finding("asset.unreadable", "error", { subject: { kind: "project", index: 0 }, key: "unreadable:personal_data", args: ["personal data", "", "no archive"] }), "tuefteln")).toBe(
      "Das Archiv Pokémon-Daten ist nicht lesbar (no archive). Stelle die Datei wieder her; bis dahin prüft das Studio ohne sie.",
    );
    expect(findingText(finding("asset.unreadable", "error", { key: "locked:map", args: ["map", "7", "19 bytes"] }), "tuefteln")).toBe(
      "Karte 0007 ist gesperrt (19 bytes): das Studio würde sie nicht Byte für Byte zurückschreiben. Stelle die Datei wieder her, dann lässt sie sich ändern.",
    );
  });

  it("keeps_the_kernel_sentence_for_a_code_it_does_not_know", () => {
    const row = finding("warp.someday", "warning", { message: "Something is off." });
    expect(findingText(row, "tuefteln")).toBe("Something is off.");
    expect(checkTitle("warp.someday", "tuefteln")).toBe("warp.someday");
    expect(groupOf("warp.someday")).toBeNull();
  });

  it("knows_the_group_of_a_code_and_the_area_of_a_finding", () => {
    expect(groupOf("warp.dest-warp")).toBe("cheap");
    expect(groupOf("event.script-missing")).toBe("per_script");
    expect(groupOf("text.too-wide")).toBe("per_archive");
    expect(areaOf(finding("warp.dest-warp"))).toBe("welt");
    expect(areaOf(finding("event.script-missing"))).toBe("welt");
    expect(areaOf(finding("script.message-missing", "error", { subject: { kind: "scriptFile", index: 5 } }))).toBe("scripts");
    expect(areaOf(finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 } }))).toBe("texte");
    expect(areaOf(finding("trainer.no-party", "error", { subject: { kind: "trainer", index: 3 } }))).toBe("pokemon");
    expect(areaOf(finding("encounter.min-max", "info", { subject: { kind: "encounter", index: 3 } }))).toBe("pokemon");
    expect(areaOf(finding("asset.unreadable", "error", { subject: { kind: "project", index: 0 } }))).toBe("projekt");
    expect(AREA_LABELS.pokemon).toBe("Pokémon");
  });

  it("says_how_far_a_pass_got", () => {
    expect(progressText({ label: "texts", done: 312, total: 724 })).toBe("Prüft … Texte 312/724");
    expect(progressText({ label: "scripts", done: 3, total: 540 })).toBe("Prüft … Scripts 3/540");
    expect(progressText(null)).toBe("Prüft …");
  });

  it("says_what_was_not_checked_and_why", () => {
    expect(skippedText({ code: "script.message-missing", why: "scripts not adopted" }, "tuefteln")).toBe(
      "Nachricht fehlt: nicht geprüft, die Scripts sind nicht übernommen. Übernimm sie unter Script, dann prüft das Studio sie.",
    );
    expect(skippedText({ code: "script.message-missing", why: "28 script files that no header uses (common scripts) are not checked: 0101, 0164" }, "tuefteln")).toBe(
      "Nachricht fehlt: 28 Common-Scripts ohne Header nicht geprüft.",
    );
    expect(skippedText({ code: "event.script-missing", why: "script file 101 is not checked: rotom cannot decode it: bad jump" }, "tuefteln")).toBe(
      "Script fehlt: Scriptdatei 0101 nicht geprüft (rotom cannot decode it: bad jump). Behebe den Fehler in der Datei, dann prüft das Studio sie.",
    );
    expect(skippedText({ code: "header.level-target", why: "level script file 0012 is not checked: rotom cannot decode it: x" }, "tuefteln")).toBe(
      "Level-Script ruft fehlendes Script: Level-Script 0012 nicht geprüft (rotom cannot decode it: x). Behebe den Fehler in der Datei, dann prüft das Studio sie.",
    );
    expect(skippedText({ code: "text.too-wide", why: "the header table cannot be read (it names the dialog archives)" }, "tuefteln")).toBe(
      "Zeile zu breit: nicht geprüft (the header table cannot be read (it names the dialog archives)). Sieh unter „Datei nicht lesbar“ nach, was fehlt, und prüfe dann erneut.",
    );
    // The ignore file could not be read (ruling R15): the project opened with nothing ignored, and the file is not written.
    expect(skippedText({ code: "g4-checks.json", why: "key must be a string at line 1 column 3" }, "tuefteln")).toBe(
      "g4-checks.json ist nicht lesbar (key must be a string at line 1 column 3): nichts ist ausgeblendet oder abgeschaltet, und das Studio schreibt die Datei nicht. Repariere oder lösche sie, dann geht beides wieder.",
    );
  });
});
