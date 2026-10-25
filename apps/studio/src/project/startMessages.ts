import type { DestCheck } from "../api";

const NOT_EMPTY = "Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.";
const A_FILE = "Dort liegt schon eine Datei.";
const UNREADABLE = "Den Zielordner kann Gen4 Studio nicht lesen.";

const number = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 0 });

/** Bytes as whole megabytes (10^6), German format: "320 MB". */
export function megabytes(bytes: number): string {
  return `${number.format(Math.round(bytes / 1_000_000))} MB`;
}

const tooLittle = (free: string, needed: string) => `Zu wenig Platz: ${free} frei, gebraucht etwa ${needed}.`;

/** Why a project cannot go to a checked destination, or null when it can. */
export function destProblem(check: DestCheck): string | null {
  switch (check.state) {
    case "not_empty":
      return NOT_EMPTY;
    case "not_a_directory":
      return A_FILE;
    case "unreadable":
      return UNREADABLE;
    default:
      break;
  }
  if (check.free_bytes !== null && check.free_bytes < check.needed_bytes) return tooLittle(megabytes(check.free_bytes), megabytes(check.needed_bytes));
  return check.ok ? null : UNREADABLE;
}

/**
 * A refusal of `create_project` in German, for the ones the kernel words in English (`g4_kernel::start::create_project`,
 * `build::extract`); any other text stays as it is.
 */
export function startMessage(text: string): string {
  if (text.includes("is in the way")) return "Ein alter Zwischenordner liegt im Weg. Lösche ihn oder wähle einen anderen Namen.";
  if (text.includes("cannot read destination")) return UNREADABLE;
  const space = /not enough free space: (\d+) MB needed, (\d+) MB free/.exec(text);
  if (space) return tooLittle(`${number.format(Number(space[2]))} MB`, `${number.format(Number(space[1]))} MB`);
  if (text.includes("not enough free space")) return "Zu wenig Platz auf dem Laufwerk des Ziels.";
  if (text.includes("encrypted")) return "Die ROM ist verschlüsselt. Nutze einen entschlüsselten Dump.";
  if (/^destination .* is not empty$/.test(text)) return NOT_EMPTY;
  if (/^destination .* exists and is not a folder$/.test(text)) return A_FILE;
  return text;
}
