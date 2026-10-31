import type { BaseCheck, BaseMarkDto, BaseStatus, RomInfo } from "../api";

/** Test data for Technik › Veröffentlichen: the base of the test project (Platin DE) in its states. */

export const SHA1 = "d40f19ac24d67c721a4ab25e1c013154443d75b4";

export const mark = (over: Partial<BaseMarkDto> = {}): BaseMarkDto => ({
  version: 2,
  origin: "dspre",
  sha1: SHA1,
  game_code: "CPUD",
  title: "POKEMON PL",
  revision: 0,
  size: 134217728,
  created: "2026-10-27T10:00:00Z",
  assigned: "2026-10-27T10:00:00Z",
  studio_version: "0.1.0",
  secure_area_crc: 4660,
  secure_area_sha1: "0123456789abcdef0123456789abcdef01234567",
  ...over,
});

export const found = (over: Partial<Extract<BaseStatus, { state: "found" }>> = {}): BaseStatus => ({
  state: "found",
  mark: mark(),
  label: "Platin DE (Testprojekt)",
  path: "/Users/me/Roms/Pokemon - Platin-Edition (Germany).nds",
  source: "known",
  patch_name: "Sinnoh-Remix-CPUD",
  ...over,
});

export const missing = (over: Partial<Extract<BaseStatus, { state: "missing" }>> = {}): BaseStatus => ({
  state: "missing",
  mark: mark(),
  label: "Platin DE (Testprojekt)",
  reason: "Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).",
  ...over,
});

export const romInfo = (over: Partial<RomInfo> = {}): RomInfo => ({
  title: "POKEMON PL",
  game_code: "CPUD",
  family: "pt",
  language: "german",
  revision: 0,
  size: 134217728,
  sha1: SHA1,
  known_base: "Platin DE (Testprojekt)",
  support: { kind: "verified" },
  ...over,
});

export const check = (over: Partial<BaseCheck> = {}): BaseCheck => ({ info: romInfo(), warnings: [], refused: null, ...over });
