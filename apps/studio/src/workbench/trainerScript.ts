import type { DataRowDto, EventPatch } from "../api";
import { TRAINER_SCRIPTS, ownName, trainerRowLabel } from "../data/trainer/trainerModel";
import { id4 } from "./terms";

/**
 * How an object event starts a trainer battle (research trainer-mechanics-3b3b §1.1): its script `3000 + ID − 1` (3000 to 4999) runs the
 * shared battle code, which takes the trainer ID back from the number; `5000 + ID − 1` is the same trainer as a double battle partner.
 */
export const TRAINER_SCRIPT = 3000;
export const PARTNER_SCRIPT = 5000;
/** IDs per range: 3000 to 4999, 5000 to 6999. */
const RANGE = 2000;

/** The sight range in tiles a figure gets when it becomes a trainer without one (object `data[0]`). */
export const DEFAULT_SIGHT = 3;

export interface ScriptTrainer {
  id: number;
  /** From the 5000 range: the trainer is a double battle partner. */
  partner: boolean;
}

/** The trainer an object's script starts the battle of; null outside 3000 to 6999. */
export function trainerOfScript(script: number): ScriptTrainer | null {
  if (script >= TRAINER_SCRIPT && script < TRAINER_SCRIPT + RANGE) return { id: script - TRAINER_SCRIPT + 1, partner: false };
  if (script >= PARTNER_SCRIPT && script < PARTNER_SCRIPT + RANGE) return { id: script - PARTNER_SCRIPT + 1, partner: true };
  return null;
}

/** The script that starts trainer `id`'s battle; as a double battle partner (`partner`) from the 5000 range. */
export const trainerScript = (id: number, partner = false): number => (partner ? PARTNER_SCRIPT : TRAINER_SCRIPT) + id - 1;

/**
 * The fields that make an object trainer `id`: the script (a `partner` figure keeps the partner range, ruling R9), and when the figure
 * has neither a sightline nor a sight range (both 0, a figure that never was a trainer) the sightline on with 3 tiles. A sightline that is
 * set stays as it is.
 */
export function trainerPatch(id: number, object: { trainer_type: number; sight: number }, partner = false): EventPatch {
  const script = trainerScript(id, partner);
  return object.trainer_type === 0 && object.sight === 0 ? { script, trainer_type: 1, sight: DEFAULT_SIGHT } : { script };
}

/**
 * The trainers an object may start: all but the empty dummy 0 (it has no script) and the ID of the "seen" slot of the battle script
 * (Platinum 929, HeartGold 740), which the game runs on sight and which never gets a trainer (research trainer-script-extension §2.3).
 */
export function placeableTrainers(rows: readonly DataRowDto[], family: string | null): DataRowDto[] {
  const seen = family === null ? undefined : TRAINER_SCRIPTS[family]?.seenSlot;
  return rows.filter((row) => row.index !== 0 && row.index !== seen);
}

/** "Teenager Sebastian"; a trainer the list does not (yet) have is "Trainer 0356". */
export function trainerLabel(rows: readonly DataRowDto[] | null, id: number): string {
  const row = rows?.find((r) => r.index === id);
  return row ? trainerRowLabel(row) : `Trainer ${id4(id)}`;
}

/** The trainer an object's script starts, by name: "Teenager Sebastian", a partner "Teenager Sebastian (Partner)"; null for no trainer. */
export function scriptTrainerLabel(rows: readonly DataRowDto[] | null, script: number): string | null {
  const trainer = trainerOfScript(script);
  if (trainer === null) return null;
  const label = trainerLabel(rows, trainer.id);
  return trainer.partner ? `${label} (Partner)` : label;
}

/** The undo step of placing trainer `id`: "Trainer Jens platziert"; without a name of its own "Trainer 0930 platziert". */
export function placedLabel(rows: readonly DataRowDto[] | null, id: number): string {
  const row = rows?.find((r) => r.index === id);
  const own = row ? ownName(row.name, id) : "";
  return `Trainer ${own === "" ? id4(id) : own} platziert`;
}
