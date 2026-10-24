import { useEffect, useState } from "react";
import type { FocusEvent, ReactNode } from "react";
import { dataRaw, errorText, fieldLabels } from "../../api";
import type { FieldValueDto, RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, Reference, SelectField, Switch } from "../../ui/primitives";
import type { NumberFieldProps, SelectOption } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import {
  BATTLE_FUNCTIONS,
  BATTLE_POCKETS,
  FIELD_FUNCTIONS,
  FIELD_POCKETS,
  GIFT_TYPES,
  ONE_BIT_FIELDS,
  PARTY_FIELDS,
  id4,
  itemLabel,
  lockText,
  pocketBitLabel,
  sellPrice,
  sharedWarning,
  withBit,
} from "./itemModel";
import type { ItemField, ItemValues, PocketBit } from "./itemModel";
import { dataMemberOf } from "./ItemsList";
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

/** What every field of the hand works with: the item's values and how to write one. */
interface Editing {
  v: ItemValues;
  /** Number fields show the Dez | Hex switch (Tüfteln). */
  showBase: boolean;
  set(field: ItemField, value: number): void;
  setPocket(pocket: PocketBit, on: boolean): void;
}

function ItemNumber({ e, field, label = fieldLabels.item[field], ...limits }: { e: Editing; field: ItemField; label?: string } & Pick<NumberFieldProps, "bits" | "min" | "max" | "hint">) {
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

function ItemSwitch({ e, field }: { e: Editing; field: ItemField }) {
  return (
    <div data-field={field}>
      <Switch label={fieldLabels.item[field]} checked={(e.v[field] ?? 0) !== 0} onChange={(on) => e.set(field, on ? 1 : 0)} />
    </div>
  );
}

/** Werte: price, field pocket, the battle pockets as switches, the two switches of the bag. */
function ValuesColumn({ e }: { e: Editing }) {
  const battle = e.v.battle_pocket ?? 0;
  return (
    <Column title="Werte">
      <ItemNumber e={e} field="price" bits={16} hint={`Verkauf ${sellPrice(e.v.price ?? 0)} ₽`} />
      <ItemSelect e={e} field="field_pocket" options={FIELD_POCKETS} />
      <div className={styles.flags} data-field="battle_pocket" role="group" aria-label={fieldLabels.item.battle_pocket}>
        <p className={styles.note}>{fieldLabels.item.battle_pocket}</p>
        {BATTLE_POCKETS.map((p) => (
          <Switch key={p.bit} label={p.label} checked={(battle & p.mask) !== 0} onChange={(on) => e.setPocket(p, on)} />
        ))}
      </div>
      <ItemSwitch e={e} field="prevent_toss" />
      <ItemSwitch e={e} field="can_register" />
    </Column>
  );
}

/** Effekte: holding, pluck, fling, natural gift. */
function EffectsColumn({ e }: { e: Editing }) {
  return (
    <Column title="Effekte">
      <ItemNumber e={e} field="hold_effect" bits={8} hint="Nummer des Effekts beim Tragen" />
      <ItemNumber e={e} field="hold_param" bits={8} />
      <ItemNumber e={e} field="pluck_effect" bits={8} />
      <ItemNumber e={e} field="fling_effect" bits={8} />
      <ItemNumber e={e} field="fling_power" bits={8} />
      <ItemNumber e={e} field="natural_gift_power" bits={8} />
      <ItemSelect e={e} field="natural_gift_type" options={GIFT_TYPES} />
    </Column>
  );
}

/** Funktionen: the use outside and in battle, and whether the party parameters count. */
function FunctionsColumn({ e }: { e: Editing }) {
  return (
    <Column title="Funktionen">
      <ItemSelect e={e} field="field_use_func" options={FIELD_FUNCTIONS} />
      <ItemSelect e={e} field="battle_use_func" options={BATTLE_FUNCTIONS} />
      <ItemSwitch e={e} field="party_use" />
    </Column>
  );
}

/** The range of a field from its width: one bit is a checkbox, a signed byte −128 to 127, the rest 0 to 2^bits − 1. */
function limits(field: FieldValueDto): Pick<NumberFieldProps, "min" | "max"> {
  if (field.signed) return { min: -(2 ** (field.bits - 1)), max: 2 ** (field.bits - 1) - 1 };
  return { min: 0, max: 2 ** field.bits - 1 };
}

/** The party parameters (Tüfteln): the effect bits as checkboxes, stages, EVs, amounts and friendship as numbers. */
function PartyFields({ e, doc }: { e: Editing; doc: ItemDoc }) {
  const widths = new Map(doc.fields.map((f) => [f.name, f]));
  return (
    <div className={styles.flags} role="group" aria-label="Wirkung im Team">
      <p className={styles.note}>Wirkung im Team, gilt mit dem Schalter in Funktionen</p>
      {PARTY_FIELDS.map((field) => {
        const width = widths.get(field);
        if (width === undefined) return null;
        if (ONE_BIT_FIELDS.includes(field)) {
          return (
            <div key={field} data-field={field}>
              <Checkbox label={fieldLabels.item[field]} checked={(e.v[field] ?? 0) !== 0} onChange={(on) => e.set(field, on ? 1 : 0)} />
            </div>
          );
        }
        return <ItemNumber key={field} e={e} field={field} {...limits(width)} />;
      })}
    </div>
  );
}

/** The bytes of the item as the store holds them, read again with every new document (an edit, undo, redo). */
function RawBytes({ index, doc, highlight }: { index: number; doc: ItemDoc; highlight: string | undefined }) {
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

/** Rohdaten (Tüfteln): the bytes with the field focused last marked, then the party parameters. */
function RawColumn({ e, index, doc, highlight }: { e: Editing | null; index: number; doc: ItemDoc; highlight: string | undefined }) {
  return (
    <Column title="Rohdaten" note="Tüfteln">
      <RawBytes index={index} doc={doc} highlight={highlight} />
      {e !== null && <PartyFields e={e} doc={doc} />}
    </Column>
  );
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
 * The editor (Hand) of an item in columns: Werte, Effekte, Funktionen and, while tinkering, Rohdaten with the party parameters. Every field
 * commits on its own as one undo step; a battle pocket sets or clears its bit from the newest value of the pockets. Data that other item
 * ids share is named above the columns and in every label; a refusal of the backend shows there too; an item of the wrong length is named
 * there and shows no fields.
 */
export function ItemHand() {
  const { row, labelOf, selected } = useDataArea("gegenstaende");
  const tinkering = useShell().state.depth === "tuefteln";
  const { doc, values, error, editError, edit } = useItem(selected, row === null ? null : dataMemberOf(row));
  // The field focused last, for the raw data's mark (its key in the field map).
  const [lastField, setLastField] = useState<string | undefined>(undefined);

  if (row === null || selected === null) return null;
  if (doc === null || values === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what="Dieser Gegenstand ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}</div>;
  }
  const name = labelOf(row);
  const index = selected;
  const shared = doc.shared_with.length;
  const editing: Editing | null =
    doc.locked !== null
      ? null
      : {
          v: values,
          showBase: tinkering,
          set(field, value) {
            void edit(index, { field, next: () => value, label: (old, v) => itemLabel(name, field, old, v, shared) });
          },
          setPocket(pocket, on) {
            void edit(index, { field: "battle_pocket", next: (old) => withBit(old, pocket.mask, on), label: () => pocketBitLabel(name, pocket, on, shared) });
          },
        };

  const onFocus = (event: FocusEvent<HTMLElement>) => {
    const field = (event.target as HTMLElement).closest<HTMLElement>("[data-field]")?.dataset.field;
    if (field !== undefined) setLastField(field);
  };

  return (
    <div className={styles.hand} onFocus={onFocus} role="group" aria-label={`${name} bearbeiten`} data-item={index}>
      {(editError !== null || doc.locked !== null || shared > 0) && (
        <div className={styles.notices}>
          {doc.locked !== null && <Notice kind="warning">{`${lockText(doc.locked, index)}. Seine Felder sind gesperrt.`}</Notice>}
          {doc.locked === null && shared > 0 && <SharedWarning ids={doc.shared_with} />}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.columns}>
        {editing !== null && <ValuesColumn e={editing} />}
        {editing !== null && <EffectsColumn e={editing} />}
        {editing !== null && <FunctionsColumn e={editing} />}
        {tinkering && <RawColumn e={editing} index={index} doc={doc} highlight={lastField} />}
      </div>
    </div>
  );
}
