import type { ProjectInfo } from "../api";
import { t } from "../i18n";

/** The catalog key of each game family's name. */
const FAMILY: Record<string, "app:gameLabel.dp" | "app:gameLabel.pt" | "app:gameLabel.hgss"> = { dp: "app:gameLabel.dp", pt: "app:gameLabel.pt", hgss: "app:gameLabel.hgss" };
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
  const key = FAMILY[family.toLowerCase()];
  return `${key === undefined ? family : t(key)}, ${LANGUAGE[language.toLowerCase()] ?? language.toUpperCase()}`;
}

/** The folder name of a project path (either kind of separator, a trailing one ignored). */
export function projectName(root: string): string {
  return root.split(/[\\/]/).filter(Boolean).pop() ?? root;
}
