import type { BuildStep } from "../api";
import i18next from "i18next";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";

/** The folders a step packs that have a word of their own (`pack:<folder>`): the key of the step's name in the `technik` catalog. Bauen calls matrices world maps. */
const PACK_FOLDERS = ["maps", "textArchives", "scripts", "eventFiles", "matrices", "areaData", "trainerProperties", "trainerParty", "encounters", "personalPokeData"] as const;
type PackFolder = (typeof PACK_FOLDERS)[number];
const isPackFolder = (folder: string): folder is PackFolder => (PACK_FOLDERS as readonly string[]).includes(folder);

/** The steps with a fixed name (ids of the build, not translated): the key of the name in the catalog. */
const NAME_KEYS = {
  textproject: "stepWords.name.textproject",
  rotom: "stepWords.name.rotom",
  world: "stepWords.name.world",
  chatot: "stepWords.name.chatot",
  "blz:arm9": "stepWords.name.blzArm9",
  "blz:overlays": "stepWords.name.blzOverlays",
  rom: "stepWords.name.rom",
  crc: "stepWords.name.crc",
} as const;
const isNamed = (id: string): id is keyof typeof NAME_KEYS => Object.prototype.hasOwnProperty.call(NAME_KEYS, id);

/** Why a step ran fully, as an addition to its result (reason code of the build → key in the catalog). Reasons of `blz:*` and `crc` are their result itself. */
const REASON_KEYS = {
  "no-state": "stepWords.reason.noState",
  "build.cache-stale": "stepWords.reason.cacheStale",
  "output-changed": "stepWords.reason.outputChanged",
  removed: "stepWords.reason.removed",
} as const;
const isReason = (reason: string): reason is keyof typeof REASON_KEYS => Object.prototype.hasOwnProperty.call(REASON_KEYS, reason);

/** The prefix of the steps that only watch a folder (no row in the list). */
export const WATCH_PREFIX = "watch:";

/** The name of a step: Bauen names no folder or identifier, Tüfteln keeps the folder name a word is missing for. */
function stepName(id: string, depth: Depth): string {
  if (isNamed(id)) return t(`technik:${NAME_KEYS[id]}`);
  if (id.startsWith("pack:")) {
    const folder = id.slice("pack:".length);
    if (folder === "matrices" && !depthAtLeast(depth, "tuefteln")) return t("technik:stepWords.name.worldMaps");
    if (isPackFolder(folder)) return t(`technik:stepWords.pack.${folder}`);
    return !depthAtLeast(depth, "tuefteln") ? t("technik:stepWords.name.gameData") : t("technik:stepWords.name.packFolder", { folder });
  }
  return !depthAtLeast(depth, "tuefteln") ? t("technik:stepWords.name.other") : id;
}

/** What a `blz:*` row did: from the cache, compressed now, or neither. */
function blzResult(step: BuildStep): string {
  if (step.reason === "not-compressed") return t("technik:stepWords.blz.notCompressed");
  if (step.reason === "fallback") return t("technik:stepWords.blz.fallback");
  if (step.status === "skipped") return t("technik:stepWords.skipped");
  const compressed = step.total - step.cached;
  if (step.cached > 0 && compressed > 0) return t("technik:stepWords.blz.cachedAndCompressed", { cached: step.cached, compressed });
  if (step.cached > 0) return t("technik:stepWords.blz.cached", { cached: step.cached });
  return t("technik:stepWords.blz.compressed", { compressed });
}

function crcResult(step: BuildStep): string {
  if (step.reason === null) return t("technik:stepWords.crc.fromBase");
  if (step.reason === "no-base-mark") return t("technik:stepWords.crc.noBaseMark");
  if (step.reason === "secure-area-changed") return t("technik:stepWords.crc.secureAreaChanged");
  return t("technik:stepWords.crc.notTaken");
}

/** The result of a step that ran (the words of its kind). */
function ranResult(step: BuildStep): string {
  if (step.id === "rotom") return t("technik:stepWords.ran.rotom", { done: step.done, total: step.total });
  if (step.id === "chatot") return t("technik:stepWords.ran.chatot", { count: step.done });
  if (step.id.startsWith("pack:")) {
    return step.status === "incremental" ? t("technik:stepWords.ran.packIncremental", { count: step.done }) : t("technik:stepWords.ran.packFull", { count: step.total });
  }
  if (step.id === "rom") return t("technik:stepWords.ran.rom");
  return step.status === "incremental" ? t("technik:stepWords.ran.incremental") : t("technik:stepWords.ran.full");
}

/** The addition for a step that ran fully with a reason; Bauen names no unknown reason, Tüfteln does. After „Neu bauen (ohne Cache)“ no step has a state: that is no first build. */
function reasonText(step: BuildStep, reason: string, depth: Depth, fresh: boolean): string {
  if (fresh && reason === "no-state") return t("technik:stepWords.reason.fresh");
  // A removed output of the `rom` step is also a ROM built to another path than last time.
  if (step.id === "rom" && reason === "removed") return t("technik:stepWords.reason.romRemoved");
  if (isReason(reason)) return t(`technik:${REASON_KEYS[reason]}`);
  return !depthAtLeast(depth, "tuefteln") ? t("technik:stepWords.reason.full") : t("technik:stepWords.reason.fullBecause", { reason });
}

/** Name and result of a build step in the words of Bauen or Tüfteln; `fresh`: the build was started with „Neu bauen (ohne Cache)“. */
export function stepLabel(step: BuildStep, depth: Depth, fresh = false): { name: string; result: string } {
  const name = stepName(step.id, depth);
  if (step.id.startsWith("blz:")) return { name, result: blzResult(step) };
  if (step.status === "skipped") return { name, result: step.id === "rom" ? t("technik:stepWords.upToDate") : t("technik:stepWords.skipped") };
  if (step.id === "crc") return { name, result: crcResult(step) };
  const result = ranResult(step);
  return { name, result: step.status === "full" && step.reason !== null ? `${result} (${reasonText(step, step.reason, depth, fresh)})` : result };
}

/** The line under „Schritte“ when they are those of a release („Veröffentlichen“): which build, and where its ROM went (relative to the project when inside it). */
export function patchBuildNote(rom: string, project: string): string {
  const root = project.endsWith("/") ? project : `${project}/`;
  return t("technik:stepWords.patchBuildNote", { rom: rom.startsWith(root) ? rom.slice(root.length) : rom });
}

/** A duration in seconds with the decimal mark of the language and two places („0,33 s“, „0.33 s“); under 10 ms „< 0,01 s“. */
export function formatSeconds(ms: number): string {
  if (ms < 10) return t("technik:stepWords.underTenMs");
  const hundredths = Math.round(ms / 10);
  const value = new Intl.NumberFormat(i18next.language, { minimumFractionDigits: 2, maximumFractionDigits: 2, useGrouping: false }).format(hundredths / 100);
  return t("technik:stepWords.seconds", { value });
}
