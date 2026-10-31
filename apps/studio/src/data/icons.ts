import { useEffect, useState } from "react";
import { getBattleSprite, getItemIcon, getMonIcon, getTrainerSprite } from "../api";
import type { BattleSpriteRequest, ItemEntryDto } from "../api";

/**
 * The icons of the open project as blob URLs, by key: `mon:<species>` for a menu icon, `item:<icon>:<palette>` for a bag icon (several
 * item ids share one), `battle:<species>:<form>:<side>:<female>:<shiny>:<frame>` for a battle sprite and `trainer:<class>` for a
 * trainer's battle image. Concurrent asks share one request; an icon the game does not have and a failed request are remembered as `null`
 * like any other result, for as long as the project is open. `resetIcons` frees the URLs and forgets all (another project was opened).
 */
const requests = new Map<string, Promise<string | null>>();
/** The settled entries of `requests`, for a synchronous look. */
const settled = new Map<string, string | null>();
/** Bumped by `resetIcons`; a request started before it belongs to the previous project. */
let generation = 0;

export const monIconKey = (species: number) => `mon:${species}`;
export const itemIconKey = (entry: ItemEntryDto) => `item:${entry.icon}:${entry.palette}`;
export const battleSpriteKey = ({ species, form, side, female, shiny, frame }: BattleSpriteRequest) =>
  `battle:${species}:${form ?? 0}:${side}:${female ? 1 : 0}:${shiny ? 1 : 0}:${frame ?? 0}`;
export const trainerSpriteKey = (cls: number) => `trainer:${cls}`;

/** The URL of the icon `key` once `fetch` delivered its PNG bytes; null when there is none or it could not be read. */
export function loadIcon(key: string, fetch: () => Promise<ArrayBuffer | null>): Promise<string | null> {
  const known = requests.get(key);
  if (known) return known;
  const started = generation;
  const request = fetch().then(
    (bytes) => {
      // The project changed while the request ran: nobody will free a URL made now.
      if (started !== generation) return null;
      const url = bytes === null ? null : URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
      settled.set(key, url);
      return url;
    },
    () => {
      // A failure is remembered like a missing icon until another project: a row scrolled back into view does not ask again.
      if (started === generation) settled.set(key, null);
      return null;
    },
  );
  requests.set(key, request);
  return request;
}

/** The menu icon of a species (personal member). */
export const monIcon = (species: number) => loadIcon(monIconKey(species), () => getMonIcon(species));
/** The bag icon of item `id`, whose item table entry is `entry`. */
export const itemIcon = (id: number, entry: ItemEntryDto) => loadIcon(itemIconKey(entry), () => getItemIcon(id));
/** A battle sprite of a Pokémon (80 x 80). */
export const battleSprite = (req: BattleSpriteRequest) => loadIcon(battleSpriteKey(req), () => getBattleSprite(req));
/** The battle image of trainer class `cls` (80 x 80). */
export const trainerSprite = (cls: number) => loadIcon(trainerSpriteKey(cls), () => getTrainerSprite(cls));

/** The URL if the icon has loaded (`null`: the game has none), `undefined` while it is unknown or loading. */
export function peekIcon(key: string): string | null | undefined {
  return settled.get(key);
}

/** Forgets every icon and frees the URLs (another project was opened). */
export function resetIcons(): void {
  generation++;
  for (const url of settled.values()) if (url !== null && typeof URL.revokeObjectURL === "function") URL.revokeObjectURL(url);
  settled.clear();
  requests.clear();
}

/**
 * The URL of icon `key` for a row: what is known at once, else loaded while `active` (the row is in view). Null while there is none;
 * `key` null asks for nothing.
 */
export function useIcon(key: string | null, load: () => Promise<string | null>, active: boolean): string | null {
  const [loaded, setLoaded] = useState<{ key: string; url: string | null } | null>(null);
  const known = key === null ? null : peekIcon(key);
  useEffect(() => {
    if (key === null || !active || peekIcon(key) !== undefined) return;
    let current = true;
    void load().then((url) => current && setLoaded({ key, url }));
    return () => {
      current = false;
    };
    // `load` belongs to `key`: a new closure for the same key asks for the same icon.
  }, [key, active]);
  if (key === null) return null;
  if (known !== undefined) return known;
  return loaded?.key === key ? loaded.url : null;
}
