import { useMemo, useState } from "react";
import type { HeaderSummary, WarpDto } from "../../api";
import { Button, InspectorRow, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { useEvents } from "../useEvents";
import styles from "./Editor.module.css";

/** A door of the destination by its number and tile. */
export const doorText = (warp: Pick<WarpDto, "index" | "x" | "z">) => `Tür ${warp.index} · x ${warp.x}, z ${warp.z}`;

/**
 * The line „Gegenstück“ of a door: the door it leads to in the destination's place, and whether that one leads back to this door (warp
 * `self.warp` of `self.header`); a door number the destination does not have is said as such.
 */
export function counterpartText(destHeader: HeaderSummary, target: WarpDto | undefined, value: number, self: { header: number; warp: number }): string {
  if (target === undefined) return `Ziel hat keine Tür ${value}`;
  const back = target.dest_header_id === self.header && target.dest_warp_id === self.warp;
  return `Tür ${target.index} in ${headerLabel(destHeader)} · ${back ? "führt zurück" : "führt nicht zurück"}`;
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
  const [picking, setPicking] = useState(false);
  const events = useEvents(destHeader?.event_file_id ?? null, onError);
  const current = events?.warps.find((w) => w.index === value);
  const gone = destHeader?.deleted === true || events?.deleted === true;
  const items = useMemo<PickerItem[]>(() => (picking && events && !gone ? events.warps.map((w) => ({ id: w.index, label: doorText(w), area: "welt" as const })) : []), [picking, events, gone]);
  const missing = events !== null && current === undefined;
  const hint = gone ? "Das Ziel ist gelöscht." : missing ? `Das Ziel hat keine Tür ${value}. Wähle eine seiner Türen.` : undefined;
  return (
    <>
      <InspectorRow label="Tür" hint={hint}>
        <div className={styles.inline} data-warp-field="">
          <span>{current ? doorText(current) : `Tür ${value}`}</span>
          <Button kind="quiet" size="s" aria-label="Tür ändern…" disabled={disabled || destHeader === undefined || gone} onClick={() => setPicking(true)}>
            Ändern…
          </Button>
        </div>
      </InspectorRow>
      {self && (destHeader !== undefined || onConnect) && (
        <InspectorRow label="Gegenstück">
          <div className={styles.inline}>
            {destHeader !== undefined && events !== null && <span data-warp-counterpart="">{counterpartText(destHeader, current, value, self)}</span>}
            {onConnect && (
              <Button kind="quiet" size="s" disabled={disabled} onClick={onConnect} data-connect-doors="">
                Tür verbinden…
              </Button>
            )}
          </div>
        </InspectorRow>
      )}
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title="Tür wählen"
        items={items}
        placeholder="Tür suchen"
        emptyText={events === null ? "Lädt …" : "Das Ziel hat keine Türen."}
        onPick={(item) => onCommit(Number(item.id))}
      />
    </>
  );
}
