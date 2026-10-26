// @vitest-environment node
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { HeaderListing, MatrixDto, MatrixListing, WarpListing, WorldMeta } from "../api";
import { type Area, type World, buildWorld } from "./worldModel";

/**
 * Gated: the world model on the three vanilla projects. `G4_WORLD_DUMP` is a folder with one sub-folder per project
 * (`3e-pt`, `3e-hgd`, `3e-hgu`), each written by `scripts/agent/world-dump.sh` (see docs/dev-setup.md, "Weltmodell (3e.2)").
 */
const dump = import.meta.env.G4_WORLD_DUMP as string | undefined;

interface Loaded {
  world: World;
  headerCount: number;
  ms: number;
}

const read = <T>(folder: string, file: string): T => JSON.parse(readFileSync(`${folder}/${file}`, "utf8")) as T;

function load(project: string, family: string, gameCode: string): Loaded {
  const folder = `${dump}/${project}`;
  expect(existsSync(folder), `${folder} is missing (run scripts/agent/world-dump.sh)`).toBe(true);
  expect(read<{ game_code: string }>(folder, "project.json").game_code).toBe(gameCode);
  const headers = read<HeaderListing>(folder, "headers.json").headers;
  const cells = new Map<number, MatrixDto>();
  for (const file of readdirSync(folder)) {
    const match = /^matrix-(\d+)\.json$/.exec(file);
    if (match) cells.set(Number(match[1]), read<MatrixDto>(folder, file));
  }
  const start = performance.now();
  const world = buildWorld({
    family,
    headers,
    matrices: read<MatrixListing>(folder, "matrices.json").matrices,
    links: read<WarpListing>(folder, "warps.json").links,
    cells,
    meta: read<{ meta: WorldMeta }>(folder, "meta.json").meta,
  });
  return { world, headerCount: headers.length, ms: performance.now() - start };
}

const areasOf = (world: World): Area[] => [...world.worldMaps.flatMap((m) => m.areas), ...world.moreAreas];

/** Every room with the area it lies in (null: "Nicht verbunden"). */
const roomsOf = (world: World) => [...areasOf(world).flatMap((a) => a.rooms.map((room) => ({ room, area: a }))), ...world.unconnected.map((room) => ({ room, area: null }))];

function expectSound({ world, headerCount, ms }: Loaded) {
  expect(world.assignedRooms / world.roomCount).toBeGreaterThanOrEqual(0.9);
  // Every header exactly once: as a header of an area, as a room or among the others.
  const ids = [...areasOf(world).flatMap((a) => a.headers.map((h) => h.id)), ...roomsOf(world).map(({ room }) => room.header.id), ...world.other.map((h) => h.id)];
  expect(ids.length).toBe(headerCount);
  expect(new Set(ids).size).toBe(headerCount);
  expect(ms).toBeLessThan(200);
}

const moreNames = (world: World) => world.moreAreas.map((a) => a.name);

const projects = [
  { project: "3e-pt", family: "Pt", gameCode: "CPUD", maps: ["Sinnoh"] },
  { project: "3e-hgd", family: "Hgss", gameCode: "IPKD", maps: ["Johto", "Kanto"] },
  { project: "3e-hgu", family: "Hgss", gameCode: "IPKE", maps: ["Johto", "Kanto"] },
];

describe("the world model on the vanilla projects", () => {
  if (!dump) {
    it.skip("needs G4_WORLD_DUMP=<folder of the scripts/agent/world-dump.sh dumps> (see docs/dev-setup.md, Weltmodell (3e.2))", () => {});
    return;
  }

  for (const { project, family, gameCode, maps } of projects) {
    describe(`${project} (${gameCode})`, () => {
      const loaded = load(project, family, gameCode);

      it("has the expected world maps", () => {
        expect(loaded.world.worldMaps.map((m) => m.name)).toEqual(maps);
      });

      it("assigns at least nine in ten rooms, places every header once and builds in under 200 ms", () => {
        expectSound(loaded);
      });
    });
  }

  describe("3e-pt (CPUD): Sinnoh", () => {
    const { world } = load("3e-pt", "Pt", "CPUD");

    it("lists Großmoor and the Kampfzone under the more areas", () => {
      expect(moreNames(world)).toContain("Großmoor");
      expect(moreNames(world)).toContain("Kampfzone");
    });

    it("puts every Kraterberg room under an area, some of them under another place (their entrance)", () => {
      const rooms = roomsOf(world).filter(({ room }) => room.placeName === "Kraterberg");
      expect(rooms.length).toBeGreaterThan(0);
      expect(rooms.every(({ area }) => area !== null)).toBe(true);
      expect(rooms.some(({ area }) => area !== null && area.name !== "Kraterberg")).toBe(true);
    });
  });

  for (const { project, gameCode } of projects.slice(1)) {
    it(`${project} (${gameCode}): the Safari Zone is an area of its own under the more areas`, () => {
      const { world } = load(project, "Hgss", gameCode);
      expect(world.moreAreas.some((a) => /^Safari/.test(a.name))).toBe(true);
    });
  }
});
