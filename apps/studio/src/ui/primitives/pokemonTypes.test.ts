import { describe, expect, it } from "vitest";
import { typen } from "../tokens/tokens";
import { pokemonType, pokemonTypes, typeName } from "./pokemonTypes";

describe("pokemonTypes", () => {
  it("lists_the_18_types_in_the_order_of_the_game_data", () => {
    expect(pokemonTypes.map((t) => [t.id, t.name])).toEqual([
      [0, "Normal"],
      [1, "Kampf"],
      [2, "Flug"],
      [3, "Gift"],
      [4, "Boden"],
      [5, "Gestein"],
      [6, "Käfer"],
      [7, "Geist"],
      [8, "Stahl"],
      [9, "???"],
      [10, "Feuer"],
      [11, "Wasser"],
      [12, "Pflanze"],
      [13, "Elektro"],
      [14, "Psycho"],
      [15, "Eis"],
      [16, "Drache"],
      [17, "Unlicht"],
    ]);
  });

  it("gives_every_type_the_key_of_its_colour_tokens", () => {
    expect(pokemonTypes.map((t) => t.key)).toEqual([
      "normal",
      "kampf",
      "flug",
      "gift",
      "boden",
      "gestein",
      "kaefer",
      "geist",
      "stahl",
      "fragezeichen",
      "feuer",
      "wasser",
      "pflanze",
      "elektro",
      "psycho",
      "eis",
      "drache",
      "unlicht",
    ]);
    // Every key has its colours in the generated tokens, and the tokens know no other type.
    expect(Object.keys(typen).sort()).toEqual(pokemonTypes.map((t) => t.key).sort());
  });

  it("looks_up_a_type_by_its_number_and_names_an_unknown_one_by_number", () => {
    expect(pokemonType(10)?.name).toBe("Feuer");
    expect(pokemonType(19)).toBeUndefined();
    expect(pokemonType(-1)).toBeUndefined();
    expect(typeName(2)).toBe("Flug");
    expect(typeName(19)).toBe("Typ 19");
  });
});
