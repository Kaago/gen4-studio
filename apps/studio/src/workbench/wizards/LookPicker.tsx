import { useMemo, useRef, useState } from "react";
import type { HeaderSummary } from "../../api";
import { Button, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { thumbnails } from "../../world/thumbnailStore";
import { useThumbnail } from "../../world/useThumbnail";
import { mapOfHeader } from "../headers";
import { UNNAMED_PLACE } from "../places";
import { useMatrix } from "../useMatrix";
import { useVisible } from "../useVisible";
import styles from "./PlaceStep.module.css";

/** How many place names a look lists before „…“. */
const NAMES_SHOWN = 3;

/** One look (area data) of the game: the places that carry it, as one line („Zweiblattdorf, Sandgemme, Fleetburg …“). */
export interface Look {
  areaDataId: number;
  label: string;
}

/**
 * The looks the headers carry, lowest area data first, each named by its places in header order (each name once, at most three, then
 * „…“); a look whose headers have no place name is „Ohne Ortsnamen“.
 */
export function lookItems(headers: readonly HeaderSummary[]): Look[] {
  const places = new Map<number, string[]>();
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
      label: names.length === 0 ? UNNAMED_PLACE : `${names.slice(0, NAMES_SHOWN).join(", ")}${names.length > NAMES_SHOWN ? " …" : ""}`,
    }));
}

const ignore = () => {};

/** The template's map drawn in one look, 64 px; a flat tile until (or unless) the picture comes, drawn only while in view. */
function LookPreview({ map, areaDataId }: { map: number | null; areaDataId: number }) {
  const tile = useRef<HTMLSpanElement>(null);
  const visible = useVisible(tile, true);
  const picture = useThumbnail(thumbnails, map, areaDataId, visible);
  return <span ref={tile} className={styles.look} style={typeof picture === "string" ? { backgroundImage: `url("${picture}")` } : undefined} /* dyn */ data-look={areaDataId} />;
}

interface Props {
  headers: readonly HeaderSummary[];
  /** The header the new one is copied from: its map shows each look. */
  template: HeaderSummary | null;
  /** The chosen look (area data). */
  value: number;
  onChange: (areaDataId: number) => void;
}

/**
 * „Aussehen wie …“: a button with the chosen look that opens the picker of every look the game uses, each with the places that carry
 * it and the template's map drawn in it.
 */
export function LookPicker({ headers, template, value, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const looks = useMemo(() => lookItems(headers), [headers]);
  // A failed read only leaves the pictures out.
  const matrix = useMatrix(template?.matrix_id ?? null, 0, ignore);
  const map = matrix && template ? mapOfHeader(matrix, template.id) : null;
  const items = useMemo<PickerItem[]>(
    () => (open ? looks.map((look) => ({ id: look.areaDataId, label: look.label, preview: <LookPreview map={map} areaDataId={look.areaDataId} /> })) : []),
    [open, looks, map],
  );
  const chosen = looks.find((l) => l.areaDataId === value);
  // The button names the look by its first place.
  const name = chosen ? chosen.label.split(", ")[0]!.replace(/ …$/, "") : UNNAMED_PLACE;
  return (
    <>
      <div className={styles.lookRow}>
        <LookPreview map={map} areaDataId={value} />
        <Button kind="secondary" size="s" data-look-picker="" onClick={() => setOpen(true)}>
          {`Aussehen wie ${name}`}
        </Button>
      </div>
      <Picker open={open} onOpenChange={setOpen} title="Aussehen wählen" items={items} placeholder="Ort suchen" onPick={(item) => onChange(Number(item.id))} />
    </>
  );
}
