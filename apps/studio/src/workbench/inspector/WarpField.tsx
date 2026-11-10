import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HeaderSummary, WarpDto } from "../../api";
import { t as translate } from "../../i18n";
import { Button, InspectorRow, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { useEvents } from "../useEvents";
import styles from "./Editor.module.css";

/** A door of the destination by its number and tile. */
export const doorText = (warp: Pick<WarpDto, "index" | "x" | "z">) => translate("workbench:warpField.doorAt", { index: warp.index, x: warp.x, z: warp.z });

/**
 * The line „Gegenstück“ of a door: the door it leads to in the destination's place, and whether that one leads back to this door (warp
 * `self.warp` of `self.header`); a door number the destination does not have is said as such.
 */
export function counterpartText(destHeader: HeaderSummary, target: WarpDto | undefined, value: number, self: { header: number; warp: number }): string {
  if (target === undefined) return translate("workbench:warpField.targetHasNoDoor", { value });
  const back = target.dest_header_id === self.header && target.dest_warp_id === self.warp;
  const words = { index: target.index, place: headerLabel(destHeader) };
  return back ? translate("workbench:warpField.leadsBack", words) : translate("workbench:warpField.leadsNotBack", words);
}

/**
 * A door's field „Tür“ (the destination warp) in Erzählen: the door of the destination header's event file it leads to, by its number
 * and tile; „Ändern…“ picks another of that file's doors („Tür wählen“). Without a known destination there is nothing to pick, nor with a
 * deleted one (the header or its event file a tombstone, 3d.4): the kernel refuses a new reference to it. With `self` (this door: its
 * header and warp) the row „Gegenstück“ says whether that door leads back; `onConnect` adds „Tür verbinden…“, which opens the wizard with
 * this door as door A.
 */
export function WarpField({
  destHeader,
  value,
  disabled,
  onCommit,
  onError,
  self,
  onConnect,
}: {
  destHeader: HeaderSummary | undefined;
  value: number;
  disabled?: boolean;
  onCommit: (warp: number) => void;
  onError: (e: unknown) => void;
  self?: { header: number; warp: number };
  onConnect?: () => void;
}) {
  const { t } = useTranslation("workbench");
  const [picking, setPicking] = useState(false);
  const events = useEvents(destHeader?.event_file_id ?? null, onError);
  const current = events?.warps.find((w) => w.index === value);
  const gone = destHeader?.deleted === true || events?.deleted === true;
  const items = useMemo<PickerItem[]>(() => (picking && events && !gone ? events.warps.map((w) => ({ id: w.index, label: doorText(w), area: "welt" as const })) : []), [picking, events, gone]);
  const missing = events !== null && current === undefined;
  const hint = gone ? t("warpField.targetDeleted") : missing ? t("warpField.targetMissingDoor", { value }) : undefined;
  return (
    <>
      <InspectorRow label={t("warpField.door")} hint={hint}>
        <div className={styles.inline} data-warp-field="">
          <span>{current ? doorText(current) : t("warpField.doorNumber", { value })}</span>
          <Button kind="quiet" size="s" aria-label={t("warpField.changeDoor")} disabled={disabled || destHeader === undefined || gone} onClick={() => setPicking(true)}>
            {t("inspectorField.change")}
          </Button>
        </div>
      </InspectorRow>
      {self && (destHeader !== undefined || onConnect) && (
        <InspectorRow label={t("warpField.counterpart")}>
          <div className={styles.inline}>
            {destHeader !== undefined && events !== null && <span data-warp-counterpart="">{counterpartText(destHeader, current, value, self)}</span>}
            {onConnect && (
              <Button kind="quiet" size="s" disabled={disabled} onClick={onConnect} data-connect-doors="">
                {t("warpField.connect")}
              </Button>
            )}
          </div>
        </InspectorRow>
      )}
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title={t("warpField.chooseDoor")}
        items={items}
        placeholder={t("warpField.searchDoor")}
        emptyText={events === null ? t("inspectorField.loading") : t("warpField.targetHasNoDoors")}
        onPick={(item) => onCommit(Number(item.id))}
      />
    </>
  );
}
