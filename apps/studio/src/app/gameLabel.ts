import type { ProjectInfo } from "../api";

const FAMILY: Record<string, string> = { dp: "Diamant/Perl", pt: "Platin", hgss: "HeartGold/SoulSilver" };
const LANGUAGE: Record<string, string> = {
  english: "EN",
  en: "EN",
  french: "FR",
  fr: "FR",
  german: "DE",
  de: "DE",
  italian: "IT",
  it: "IT",
  spanish: "ES",
  es: "ES",
  japanese: "JP",
  ja: "JP",
  jp: "JP",
  korean: "KR",
  ko: "KR",
  kr: "KR",
};

/** "Platin, DE": the game family and language of a project as the title bar and the start pane write them. */
export function gameLabel({ family, language }: Pick<ProjectInfo, "family" | "language">): string {
  return `${FAMILY[family.toLowerCase()] ?? family}, ${LANGUAGE[language.toLowerCase()] ?? language.toUpperCase()}`;
}

/** The folder name of a project path (either kind of separator, a trailing one ignored). */
export function projectName(root: string): string {
  return root.split(/[\\/]/).filter(Boolean).pop() ?? root;
}
