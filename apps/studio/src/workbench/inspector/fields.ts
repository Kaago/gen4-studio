import { headerFamily } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { AssetKind, HeaderFamily, MapHeader } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { termLabel } from "../terms";

/**
 * What an id field points at; pickers get their upper limit from the archive's member count. `music` marks a number field (`u16`) that
 * names a music piece: Erzählen picks it by name (`MusicField`), Bauen and Tüfteln show the name beside the number.
 */
export type PickKind = "matrix" | "areaData" | "eventFile" | "textArchive" | "script" | "header" | "mapTexture" | "buildingTexture" | "music";

export interface FieldSpec {
  /** Property name in the serialized header (uxie struct). */
  key: string;
  label: string;
  /** `place`: the header's place, chosen by name from the location bank (`PlacePicker`); a number field where no picker can be shown. */
  kind: "u8" | "u16" | "bool" | "pick" | "select" | "place";
  pick?: PickKind;
  /** A `place` field stored in 16 bits (Diamond/Pearl); 8 bits otherwise. */
  wide?: boolean;
  /** Choices of a `select` field (a stored value without a choice is still shown). */
  options?: { value: number; label: string }[];
  /** The lowest depth that shows the field; absent: Bauen. */
  from?: Depth;
}

/** The lowest depth that shows a field (`from`, else Bauen). */
export const fieldFrom = (spec: { from?: Depth }): Depth => spec.from ?? "bauen";

/** The fields `depth` shows: in Erzählen those from Erzählen, in Bauen and Tüfteln all of them. */
export function shownFields<T extends { from?: Depth }>(specs: readonly T[], depth: Depth): T[] {
  // In 3d.2 no field starts at Bauen or Tüfteln with a lower one between, so the rank of the field's depth decides.
  return specs.filter((spec) => depthAtLeast(depth, fieldFrom(spec)));
}

const u8 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u8" });
const u16 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u16" });
const pick = (key: string, label: string, kind: PickKind): FieldSpec => ({ key, label, kind: "pick", pick: kind });
const place = (wide: boolean): FieldSpec => ({ key: "location_name", label: "Ortsname", kind: "place", from: "erzaehlen", ...(wide && { wide }) });

/** Storage width of the id fields that are pickers (all others carry their width in `kind`). */
const PICK_U8 = new Set(["area_data_id"]);

const areaData: FieldSpec = { ...pick("area_data_id", "Area-Data", "areaData"), from: "erzaehlen" };
const matrix = pick("matrix_id", "Matrix", "matrix");
const script = (): FieldSpec => pick("script_file_id", termLabel("scriptFile"), "script");
const levelScript = (): FieldSpec => pick("level_script_id", termLabel("levelScript"), "script");
const textArchive = pick("text_archive_id", "Textarchiv", "textArchive");
const musicDay: FieldSpec = { ...u16("music_day_id", "Musik Tag"), pick: "music", from: "erzaehlen" };
const musicNight: FieldSpec = { ...u16("music_night_id", "Musik Nacht"), pick: "music", from: "erzaehlen" };
const eventFile = pick("event_file_id", "Eventdatei", "eventFile");
const weather = u8("weather_id", "Wetter");
const camera = u8("camera_angle_id", "Kamera");
const battleBackground = u8("battle_background", "Kampfhintergrund");
const flags = u8("flags", "Flags");

/**
 * The editable fields of a header in the order of the uxie struct. The fields uxie calls
 * `unknown*` are left out: they are kept verbatim, not edited.
 */
const SPECS: Record<HeaderFamily, () => FieldSpec[]> = {
  DP: () => [
    areaData,
    matrix,
    script(),
    levelScript(),
    textArchive,
    musicDay,
    musicNight,
    u16("wild_pokemon", "Begegnungen"),
    eventFile,
    place(true),
    weather,
    camera,
    u8("location_specifier", "Ortskennung"),
    battleBackground,
    flags,
  ],
  Pt: () => [
    areaData,
    matrix,
    script(),
    levelScript(),
    textArchive,
    musicDay,
    musicNight,
    u16("wild_pokemon", "Begegnungen"),
    eventFile,
    place(false),
    u8("area_icon", "Gebietssymbol"),
    weather,
    camera,
    u8("location_specifier", "Ortskennung"),
    battleBackground,
    flags,
  ],
  HGSS: () => [
    u8("wild_pokemon", "Begegnungen"),
    areaData,
    u8("worldmap_x", "Weltkarte X"),
    u8("worldmap_y", "Weltkarte Y"),
    matrix,
    script(),
    levelScript(),
    textArchive,
    musicDay,
    musicNight,
    eventFile,
    place(false),
    u8("area_icon", "Gebietssymbol"),
    { key: "kanto_flag", label: "Kanto", kind: "bool" },
    weather,
    u8("location_type", "Ortstyp"),
    camera,
    u8("follow_mode", "Folgemodus"),
    battleBackground,
    flags,
  ],
};

export function headerFieldSpecs(family: HeaderFamily): FieldSpec[] {
  return SPECS[family]();
}

function typeMax(spec: FieldSpec): number {
  switch (spec.kind) {
    case "bool":
      return 1;
    case "u8":
    case "select":
      return 0xff;
    case "u16":
      return 0xffff;
    case "place":
      return spec.wide ? 0xffff : 0xff;
    case "pick":
      return PICK_U8.has(spec.key) ? 0xff : 0xffff;
  }
}

/** The storage width of the field: what a number field in front of it holds at most. */
export function fieldBits(spec: FieldSpec): 8 | 16 {
  return typeMax(spec) === 0xff ? 8 : 16;
}

const PICK_ASSET: Partial<Record<PickKind, AssetKind>> = {
  matrix: "Matrix",
  areaData: "AreaData",
  eventFile: "Event",
  textArchive: "TextArchive",
  script: "Script",
  mapTexture: "MapTexture",
  buildingTexture: "BuildingTexture",
};

/** Highest valid index of a picker (member count − 1); undefined when the target has no known count. */
export function pickMax(kind: PickKind, counts: Partial<Record<AssetKind, number>>): number | undefined {
  const asset = PICK_ASSET[kind];
  const count = asset === undefined ? undefined : counts[asset];
  return count === undefined ? undefined : Math.max(0, count - 1);
}

type Fields = Record<string, number | boolean>;

function fields(header: MapHeader): Fields {
  return Object.values(header)[0] as unknown as Fields;
}

/** The field as a number (booleans as 0/1). */
export function fieldValue(header: MapHeader, key: string): number {
  return Number(fields(header)[key]);
}

/** A copy of `header` with `key` set; booleans stay booleans. */
export function withField(header: MapHeader, key: string, value: number): MapHeader {
  const current = fields(header);
  const next = { ...current, [key]: typeof current[key] === "boolean" ? value !== 0 : value };
  return { [headerFamily(header)]: next } as unknown as MapHeader;
}
