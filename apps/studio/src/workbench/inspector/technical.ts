import { headerFamily } from "../../api";
import type { AreaDataDto, AssetKind, BuildingDto, EventKind, MapHeader } from "../../api";
import { t } from "../../i18n";
import { formatNumber } from "../../ui/primitives";
import type { EventData } from "../../world/events";
import type { InspectorModel } from "../Inspector";
import { id4, termLabel } from "../terms";
import { areaFieldSpecs, areaValue } from "./areaFields";
import { eventFieldSpecs, EVENT_FIELDS } from "./eventFields";
import { fieldValue, headerFieldSpecs } from "./fields";

/** One raw field of a record: the key as the codec calls it, the value in decimal, and in hex when that means something. */
export interface TechnicalRow {
  key: string;
  dec: number;
  /** `0x` plus capitals; null for a value that is not a whole, non-negative number (a building's position). */
  hex: string | null;
}

/** The loaded record of an editor, as far as the technical section needs it. */
export type TechnicalSource =
  | { kind: "header"; header: MapHeader }
  | { kind: "area"; area: AreaDataDto }
  | { kind: "building"; building: BuildingDto }
  | { kind: "event"; eventKind: EventKind; event: EventData; family: string | null }
  /** A cell of a matrix; a section the matrix does not have (or an empty cell) is null. */
  | { kind: "matrixCell"; map: number | null; header: number | null; altitude: number | null };

function row(key: string, dec: number): TechnicalRow {
  return { key, dec, hex: Number.isInteger(dec) && dec >= 0 ? formatNumber(dec, "hex") : null };
}

const BUILDING_KEYS = ["model_id", "x", "y", "z", "sx", "sy", "sz"] as const;

/**
 * Every field of the record by its raw key, in the order of the form: all `FieldSpec` keys, then the fields the form shows but does
 * not edit (a warp's destination header, the height of an object or sign). Switches count as 0 or 1.
 */
export function technicalRows(source: TechnicalSource): TechnicalRow[] {
  switch (source.kind) {
    case "header":
      return headerFieldSpecs(headerFamily(source.header)).map((spec) => row(spec.key, fieldValue(source.header, spec.key)));
    case "area":
      return areaFieldSpecs(source.area.family).map((spec) => row(spec.key, areaValue(source.area, spec.key)));
    case "building":
      return BUILDING_KEYS.map((key) => row(key, source.building[key]));
    case "event": {
      const keys: string[] = eventFieldSpecs(source.eventKind, source.family).map((spec) => spec.key);
      if (source.eventKind === "warp") keys.push("dest_header_id");
      if (source.eventKind === "object" || source.eventKind === "bg") keys.push("y");
      return keys.map((key) => row(key, EVENT_FIELDS.value(source.event, key)));
    }
    case "matrixCell":
      return [
        { key: "map", value: source.map },
        { key: "header", value: source.header },
        { key: "altitude", value: source.altitude },
      ].flatMap(({ key, value }) => (value === null ? [] : [row(key, value)]));
  }
}

/** The archive and the member a model's record lives in, as a line: "Eventdatei 0411, Index 3". Empty for a model without an editor. */
export function assetLine(model: InspectorModel): string {
  switch (model.kind) {
    case "header":
      return t("workbench:technical.headerLine", { term: termLabel("header"), id: id4(model.id) });
    case "area":
      return `${termLabel("areaData")} ${id4(model.id)}`;
    case "building":
      return t("workbench:technical.indexLine", { term: termLabel("map"), id: id4(model.map), index: model.index });
    case "event":
      return t("workbench:technical.indexLine", { term: termLabel("eventFile"), id: id4(model.eventFile), index: model.index });
    case "matrixCell":
      return t("workbench:technical.cellLine", { term: termLabel("matrix"), id: id4(model.matrix), x: model.x, y: model.y });
    case "none":
    case "eventHover":
    case "tile":
      return "";
  }
}

/** The folder below `unpacked/` of each archive; the names are the same in every game family (`archives.yaml`). */
const UNPACKED: Partial<Record<AssetKind, string>> = {
  Map: "maps",
  Matrix: "matrices",
  AreaData: "areaData",
  Event: "eventFiles",
};

function member(kind: AssetKind, index: number): string {
  return `unpacked/${UNPACKED[kind]}/${id4(index)}`;
}

/**
 * The file of the model's record, relative to the project folder (`unpacked/eventFiles/0411`). Null when the record has none of its
 * own: a header lives in the header table, one file for all of them.
 */
export function filePath(model: InspectorModel): string | null {
  switch (model.kind) {
    case "area":
      return member("AreaData", model.id);
    case "building":
      return member("Map", model.map);
    case "event":
      return member("Event", model.eventFile);
    case "matrixCell":
      return member("Matrix", model.matrix);
    case "header":
    case "none":
    case "eventHover":
    case "tile":
      return null;
  }
}
