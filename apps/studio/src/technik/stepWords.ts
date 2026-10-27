import type { BuildStep } from "../api";
import type { Depth } from "../ui/frame/shellState";

/** The words of the folders a step packs (the areas' German words); Bauen calls matrices world maps. */
const PACK_WORDS: Readonly<Record<string, string>> = {
  maps: "Karten",
  textArchives: "Texte",
  scripts: "Scripts",
  eventFiles: "Eventdateien",
  matrices: "Matrizen",
  areaData: "Gebietsdaten",
  trainerProperties: "Trainer",
  trainerParty: "Trainer",
  encounters: "Begegnungen",
  personalPokeData: "Pokémon-Daten",
};

const NAMES: Readonly<Record<string, string>> = {
  rotom: "Scripts",
  chatot: "Texte",
  "blz:arm9": "Spielcode (ARM9)",
  "blz:overlays": "Overlays",
  rom: "ROM schreiben",
  crc: "Prüfsummen",
};

/** Why a step ran fully, as an addition to its result. Reasons of `blz:*` and `crc` are their result itself. */
const REASONS: Readonly<Record<string, string>> = {
  "no-state": "erster Build, voll",
  "build.cache-stale": "neues Werkzeug, voll gebaut",
  "output-changed": "Ausgabe geändert, voll gebaut",
  removed: "Datei entfernt, voll gebaut",
};

/** Reasons that read differently for the `rom` step: a removed output there is also a ROM built to another path than last time. */
const ROM_REASONS: Readonly<Record<string, string>> = {
  removed: "anderes Ziel oder ROM gelöscht, voll gebaut",
};

/** The prefix of the steps that only watch a folder (no row in the list). */
export const WATCH_PREFIX = "watch:";

/** The name of a step: Bauen names no folder or identifier, Tüfteln keeps the folder name a word is missing for. */
function stepName(id: string, depth: Depth): string {
  const named = NAMES[id];
  if (named !== undefined) return named;
  if (id.startsWith("pack:")) {
    const folder = id.slice("pack:".length);
    if (folder === "matrices" && depth === "bauen") return "Weltkarten packen";
    return `${PACK_WORDS[folder] ?? (depth === "bauen" ? "Spieldaten" : folder)} packen`;
  }
  return depth === "bauen" ? "Weiterer Schritt" : id;
}

/** What a `blz:*` row did: from the cache, compressed now, or neither. */
function blzResult(step: BuildStep): string {
  if (step.reason === "not-compressed") return "nicht komprimiert";
  if (step.reason === "fallback") return "ohne Cache komprimiert";
  if (step.status === "skipped") return "übersprungen";
  const compressed = step.total - step.cached;
  if (step.cached > 0 && compressed > 0) return `${step.cached} aus dem Cache, ${compressed} komprimiert`;
  if (step.cached > 0) return `${step.cached} aus dem Cache`;
  return `${compressed} komprimiert`;
}

function crcResult(step: BuildStep): string {
  if (step.reason === null) return "aus der Basis übernommen";
  if (step.reason === "no-base-mark") return "keine Basis-Marke";
  if (step.reason === "secure-area-changed") return "nicht übernommen: Sicherheitsbereich geändert";
  return "nicht übernommen";
}

/** The result of a step that ran (the words of its kind). */
function ranResult(step: BuildStep): string {
  if (step.id === "rotom") return `${step.done} von ${step.total} neu`;
  if (step.id === "chatot") return `${step.done} kodiert`;
  if (step.id.startsWith("pack:")) return step.status === "incremental" ? `${step.done} Member eingesetzt` : `${step.total} Member gepackt`;
  if (step.id === "rom") return "geschrieben";
  return step.status === "incremental" ? "teilweise neu gebaut" : "voll gebaut";
}

/** The addition for a step that ran fully with a reason; Bauen names no unknown reason, Tüfteln does. After „Neu bauen (ohne Cache)“ no step has a state: that is no first build. */
function reasonText(step: BuildStep, reason: string, depth: Depth, fresh: boolean): string {
  if (fresh && reason === "no-state") return "ohne Cache, voll";
  const own = step.id === "rom" ? ROM_REASONS[reason] : undefined;
  if (own !== undefined) return own;
  return REASONS[reason] ?? (depth === "bauen" ? "voll gebaut" : `voll gebaut: ${reason}`);
}

/** Name and result of a build step in the words of Bauen or Tüfteln; `fresh`: the build was started with „Neu bauen (ohne Cache)“. */
export function stepLabel(step: BuildStep, depth: Depth, fresh = false): { name: string; result: string } {
  const name = stepName(step.id, depth);
  if (step.id.startsWith("blz:")) return { name, result: blzResult(step) };
  if (step.status === "skipped") return { name, result: step.id === "rom" ? "aktuell" : "übersprungen" };
  if (step.id === "crc") return { name, result: crcResult(step) };
  const result = ranResult(step);
  return { name, result: step.status === "full" && step.reason !== null ? `${result} (${reasonText(step, step.reason, depth, fresh)})` : result };
}

/** The line under „Schritte“ when they are those of „Patch erstellen“: which build, and where its ROM went (relative to the project when inside it). */
export function patchBuildNote(rom: string, project: string): string {
  const root = project.endsWith("/") ? project : `${project}/`;
  return `Build von „Patch erstellen“ nach ${rom.startsWith(root) ? rom.slice(root.length) : rom}`;
}

/** A duration in seconds with a decimal comma and two places („0,33 s“); under 10 ms „< 0,01 s“. */
export function formatSeconds(ms: number): string {
  if (ms < 10) return "< 0,01 s";
  const hundredths = Math.round(ms / 10);
  return `${Math.floor(hundredths / 100)},${String(hundredths % 100).padStart(2, "0")} s`;
}
