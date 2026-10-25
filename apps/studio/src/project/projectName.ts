import type { RomInfo } from "../api";

/** The ending DSPRE expects on a project folder. */
export const DSPRE_SUFFIX = "_DSPRE_contents";

const MAX_NAME = 64;

const LANGUAGES: Record<NonNullable<RomInfo["language"]>, string> = {
  german: "Deutsch",
  english: "Englisch",
  french: "Französisch",
  italian: "Italienisch",
  spanish: "Spanisch",
  japanese: "Japanisch",
  korean: "Koreanisch",
};

/** The game of a ROM in German: "Platin", "HeartGold" (IPK*), "SoulSilver" (IPG*), "Diamant/Perl". */
export function gameName({ family, game_code }: Pick<RomInfo, "family" | "game_code">): string {
  switch (family) {
    case "pt":
      return "Platin";
    case "hgss":
      return game_code.startsWith("IPG") ? "SoulSilver" : game_code.startsWith("IPK") ? "HeartGold" : "HeartGold/SoulSilver";
    case "dp":
      return "Diamant/Perl";
    default:
      return "Unbekanntes Spiel";
  }
}

export function languageName(language: RomInfo["language"]): string {
  return language ? LANGUAGES[language] : "Unbekannte Sprache";
}

/** The name the wizard proposes for a project from this ROM. */
export function suggestName(info: RomInfo): string {
  const game = gameName(info);
  return game === "Platin" || game === "HeartGold" || game === "SoulSilver" ? `Neues ${game}` : "Neues Spiel";
}

/** What is wrong with a project name, or null: it becomes a folder name. */
export function nameProblem(name: string): string | null {
  const trimmed = name.trim();
  if (trimmed === "") return "Gib dem Projekt einen Namen.";
  if (/[/\\:]/.test(trimmed) || trimmed.startsWith(".")) return "Der Name darf kein /, \\ oder : enthalten und nicht mit einem Punkt beginnen.";
  if (trimmed.length > MAX_NAME) return `Höchstens ${MAX_NAME} Zeichen.`;
  return null;
}

/** The project folder: `name` (with the DSPRE ending if asked) in `parent`, joined with the parent's own separator. */
export function destOf(parent: string, name: string, dspreSuffix: boolean): string {
  const sep = parent.includes("\\") && !parent.includes("/") ? "\\" : "/";
  const base = parent.replace(/[\\/]+$/, "");
  return `${base}${sep}${name.trim()}${dspreSuffix ? DSPRE_SUFFIX : ""}`;
}
