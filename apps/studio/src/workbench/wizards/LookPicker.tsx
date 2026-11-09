import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HeaderSummary } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { Button, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { thumbnails } from "../../world/thumbnailStore";
import { useThumbnail } from "../../world/useThumbnail";
import { mapOfHeader } from "../headers";
import { useMatrix } from "../useMatrix";
import { useVisible } from "../useVisible";
import { lookItems, lookText } from "./looks";
import styles from "./PlaceStep.module.css";

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
  /** How a look no header carries is named (see `lookText`). Default: Tüfteln, like the other components (callers pass it). */
  depth?: Depth;
  /** The button cannot open the picker (a locked record). */
  disabled?: boolean;
}

/**
 * „Aussehen wie …“: a button with the chosen look that opens the picker of every look the game uses, each with the places that carry
 * it and the template's map drawn in it.
 */
export function LookPicker({ headers, template, value, onChange, depth = "tuefteln", disabled }: Props) {
  const { t } = useTranslation("workbench");
  const [open, setOpen] = useState(false);
  const looks = useMemo(() => lookItems(headers, template), [headers, template]);
  // A failed read only leaves the pictures out.
  const matrix = useMatrix(template?.matrix_id ?? null, 0, ignore);
  const map = matrix && template ? mapOfHeader(matrix, template.id) : null;
  const items = useMemo<PickerItem[]>(
    () => (open ? looks.map((look) => ({ id: look.areaDataId, label: look.label, preview: <LookPreview map={map} areaDataId={look.areaDataId} /> })) : []),
    [open, looks, map],
  );
  return (
    <>
      <div className={styles.lookRow}>
        <LookPreview map={map} areaDataId={value} />
        <Button kind="secondary" size="s" data-look-picker="" disabled={disabled} onClick={() => setOpen(true)}>
          {lookText(looks, value, depth)}
        </Button>
      </div>
      <Picker open={open} onOpenChange={setOpen} title={t("lookPicker.title")} items={items} placeholder={t("lookPicker.search")} onPick={(item) => onChange(Number(item.id))} />
    </>
  );
}
