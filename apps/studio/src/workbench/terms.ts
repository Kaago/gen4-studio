/** The words of the world explorer: every panel takes label and explanation from here (spec §3). */
export type TermKey = "place" | "header" | "outside" | "room" | "matrix" | "map" | "warp" | "eventFile" | "areaData" | "patch" | "textArchive" | "message";

export const TERMS: Record<TermKey, { label: string; plural: string; tech: string | null; short: string }> = {
  place: {
    label: "Ort", plural: "Orte",
    tech: null,
    short: "Alle Header mit demselben Ortsnamen; Header ohne Ortsnamen stehen unter \"Ohne Ortsnamen\".",
  },
  header: {
    label: "Header", plural: "Header",
    tech: "MapHeader",
    short: "Ein Eintrag der Ortstabelle: verweist auf Raster, Eventdatei, Area-Data, Musik und Wetter.",
  },
  outside: {
    label: "Außen", plural: "Außen",
    tech: null,
    short: "Ein Header, dessen Raster einen Header-Abschnitt hat (die Region, meist Matrix 0): die Zellen des Rasters sagen, welcher Header wo gilt.",
  },
  room: {
    label: "Raum", plural: "Räume",
    tech: null,
    short: "Ein Header, dessen Raster keinen Header-Abschnitt hat: Innenräume, Höhlen und Dungeons.",
  },
  matrix: { label: "Raster", plural: "Raster", tech: "Matrix", short: "Ein Raster aus Karten, 1 × 1 bis 255 × 255." },
  map: { label: "Karte", plural: "Karten", tech: "MapFile", short: "32 × 32 Kacheln mit Modell, Gebäuden und Kollision." },
  warp: { label: "Eingang / Ausgang", plural: "Eingänge / Ausgänge", tech: "Warp", short: "Ein Warp-Paar verbindet zwei Header." },
  eventFile: {
    label: "Eventdatei", plural: "Eventdateien",
    tech: "EventFile",
    short: "Warps, Objekte, Trigger und Hintergrund-Events eines Headers.",
  },
  areaData: { label: "Area-Data", plural: "Area-Data", tech: "AreaData", short: "Welche Texturen (Tilesets) ein Header benutzt." },
  textArchive: {
    label: "Textarchiv", plural: "Textarchive",
    tech: "TextArchive",
    short: "Die Nachrichten eines Headers; Scripts zeigen sie über die Nummer.",
  },
  message: { label: "Nachricht", plural: "Nachrichten", tech: null, short: "Ein Text eines Textarchivs; Scripts rufen ihn über seine Nummer ab." },
  patch: {
    label: "ROM-Patch", plural: "ROM-Patches",
    tech: null,
    short: "Eine dauerhafte Änderung am Spielcode. Der Patch „Dynamische Header“ verlegt die Header in eine Datei je Header, damit neue angelegt werden können; DSPRE kennt ihn.",
  },
};

/** An id as the Gate documents and the lists write it: four digits. */
export const id4 = (n: number) => String(n).padStart(4, "0");

/** How an id is written next to a member; the header stays "Header" (the word the Gate documents use), the rest follows the technical name. */
const ID_NAME: Partial<Record<TermKey, string>> = { header: "Header" };

/** The technical name of a member with its id: "Matrix 0131", "Header 0415". */
export function techLabel(key: TermKey, id: number): string {
  const term = TERMS[key];
  return `${ID_NAME[key] ?? term.tech ?? term.label} ${id4(id)}`;
}
