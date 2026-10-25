import type { AssetKind, CheckGroup, CheckProblemDto, ChecksProgressDto, SubjectKind } from "../api";
import { kindLabel } from "../assets";
import { id4 } from "../workbench/terms";

/**
 * The German words of the checks: per catalogue code its title (the head of its group in the Problems panel and its switch in Technik) and
 * its sentence with a way out („Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür.“), built from the kernel's `args`.
 *
 * In a sentence `{n}` is argument n as the kernel gives it, `{n:id}` a number with four digits („Header 0414“), `{n:field}` a field name in
 * German. `group` and `severity` (the highest the check reports) are the kernel's: the Rust test `commands::checks::messages_ts` holds
 * codes, order, groups and severities against `g4_kernel::checks::Registry::all()`, so this list is complete and in catalogue order.
 */
export interface CheckMessage {
  group: CheckGroup;
  severity: "error" | "warning" | "info";
  title: string;
  text: string;
}

export const CHECK_MESSAGES: Readonly<Record<string, CheckMessage>> = {
  "warp.dest-header": {
    group: "cheap",
    severity: "error",
    title: "Tür führt ins Leere",
    text: "Die Tür führt zu Header {1:id}, den es nicht gibt (das Projekt hat {2} Header). Wähle ein Ziel oder lösche die Tür.",
  },
  "warp.dest-warp": {
    group: "cheap",
    severity: "error",
    title: "Ziel-Warp fehlt",
    text: "Die Tür führt zu Warp {2} in Header {1:id}, aber dessen Eventdatei {3:id} hat nur {4} Warps. Wähle einen anderen Ziel-Warp oder lege dort einen Warp an.",
  },
  "warp.off-map": {
    group: "cheap",
    severity: "error",
    title: "Tür außerhalb der Karte",
    text: "Die Tür steht auf x {1}, z {2}, außerhalb der Karten von Header {3:id}. Verschiebe sie auf eine Karte oder lösche sie.",
  },
  "warp.never-fires": {
    group: "cheap",
    severity: "info",
    title: "Tür löst nie aus",
    text: "Die Tür auf x {1}, z {2} steht auf keiner Tür- oder Treppenkachel, und kein Warp führt zu ihr. Setze sie auf eine Tür oder blende den Hinweis aus.",
  },
  "event.trainer-missing": {
    group: "cheap",
    severity: "error",
    title: "Trainer fehlt",
    text: "Script {1} startet einen Kampf gegen Trainer {2}, den es nicht gibt (das Projekt hat {3} Trainer). Wähle einen Trainer oder ein anderes Script.",
  },
  "event.off-map": {
    group: "cheap",
    severity: "error",
    title: "Event außerhalb der Karte",
    text: "Das Event steht auf x {2}, z {3}, außerhalb der Karten von Header {4:id}. Verschiebe es auf eine Karte oder lösche es.",
  },
  "event.trainer-blind": {
    group: "cheap",
    severity: "warning",
    title: "Trainer sieht nicht",
    text: "Die Figur kämpft als Trainer {2}, hat aber Trainerart 0: sie entdeckt den Spieler nie und kämpft nur, wenn man sie anspricht. Setze die Trainerart oder blende die Warnung aus.",
  },
  "event.trigger-empty": {
    group: "cheap",
    severity: "warning",
    title: "Auslöser ohne Script",
    text: "Der Auslöser auf x {1}, z {2} hat Script 0 und tut nichts. Wähle ein Script oder lösche den Auslöser.",
  },
  "event.same-spot": {
    group: "cheap",
    severity: "warning",
    title: "Zwei Figuren auf einem Feld",
    text: "Objekt {1} steht auf x {2}, z {3} an derselben Stelle wie Objekt {0}, beide ohne Versteck-Flag. Verschiebe eine Figur oder gib einer ein Flag.",
  },
  "header.ref-range": {
    group: "cheap",
    severity: "error",
    title: "Header verweist ins Leere",
    text: "Header {0:id} nennt als {1:field} {2}, aber es gibt nur {3}. Wähle im Header einen gültigen Wert.",
  },
  "header.wild-dangling": {
    group: "cheap",
    severity: "error",
    title: "Begegnungen fehlen",
    text: "Header {0:id} nennt Begegnungsdatei {1:id}, aber es gibt nur {2}. Wähle im Header eine Begegnungsdatei oder „keine“.",
  },
  "header.text-split": {
    group: "cheap",
    severity: "warning",
    title: "Scriptdatei mit mehreren Textarchiven",
    text: "Die Header {2} teilen Scriptdatei {0:id}, nutzen aber die Textarchive {1}; ihre Nachrichtennummern passen nur zu einem. Gib den Headern dasselbe Textarchiv oder eigene Scriptdateien.",
  },
  "matrix.map-range": {
    group: "cheap",
    severity: "error",
    title: "Zelle nennt fehlende Karte",
    text: "Die Zelle nennt Karte {3:id}, aber es gibt nur {4}. Wähle eine vorhandene Karte oder leere die Zelle.",
  },
  "matrix.header-range": {
    group: "cheap",
    severity: "error",
    title: "Zelle nennt fehlenden Header",
    text: "Die Zelle nennt Header {3:id}, aber es gibt nur {4}. Wähle einen vorhandenen Header.",
  },
  "matrix.header-elsewhere": {
    group: "cheap",
    severity: "error",
    title: "Header einer anderen Matrix",
    text: "Die Zelle nennt Header {3:id}, der Matrix {4} nutzt; seine Events erscheinen hier nicht. Wähle einen Header dieser Matrix oder ändere die Matrix des Headers.",
  },
  "map.model-invalid": {
    group: "cheap",
    severity: "error",
    title: "Kartenmodell ungültig",
    text: "Das Modell von Karte {0:id} ist ungültig ({1}). Importiere ein gültiges Modell.",
  },
  "map.bdhc-size": {
    group: "cheap",
    severity: "error",
    title: "Terrain ungültig",
    text: "Das Terrain (BDHC) von Karte {0:id} ist ungültig ({1}). Importiere ein gültiges Terrain.",
  },
  "map.building-range": {
    group: "cheap",
    severity: "error",
    title: "Gebäude ohne Modell",
    text: "Ein Gebäude von Karte {0:id} nutzt Modell {2}, aber es gibt nur {3}. Wähle ein vorhandenes Modell oder lösche das Gebäude.",
  },
  "area.tileset-range": {
    group: "cheap",
    severity: "error",
    title: "Texturen fehlen",
    text: "Area-Data {0:id} nennt als {1:field} {2}, aber es gibt nur {3}. Wähle ein vorhandenes Texturpaket.",
  },
  "trainer.no-party": {
    group: "cheap",
    severity: "error",
    title: "Trainer ohne Team",
    text: "Trainer {0:id} hat kein Pokémon: der Kampf läuft gegen ein unsichtbares Pokémon auf Stufe 0. Gib ihm ein Team.",
  },
  "trainer.empty-species": {
    group: "cheap",
    severity: "error",
    title: "Teamplatz ohne Pokémon",
    text: "Ein Platz im Team hat kein Pokémon. Wähle ein Pokémon oder entferne den Platz.",
  },
  "trainer.id-range": {
    group: "cheap",
    severity: "error",
    title: "Trainer nennt Unbekanntes",
    text: "Als {2:field} steht {3}, aber es gibt nur {4}. Wähle einen vorhandenen Wert.",
  },
  "trainer.no-moves": {
    group: "cheap",
    severity: "warning",
    title: "Pokémon ohne Attacken",
    text: "Das Team hat eigene Attacken, aber dieses Pokémon hat keine und kann nicht angreifen. Gib ihm Attacken oder schalte die eigenen Attacken aus.",
  },
  "trainer.double-one": {
    group: "cheap",
    severity: "warning",
    title: "Doppelkampf mit einem Pokémon",
    text: "Trainer {0:id} kämpft im Doppelkampf, hat aber nur {1} Pokémon. Gib ihm ein zweites Pokémon oder wähle einen Einzelkampf.",
  },
  "encounter.empty-slot": {
    group: "cheap",
    severity: "warning",
    title: "Leerer Platz gibt Glitch-Pokémon",
    text: "Der Platz ist leer, wird aber genutzt: dort erscheint ein Glitch-Pokémon. Wähle ein Pokémon für den Platz.",
  },
  "encounter.level-zero": {
    group: "cheap",
    severity: "warning",
    title: "Pokémon auf Level 0",
    text: "Art {2:id} erscheint auf Level 0. Gib dem Platz ein Level ab 1.",
  },
  "encounter.level-high": {
    group: "cheap",
    severity: "warning",
    title: "Level über 100",
    text: "Der Platz hat Level {2}, mehr als 100. Wähle ein Level bis 100.",
  },
  "encounter.min-max": {
    group: "cheap",
    severity: "info",
    title: "Mindestlevel über Höchstlevel",
    text: "Das Mindestlevel {2} liegt über dem Höchstlevel {3}; das Spiel tauscht sie. Tausche die beiden oder blende den Hinweis aus.",
  },
  "encounter.rate-byte": {
    group: "cheap",
    severity: "warning",
    title: "Rate über 255",
    text: "Die Rate {1} ist größer als 255, das Spiel liest nur {2}. Wähle eine Rate bis 255.",
  },
  "asset.unreadable": {
    group: "cheap",
    severity: "error",
    title: "Datei nicht lesbar",
    text: "{asset} ist nicht lesbar ({2}). Stelle die Datei wieder her; bis dahin prüft das Studio ohne sie.",
  },
  "event.script-missing": {
    group: "per_script",
    severity: "error",
    title: "Script fehlt",
    text: "Das Event ruft Script {2} auf, aber Scriptdatei {3:id} hat nur Scripts bis {4}. Lege das Script an oder wähle ein vorhandenes.",
  },
  "header.level-target": {
    group: "per_script",
    severity: "warning",
    title: "Level-Script ruft fehlendes Script",
    text: "Das Level-Script {1:id} startet Script {2}, aber Scriptdatei {3:id} hat nur Scripts bis {4}. Lege das Script an oder ändere das Level-Script.",
  },
  "header.script-kind": {
    group: "per_script",
    severity: "error",
    title: "Falsche Scriptart",
    text: "Header {0:id} nennt als {1:field} die Datei {2:id}, die von der anderen Art ist. Wähle eine Datei der richtigen Art.",
  },
  "script.message-missing": {
    group: "per_script",
    severity: "error",
    title: "Nachricht fehlt",
    text: "Das Script nutzt Nachricht {2}, aber Textarchiv {3:id} hat nur {4} Nachrichten. Lege die Nachricht an oder wähle eine vorhandene.",
  },
  "script.unreadable": {
    group: "per_script",
    severity: "error",
    title: "Scriptdatei nicht lesbar",
    text: "Scriptdatei {0:id} ist nicht lesbar ({1}), der Build würde daran scheitern. Repariere die Datei oder stelle sie wieder her.",
  },
  "text.too-wide": {
    group: "per_archive",
    severity: "warning",
    title: "Zeile zu breit",
    text: "Die Zeile ist {3} px breit, das Textfenster nur {4} px. Brich sie um oder kürze den Text.",
  },
  "text.three-lines": {
    group: "per_archive",
    severity: "warning",
    title: "Zu viele Zeilen",
    text: "Seite {2} hat {3} Zeilen, das Textfenster zeigt nur {4}. Teile die Seite mit einem Seitenumbruch oder kürze den Text.",
  },
  "text.encode": {
    group: "per_archive",
    severity: "error",
    title: "Text nicht kodierbar",
    text: "Die Nachricht enthält ein Zeichen, das das Spiel nicht kennt ({2}); der Build bricht daran ab. Ersetze das Zeichen.",
  },
  "text.json-broken": {
    group: "per_archive",
    severity: "error",
    title: "Textarchiv nicht lesbar",
    text: "Textarchiv {0:id} ist nicht lesbar ({1}). Repariere die JSON-Datei oder stelle sie wieder her.",
  },
};

/** Every code, in catalogue order. */
export const CHECK_CODES: readonly string[] = Object.keys(CHECK_MESSAGES);

/** Where a finding is listed in the Problems panel: by its subject; the project's own files (an archive that cannot be read) apart. */
export type CheckArea = "welt" | "scripts" | "texte" | "pokemon" | "projekt";

export const AREA_LABELS: Readonly<Record<CheckArea, string>> = { welt: "Welt", scripts: "Scripts", texte: "Texte", pokemon: "Pokémon", projekt: "Projekt" };

const AREA_OF: Readonly<Record<SubjectKind, CheckArea>> = {
  header: "welt",
  eventFile: "welt",
  matrix: "welt",
  map: "welt",
  areaData: "welt",
  scriptFile: "scripts",
  textArchive: "texte",
  trainer: "pokemon",
  encounter: "pokemon",
  project: "projekt",
};

/** The area a finding is listed under. */
export const areaOf = (f: Pick<CheckProblemDto, "subject">): CheckArea => AREA_OF[f.subject.kind];

/** The group of the check `code` (what a cheap run covers); null for a code this list does not know. */
export const groupOf = (code: string): CheckGroup | null => CHECK_MESSAGES[code]?.group ?? null;

/** The title of the check `code`; the code itself when it is not known. */
export const checkTitle = (code: string): string => CHECK_MESSAGES[code]?.title ?? code;

/** Field names of headers, area data and trainers in the words of the editors. */
const FIELD_WORDS: Readonly<Record<string, string>> = {
  matrix_id: "Matrix",
  event_file_id: "Eventdatei",
  script_file_id: "Scriptdatei",
  level_script_id: "Level-Script",
  text_archive_id: "Textarchiv",
  area_data_id: "Area-Data",
  wild_pokemon: "Begegnungen",
  map_tileset: "Karten-Texturen",
  buildings_tileset: "Gebäude-Texturen",
  class: "Klasse",
  items: "Gegenstand",
  species: "Pokémon",
  item: "Gegenstand",
  moves: "Attacke",
};

/** The kernel's names of asset kinds (`AssetKind::label`), for `asset.unreadable`. */
const ASSET_KINDS: Readonly<Record<string, AssetKind>> = {
  map: "Map",
  matrix: "Matrix",
  "area data": "AreaData",
  "event file": "Event",
  "text archive": "TextArchive",
  "map texture": "MapTexture",
  "building texture": "BuildingTexture",
  "building model": "BuildingModel",
  "interior building model": "InteriorBuildingModel",
  "header table": "HeaderTable",
  "dynamic headers": "DynamicHeaders",
  "script file": "Script",
  "encounter file": "Encounter",
  "personal data": "Personal",
  learnset: "Learnset",
  "evolution data": "Evolution",
  move: "Move",
  item: "Item",
  "trainer properties": "TrainerProperties",
  "trainer party": "TrainerParty",
  "trainer text table": "TrainerTextTable",
  "trainer text offsets": "TrainerTextOffsets",
  "item icons": "ItemIcons",
  "menu icons": "MonIcons",
  trainer: "Trainer",
};

/** „Eventdatei 0411“, or „Das Archiv Pokémon-Daten“ for a whole archive (no index). */
function assetName(kind: string, index: string): string {
  const known = ASSET_KINDS[kind];
  const word = known ? kindLabel(known) : kind;
  return index === "" ? `Das Archiv ${word}` : `${word} ${id4(Number(index))}`;
}

/** Puts the arguments into `text` (see `CheckMessage`). */
function fill(text: string, args: readonly string[]): string {
  return text.replace(/\{(\d+)(?::(id|field))?\}/g, (_, n: string, format?: string) => {
    const value = args[Number(n)] ?? "";
    if (format === "id" && /^\d+$/.test(value)) return id4(Number(value));
    if (format === "field") return FIELD_WORDS[value] ?? value;
    return value;
  });
}

/** The German sentence of a finding with its way out; the kernel's (English) sentence for a code this list does not know. */
export function findingText(f: Pick<CheckProblemDto, "code" | "key" | "args" | "message">): string {
  const m = CHECK_MESSAGES[f.code];
  if (!m) return f.message;
  if (f.code === "asset.unreadable") {
    const [kind = "", index = "", why = ""] = f.args;
    const name = assetName(kind, index);
    if (f.key.startsWith("locked:")) {
      return `${name} ist gesperrt (${why}): das Studio würde sie nicht Byte für Byte zurückschreiben. Stelle die Datei wieder her, dann lässt sie sich ändern.`;
    }
    return fill(m.text, f.args).replace("{asset}", name);
  }
  return fill(m.text, f.args);
}

const PROGRESS_WORDS: Readonly<Record<ChecksProgressDto["label"], string>> = { cheap: "Welt und Daten", load: "Dateien", scripts: "Scripts", texts: "Texte" };

/** „Prüft … Texte 312/724“: how far a running pass got; „Prüft …“ before it says. */
export function progressText(progress: ChecksProgressDto | null | undefined): string {
  if (!progress) return "Prüft …";
  return `Prüft … ${PROGRESS_WORDS[progress.label] ?? progress.label} ${progress.done}/${progress.total}`;
}

/** The project's ignore file; a report names it as `skipped` code when it could not be read. */
const IGNORE_FILE = "g4-checks.json";

const COMMON_SCRIPTS = /^(\d+) script files that no header uses \(common scripts\) are not checked/;
const ONE_FILE = /^(level )?script file (\d+) is not checked: (.+)$/;

/** A check that did not run, or left part of its work undone, as a hint line („Nachricht fehlt: 28 Common-Scripts ohne Header nicht geprüft.“). */
export function skippedText({ code, why }: { code: string; why: string }): string {
  // Not a check: the ignore file could not be read when the project opened (ruling R15).
  if (code === IGNORE_FILE) return `${IGNORE_FILE} ist nicht lesbar (${why}): nichts ist ausgeblendet oder abgeschaltet, und das Studio schreibt die Datei nicht. Repariere oder lösche sie, dann geht beides wieder.`;
  const title = checkTitle(code);
  if (why === "scripts not adopted") return `${title}: nicht geprüft, die Scripts sind nicht übernommen. Übernimm sie unter Script, dann prüft das Studio sie.`;
  const common = COMMON_SCRIPTS.exec(why);
  if (common) return `${title}: ${common[1]} Common-Scripts ohne Header nicht geprüft.`;
  const file = ONE_FILE.exec(why);
  if (file) return `${title}: ${file[1] ? "Level-Script" : "Scriptdatei"} ${id4(Number(file[2]))} nicht geprüft (${file[3]}).`;
  return `${title}: nicht geprüft (${why}).`;
}
