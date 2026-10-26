import { useEffect, useRef, useState } from "react";
import { createPlace, getTextArchive, placeSlots } from "../../api";
import type { HeaderSummary, PlaceCreated } from "../../api";
import { toast } from "../../ui/frame/toasts";
import { Checkbox, Dialog, Notice, TextField } from "../../ui/primitives";
import { useWorld } from "../WorkbenchContext";
import { placeNameOf } from "../worldModel";
import { nameWarning, presetChoice, reuseWarning } from "./placeChoices";
import type { PlaceChoice } from "./placeChoices";
import styles from "./PlacePicker.module.css";

export interface NewPlaceDialogProps {
  open: boolean;
  /** The area or room the dialog is opened from: it decides what „Mitnehmen“ offers and ticks. */
  header: HeaderSummary;
  onClose: () => void;
  /** The place was created (one undo step): `bank` is the location bank that changed (null while it was not known yet). */
  onCreated: (created: PlaceCreated, bank: number | null) => void | Promise<void>;
  /** A refusal of the kernel (an empty name, an unknown character, a full bank); the dialog stays open. */
  onError: (e: unknown) => void;
}

/**
 * „Neuer Ort“: a name and the area and rooms that take it along (`presetChoice`). Says before the click when the name is longer than
 * the game shows and when the new place takes the slot of an unused Sinnoh name. Mounted anew for every opening.
 */
export function NewPlaceDialog(props: NewPlaceDialogProps) {
  return props.open ? <NewPlaceForm {...props} /> : null;
}

function NewPlaceForm({ header, onClose, onCreated, onError }: NewPlaceDialogProps) {
  const { world } = useWorld();
  const [choices, setChoices] = useState<PlaceChoice[]>(() =>
    world ? presetChoice(world, header) : [{ id: header.id, label: placeNameOf(header) || header.internal_name, checked: true, headers: [header.id] }],
  );
  const [name, setName] = useState("");
  const [bank, setBank] = useState<number | null>(null);
  const [reuse, setReuse] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const fail = useRef(onError);
  fail.current = onError;

  // The room in the bank: on a full HeartGold/SoulSilver bank the name of the slot the new place takes.
  useEffect(() => {
    let live = true;
    void (async () => {
      try {
        const slots = await placeSlots();
        if (!live) return;
        setBank(slots.bank);
        if (slots.free > 0 || slots.reusable.length === 0) return;
        const archive = await getTextArchive(slots.bank);
        if (live) setReuse(reuseWarning(slots, archive.messages.map((m) => m.text)));
      } catch (e) {
        if (live) fail.current(e);
      }
    })();
    return () => {
      live = false;
    };
  }, []);

  const toggle = (id: number, checked: boolean) => setChoices((list) => list.map((c) => (c.id === id ? { ...c, checked } : c)));

  async function create() {
    if (working) return;
    setWorking(true);
    let created: PlaceCreated;
    try {
      created = await createPlace(name, choices.filter((c) => c.checked).flatMap((c) => c.headers));
    } catch (e) {
      setWorking(false);
      onError(e);
      return;
    }
    toast.info(`Ort „${name}“ angelegt`);
    onClose();
    await onCreated(created, bank);
  }
  const leave = () => {
    if (!working) onClose();
  };
  const long = nameWarning(name);

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title="Neuer Ort"
      actions={[
        { label: "Abbrechen", onClick: leave },
        { label: "Ort anlegen", kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.form} data-new-place="" aria-busy={working}>
        <TextField label="Name des Orts" showLabel value={name} onChange={setName} onCommit={setName} disabled={working} />
        {long && <Notice kind="warning">{long}</Notice>}
        {reuse && <Notice kind="warning">{reuse}</Notice>}
        <div className={styles.choices} role="group" aria-label="Mitnehmen">
          <p className={styles.legend}>Mitnehmen</p>
          {choices.map((c) => (
            <Checkbox key={c.id} label={c.label} checked={c.checked} disabled={working} onChange={(checked) => toggle(c.id, checked)} />
          ))}
        </div>
      </div>
    </Dialog>
  );
}
