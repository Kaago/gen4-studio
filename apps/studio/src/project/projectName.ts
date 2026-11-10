import type { RomInfo } from "../api";
import { t } from "../i18n";

/** The ending DSPRE expects on a project folder. */
export const DSPRE_SUFFIX = "_DSPRE_contents";

const MAX_NAME = 64;

/** The game of a ROM in the language shown: "Platin", "HeartGold" (IPK*), "SoulSilver" (IPG*), "Diamant/Perl". */
export function gameName({ family, game_code }: Pick<RomInfo, "family" | "game_code">): string {
  switch (family) {
    case "pt":
      return t("project:games.pt");
    case "hgss":
      return game_code.startsWith("IPG") ? "SoulSilver" : game_code.startsWith("IPK") ? "HeartGold" : "HeartGold/SoulSilver";
    case "dp":
      return t("project:games.dp");
    default:
      return t("project:games.unknown");
  }
}

/** The language of a ROM in the language shown ("Deutsch", "Englisch"). */
export function languageName(language: RomInfo["language"]): string {
  return language ? t(`project:languages.${language}`) : t("project:languages.unknown");
}

/** The name the wizard proposes for a project from this ROM: „Neues Platin“, „Neues HeartGold“; „Neues Spiel“ for any other game. */
export function suggestName(info: RomInfo): string {
  const named = info.family === "pt" || (info.family === "hgss" && (info.game_code.startsWith("IPG") || info.game_code.startsWith("IPK")));
  return named ? t("project:suggestName.game", { game: gameName(info) }) : t("project:suggestName.other");
}

/** What is wrong with a project name, or null: it becomes a folder name. */
export function nameProblem(name: string): string | null {
  const trimmed = name.trim();
  if (trimmed === "") return t("project:nameProblem.empty");
  if (/[/\\:]/.test(trimmed) || trimmed.startsWith(".")) return t("project:nameProblem.characters");
  if (trimmed.length > MAX_NAME) return t("project:nameProblem.tooLong", { max: MAX_NAME });
  return null;
}

/** The project folder: `name` (with the DSPRE ending if asked) in `parent`, joined with the parent's own separator. */
export function destOf(parent: string, name: string, dspreSuffix: boolean): string {
  const sep = parent.includes("\\") && !parent.includes("/") ? "\\" : "/";
  const base = parent.replace(/[\\/]+$/, "");
  return `${base}${sep}${name.trim()}${dspreSuffix ? DSPRE_SUFFIX : ""}`;
}
