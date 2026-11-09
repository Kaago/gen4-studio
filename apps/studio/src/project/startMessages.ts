import i18next from "i18next";
import type { DestCheck } from "../api";
import { t } from "../i18n";

const notEmpty = () => t("project:start.notEmpty");
const aFile = () => t("project:start.aFile");
const unreadable = () => t("project:start.unreadable");

/** A whole number in the format of the language shown ("1.234" in German, "1,234" in English). */
const number = (n: number) => new Intl.NumberFormat(i18next.language, { maximumFractionDigits: 0 }).format(n);

/** Bytes as whole megabytes (10^6), in the format of the language shown: "320 MB". */
export function megabytes(bytes: number): string {
  return `${number(Math.round(bytes / 1_000_000))} MB`;
}

const tooLittle = (free: string, needed: string) => t("project:start.tooLittle", { free, needed });

/** Why a project cannot go to a checked destination, or null when it can. */
export function destProblem(check: DestCheck): string | null {
  switch (check.state) {
    case "not_empty":
      return notEmpty();
    case "not_a_directory":
      return aFile();
    case "unreadable":
      return unreadable();
    default:
      break;
  }
  if (check.free_bytes !== null && check.free_bytes < check.needed_bytes) return tooLittle(megabytes(check.free_bytes), megabytes(check.needed_bytes));
  return check.ok ? null : unreadable();
}

/**
 * A refusal of `create_project` in the language shown, for the ones the kernel words in English (`g4_kernel::start::create_project`,
 * `build::extract`); any other text stays as it is.
 */
export function startMessage(text: string): string {
  if (text.includes("is in the way")) return t("project:start.inTheWay");
  if (text.includes("cannot read destination")) return unreadable();
  const space = /not enough free space: (\d+) MB needed, (\d+) MB free/.exec(text);
  if (space) return tooLittle(`${number(Number(space[2]))} MB`, `${number(Number(space[1]))} MB`);
  if (text.includes("not enough free space")) return t("project:start.noSpace");
  if (text.includes("encrypted")) return t("project:start.encrypted");
  if (/^destination .* is not empty$/.test(text)) return notEmpty();
  if (/^destination .* exists and is not a folder$/.test(text)) return aFile();
  return text;
}
