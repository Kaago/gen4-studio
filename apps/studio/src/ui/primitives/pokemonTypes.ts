import { t } from "../../i18n";
import type { typen } from "../tokens/tokens";

/** The key of a type's colour tokens (`--g4-szene-typ-<key>-grund`, `typen[key]`). */
export type TypeKey = keyof typeof typen;

export type PokemonType = {
  /** The number in the game data (Personal and move entries). */
  id: number;
  /** The name as the games show it in the language that is shown (`pokemonTypes.<key>` in the `ui` catalog); read on every access, never kept. */
  readonly name: string;
  key: TypeKey;
};

/** A type whose `name` is read from the catalog each time it is asked for (i18next is not ready when this module loads). */
const typeEntry = (id: number, key: TypeKey): PokemonType => ({
  id,
  key,
  get name() {
    return t(`ui:pokemonTypes.${key}`);
  },
});

/** The 18 types of the 4th generation, indexed by their number in the game data (9 is the "???" type). */
export const pokemonTypes: readonly PokemonType[] = [
  typeEntry(0, "normal"),
  typeEntry(1, "kampf"),
  typeEntry(2, "flug"),
  typeEntry(3, "gift"),
  typeEntry(4, "boden"),
  typeEntry(5, "gestein"),
  typeEntry(6, "kaefer"),
  typeEntry(7, "geist"),
  typeEntry(8, "stahl"),
  typeEntry(9, "fragezeichen"),
  typeEntry(10, "feuer"),
  typeEntry(11, "wasser"),
  typeEntry(12, "pflanze"),
  typeEntry(13, "elektro"),
  typeEntry(14, "psycho"),
  typeEntry(15, "eis"),
  typeEntry(16, "drache"),
  typeEntry(17, "unlicht"),
];

/** The type with this number, or `undefined` for a number the 4th generation does not know (a hack's own type). */
export function pokemonType(id: number): PokemonType | undefined {
  return pokemonTypes[id];
}

/** The name of a type in the language that is shown; an unknown number reads "Typ 19" ("Type 19"). */
export function typeName(id: number): string {
  return pokemonType(id)?.name ?? t("ui:pokemonTypes.unknown", { id });
}
