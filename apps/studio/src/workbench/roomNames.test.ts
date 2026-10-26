import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { roomName } from "./roomNames";

const header = (internal_name: string): HeaderSummary => ({
  id: 1,
  internal_name,
  location_name: "Jubelstadt",
  location_index: 6,
  kanto: false,
  matrix_id: 1,
  area_data_id: 0,
  event_file_id: 1,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

describe("roomName", () => {
  it("names_a_pokemon_center_and_its_floors", () => {
    expect(roomName(header("C01PC0101"), "Jubelstadt", "Jubelstadt", 1)).toBe("Pokémon-Center");
    expect(roomName(header("C01PC0102"), "Jubelstadt", "Jubelstadt", 1)).toBe("Pokémon-Center, 1. Stock");
  });

  it("names_a_house_by_its_number_and_floor", () => {
    expect(roomName(header("T01R0101"), "Zweiblattdorf", "Zweiblattdorf", 1)).toBe("Haus 1");
    expect(roomName(header("T01R0102"), "Zweiblattdorf", "Zweiblattdorf", 1)).toBe("Haus 1, 1. Stock");
    expect(roomName(header("T01R0201"), "Zweiblattdorf", "Zweiblattdorf", 1)).toBe("Haus 2");
  });

  it("names_a_mart", () => {
    expect(roomName(header("C03FS0101"), "Erzelingen", "Erzelingen", 1)).toBe("Pokémarkt");
  });

  it("names_a_gym_and_its_rooms", () => {
    expect(roomName(header("C01GYM0101"), "Jubelstadt", "Jubelstadt", 1)).toBe("Arena");
    expect(roomName(header("C05GYM0103"), "Herzhofen", "Herzhofen", 1)).toBe("Arena, Raum 3");
  });

  it("names_a_level_of_a_dungeon_without_a_leading_zero", () => {
    expect(roomName(header("D05R0103"), "Kraterberg", "Route 211", 1)).toBe("Ebene 3");
    expect(roomName(header("D05R0112"), "Kraterberg", "Route 211", 1)).toBe("Ebene 12");
  });

  it("falls_back_to_the_place_name_or_a_numbered_room", () => {
    expect(roomName(header("C01R0206"), "Jubelstadt TV", "Jubelstadt", 1)).toBe("Haus 2, 5. Stock");
    expect(roomName(header("C01TV"), "Jubelstadt TV", "Jubelstadt", 2)).toBe("Jubelstadt TV");
    expect(roomName(header("UNION"), "Jubelstadt", "Jubelstadt", 2)).toBe("Raum 2");
    expect(roomName(header("UNION"), "", "Jubelstadt", 3)).toBe("Raum 3");
  });
});
