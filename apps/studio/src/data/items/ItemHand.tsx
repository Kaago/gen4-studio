import { useEffect, useState } from "react";
import type { FocusEvent, ReactNode } from "react";
import { dataRaw, errorText, fieldLabels } from "../../api";
import type { FieldValueDto, ProjectInfo, RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, Reference, SelectField, Switch } from "../../ui/primitives";
import type { NumberFieldProps, SelectOption } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { useDataArea } from "../useDataArea";
import {
  BATTLE_FUNCTIONS,
  BATTLE_POCKETS,
  FIELD_GROUPS,
  FIELD_POCKETS,
  GIFT_TYPES,
  ONE_BIT_FIELDS,
  dataMemberOf,
  fieldFunctions,
  itemLabel,
  itemValues,
  lockText,
  pocketBitLabel,
  sellPrice,
  sharedWarning,
  withBit,
} from "./itemModel";
import type { FieldGroup, ItemField, ItemValues, PocketBit } from "./itemModel";
import { useItem } from "./useItem";
import type { ItemDoc } from "./useItem";
import styles from "./Items.module.css";

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

/** What every field of the hand works with: the item's values and widths, and how to write one. */
interface Editing {
  v: ItemValues;
  widths: ReadonlyMap<string, FieldValueDto>;
  family: ProjectInfo["family"] | undefined;
  /** Number fields show the Dez | Hex switch (Tüfteln). */
  showBase: boolean;
  set(field: ItemField, value: number): void;
  setPocket(pocket: PocketBit, on: boolean): void;
}

function ItemNumber({ e, field, ...limits }: { e: Editing; field: ItemField } & Pick<NumberFieldProps, "bits" | "min" | "max" | "hint">) {
  const label = fieldLabels.item[field];
  return (
    <InspectorRow label={label}>
      <div data-field={field}>
        <NumberField label={label} value={e.v[field] ?? null} onCommit={(value) => e.set(field, value)} showBase={e.showBase} {...limits} />
      </div>
    </InspectorRow>
  );
}

function ItemSelect({ e, field, options }: { e: Editing; field: ItemField; options: readonly SelectOption<number>[] }) {
  const label = fieldLabels.item[field];
  return (
    <InspectorRow label={label}>
      <div data-field={field}>
        <SelectField label={label} value={e.v[field] ?? 0} options={options} onChange={(value) => e.set(field, value)} />
      </div>
    </InspectorRow>
  );
}

/** The battle pockets as one switch per bit. */
function BattlePockets({ e }: { e: Editing }) {
  const battle = e.v.battle_pocket ?? 0;
  return (
    <div className={styles.flags} data-field="battle_pocket" role="group" aria-label={fieldLabels.item.battle_pocket}>
      <p className={styles.note}>{fieldLabels.item.battle_pocket}</p>
      {BATTLE_POCKETS.map((p) => (
        <Switch key={p.bit} label={p.label} checked={(battle & p.mask) !== 0} onChange={(on) => e.setPocket(p, on)} />
      ))}
    </div>
  );
}

/** The range of a field from its width: a signed byte −128 to 127, the rest 0 to 2^bits − 1. */
function limits(width: FieldValueDto): Pick<NumberFieldProps, "min" | "max"> {
  if (width.signed) return { min: -(2 ** (width.bits - 1)), max: 2 ** (width.bits - 1) - 1 };
  return { min: 0, max: 2 ** width.bits - 1 };
}

/**
 * The editor of one field: choices for pockets, functions and the natural gift type, switches for the pockets of the battle bag and the
 * flags (checkboxes among the raw party parameters), numbers within the field's width for the rest.
 */
function FieldEditor({ e, field, raw }: { e: Editing; field: ItemField; raw: boolean }) {
  const width = e.widths.get(field);
  if (width === undefined) return null;
  switch (field) {
    case "price":
      return <ItemNumber e={e} field={field} bits={16} hint={`Verkauf ${sellPrice(e.v.price ?? 0)} ₽`} />;
    case "field_pocket":
      return <ItemSelect e={e} field={field} options={FIELD_POCKETS} />;
    case "battle_pocket":
      return <BattlePockets e={e} />;
    case "natural_gift_type":
      return <ItemSelect e={e} field={field} options={GIFT_TYPES} />;
    case "field_use_func":
      return <ItemSelect e={e} field={field} options={fieldFunctions(e.family, e.v.field_use_func ?? 0)} />;
    case "battle_use_func":
      return <ItemSelect e={e} field={field} options={BATTLE_FUNCTIONS} />;
    case "hold_effect":
      return <ItemNumber e={e} field={field} {...limits(width)} hint="Nummer des Effekts beim Tragen" />;
  }
  if (ONE_BIT_FIELDS.includes(field)) {
    const on = (e.v[field] ?? 0) !== 0;
    const set = (next: boolean) => e.set(field, next ? 1 : 0);
    return <div data-field={field}>{raw ? <Checkbox label={fieldLabels.item[field]} checked={on} onChange={set} /> : <Switch label={fieldLabels.item[field]} checked={on} onChange={set} />}</div>;
  }
  return <ItemNumber e={e} field={field} {...limits(width)} />;
}

/** A group of the Rohdaten column: its title as a note, then its fields. */
function RawGroup({ e, group }: { e: Editing; group: FieldGroup }) {
  return (
    <div className={styles.flags} role="group" aria-label={group.title}>
      <p className={styles.note}>{group.title === "Wirkung im Team" ? "Wirkung im Team, gilt mit dem Schalter in Funktionen" : group.title}</p>
      {group.fields.map((field) => (
        <FieldEditor key={field} e={e} field={field} raw />
      ))}
    </div>
  );
}

/** The bytes of the item as the store holds them, read again with every new document (an edit, undo, redo). */
function RawBytes({ index, doc, highlight }: { index: number; doc: ItemDoc | null; highlight: string | undefined }) {
  const [raw, setRaw] = useState<{ raw: RawDto | null; error: string | null } | null>(null);
  useEffect(() => {
    let current = true;
    dataRaw("item", index).then(
      (r) => current && setRaw({ raw: r, error: null }),
      (err: unknown) => current && setRaw({ raw: null, error: errorText(err) }),
    );
    return () => {
      current = false;
    };
  }, [index, doc]);
  if (raw === null) return <p className={styles.note}>Lädt …</p>;
  if (raw.raw === null) return <p className={styles.note}>{`Nicht lesbar: ${raw.error ?? ""}`}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} highlight={highlight} path={raw.raw.path} member={raw.raw.member} labelOf={(name) => (fieldLabels.item as Record<string, string>)[name] ?? name} />;
}

/** The warning of an item whose data other item ids share, with those items to open. */
function SharedWarning({ ids }: { ids: readonly number[] }) {
  const { rows, labelOf, goTo } = useDataArea("gegenstaende");
  const byId = new Map(rows.map((r) => [r.index, r]));
  return (
    <Notice kind="warning">
      {sharedWarning(ids.length)}
      <ul className={styles.shared} aria-label="Gegenstände mit denselben Daten">
        {ids.map((id) => {
          const row = byId.get(id);
          return (
            <li key={id}>
              <Reference area="gegenstaende" label={row ? labelOf(row) : `Gegenstand ${id4(id)}`} tech={id4(id)} onOpen={() => goTo({ kind: "item", id })} dataAttrs={{ "data-item": id }} />
            </li>
          );
        })}
      </ul>
    </Notice>
  );
}

/**
 * The editor (Hand) of an item in the columns of `FIELD_GROUPS` (Werte, Effekte, Funktionen) and, while tinkering, Rohdaten: the bytes
 * and the party parameters. Every field commits on its own as one undo step, built from the item as the backend holds it when the edit
 * runs (`useItem`), so quick edits build on each other. Data that other item ids share is named above the columns and in every label; a
 * refusal of the backend shows there too; an item of the wrong length is named there and shows no fields.
 */
export function ItemHand() {
  const { row, labelOf, selected, project } = useDataArea("gegenstaende");
  const tinkering = useShell().state.depth === "tuefteln";
  const { doc, read, error, editError, update } = useItem(selected, row === null ? null : dataMemberOf(row));
  // The field focused last, for the raw data's mark (its key in the field map).
  const [lastField, setLastField] = useState<string | undefined>(undefined);

  if (row === null || selected === null) return null;
  if (doc === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what="Dieser Gegenstand ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}</div>;
  }
  const name = labelOf(row);
  const index = selected;
  const family = project?.family;
  // Patch and label come from the item as the backend holds it when the edit runs; a locked item writes nothing.
  const write = (field: ItemField, next: (old: number) => number, label: (old: number, value: number, shared: number) => string) =>
    void update(index, (now) => {
      if (now.locked !== null) return null;
      const old = itemValues(now.fields)[field];
      if (old === undefined) return null;
      const value = next(old);
      return value === old ? null : { patch: { Item: { field, value } }, label: label(old, value, now.shared_with.length) };
    });
  const editing: Editing | null =
    doc.locked !== null
      ? null
      : {
          v: itemValues(doc.fields),
          widths: new Map(doc.fields.map((f) => [f.name, f])),
          family,
          showBase: tinkering,
          set(field, value) {
            write(field, () => value, (old, v, shared) => itemLabel(name, field, old, v, { family, shared }));
          },
          setPocket(pocket, on) {
            write("battle_pocket", (old) => withBit(old, pocket.mask, on), (_old, _v, shared) => pocketBitLabel(name, pocket, on, { shared }));
          },
        };

  const onFocus = (event: FocusEvent<HTMLElement>) => {
    const field = (event.target as HTMLElement).closest<HTMLElement>("[data-field]")?.dataset.field;
    if (field !== undefined) setLastField(field);
  };

  return (
    <div className={styles.hand} onFocus={onFocus} role="group" aria-label={`${name} bearbeiten`} data-item={index}>
      {(editError !== null || doc.locked !== null || doc.shared_with.length > 0) && (
        <div className={styles.notices}>
          {doc.locked !== null && <Notice kind="warning">{`${lockText(doc.locked, index)}. Seine Felder sind gesperrt.`}</Notice>}
          {doc.locked === null && doc.shared_with.length > 0 && <SharedWarning ids={doc.shared_with} />}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.columns}>
        {editing !== null &&
          FIELD_GROUPS.filter((g) => !g.tinkerOnly).map((g) => (
            <Column key={g.title} title={g.title}>
              {g.fields.map((field) => (
                <FieldEditor key={field} e={editing} field={field} raw={false} />
              ))}
            </Column>
          ))}
        {tinkering && (
          <Column title="Rohdaten" note="Tüfteln">
            <RawBytes index={index} doc={read} highlight={lastField} />
            {editing !== null && FIELD_GROUPS.filter((g) => g.tinkerOnly).map((g) => <RawGroup key={g.title} e={editing} group={g} />)}
          </Column>
        )}
      </div>
    </div>
  );
}
