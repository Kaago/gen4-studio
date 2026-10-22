/** The words of the world explorer: every panel takes label and explanation from here (spec §3). */
export type TermKey = "place" | "header" | "outside" | "room" | "matrix" | "map" | "warp" | "eventFile" | "areaData";

export const TERMS: Record<TermKey, { label: string; tech: string | null; short: string }> = {
  place: {
    label: "Ort",
    tech: null,
    short: "Alle Header mit demselben Ortsnamen; Header ohne Ortsnamen stehen unter \"Ohne Ortsnamen\".",
  },
  header: {
    label: "Header",
    tech: null,
    short: "Ein Eintrag der Ortstabelle: verweist auf Raster, Eventdatei, Area-Data, Musik und Wetter.",
  },
  outside: {
    label: "Außen",
    tech: null,
    short: "Ein Header, dessen Raster einen Header-Abschnitt hat: die Zellen des Rasters sagen, welcher Header wo gilt.",
  },
  room: {
    label: "Raum",
    tech: null,
    short: "Ein Header, dessen Raster keinen Header-Abschnitt hat: Innenräume, Höhlen und Dungeons.",
  },
  matrix: { label: "Raster", tech: "Matrix", short: "Ein Raster aus Karten, 1 × 1 bis 255 × 255." },
  map: { label: "Karte", tech: "MapFile", short: "32 × 32 Kacheln mit Modell, Gebäuden und Kollision." },
  warp: { label: "Eingang / Ausgang", tech: "Warp", short: "Ein Warp-Paar verbindet zwei Header." },
  eventFile: {
    label: "Eventdatei",
    tech: "EventFile",
    short: "Warps, Objekte, Trigger und Hintergrund-Events eines Headers.",
  },
  areaData: { label: "Area-Data", tech: "AreaData", short: "Welche Texturen (Tilesets) ein Header benutzt." },
};

/** An id as the Gate documents and the lists write it: four digits. */
export const id4 = (n: number) => String(n).padStart(4, "0");

/** The technical name of a member with its id: "Matrix 0131", "Header 0415". */
export function techLabel(key: TermKey, id: number): string {
  const term = TERMS[key];
  return `${term.tech ?? term.label} ${id4(id)}`;
}
