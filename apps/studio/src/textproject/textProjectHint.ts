import type { TextProjectStatus } from "../api";
import { matchesAnyLanguage, t } from "../i18n";

/** The kernel's reason a Diamond or Pearl folder cannot be set up (`textproject::NO_DP`) in the language that is shown; `shouldHint` knows it in either language. */
export const noDp = (): string => t("textproject:textProjectHint.noDp");

/** Where „Nicht mehr fragen“ is kept, per project folder: a convenience of the user, no project file. */
const key = (root: string) => `g4.textProjectHint.${root}`;

/**
 * Whether opening the folder at `root` hints that it can be set up as text project: a DSPRE folder (Platinum, HeartGold, SoulSilver; one
 * without its base ROM too, the hint says it needs it) whose hint was not dismissed. Storage that cannot be read hints.
 */
export function shouldHint(root: string, status: TextProjectStatus): boolean {
  if (status.state !== "dspre") return false;
  // The kernel words the reason in the language it was set to, which may differ from the one shown: a sentence of either catalog counts.
  if (!status.adoptable && matchesAnyLanguage("textproject:textProjectHint.noDp", status.reason)) return false;
  try {
    return localStorage.getItem(key(root)) === null;
  } catch {
    return true;
  }
}

/** „Nicht mehr fragen“ for the folder at `root`; storage that cannot be written keeps nothing (the next opening asks again). */
export function dismissHint(root: string): void {
  try {
    localStorage.setItem(key(root), "dismissed");
  } catch {
    /* nothing kept: the next opening hints again */
  }
}

/** The hint's sentence and whether it offers „Einrichten…“ (`setUp`) or, without the base ROM, „Basis-ROM zuordnen…“. */
export function hintOf(status: TextProjectStatus): { text: string; setUp: boolean } {
  const setUp = status.state === "dspre" && status.adoptable;
  return { text: `${t("textproject:textProjectHint.lead")} ${setUp ? t("textproject:textProjectHint.opensFolder") : t("textproject:textProjectHint.needsBase")}`, setUp };
}
