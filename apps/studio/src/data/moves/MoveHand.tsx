import { useEffect, useState } from "react";
import type { FocusEvent, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { dataRaw, errorText, fieldLabels } from "../../api";
import type { RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { depthAtLeast } from "../../ui/frame/shellState";
import { Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, Segmented, SelectField, pokemonTypes } from "../../ui/primitives";
import type { NumberFieldProps, SegmentedOption, SelectOption } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import { CATEGORIES, FLAGS, NAMED_FLAGS, TARGETS, categoryName, flagLabel, hasFlag, hex2, lockText, moveLabel, moveValues, ppMax, withFlag } from "./moveModel";
import type { MoveFlag, MoveField, MoveValues } from "./moveModel";
import { useMove } from "./useMove";
import type { MoveDoc } from "./useMove";
import styles from "./Moves.module.css";

const TYPES: readonly SelectOption<number>[] = pokemonTypes.map((t) => ({ value: t.id, label: t.name }));

/** The three categories; a value the games do not know shows as a fourth, locked choice ("Wert 3"). */
function categoryOptions(value: number): SegmentedOption<string>[] {
  const known = Object.entries(CATEGORIES).map(([v, label]) => ({ value: v, label }));
  return CATEGORIES[value] === undefined ? [...known, { value: String(value), label: categoryName(value), disabled: true }] : known;
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
  const { t } = useTranslation("data");
  const category = e.v.category ?? 0;
  return (
    <Column title={t("moveHand.baseValues")}>
      <MoveSelect e={e} field="move_type" options={TYPES} />
      <InspectorRow label={t("moveHand.category")}>
        <div data-field="category">
          <Segmented ariaLabel={t("moveHand.category")} size="s" options={categoryOptions(category)} value={String(category)} onChange={(value) => e.set("category", Number(value))} />
        </div>
      </InspectorRow>
      <MoveNumber e={e} field="power" bits={8} hint={t("moveHand.powerHint")} />
      <MoveNumber e={e} field="accuracy" bits={8} hint={t("moveHand.accuracyHint")} />
      <MoveNumber e={e} field="pp" bits={8} hint={t("moveHand.ppHint", { max: ppMax(e.v.pp ?? 0) })} />
      <MoveNumber e={e} field="priority" min={-7} max={5} hint={t("moveHand.priorityHint")} />
      <MoveNumber e={e} field="effect_chance" bits={8} hint={t("moveHand.percentHint")} />
    </Column>
  );
}

/** Verhalten: the target and the named flags. */
function BehaviourColumn({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  return (
    <Column title={t("moveHand.behaviour")}>
      <MoveSelect e={e} field="target" options={TARGETS} />
      <div className={styles.flags} data-field="flags" role="group" aria-label={t("moveView.traits")}>
        {NAMED_FLAGS.map((f) => (
          <Checkbox key={f.bit} label={f.label} checked={hasFlag(e.flags, f.mask)} onChange={(on) => e.setFlag(f, on)} />
        ))}
      </div>
    </Column>
  );
}

/** The bytes of the move as the store holds them, read again with every document read (after an edit, undo, redo). */
function RawBytes({ index, doc, highlight }: { index: number; doc: MoveDoc | null; highlight: string | undefined }) {
  const { t } = useTranslation("data");
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
  if (raw === null) return <p className={styles.note}>{t("loading")}</p>;
  if (raw.raw === null) return <p className={styles.note}>{t("notReadable", { error: raw.error ?? "" })}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} highlight={highlight} path={raw.raw.path} member={raw.raw.member} labelOf={(name) => (fieldLabels.move as Record<string, string>)[name] ?? name} />;
}

/** Rohdaten (Tüfteln): the effect id, every bit of the flags byte, the bytes with the field focused last marked. */
function RawColumn({ e, index, doc, highlight }: { e: Editing | null; index: number; doc: MoveDoc | null; highlight: string | undefined }) {
  const { t } = useTranslation("data");
  return (
    <Column title={t("rawData")} note={t("tinker")}>
      {e !== null && (
        <>
          <MoveNumber e={e} field="effect" label={t("moveHand.effectId")} bits={16} hint={t("moveHand.effectHint")} />
          <div className={styles.flags} data-field="flags" role="group" aria-label={t("moveHand.traitsByte", { hex: hex2(e.flags) })}>
            <p className={styles.note}>{t("moveHand.traitsByte", { hex: hex2(e.flags) })}</p>
            {FLAGS.map((f) => (
              <Checkbox key={f.bit} label={t("moveModel.bit", { bit: f.bit })} ariaLabel={`${t("moveModel.bit", { bit: f.bit })}, ${hex2(f.mask)}${f.named ? `, ${f.label}` : ""}`} checked={hasFlag(e.flags, f.mask)} onChange={(on) => e.setFlag(f, on)} />
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
  const { t } = useTranslation("data");
  const { row, labelOf, selected } = useDataArea("pokemon");
  const tinkering = depthAtLeast(useShell().viewDepth, "tuefteln");
  // `doc` is the move with the edits still waiting laid over it: a checkbox shows its new state at once and keeps it.
  const { doc, read, error, editError, update } = useMove(selected);
  // The field focused last, for the raw data's mark (its key in the field map).
  const [lastField, setLastField] = useState<string | undefined>(undefined);

  if (row === null || selected === null) return null;
  if (doc === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what={t("moveHand.unreadable")} why={error} /> : <p className={styles.note}>{t("loading")}</p>}</div>;
  }
  const name = labelOf(row);
  const index = selected;
  const v = moveValues(doc.fields);
  // Patch and label come from the move as the backend holds it when the edit runs, after the edits before it.
  const editing: Editing | null =
    doc.locked !== null
      ? null
      : {
          v,
          showBase: tinkering,
          flags: v.flags ?? 0,
          set(field, value) {
            void update(index, (now) => {
              const old = moveValues(now.fields)[field];
              return old === undefined || old === value ? null : { patch: { Move: { field, value } }, label: moveLabel(name, field, old, value) };
            });
          },
          setFlag(flag, on) {
            void update(index, (now) => {
              const old = moveValues(now.fields).flags;
              if (old === undefined) return null;
              const next = withFlag(old, flag.mask, on);
              return next === old ? null : { patch: { Move: { field: "flags", value: next } }, label: flagLabel(name, flag, on) };
            });
          },
        };

  const onFocus = (event: FocusEvent<HTMLElement>) => {
    const field = (event.target as HTMLElement).closest<HTMLElement>("[data-field]")?.dataset.field;
    if (field !== undefined) setLastField(field);
  };

  return (
    <div className={styles.hand} onFocus={onFocus} role="group" aria-label={t("moveHand.edit", { name })} data-move={index}>
      {(editError !== null || doc.locked !== null) && (
        <div className={styles.notices}>
          {doc.locked !== null && <Notice kind="warning">{t("moveHand.lockedFields", { reason: lockText(doc.locked, index) })}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.columns}>
        {editing !== null && <BaseColumn e={editing} />}
        {editing !== null && <BehaviourColumn e={editing} />}
        {tinkering && <RawColumn e={editing} index={index} doc={read} highlight={lastField} />}
      </div>
    </div>
  );
}
