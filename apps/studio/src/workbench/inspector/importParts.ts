import i18next from "i18next";
import type { BudgetState, MapPart, PartMessage, PartReport, TexturePreview, TmjError, TmjImportReport } from "../../api";
import { t } from "../../i18n";
import type { PickerItem } from "../../ui/primitives";
import type { CheckState } from "../../ui/primitives/CheckList";
import { id4, termLabel } from "../terms";

/** The name of a map part in the language that is shown. */
export const partName = (part: MapPart): string => t(`workbench:importParts.partName.${part}`);

/**
 * The catalog key of the sentence of every code the part checks give (`g4_kernel::map_import::PartCode`; the Rust test
 * `commands::map_import::tests::every_part_code_has_its_german_sentence` holds this list against `PartCode::ALL`, one line per code). The
 * sentences live in `workbench:importParts.part.<key>`: an error or a warning with its way out, a hint as a note. In a sentence `{n}` is
 * argument n as the kernel gives it, `{n:num}` a number in the digits of the language („65.000“, "65,000"), `{n:family}` a game family,
 * `{n:dec}` a decimal number with the language's decimal mark („7,6“, "7.6"), `{n:ignored}` the parts of a model left out (tokens joined
 * by `,`, each as a word of the language). `fill` puts them in; i18next does not (it interpolates `{{name}}`).
 */
export const PART_TEXTS: Readonly<Record<string, string>> = {
  "model.invalid": "modelInvalid",
  "model.blocks": "modelBlocks",
  "model.count": "modelCount",
  "model.too-big": "modelTooBig",
  "model.unaligned": "modelUnaligned",
  "model.texture-removed": "modelTextureRemoved",
  "model.bdhc-unchanged": "modelBdhcUnchanged",
  "model.textures-missing": "modelTexturesMissing",
  "model.textures-unreadable": "modelTexturesUnreadable",
  "model.textures-no-header": "modelTexturesNoHeader",
  "model.textures-unchecked": "modelTexturesUnchecked",
  "permissions.size": "permissionsSize",
  "buildings.size": "buildingsSize",
  "buildings.unreadable": "buildingsUnreadable",
  "bdhc.magic": "bdhcMagic",
  "bdhc.short": "bdhcShort",
  "bdhc.too-big": "bdhcTooBig",
  "bdhc.counters": "bdhcCounters",
  "bgs.family": "bgsFamily",
  "bgs.short": "bgsShort",
  "bgs.signature": "bgsSignature",
  "bgs.length": "bgsLength",
  "whole.unreadable": "wholeUnreadable",
  "whole.invalid": "wholeInvalid",
  "whole.bgs-dropped": "wholeBgsDropped",
  "whole.foreign": "wholeForeign",
  "whole.blank-bgs": "wholeBlankBgs",
  "whole.buildings-tail": "wholeBuildingsTail",
  "part.duplicate": "partDuplicate",
  "bdhc.structure": "bdhcStructure",
  "model.source-unreadable": "modelSourceUnreadable",
  "model.file-missing": "modelFileMissing",
  "model.file-outside": "modelFileOutside",
  "model.no-geometry": "modelNoGeometry",
  "model.material-untextured": "modelMaterialUntextured",
  "model.uv-missing": "modelUvMissing",
  "model.out-of-range": "modelOutOfRange",
  "model.uv-range": "modelUvRange",
  "model.texture-count": "modelTextureCount",
  "model.converted": "modelConverted",
  "model.faces-dropped": "modelFacesDropped",
  "model.parts-ignored": "modelPartsIgnored",
  "part.no-area": "partNoArea",
  "texture.side": "textureSide",
  "texture.name": "textureName",
  "texture.taken": "textureTaken",
  "texture.pack-full": "texturePackFull",
  "texture.unreadable": "textureUnreadable",
  "texture.new": "textureNew",
  "texture.reused": "textureReused",
  "texture.vram-high": "textureVramHigh",
};

/** The catalog key of each game family's word (`workbench:importParts.family.<key>`). */
const FAMILY_KEYS: Readonly<Record<string, "pt" | "hgss" | "dp">> = { Pt: "pt", Hgss: "hgss", Dp: "dp" };

/** The catalog key of each token of `model.parts-ignored` (`workbench:importParts.ignored.<key>`; an unknown token stays as it is). */
const IGNORED_KEYS: Readonly<Record<string, "lines" | "points" | "animations" | "skins" | "cameras" | "lights" | "vertexColours" | "textureOptions">> = {
  lines: "lines",
  points: "points",
  animations: "animations",
  skins: "skins",
  cameras: "cameras",
  lights: "lights",
  "vertex-colours": "vertexColours",
  "texture-options": "textureOptions",
};

/**
 * What a part check says, as the dialog shows it: the sentence of its code in the language that is shown; the kernel's (English)
 * sentence for a code it does not know.
 */
export function partText({ code, args, message }: PartMessage): string {
  const key = PART_TEXTS[code];
  return key === undefined ? message : fill(t(`workbench:importParts.part.${key}` as "workbench:importParts.part.modelInvalid"), args);
}

/**
 * The catalog key of the sentence of the codes of world maps and the Tiled exchange (6.5b) that the import dialog and „Quelle“ show
 * (`workbench:importParts.world.<key>`); apart from PART_TEXTS, which holds exactly `PartCode::ALL`. Placeholders as in PART_TEXTS.
 */
export const WORLD_TEXTS: Readonly<Record<string, string>> = {
  "world.side-file": "worldSideFile",
  "world.adopt-texture": "worldAdoptTexture",
  "tmj.buildings-ignored": "tmjBuildingsIgnored",
};

/** A message of a world map or a TMJ import: its sentence from WORLD_TEXTS, else as a part check says it. */
export function worldText(m: PartMessage): string {
  const key = WORLD_TEXTS[m.code];
  return key === undefined ? partText(m) : fill(t(`workbench:importParts.world.${key}` as "workbench:importParts.world.worldSideFile"), m.args);
}

/** A whole number in the digits of the language that is shown („65.000“, "65,000"). */
const num = (n: number) => n.toLocaleString(i18next.language);

/** The decimal mark of the language that is shown („,“ in German, "." in English). */
const decimalMark = () => new Intl.NumberFormat(i18next.language).formatToParts(1.5).find((p) => p.type === "decimal")?.value ?? ".";

/** `text` with its placeholders filled from `args` (see PART_TEXTS). */
function fill(text: string, args: readonly string[]): string {
  return text.replace(/\{(\d+)(?::(num|family|dec|ignored))?\}/g, (_, n: string, format?: string) => {
    const value = args[Number(n)] ?? "";
    if (format === "num" && /^\d+$/.test(value)) return num(Number(value));
    if (format === "family") {
      const key = FAMILY_KEYS[value];
      return key === undefined ? value : t(`workbench:importParts.family.${key}`);
    }
    if (format === "dec") return value.replace(".", decimalMark());
    if (format === "ignored") {
      return value
        .split(",")
        .map((token) => {
          const key = IGNORED_KEYS[token];
          return key === undefined ? token : t(`workbench:importParts.ignored.${key}`);
        })
        .join(", ");
    }
    return value;
  });
}

const FORMAT_WORDS: Readonly<Record<string, string>> = { obj: "OBJ", gltf: "glTF" };

/**
 * The lines under a converted part (model or textures): „OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen“ for a model, one per texture
 * „g4_imp_cube · 16 × 16 · 16 Farben · neu“ (a texture of the pack „g4_imp_cube · 16 × 16 · im Paket“), then
 * „VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B“. Nothing for a part without a preview.
 */
export function previewLines(r: PartReport): string[] {
  const p = r.preview;
  if (p === undefined) return [];
  const lines: string[] = [];
  if (p.model !== null) {
    const { format, stats, strips } = p.model;
    lines.push(t("workbench:importParts.modelLine", { format: FORMAT_WORDS[format] ?? format, quads: num(stats.quads), triangles: num(stats.triangles), strips: num(strips) }));
  }
  lines.push(...textureLines(p.textures));
  lines.push(t("workbench:importParts.vramLine", { used: num(p.vram_bytes), max: num(p.vram_limit) }));
  return lines;
}

/** One line per texture: „g4_imp_cube · 16 × 16 · 16 Farben · neu“, a texture of the pack „g4_imp_cube · 16 × 16 · im Paket“. */
export function textureLines(textures: readonly TexturePreview[]): string[] {
  return textures.map((texture) => {
    const words = { name: texture.name, size: `${texture.width} × ${texture.height}` };
    return texture.new ? t("workbench:importParts.textureNew", { ...words, colors: texture.colors }) : t("workbench:importParts.textureInPack", words);
  });
}

/** The game's limit of a map model (`world::MODEL_BUDGET`, 0xF000 bytes). */
export const MODEL_BUDGET = 0xf000;

/** A world bake's model against the limit as „Budget“ shows it: full above it, tight from 90 % (`terrain::budget_state`). */
export function modelBudget(bytes: number): { used: number; max: number; state: BudgetState } {
  const state: BudgetState = bytes > MODEL_BUDGET ? "full" : bytes * 10 >= MODEL_BUDGET * 9 ? "tight" : "calm";
  return { used: bytes, max: MODEL_BUDGET, state };
}

/** What a TMJ would change: „2 Ebenen, 17 Zellen geändert“ (singular for one), „Keine Änderung.“ when the file equals the world (no bake). */
export function tmjChangeText(r: TmjImportReport): string {
  if (r.bake === null) return t("workbench:importParts.noChange");
  const layers = t("workbench:importParts.layers", { count: r.layers, n: num(r.layers) });
  const cells = t("workbench:importParts.cells", { count: r.cells, n: num(r.cells) });
  return t("workbench:importParts.changed", { layers, cells });
}

/** The errors of a TMJ as lines: the first three „path: message“ (the message alone without a path: no JSON, no TMJ), then „… und 2 weitere“. */
export function tmjErrorLines(errors: readonly TmjError[]): string[] {
  const lines = errors.slice(0, 3).map((e) => (e.path ? `${e.path}: ${e.message}` : e.message));
  if (errors.length > 3) lines.push(t("workbench:importParts.moreErrors", { n: num(errors.length - 3) }));
  return lines;
}

/** Whether the report is a whole map file of the other game family (only then can its buildings be taken over). */
export const isForeignWhole = (r: PartReport) => r.part === "whole" && r.notes.some((n) => n.code === "whole.foreign");

/** "1 Teil" / "3 Teile". */
export const partsWord = (n: number) => t("workbench:importParts.parts", { count: n });

/**
 * A broken model or terrain inside a whole map file: the kernel gives the code of the model or terrain check (its English message starts
 * with „model: “ or „terrain: “), whose own way out names a model or terrain file. The way out here is another map file; the kernel's reason
 * is the detail in brackets (`sentence` is the catalog key under `workbench:importParts`).
 */
const WHOLE_INNER: readonly [prefix: string, sentence: "wholeModelInvalid" | "wholeTerrainDamaged", kernel: string][] = [
  ["model.", "wholeModelInvalid", "model: "],
  ["bdhc.", "wholeTerrainDamaged", "terrain: "],
];

/** How a part shows in the check list: an otherwise fine part with 0 bytes (no buildings, an empty BGS) is a hint, not a ready part. */
export function partView(r: PartReport): {
  state: CheckState;
  message: string | null;
} {
  if (r.state.kind === "ok") return r.size === 0 ? { state: "info", message: t("workbench:importParts.empty") } : { state: "ok", message: null };
  const { code, message } = r.state;
  const inner = r.part === "whole" && r.state.kind === "error" ? WHOLE_INNER.find(([prefix]) => code.startsWith(prefix)) : undefined;
  if (inner) {
    const [, sentence, kernel] = inner;
    const why = (message.startsWith(kernel) ? message.slice(kernel.length) : message).replace(/[.!?]$/, "");
    return { state: "error", message: t(`workbench:importParts.${sentence}`, { why }) };
  }
  return { state: r.state.kind, message: partText(r.state) };
}

/** Whether the dialog chooses the part up front: not when it cannot be imported or is empty (it stays choosable). */
export const preselected = (r: PartReport) => r.state.kind !== "error" && r.size > 0;

/** The rows of the source map picker: every map, the target map itself disabled (a map cannot import from itself). */
export function sourceMapItems(mapCount: number, target: number): PickerItem[] {
  return Array.from({ length: mapCount }, (_, i) => ({
    id: i,
    label: `${termLabel("map")} ${id4(i)}`,
    tech: String(i),
    area: "welt" as const,
    disabled: i === target,
  }));
}
