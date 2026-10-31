import { useEffect, useMemo, useRef, useState } from "react";
import { assignPlace, getTextArchive, placeSlots } from "../../api";
import type { HeaderFamily, HeaderSummary } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Button, InspectorRow, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { UNNAMED_PLACE, placeColor } from "../places";
import { id4 } from "../terms";
import { useWorld } from "../WorkbenchContext";
import { NO_PLACE_INDEX } from "../worldModel";
import { NewPlaceDialog } from "./NewPlaceDialog";
import { DP_PLACES, placeItems, placeKeyOf } from "./placeChoices";
import styles from "./PlacePicker.module.css";

export interface PlacePickerProps {
  /** The area or room whose place is shown and chosen. */
  header: HeaderSummary;
  /** Bauen calls the row „Ort“; Tüfteln „Ortsname“ and adds the message and the archive of the name. */
  depth: Depth;
  /** A place went to the header: an existing one (`bank` null) or a new one, which changed the location bank `bank`. */
  onAssigned: (bank: number | null) => void | Promise<void>;
  /** The header list: how many headers carry each place. */
  headers: readonly HeaderSummary[];
  /** Diamond/Pearl has no place commands: „Neuer Ort…“ is locked there. */
  family: HeaderFamily;
  onError: (e: unknown) => void;
  /** Writes the picked index as the header field instead of `assignPlace` (Diamond/Pearl, where the kernel has no place commands). */
  commit?: (index: number) => void;
}

const NO_PLACE_SHORT = "Kein Ort";

/**
 * The field „Ort“ of an area or room: a button with the place's colour and name that opens the picker „Ort wählen“ over every name of
 * the location bank; picking one gives it to the header (one undo step), „Neuer Ort…“ below the list opens the dialog „Neuer Ort“.
 */
export function PlacePicker({ header, depth, onAssigned, headers, family, onError, commit }: PlacePickerProps) {
  const tinker = depthAtLeast(depth, "tuefteln");
  const { world } = useWorld();
  const [picking, setPicking] = useState(false);
  const [creating, setCreating] = useState(false);
  const [bank, setBank] = useState<number | null>(null);
  const [names, setNames] = useState<string[] | null>(null);
  const fail = useRef(onError);
  fail.current = onError;

  // Tüfteln names the archive of the bank beside the name.
  useEffect(() => {
    if (!tinker || bank !== null) return;
    let live = true;
    placeSlots().then(
      (slots) => live && setBank(slots.bank),
      (e) => live && fail.current(e),
    );
    return () => {
      live = false;
    };
  }, [tinker, bank]);

  // The names are read on every opening: a place created or renamed since then is in the list.
  async function open() {
    setNames(null);
    setPicking(true);
    try {
      const at = bank ?? (await placeSlots()).bank;
      setBank(at);
      setNames((await getTextArchive(at)).messages.map((m) => m.text));
    } catch (e) {
      onError(e);
    }
  }

  const items = useMemo<PickerItem[]>(
    () =>
      names === null
        ? []
        : placeItems(headers, names, world).map((item) => {
            const index = Number(item.id);
            return {
              ...item,
              preview: <span data-place-dot="" className={styles.dot} style={{ background: placeColor(placeKeyOf(index, names[index] ?? "")) }} /* dyn */ />,
              tech: tinker ? `Nachricht ${index}` : undefined,
            };
          }),
    [names, headers, tinker],
  );

  async function pick(index: number) {
    if (index === header.location_index) return;
    if (commit) {
      commit(index);
      return;
    }
    try {
      await assignPlace([header.id], index);
    } catch (e) {
      onError(e);
      return;
    }
    await onAssigned(null);
  }

  const index = header.location_index;
  const name = index === NO_PLACE_INDEX ? NO_PLACE_SHORT : header.location_name.trim() || UNNAMED_PLACE;
  const locked = family === "DP";
  return (
    <>
      <InspectorRow label={tinker ? "Ortsname" : "Ort"}>
        <Button kind="secondary" size="s" data-place-picker="" title="Ort wählen" onClick={() => void open()}>
          <span data-place-dot="" className={styles.dot} style={{ background: placeColor(placeKeyOf(index, header.location_name)) }} /* dyn */ />
          {name}
          {tinker && <span className={styles.tech}>{`Nachricht ${index}${bank === null ? "" : ` · Archiv ${id4(bank)}`}`}</span>}
        </Button>
      </InspectorRow>
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title="Ort wählen"
        items={items}
        placeholder="Ort oder Nummer suchen"
        emptyText={names === null ? "Lade Orte …" : undefined}
        onPick={(item) => void pick(Number(item.id))}
        footer={
          <div className={styles.footer}>
            <Button
              kind="secondary"
              size="s"
              icon="plus"
              disabled={locked}
              onClick={() => {
                setPicking(false);
                // On the next tick: the picker's focus return must not run into the dialog that opens.
                setTimeout(() => setCreating(true), 0);
              }}
            >
              Neuer Ort…
            </Button>
            {locked && <span>{DP_PLACES}</span>}
          </div>
        }
      />
      <NewPlaceDialog open={creating} header={header} onClose={() => setCreating(false)} onCreated={(_, changed) => onAssigned(changed)} onError={onError} />
    </>
  );
}
