import type { BuildingDto, BuildingPatch } from "../../api";
import { t } from "../../i18n";
import type { RecordBackend, RecordFields } from "./headerCommit";

/** The editable number fields of a building, in display order. `step` is the arrow step of the input. */
export interface BuildingFieldSpec {
  key: "x" | "y" | "z" | "sx" | "sy" | "sz";
  label: string;
  step: number;
}

/** The position fields, labelled in the language that is shown when called. */
export const buildingPositionFields = (): BuildingFieldSpec[] => [
  { key: "x", label: t("workbench:buildingFields.x"), step: 0.5 },
  { key: "y", label: t("workbench:buildingFields.y"), step: 0.5 },
  { key: "z", label: t("workbench:buildingFields.z"), step: 0.5 },
];

/** The scale fields, labelled in the language that is shown when called. */
export const buildingScaleFields = (): BuildingFieldSpec[] => [
  { key: "sx", label: t("workbench:buildingFields.sx"), step: 0.25 },
  { key: "sy", label: t("workbench:buildingFields.sy"), step: 0.25 },
  { key: "sz", label: t("workbench:buildingFields.sz"), step: 0.25 },
];

const KEYS = ["model_id", "x", "y", "z", "sx", "sy", "sz"] as const;

function value(b: BuildingDto, key: string): number {
  return Number((b as unknown as Record<string, unknown>)[key]);
}

export const BUILDING_FIELDS: RecordFields<BuildingDto> = {
  value,
  with: (b, key, v) => ({ ...b, [key]: v }),
};

/** The fields in which `after` differs from `before`; the rotation is never part of it. */
export function buildingPatch(before: BuildingDto, after: BuildingDto): BuildingPatch {
  const patch: BuildingPatch = {};
  for (const key of KEYS) {
    if (before[key] !== after[key]) patch[key] = after[key];
  }
  return patch;
}

/** A typed number, or null when the text is blank or not a finite number. */
export function parseNumberField(text: string): number | null {
  if (text.trim() === "") return null;
  const n = Number(text);
  return Number.isFinite(n) ? n : null;
}

/**
 * Backend of an `EditSession` over the buildings of ONE map: the session id is the building's
 * index. A write sends only the fields that differ from what the session last read or wrote,
 * so values that were not edited cannot be rounded or reset by a whole-record write.
 */
export function buildingBackend(
  map: number,
  deps: {
    getBuilding: (map: number, index: number) => Promise<BuildingDto>;
    update: (map: number, index: number, patch: BuildingPatch) => Promise<void>;
  },
): RecordBackend<BuildingDto> {
  let last: BuildingDto | null = null;
  return {
    get: async (index) => (last = await deps.getBuilding(map, index)),
    set: async (index, record) => {
      const patch = last ? buildingPatch(last, record) : {};
      await deps.update(map, index, patch);
      last = record;
    },
  };
}
