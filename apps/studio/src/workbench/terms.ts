import type { Depth } from "../ui/frame/shellState";

/** The words of the world explorer: every panel takes label and explanation from here (spec §3). */
export type TermKey =
  | "worldMap"
  | "area"
  | "room"
  | "place"
  | "landscape"
  | "header"
  | "outside"
  | "matrix"
  | "map"
  | "warp"
  | "eventFile"
  | "areaData"
  | "patch"
  | "textArchive"
  | "message"
  | "scriptFile"
  | "script"
  | "levelScript"
  | "problem";

export const TERMS: Record<TermKey, { label: string; plural: string; tech: string | null; short: string }> = {
  worldMap: {
    label: "Weltkarte", plural: "Weltkarten",
    tech: null,
    short: "Die Karten einer Region nebeneinander, zum Beispiel Sinnoh; HeartGold und SoulSilver haben Johto und Kanto.",
  },
  area: {
    label: "Gebiet", plural: "Gebiete",
    tech: null,
    short: "Ein Stück einer Weltkarte mit einem Namen, etwa eine Stadt oder eine Route; Gebiete mit eigener Karte stehen unter \"Weitere Gebiete\".",
  },
  room: {
    label: "Raum", plural: "Räume",
    tech: null,
    short: "Innenräume, Höhlen und Dungeons; ein Raum gehört zu dem Gebiet, in dem sein Eingang liegt.",
  },
  place: {
    label: "Ort", plural: "Orte",
    tech: null,
    short: "Der Name, den das Spiel beim Betreten zeigt; Gebiete und Räume tragen ihn, mehrere können denselben haben.",
  },
  landscape: {
    label: "Landschaft", plural: "Landschaften",
    tech: null,
    short: "Die Zellen einer Weltkarte, die zu keinem Gebiet gehören: Meer, Berge und Rand.",
  },
  header: {
    label: "Header", plural: "Header",
    tech: "MapHeader",
    short: "Ein Eintrag der Ortstabelle: verweist auf Matrix, Eventdatei, Area-Data, Musik und Wetter. Nur in Tüfteln; Bauen sagt Gebiet, Raum oder Ort.",
  },
  outside: {
    label: "Außen", plural: "Außen",
    tech: null,
    short: "Ein Gebiet auf einer Weltkarte, im Freien: die Zellen der Weltkarte sagen, welches Gebiet wo gilt (die Region).",
  },
  matrix: {
    label: "Raster", plural: "Raster",
    tech: "Matrix",
    short: "Ein Raster aus Karten, 1 × 1 bis 255 × 255. Nur in Tüfteln; Bauen sagt Weltkarte.",
  },
  map: { label: "Karte", plural: "Karten", tech: "MapFile", short: "32 × 32 Kacheln mit Modell, Gebäuden und Kollision." },
  warp: { label: "Eingang / Ausgang", plural: "Eingänge / Ausgänge", tech: "Warp", short: "Ein Warp-Paar verbindet zwei Orte." },
  eventFile: {
    label: "Eventdatei", plural: "Eventdateien",
    tech: "EventFile",
    short: "Warps, Objekte, Trigger und Hintergrund-Events eines Ortes.",
  },
  areaData: { label: "Area-Data", plural: "Area-Data", tech: "AreaData", short: "Welche Texturen (Tilesets) ein Ort benutzt." },
  textArchive: {
    label: "Textarchiv", plural: "Textarchive",
    tech: "TextArchive",
    short: "Die Nachrichten eines Ortes; Scripts zeigen sie über die Nummer.",
  },
  message: { label: "Nachricht", plural: "Nachrichten", tech: null, short: "Ein Text eines Textarchivs; Scripts rufen ihn über seine Nummer ab." },
  scriptFile: {
    label: "Scriptdatei", plural: "Scriptdateien",
    tech: "ScriptFile",
    short: "Alle Scripts eines Ortes in einer Datei; Events nennen die Nummer #n darin.",
  },
  script: { label: "Script", plural: "Scripts", tech: null, short: "Ein Ablauf in rotom, zum Beispiel script Main #1." },
  levelScript: {
    label: "Level-Script", plural: "Level-Scripts",
    tech: null,
    short: "Läuft beim Laden oder Betreten der Karte oder wenn eine Variable einen Wert hat.",
  },
  problem: { label: "Problem", plural: "Probleme", tech: null, short: "Ein Fehler oder eine Warnung aus Compile, Text oder Sprachdienst." },
  patch: {
    label: "ROM-Patch", plural: "ROM-Patches",
    tech: null,
    short: "Eine dauerhafte Änderung am Spielcode. Der Patch „Dynamische Header“ verlegt die Header in eine Datei je Header, damit neue angelegt werden können; DSPRE kennt ihn.",
  },
};

/** An id as the Gate documents and the lists write it: four digits. */
export const id4 = (n: number) => String(n).padStart(4, "0");

/** The number of a member as a card writes it, spelled out (no "Nr."): "Nummer 0017". */
export const numberText = (n: number) => `Nummer ${id4(n)}`;

/** How an id is written next to a member; the header stays "Header" (the word the Gate documents use), the rest follows the technical name. */
const ID_NAME: Partial<Record<TermKey, string>> = { header: "Header" };

/** The technical name of a member with its id: "Matrix 0131", "Header 0415". */
export function techLabel(key: TermKey, id: number): string {
  const term = TERMS[key];
  return `${ID_NAME[key] ?? term.tech ?? term.label} ${id4(id)}`;
}

/**
 * The word for a header or a matrix at a depth: Bauen speaks of the game (an area outside, a room inside, a place when that is
 * not known; a world map), Tüfteln of the ROM ("Header", "Matrix").
 */
export function termFor(key: "header" | "matrix", depth: Depth, kind?: "area" | "room"): string {
  if (depth === "tuefteln") return key === "header" ? "Header" : "Matrix";
  if (key === "matrix") return TERMS.worldMap.label;
  return kind === undefined ? TERMS.place.label : TERMS[kind].label;
}
