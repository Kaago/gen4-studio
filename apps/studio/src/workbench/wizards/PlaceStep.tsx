import { useEffect, useMemo, useRef, useState } from "react";
import { errorText, getTextArchive, placeSlots } from "../../api";
import type { HeaderFamily, HeaderSummary, PlaceChoice, PlaceSlots } from "../../api";
import { Button, Notice, Picker, Segmented, TextField } from "../../ui/primitives";
import type { PickerItem, SegmentedOption } from "../../ui/primitives";
import { UNNAMED_PLACE, placeColor } from "../places";
import { DP_PLACES, nameWarning, placeItems, placeKeyOf, reuseWarning } from "../places/placeChoices";
import { NO_PLACE_INDEX } from "../worldModel";
import { LookPicker } from "./LookPicker";
import { placeError } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import styles from "./PlaceStep.module.css";

/** What the step „Ort“ edits: the place of the new area or room and its look (area data). */
export interface PlaceStepValue {
  place: PlaceChoice;
  areaDataId: number;
}

interface Props {
  value: PlaceStepValue;
  onChange: (value: PlaceStepValue) => void;
  /** The header list: the places with how many areas and rooms carry them, and the looks with their places. */
  headers: readonly HeaderSummary[];
  /** The header the new one is copied from: its place is where „Vorhandener Ort“ starts, its map shows the looks. */
  template: HeaderSummary | null;
  /** Diamond/Pearl has no new places: „Neuer Ort“ is locked there. */
  family: HeaderFamily;
  /** The wizard's name of the new area or room: „Neuer Ort“ starts with it. */
  name: string;
}

type Mode = "existing" | "new";

const NO_PLACE_SHORT = "Kein Ort";

/**
 * The step „Ort“ of the wizards in Bauen: an existing place (a button with its colour and name opens the picker „Ort wählen“) or a new
 * one (its name, with the notes of „Neuer Ort“ in the inspector: a name longer than the game shows, the slot of an unused Sinnoh name
 * on a full bank); below, „Aussehen wie …“ picks the look.
 */
export function PlaceStep({ value, onChange, headers, template, family, name }: Props) {
  const [picking, setPicking] = useState(false);
  const [names, setNames] = useState<string[] | null>(null);
  const [slots, setSlots] = useState<PlaceSlots | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const { place } = value;
  const creating = "New" in place;
  // What each mode had last: switching back brings it again.
  const lastExisting = useRef<number | null>(null);
  const lastNew = useRef<string | null>(null);
  useEffect(() => {
    if ("New" in place) lastNew.current = place.New;
    else lastExisting.current = place.Existing;
  }, [place]);

  // The names of the bank, for the picker, the button and the warning before reusing a slot.
  useEffect(() => {
    let live = true;
    void (async () => {
      try {
        const room = await placeSlots();
        const archive = await getTextArchive(room.bank);
        if (!live) return;
        setSlots(room);
        setNames(archive.messages.map((m) => m.text));
      } catch (e) {
        if (live) setReadError(errorText(e));
      }
    })();
    return () => {
      live = false;
    };
  }, []);

  const set = (change: Partial<PlaceStepValue>) => onChange({ ...value, ...change });
  const choose = (mode: Mode) =>
    set({ place: mode === "new" ? { New: lastNew.current ?? name } : { Existing: lastExisting.current ?? template?.location_index ?? NO_PLACE_INDEX } });

  const items = useMemo<PickerItem[]>(
    () =>
      names === null
        ? []
        : placeItems(headers, names).map((item) => {
            const index = Number(item.id);
            return { ...item, preview: <span className={styles.dot} style={{ background: placeColor(placeKeyOf(index, names[index] ?? "")) }} /* dyn */ /> };
          }),
    [names, headers],
  );

  const locked = family === "DP";
  const modes: SegmentedOption<Mode>[] = [
    { value: "existing", label: "Vorhandener Ort" },
    { value: "new", label: "Neuer Ort", disabled: locked },
  ];

  let body;
  if ("New" in place) {
    const long = nameWarning(place.New);
    const reuse = slots && names ? reuseWarning(slots, names) : null;
    body = (
      <>
        <div className={parts.field}>
          <TextField
            label="Name des neuen Orts"
            showLabel
            value={place.New}
            onChange={(text) => set({ place: { New: text } })}
            onCommit={(text) => set({ place: { New: text } })}
            error={placeError(place) ?? undefined}
          />
        </div>
        {long && <Notice kind="warning">{long}</Notice>}
        {reuse && <Notice kind="warning">{reuse}</Notice>}
      </>
    );
  } else {
    const index = place.Existing;
    const known = names?.[index] ?? headers.find((h) => h.location_index === index)?.location_name ?? "";
    const label = index === NO_PLACE_INDEX ? NO_PLACE_SHORT : known.trim() || UNNAMED_PLACE;
    body = (
      <>
        <Button kind="secondary" size="s" data-place-picker="" title="Ort wählen" onClick={() => setPicking(true)}>
          <span className={styles.dot} style={{ background: placeColor(placeKeyOf(index, known)) }} /* dyn */ />
          {label}
        </Button>
        <Picker
          open={picking}
          onOpenChange={setPicking}
          title="Ort wählen"
          items={items}
          placeholder="Ort suchen"
          emptyText={names !== null ? undefined : readError !== null ? "Orte nicht lesbar." : "Lade Orte …"}
          onPick={(item) => set({ place: { Existing: Number(item.id) } })}
        />
      </>
    );
  }

  return (
    <>
      <section className={parts.column}>
        <h3 className={parts.heading}>Ort</h3>
        <Segmented ariaLabel="Ort" options={modes} value={creating ? "new" : "existing"} onChange={choose} />
        {locked && <p className={parts.note}>{DP_PLACES}</p>}
        {body}
        {readError !== null && <Notice kind="error">{readError}</Notice>}
      </section>
      <section className={parts.column}>
        <h3 className={parts.heading}>Aussehen</h3>
        <LookPicker headers={headers} template={template} value={value.areaDataId} onChange={(areaDataId) => set({ areaDataId })} />
      </section>
    </>
  );
}
