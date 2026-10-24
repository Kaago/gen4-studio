import type { MapPart, PartReport } from "../../api";
import type { CheckState } from "../../ui/primitives/CheckList";

/** The German name of a map part. */
export const PART_NAME: Record<MapPart, string> = {
  model: "Modell",
  permissions: "Kollision",
  buildings: "Gebäude",
  bdhc: "BDHC (Laufhöhen)",
  bgs: "Hintergrundklang",
  whole: "Ganze Karte",
};

const NOTES: [RegExp, (m: RegExpMatchArray) => string][] = [
  [
    /^terrain \(BDHC\) stays unchanged/,
    () => "BDHC bleibt unverändert, Laufhöhen prüfen",
  ],
  [
    /^(\d+) texture or palette names missing in map texture pack (\d+)/,
    (m) => `${m[1]} Texturnamen fehlen im Gebiets-Paket ${m[2]}`,
  ],
  [
    /^embedded texture block removed/,
    () => "Eingebetteter Texturblock wird entfernt",
  ],
  [
    /its buildings are left out unless taken over/,
    () =>
      "Karte der anderen Spielfamilie: ihre Gebäude bleiben weg, außer sie werden übernommen",
  ],
  [
    /^an empty background sound block is added/,
    () => "Ein leerer Hintergrundklang-Block wird angelegt",
  ],
  [
    /^(\d+) trailing bytes after the buildings are kept/,
    (m) => `${m[1]} Bytes nach den Gebäuden bleiben erhalten`,
  ],
  [/^texture names not checked/, () => "Texturnamen nicht geprüft"],
];

/** A kernel hint (English) as the sentence the dialog shows; an unknown hint stays as it is. */
export function noteText(note: string): string {
  for (const [re, say] of NOTES) {
    const m = note.match(re);
    if (m) return say(m);
  }
  return note;
}

/** Whether the report is a whole map file of the other game family (only then can its buildings be taken over). */
export const isForeignWhole = (r: PartReport) =>
  r.part === "whole" &&
  r.notes.some((n) =>
    n.includes("its buildings are left out unless taken over"),
  );

/** "1 Teil" / "3 Teile". */
export const partsWord = (n: number) => `${n} ${n === 1 ? "Teil" : "Teile"}`;

/** How a part shows in the check list: an otherwise fine part with 0 bytes (no buildings, an empty BGS) is a hint, not a ready part. */
export function partView(r: PartReport): {
  state: CheckState;
  message: string | null;
} {
  if (r.state.kind === "ok")
    return r.size === 0
      ? { state: "info", message: "Leer, wird nicht übernommen" }
      : { state: "ok", message: null };
  return { state: r.state.kind, message: r.state.message };
}

/** Whether the dialog chooses the part up front: not when it cannot be imported or is empty (it stays choosable). */
export const preselected = (r: PartReport) =>
  r.state.kind !== "error" && r.size > 0;
