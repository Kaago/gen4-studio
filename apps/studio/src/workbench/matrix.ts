import { MATRIX_EMPTY } from "../api";
import type { MatrixDto } from "../api";

const pad = (n: number) => String(n).padStart(4, "0");

/** Text of a matrix cell: "—" for an empty cell, else the map ("M0570"), plus the header (" H0415") when it has one. */
export function cellLabel(map: number | null, header: number | null): string {
  if (map === null) return "—";
  return header === null ? `M${pad(map)}` : `M${pad(map)} H${pad(header)}`;
}

/**
 * What shrinking (or growing) `old` to `width` x `height` throws away: the number of cells outside
 * the new size that hold a map, or a non-zero header or altitude.
 */
export function resizePreview(width: number, height: number, old: MatrixDto): { lostCells: number } {
  let lostCells = 0;
  for (let y = 0; y < old.height; y++) {
    for (let x = 0; x < old.width; x++) {
      if (x < width && y < height) continue;
      const i = y * old.width + x;
      if (old.maps[i] !== null || (old.headers?.[i] ?? 0) !== 0 || (old.altitudes?.[i] ?? 0) !== 0) lostCells++;
    }
  }
  return { lostCells };
}

/**
 * Reads a number field of the cell editor: an integer clamped into 0..max, "empty" for a blank
 * where `allowEmpty`, null for an input that has to be rejected (the field then shows its old value).
 */
export function parseCellField(draft: string, max: number, allowEmpty: boolean): number | "empty" | null {
  const text = draft.trim();
  if (text === "") return allowEmpty ? "empty" : null;
  const n = Number(text);
  if (!Number.isFinite(n)) return null;
  return Math.min(max, Math.max(0, Math.round(n)));
}

/** Whether two matrices place the same maps at the same heights on the same grid (what the 3D scene shows; no altitude section is all zeros). */
export function sameLayout(a: MatrixDto, b: MatrixDto): boolean {
  return (
    a.index === b.index &&
    a.width === b.width &&
    a.height === b.height &&
    a.maps.length === b.maps.length &&
    a.maps.every((m, i) => m === b.maps[i] && (a.altitudes?.[i] ?? 0) === (b.altitudes?.[i] ?? 0))
  );
}

/** The raw values of a cell in one line, as the depth Tüfteln writes them into it: "0104 0411 0" (map, header, altitude; a missing section is left out). */
export function rawCell(map: number, header: number | null, altitude: number | null): string {
  return [pad(map), ...(header === null ? [] : [pad(header)]), ...(altitude === null ? [] : [String(altitude)])].join(" ");
}

/** A u16 as four hex digits in capitals. */
const hex16 = (n: number) => n.toString(16).toUpperCase().padStart(4, "0");

/** One row of a matrix as it lies in the file: u16 hex of the maps (an empty cell is 0xFFFF) and of the headers, the altitudes as u8. */
export interface RawRow {
  maps: string[];
  headers: string[] | null;
  altitudes: number[] | null;
}

/** Row `y` of `m`: the raw lines of the section "Rohdaten, Zeile y". A section the matrix does not have is null. */
export function rawRow(m: MatrixDto, y: number): RawRow {
  const from = y * m.width;
  const to = from + m.width;
  return {
    maps: m.maps.slice(from, to).map((v) => hex16(v ?? MATRIX_EMPTY)),
    headers: m.headers ? m.headers.slice(from, to).map(hex16) : null,
    altitudes: m.altitudes ? m.altitudes.slice(from, to) : null,
  };
}
