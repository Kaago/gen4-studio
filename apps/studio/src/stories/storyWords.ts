import type { FlagKind, Progress, StepKind, ThreadDto } from "../api";
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
