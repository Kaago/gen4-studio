import type { MapPart, PartMessage, PartReport } from "../../api";
import type { PickerItem } from "../../ui/primitives";
import type { CheckState } from "../../ui/primitives/CheckList";
import { id4 } from "../terms";

/** The German name of a map part. */
export const PART_NAME: Record<MapPart, string> = {
  model: "Modell",
  permissions: "Kollision",
  buildings: "Gebäude",
  bdhc: "BDHC (Laufhöhen)",
  bgs: "Hintergrundklang",
  whole: "Ganze Karte",
};

/**
 * The German sentence of every code the part checks give (`g4_kernel::map_import::PartCode`; the Rust test
 * `commands::map_import::tests::every_part_code_has_its_german_sentence` holds this list against `PartCode::ALL`): an error or a warning
 * with its way out, a hint as a note. In a sentence `{n}` is argument n as the kernel gives it, `{n:num}` a number in German digits
 * („65.000“), `{n:family}` a game family.
 */
export const PART_TEXTS: Readonly<Record<string, string>> = {
  "model.invalid": "Die Datei ist kein gültiges NSBMD-Modell ({0}). Wähle ein Kartenmodell im NSBMD-Format.",
  "model.blocks": "Die Datei hat {0:num} Modellblöcke (MDL0), das Spiel braucht genau einen. Exportiere das Modell mit genau einem Modellblock.",
  "model.count": "Der Modellblock enthält {0:num} Modelle, das Spiel braucht genau eines. Exportiere nur das Kartenmodell.",
  "model.too-big": "Das Modell ist {0:num} Byte groß, erlaubt sind {1:num}. Wähle ein kleineres Modell.",
  "model.unaligned": "Das Modell ist {0:num} Byte groß, keine durch 4 teilbare Länge. Exportiere es neu, wenn die Karte im Spiel falsch aussieht.",
  "model.texture-removed": "Der eingebettete Texturblock ({0:num} Byte) wird entfernt; die Karte nutzt die Texturen ihres Gebiets.",
  "model.bdhc-unchanged": "BDHC bleibt unverändert, Laufhöhen prüfen.",
  "model.textures-missing": "{0:num} Texturnamen fehlen im Gebiets-Paket {1} ({2}). Sieh dir die Karte nach dem Import in der Szene an.",
  "model.textures-unreadable": "Texturnamen nicht geprüft: die Materialien des Modells sind nicht lesbar. Sieh dir die Karte nach dem Import in der Szene an.",
  "model.textures-no-header": "Texturnamen nicht geprüft: kein Header zeigt diese Karte. Sieh dir die Karte nach dem Import in der Szene an.",
  "model.textures-unchecked": "Texturnamen nicht geprüft ({0}). Sieh dir die Karte nach dem Import in der Szene an.",
  "permissions.size": "Die Kollision ist {0:num} Byte groß, erwartet sind genau {1:num}. Wähle eine vollständige .per-Datei.",
  "buildings.size": "Die Gebäude sind {0:num} Byte groß, kein Vielfaches von {1:num}. Wähle eine vollständige .bld-Datei.",
  "buildings.unreadable": "Die Gebäude sind nicht lesbar. Wähle eine andere .bld-Datei.",
  "bdhc.magic": "Die Datei beginnt nicht mit „BDHC“ und ist kein Terrain. Wähle eine .bdhc-Datei.",
  "bdhc.short": "Das Terrain ist {0:num} Byte groß, kürzer als sein Kopf mit {1:num} Byte. Wähle eine vollständige .bdhc-Datei.",
  "bdhc.too-big": "Die Terraindaten sind {0:num} Byte groß, erlaubt sind {1:num}. Wähle ein Terrain mit weniger Flächen.",
  "bdhc.counters": "Das Terrain ist {0:num} Byte groß, seine Zähler beschreiben {1:num}. Wähle eine unbeschädigte .bdhc-Datei.",
  "bgs.family": "Hintergrundklang gibt es nur in HeartGold und SoulSilver. Lass diesen Teil weg.",
  "bgs.short": "Der Hintergrundklang ist {0:num} Byte groß, kürzer als 4. Wähle eine vollständige .bgs-Datei.",
  "bgs.signature": "Der Hintergrundklang hat die Kennung {0} statt {1}. Wähle eine .bgs-Datei aus HeartGold oder SoulSilver.",
  "bgs.length": "Der Hintergrundklang ist {0:num} Byte groß, nennt aber {1:num}. Wähle eine unbeschädigte .bgs-Datei.",
  "whole.unreadable": "Die Datei ist keine Kartendatei von Platin oder HeartGold/SoulSilver ({1}). Wähle eine Kartendatei (.bin) eines dieser Spiele.",
  "whole.invalid": "Die Kartendatei ist ungültig ({0}). Wähle eine unbeschädigte Kartendatei.",
  "whole.bgs-dropped": "Der Hintergrundklang ({0:num} Byte) fällt weg, Platin hat keinen. Übernimm die Karte ohne ihn oder wähle eine andere.",
  "whole.foreign": "Kartendatei von {0:family}: ihre Gebäude bleiben weg, außer du schaltest „Gebäude übernehmen“ ein.",
  "whole.blank-bgs": "Ein leerer Hintergrundklang-Block wird angelegt.",
  "whole.buildings-tail": "{0:num} Byte nach den Gebäuden bleiben erhalten.",
  "part.duplicate": "Für diesen Teil sind mehrere Dateien gewählt. Wähle je Teil nur eine Datei.",
  "bdhc.structure": "Das Terrain ist beschädigt ({0}). Wähle eine unbeschädigte .bdhc-Datei.",
  "model.source-unreadable": "Die Datei ist kein lesbares {0}-Modell ({1}). Exportiere das Modell neu als OBJ oder glTF.",
  "model.file-missing": "Die Datei {0}, auf die das Modell verweist, fehlt. Lege sie neben das Modell oder exportiere es neu.",
  "model.file-outside": "Das Modell verweist auf {0} außerhalb seines Ordners. Lege Material und Texturen in den Ordner des Modells.",
  "model.no-geometry": "Das Modell enthält keine Flächen. Wähle ein Modell mit Flächen oder eine größere Einheit.",
  "model.material-untextured": "Das Material „{0}“ hat keine Textur, und das Gebiets-Paket hat keine Textur dieses Namens. Gib dem Material eine PNG-Textur.",
  "model.uv-missing": "Flächen mit dem Material „{0}“ haben keine Texturkoordinaten. Exportiere das Modell mit UV.",
  "model.out-of-range": "Eine Ecke liegt bei ({0}, {1}, {2}), außerhalb von {3} bis {4} Einheiten. Verkleinere das Modell oder wähle eine kleinere Einheit.",
  "model.faces-dropped": "{0:num} Flächen fallen weg: nach dem Runden auf ganze Einheiten haben sie keine Fläche mehr.",
  "texture.unreadable": "Das Bild {0} lässt sich nicht lesen ({1}). Wähle ein PNG.",
};

const FAMILY_WORDS: Readonly<Record<string, string>> = { Pt: "Platin", Hgss: "HeartGold/SoulSilver", Dp: "Diamant/Perl" };

/** What a part check says, as the dialog shows it: the German sentence of its code; the kernel's (English) sentence for a code it does not know. */
export function partText({ code, args, message }: PartMessage): string {
  const text = PART_TEXTS[code];
  if (text === undefined) return message;
  return text.replace(/\{(\d+)(?::(num|family))?\}/g, (_, n: string, format?: string) => {
    const value = args[Number(n)] ?? "";
    if (format === "num" && /^\d+$/.test(value)) return Number(value).toLocaleString("de-DE");
    if (format === "family") return FAMILY_WORDS[value] ?? value;
    return value;
  });
}

/** Whether the report is a whole map file of the other game family (only then can its buildings be taken over). */
export const isForeignWhole = (r: PartReport) => r.part === "whole" && r.notes.some((n) => n.code === "whole.foreign");

/** "1 Teil" / "3 Teile". */
export const partsWord = (n: number) => `${n} ${n === 1 ? "Teil" : "Teile"}`;

/**
 * A broken model or terrain inside a whole map file: the kernel gives the code of the model or terrain check (its English message starts
 * with „model: “ or „terrain: “), whose own way out names a model or terrain file. The way out here is another map file; the kernel's reason
 * is the detail in brackets.
 */
const WHOLE_INNER: readonly [prefix: string, lead: string, kernel: string][] = [
  ["model.", "Das Modell in der Kartendatei ist ungültig", "model: "],
  ["bdhc.", "Das Terrain in der Kartendatei ist beschädigt", "terrain: "],
];

/** How a part shows in the check list: an otherwise fine part with 0 bytes (no buildings, an empty BGS) is a hint, not a ready part. */
export function partView(r: PartReport): {
  state: CheckState;
  message: string | null;
} {
  if (r.state.kind === "ok") return r.size === 0 ? { state: "info", message: "Leer, wird nicht übernommen" } : { state: "ok", message: null };
  const { code, message } = r.state;
  const inner = r.part === "whole" && r.state.kind === "error" ? WHOLE_INNER.find(([prefix]) => code.startsWith(prefix)) : undefined;
  if (inner) {
    const [, lead, kernel] = inner;
    const why = (message.startsWith(kernel) ? message.slice(kernel.length) : message).replace(/[.!?]$/, "");
    return { state: "error", message: `${lead} (${why}). Wähle eine andere Kartendatei.` };
  }
  return { state: r.state.kind, message: partText(r.state) };
}

/** Whether the dialog chooses the part up front: not when it cannot be imported or is empty (it stays choosable). */
export const preselected = (r: PartReport) => r.state.kind !== "error" && r.size > 0;

/** The rows of the source map picker: every map, the target map itself disabled (a map cannot import from itself). */
export function sourceMapItems(mapCount: number, target: number): PickerItem[] {
  return Array.from({ length: mapCount }, (_, i) => ({
    id: i,
    label: `Karte ${id4(i)}`,
    tech: String(i),
    area: "welt" as const,
    disabled: i === target,
  }));
}
