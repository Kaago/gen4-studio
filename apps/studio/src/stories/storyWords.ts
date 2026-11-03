import { RECIPE_TITLES } from "../api";
import type { FlagKind, FlagListDto, Progress, RecipeId, StepKind, StoryUse, ThreadDto } from "../api";
import { displayName } from "../flags/flagWords";
import { storyKindWord } from "../ui/primitives";

/** From this many values the overview names only the first and the last. */
const LIST_VALUES_BELOW = 9;
/** The overview names at most this many places. */
const NAMED_PLACES = 3;

/** The name of a thread: its own name, else its pret name, else kind and number („Variable 0x4030“). */
export function threadName(t: ThreadDto): string {
  return displayName(t, "var", t.var);
}

/** „3 Werte: 0, 1, 2“; from nine values the first and the last („16 Werte: 0 … 15“); „1 Wert: 5“, „Keine Werte“. */
export function valuesText(values: readonly number[]): string {
  const n = values.length;
  if (n === 0) return "Keine Werte";
  const count = n === 1 ? "1 Wert" : `${n} Werte`;
  const shown = n < LIST_VALUES_BELOW ? values.join(", ") : `${values[0]} … ${values[n - 1]}`;
  return `${count}: ${shown}`;
}

/**
 * „3 Orte: Zweiblattdorf, Route 201, …“: the count and the names of the places' headers (a name two headers share once, at most three);
 * more named places than shown end in „+ 4“ (the places not named), places without a header in „…“. Without any header the count alone.
 */
export function placesText(t: ThreadDto): string {
  const count = t.places === 1 ? "1 Ort" : `${t.places} Orte`;
  const names = [...new Set(t.header_names)];
  if (names.length === 0) return count;
  const shown = names.slice(0, NAMED_PLACES);
  const rest = t.places - shown.length;
  const tail = rest <= 0 ? "" : names.length > shown.length ? ` + ${rest}` : ", …";
  return `${count}: ${shown.join(", ")}${tail}`;
}

/** The German word of a step's kind (as the graph's nodes say it): „Kampf“, „Erscheinen/Verschwinden“. */
export function stepKindWord(kind: StepKind): string {
  return storyKindWord(kind);
}

/** What a story's progress is kept in: the kind and the id of its flag or variable. */
export function progressLabel(p: Progress): { kind: FlagKind; id: string } {
  return "var" in p ? { kind: "var", id: p.var } : { kind: "flag", id: p.flag };
}

/** „2 Geschichten“, „1 Geschichte“, „Keine Geschichten“: the count of the list (and of the entry in the list of Geschichten). */
export function storiesCount(n: number): string {
  return n === 0 ? "Keine Geschichten" : n === 1 ? "1 Geschichte" : `${n.toLocaleString("de-DE")} Geschichten`;
}

/** „20 Variablen frei“, „1 Variable frei“, „Keine Variable frei“: the free variables of the pool. */
export function freeVarsText(n: number): string {
  return n === 0 ? "Keine Variable frei" : n === 1 ? "1 Variable frei" : `${n.toLocaleString("de-DE")} Variablen frei`;
}

/** The title of the recipe a step was made by (`made.recipe`, its serde name): „Gespräch schaltet weiter“; an id the app does not know
 * (a newer studio's recipe) as it is. */
export function recipeTitle(id: string): string {
  return Object.prototype.hasOwnProperty.call(RECIPE_TITLES, id) ? RECIPE_TITLES[id as RecipeId] : id;
}

const PREFIX: Record<FlagKind, string> = { flag: "FLAG", var: "VAR" };

/** The number of a fallback id with `prefix` (`TRAINER_0042` → 42, the kernel's `parse_fallback`); null for any other id. */
export function fallbackNumber(prefix: string, id: string): number | null {
  const digits = id.startsWith(`${prefix}_`) ? id.slice(prefix.length + 1) : "";
  return /^\d+$/.test(digits) ? Number(digits) : null;
}

/**
 * The id a story keeps its progress by (the kernel resolves each): the own name, else the pret name, else the fallback id (`VAR_16497`,
 * `FLAG_0148`, decimal with four digits at least).
 */
export function flagIdOf(kind: FlagKind, n: number, entry: { name: string | null; own_id: string | null } | null | undefined): string {
  return entry?.own_id ?? entry?.name ?? `${PREFIX[kind]}_${String(n).padStart(4, "0")}`;
}

/** The number of a flag or variable id: by an own name, pret name or alias of `list`, else as a fallback id; null where neither knows it. */
export function flagNumberOf(kind: FlagKind, id: string, list: FlagListDto | null): number | null {
  const entry = list?.entries.find((e) => e.own_id === id || e.name === id || e.aliases.includes(id));
  return entry?.number ?? fallbackNumber(PREFIX[kind], id);
}

const pad4 = (n: number) => String(n).padStart(4, "0");

/** What a step's use names: the id of a trainer, map, flag or variable, else where in the game (`Scriptdatei 0101 · Script 2`). */
export function useText(use: StoryUse): string {
  if ("trainer" in use) return use.trainer;
  if ("map" in use) return use.map;
  if ("flag" in use) return use.flag;
  if ("var" in use) return use.var;
  if ("script" in use) return `Scriptdatei ${pad4(use.script.file)} · Script ${use.script.slot}`;
  if ("message" in use) return `Textarchiv ${pad4(use.message.archive)} · Text ${use.message.index}`;
  if ("levelScript" in use) return `Level-Script ${pad4(use.levelScript.file)} · Bedingung ${use.levelScript.condition}`;
  return `Objekt ${use.object.local_id} · Eventdatei ${pad4(use.object.file)}`;
}
