import { useEffect, useRef, useState } from "react";
import type { FocusEvent, ReactNode } from "react";
import { dataRaw, errorText, fieldLabels } from "../../api";
import type { RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, Segmented, SelectField, pokemonTypes } from "../../ui/primitives";
import type { NumberFieldProps, SegmentedOption, SelectOption } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import { CATEGORIES, FLAGS, NAMED_FLAGS, TARGETS, flagLabel, hasFlag, hex2, lockText, moveLabel, moveValues, ppMax, withFlag } from "./moveModel";
import type { MoveFlag, MoveField, MoveValues } from "./moveModel";
import { useMove } from "./useMove";
import type { MoveDoc } from "./useMove";
import styles from "./Moves.module.css";

const TYPES: readonly SelectOption<number>[] = pokemonTypes.map((t) => ({ value: t.id, label: t.name }));

/** The three categories; a value the games do not know shows as a fourth, locked choice ("Wert 3"). */
function categoryOptions(value: number): SegmentedOption<string>[] {
  const known = Object.entries(CATEGORIES).map(([v, label]) => ({ value: v, label }));
  return CATEGORIES[value] === undefined ? [...known, { value: String(value), label: `Wert ${value}`, disabled: true }] : known;
}

/** A column of the hand: a bold title with its grey note. */
function Column({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className={styles.column} aria-label={title}>
      <h3 className={styles.columnTitle}>
        {title}
        {note && <span className={styles.columnNote}>{note}</span>}
      </h3>
      {children}
    </section>
  );
}

/** What every field of the hand works with: the move's values and how to write one. */
interface Editing {
  v: MoveValues;
  /** Number fields show the Dez | Hex switch (Tüfteln). */
  showBase: boolean;
  set(field: MoveField, value: number): void;
  setFlag(flag: MoveFlag, on: boolean): void;
  flags: number;
}

function MoveNumber({ e, field, label = fieldLabels.move[field], ...limits }: { e: Editing; field: MoveField; label?: string } & Pick<NumberFieldProps, "bits" | "min" | "max" | "hint">) {
  return (
    <InspectorRow label={label}>
      <div data-field={field}>
        <NumberField label={label} value={e.v[field] ?? null} onCommit={(value) => e.set(field, value)} showBase={e.showBase} {...limits} />
      </div>
    </InspectorRow>
  );
}

function MoveSelect({ e, field, options }: { e: Editing; field: MoveField; options: readonly SelectOption<number>[] }) {
  const label = fieldLabels.move[field];
  return (
    <InspectorRow label={label}>
      <div data-field={field}>
        <SelectField label={label} value={e.v[field] ?? 0} options={options} onChange={(value) => e.set(field, value)} />
      </div>
    </InspectorRow>
  );
}

/** Grundwerte: type, category, power, accuracy, PP, priority, effect chance. */
function BaseColumn({ e }: { e: Editing }) {
  const category = e.v.category ?? 0;
  return (
    <Column title="Grundwerte">
      <MoveSelect e={e} field="move_type" options={TYPES} />
      <InspectorRow label="Kategorie">
        <div data-field="category">
          <Segmented ariaLabel="Kategorie" size="s" options={categoryOptions(category)} value={String(category)} onChange={(value) => e.set("category", Number(value))} />
        </div>
      </InspectorRow>
      <MoveNumber e={e} field="power" bits={8} hint="0 heißt keine Stärke" />
      <MoveNumber e={e} field="accuracy" bits={8} hint="in Prozent, 0 trifft immer" />
      <MoveNumber e={e} field="pp" bits={8} hint={`mit AP-Plus höchstens ${ppMax(e.v.pp ?? 0)}`} />
      <MoveNumber e={e} field="priority" min={-7} max={5} hint="von −7 bis +5, höher greift zuerst an" />
      <MoveNumber e={e} field="effect_chance" bits={8} hint="in Prozent" />
    </Column>
  );
}

/** Verhalten: the target and the named flags. */
function BehaviourColumn({ e }: { e: Editing }) {
  return (
    <Column title="Verhalten">
      <MoveSelect e={e} field="target" options={TARGETS} />
      <div className={styles.flags} data-field="flags" role="group" aria-label="Merkmale">
        {NAMED_FLAGS.map((f) => (
          <Checkbox key={f.bit} label={f.label} checked={hasFlag(e.flags, f.mask)} onChange={(on) => e.setFlag(f, on)} />
        ))}
      </div>
    </Column>
  );
}

/** The bytes of the move as the store holds them, read again with every new document (an edit, undo, redo). */
function RawBytes({ index, doc, highlight }: { index: number; doc: MoveDoc; highlight: string | undefined }) {
  const [raw, setRaw] = useState<{ raw: RawDto | null; error: string | null } | null>(null);
  useEffect(() => {
    let current = true;
    dataRaw("move", index).then(
      (r) => current && setRaw({ raw: r, error: null }),
      (err: unknown) => current && setRaw({ raw: null, error: errorText(err) }),
    );
    return () => {
      current = false;
    };
  }, [index, doc]);
  if (raw === null) return <p className={styles.note}>Lädt …</p>;
  if (raw.raw === null) return <p className={styles.note}>{`Nicht lesbar: ${raw.error ?? ""}`}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} highlight={highlight} path={raw.raw.path} member={raw.raw.member} labelOf={(name) => (fieldLabels.move as Record<string, string>)[name] ?? name} />;
}

/** Rohdaten (Tüfteln): the effect id, every bit of the flags byte, the bytes with the field focused last marked. */
function RawColumn({ e, index, doc, highlight }: { e: Editing | null; index: number; doc: MoveDoc; highlight: string | undefined }) {
  return (
    <Column title="Rohdaten" note="Tüfteln">
      {e !== null && (
        <>
          <MoveNumber e={e} field="effect" label="Effekt-ID" bits={16} hint="Index der Kampfskripte" />
          <div className={styles.flags} data-field="flags" role="group" aria-label={`Merkmale-Byte ${hex2(e.flags)}`}>
            <p className={styles.note}>{`Merkmale-Byte ${hex2(e.flags)}`}</p>
            {FLAGS.map((f) => (
              <Checkbox key={f.bit} label={`Bit ${f.bit}`} ariaLabel={`Bit ${f.bit}, ${hex2(f.mask)}${f.named ? `, ${f.label}` : ""}`} checked={hasFlag(e.flags, f.mask)} onChange={(on) => e.setFlag(f, on)} />
            ))}
          </div>
        </>
      )}
      <RawBytes index={index} doc={doc} highlight={highlight} />
    </Column>
  );
}

/**
 * The editor (Hand) of a move in columns: Grundwerte, Verhalten and, while tinkering, Rohdaten. Every field commits on its own as one undo
 * step; a flag sets or clears its bit in the flags byte and keeps every other bit. A refusal of the backend shows above the columns; a
 * move of the wrong length is named there and shows no fields.
 */
export function MoveHand() {
  const { row, labelOf, selected } = useDataArea("pokemon");
  const tinkering = useShell().state.depth === "tuefteln";
  const { doc, error, editError, edit } = useMove(selected);
  // The field focused last, for the raw data's mark (its key in the field map).
  const [lastField, setLastField] = useState<string | undefined>(undefined);
  // The flags byte as the edits sent so far leave it, until the document is read again: two flags set in a row both stay.
  const pendingFlags = useRef<{ doc: MoveDoc; flags: number } | null>(null);

  if (row === null || selected === null) return null;
  if (doc === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what="Diese Attacke ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}</div>;
  }
  const name = labelOf(row);
  const index = selected;
  const v = moveValues(doc.fields);
  const flags = pendingFlags.current?.doc === doc ? pendingFlags.current.flags : (v.flags ?? 0);
  const editing: Editing | null =
    doc.locked !== null
      ? null
      : {
          v,
          showBase: tinkering,
          flags,
          set(field, value) {
            const old = v[field];
            if (old === undefined || old === value) return;
            void edit(index, { Move: { field, value } }, moveLabel(name, field, old, value));
          },
          setFlag(flag, on) {
            const next = withFlag(flags, flag.mask, on);
            if (next === flags) return;
            pendingFlags.current = { doc, flags: next };
            void edit(index, { Move: { field: "flags", value: next } }, flagLabel(name, flag, on)).then((ok) => {
              if (!ok) pendingFlags.current = null;
            });
          },
        };

  const onFocus = (event: FocusEvent<HTMLElement>) => {
    const field = (event.target as HTMLElement).closest<HTMLElement>("[data-field]")?.dataset.field;
    if (field !== undefined) setLastField(field);
  };

  return (
    <div className={styles.hand} onFocus={onFocus} role="group" aria-label={`${name} bearbeiten`} data-move={index}>
      {(editError !== null || doc.locked !== null) && (
        <div className={styles.notices}>
          {doc.locked !== null && <Notice kind="warning">{`${lockText(doc.locked, index)}. Ihre Felder sind gesperrt.`}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.columns}>
        {editing !== null && <BaseColumn e={editing} />}
        {editing !== null && <BehaviourColumn e={editing} />}
        {tinkering && <RawColumn e={editing} index={index} doc={doc} highlight={lastField} />}
      </div>
    </div>
  );
}
