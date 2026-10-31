import type { HeaderSummary } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { UNNAMED_PLACE } from "../places";
import { id4 } from "../terms";

/** How many place names a look lists before „…“. */
const NAMES_SHOWN = 3;

/** One look (area data) of the game: the places that carry it (each name once, in header order) and them as one line („Zweiblattdorf, Sandgemme, Fleetburg …“). */
export interface Look {
  areaDataId: number;
  names: string[];
  label: string;
}

/**
 * The looks the headers carry, lowest area data first, each named by its places in header order (each name once, at most three, then
 * „…“); a look whose headers have no place name is „Ohne Ortsnamen“. The look of `template` (the header the new one is copied from)
 * names the template's own place first: Route 201 shares its look with Jubelstadt and is still „wie Route 201“.
 */
export function lookItems(headers: readonly HeaderSummary[], template: HeaderSummary | null = null): Look[] {
  const places = new Map<number, string[]>();
  const own = template?.location_name.trim() ?? "";
  if (template && own !== "") places.set(template.area_data_id, [own]);
  for (const h of headers) {
    const names = places.get(h.area_data_id) ?? [];
    const name = h.location_name.trim();
    if (name !== "" && !names.includes(name)) names.push(name);
    places.set(h.area_data_id, names);
  }
  return [...places]
    .sort(([a], [b]) => a - b)
    .map(([areaDataId, names]) => ({
      areaDataId,
      names,
      label: names.length === 0 ? UNNAMED_PLACE : `${names.slice(0, NAMES_SHOWN).join(", ")}${names.length > NAMES_SHOWN ? " …" : ""}`,
    }));
}

/**
 * A look in words: „Aussehen wie <first place>“, „Aussehen: Ohne Ortsnamen“ for a look no named place carries; a look no header
 * carries is „Aussehen der Vorlage“ in Bauen (the template's is the only one the step does not choose) and its number in Tüfteln.
 */
export function lookText(looks: readonly Look[], areaDataId: number, depth: Depth): string {
  const look = looks.find((l) => l.areaDataId === areaDataId);
  if (look === undefined) return !depthAtLeast(depth, "tuefteln") ? "Aussehen der Vorlage" : `Aussehen ${id4(areaDataId)}`;
  return look.names.length > 0 ? `Aussehen wie ${look.names[0]}` : `Aussehen: ${UNNAMED_PLACE}`;
}
