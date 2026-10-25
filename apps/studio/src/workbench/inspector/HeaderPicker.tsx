import { useMemo, useRef } from "react";
import type { HeaderSummary } from "../../api";
import { Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { thumbnails } from "../../world/thumbnailStore";
import type { ThumbnailCache } from "../../world/thumbnails";
import { useThumbnail } from "../../world/useThumbnail";
import { mapOfHeader } from "../headers";
import { id4 } from "../terms";
import { useMatrix } from "../useMatrix";
import { useVisible } from "../useVisible";
import styles from "./HeaderPicker.module.css";

const ignore = () => {};

/** The picture of a header's map, read and drawn only while the row is in view; a flat tile stands in until (or unless) it comes. */
function HeaderPreview({ header, cache }: { header: HeaderSummary; cache: ThumbnailCache }) {
  const tile = useRef<HTMLSpanElement>(null);
  const visible = useVisible(tile, true);
  const matrix = useMatrix(visible ? header.matrix_id : null, 0, ignore);
  const map = matrix ? mapOfHeader(matrix, header.id) : null;
  const picture = useThumbnail(cache, map, header.area_data_id, visible);
  return <span ref={tile} className={styles.preview} style={typeof picture === "string" ? { backgroundImage: `url("${picture}")` } : undefined} /* dyn */ data-preview={header.id} />;
}

export interface HeaderPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  headers: readonly HeaderSummary[];
  /** Called with the id of the header that was chosen. */
  onPick: (id: number) => void;
  title?: string;
  /** The map pictures; the app-wide cache by default. */
  cache?: ThumbnailCache;
  /** A line under a header's name (the wizards say how many warps lead there); none by default. */
  describe?: (header: HeaderSummary) => string;
}

/** Searchable list of the project's headers: location name, identifier and the picture of its map; picking one calls `onPick` with its id. */
export function HeaderPicker({ open, onOpenChange, headers, onPick, title = "Ziel wählen", cache = thumbnails, describe }: HeaderPickerProps) {
  const items = useMemo<PickerItem[]>(
    () =>
      open
        ? headers.map((h) => ({
            id: h.id,
            label: h.location_name || h.internal_name,
            description: describe?.(h),
            tech: h.location_name ? `${id4(h.id)} · ${h.internal_name}` : id4(h.id),
            area: "welt",
            preview: <HeaderPreview header={h} cache={cache} />,
          }))
        : [],
    [open, headers, cache, describe],
  );
  return <Picker open={open} onOpenChange={onOpenChange} title={title} items={items} placeholder="Ort, Kürzel oder Nummer suchen" onPick={(item) => onPick(Number(item.id))} />;
}
