import type { HeaderSummary } from "../../api";
import type { Area, PickerItem } from "../../ui/primitives";
import { headerLabel } from "../headers";
import type { NavTarget } from "../navigation";
import { id4 } from "../terms";
import type { FieldSpec } from "./fields";

/** How an id field reads as a link: the chip's area colour and words, and where a click leads (null: nowhere to go). */
export interface PickReference {
  area: Area;
  label: string;
  tech?: string;
  target: NavTarget | null;
}

/**
 * The reference a `pick` field (or a warp's destination header, `pick: "header"`) shows for `value`. Matrices, area data,
 * script files and headers belong to the world, text archives to the texts. Targets that have no place to open
 * (a level script of 0 meaning none, an event file no header uses, a tileset, a header that does not exist) are null.
 */
export function referenceFor(spec: FieldSpec, value: number, headers: readonly HeaderSummary[]): PickReference {
  const plain = (area: Area, target: NavTarget | null, tech: string = id4(value)): PickReference => ({ area, label: spec.label, tech, target });
  switch (spec.pick) {
    case "matrix":
      return plain("welt", { kind: "matrixCell", matrix: value, x: 0, y: 0 });
    case "areaData":
      return plain("welt", { kind: "area", id: value });
    case "eventFile": {
      const owner = headers.find((h) => h.event_file_id === value);
      return plain("welt", owner ? { kind: "header", id: owner.id } : null);
    }
    case "textArchive":
      return plain("texte", { kind: "textArchive", index: value });
    case "script":
      // A level script of 0 is "none": nothing to open.
      return spec.key === "level_script_id" && value === 0 ? plain("welt", null, "keins") : plain("welt", { kind: "script", file: value });
    case "header": {
      const header = headers.find((h) => h.id === value);
      if (!header) return { area: "welt", label: "unbekannt", tech: id4(value), target: null };
      return { area: "welt", label: headerLabel(header), tech: `${id4(header.id)} · ${header.internal_name}`, target: { kind: "header", id: header.id } };
    }
    case "mapTexture":
    case "buildingTexture":
    case undefined:
      return plain("welt", null);
  }
}

/** The choices of a picker field: every index from 0 to `max`, found by label, number or the digits typed. */
export function pickerItems(spec: FieldSpec, max: number, headers: readonly HeaderSummary[]): PickerItem[] {
  return Array.from({ length: max + 1 }, (_, n) => {
    const { area, label, tech } = referenceFor(spec, n, headers);
    return { id: n, label, tech, area };
  });
}
