import type { typen } from "../tokens/tokens";

/** The key of a type's colour tokens (`--g4-szene-typ-<key>-grund`, `typen[key]`). */
export type TypeKey = keyof typeof typen;

export type PokemonType = {
  /** The number in the game data (Personal and move entries). */
  id: number;
  /** The German name as the games show it. */
  name: string;
  key: TypeKey;
};

/** The 18 types of the 4th generation, indexed by their number in the game data (9 is the "???" type). */
export const pokemonTypes: readonly PokemonType[] = [
  { id: 0, name: "Normal", key: "normal" },
  { id: 1, name: "Kampf", key: "kampf" },
  { id: 2, name: "Flug", key: "flug" },
  { id: 3, name: "Gift", key: "gift" },
  { id: 4, name: "Boden", key: "boden" },
  { id: 5, name: "Gestein", key: "gestein" },
  { id: 6, name: "Käfer", key: "kaefer" },
  { id: 7, name: "Geist", key: "geist" },
  { id: 8, name: "Stahl", key: "stahl" },
  { id: 9, name: "???", key: "fragezeichen" },
  { id: 10, name: "Feuer", key: "feuer" },
  { id: 11, name: "Wasser", key: "wasser" },
  { id: 12, name: "Pflanze", key: "pflanze" },
  { id: 13, name: "Elektro", key: "elektro" },
  { id: 14, name: "Psycho", key: "psycho" },
  { id: 15, name: "Eis", key: "eis" },
  { id: 16, name: "Drache", key: "drache" },
  { id: 17, name: "Unlicht", key: "unlicht" },
];

/** The type with this number, or `undefined` for a number the 4th generation does not know (a hack's own type). */
export function pokemonType(id: number): PokemonType | undefined {
  return pokemonTypes[id];
}

/** The German name of a type; an unknown number reads "Typ 19". */
export function typeName(id: number): string {
  return pokemonType(id)?.name ?? `Typ ${id}`;
}
