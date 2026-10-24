import { useState } from "react";
import type { EvoEntryDto } from "../../api";
import { NumberField, Reference, SelectField, Table } from "../../ui/primitives";
import type { TableRow } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import { PickField } from "./pickers";
import type { Choice } from "./pickers";
import type { Editing } from "./PokedexHand";
import { EVO_METHODS, PARAM_LABEL, id4, itemName, moveName, paramKind } from "./pokedexModel";
import type { ParamKind } from "./pokedexModel";
import styles from "./Pokedex.module.css";

const COLUMNS = [
  { key: "method", label: "Methode", width: 260 },
  { key: "param", label: "Parameter", width: 200 },
  { key: "target", label: "Ziel" },
];

/** The picker of a parameter that names an item, a move or a species. */
const PICKED: Partial<Record<ParamKind, Choice>> = { item: "item", move: "move", species: "species" };

/**
 * Entwicklung: the seven entries of the species (the game reads them on the species that evolves), each with its method, the parameter
 * the method asks for (a level, an item, a move, a species, the beauty, or none) and the target with a jump to it.
 */
export function EvolutionTab({ e }: { e: Editing }) {
  const { rows, labelOf, goTo } = useDataArea("pokemon");
  const [selected, setSelected] = useState<string | undefined>(undefined);
  const off = e.locked.evolution;
  const speciesName = (index: number) => {
    const row = rows.find((r) => r.index === index);
    return row ? labelOf(row) : `Art ${id4(index)}`;
  };
  const methodName = (method: number) => EVO_METHODS[method]?.label ?? `Methode ${method}`;
  const paramText = (kind: ParamKind, param: number) =>
    kind === "item" ? itemName(e.names.items, param) : kind === "move" ? moveName(e.names.moves, param) : kind === "species" ? speciesName(param) : String(param);

  const write = (slot: number, next: EvoEntryDto, detail: string) =>
    void e.edit({ Evolution: { slot, method: next.method, param: next.param, target: next.target } }, `${e.name}: Entwicklung ${slot + 1} ${detail}`);

  const setMethod = (slot: number, entry: EvoEntryDto, method: number) => {
    // "Keine" empties the entry; another method keeps the parameter only when it means the same kind of thing.
    const next = method === 0 ? { method, param: 0, target: 0 } : { ...entry, method, param: paramKind(method) === paramKind(entry.method) ? entry.param : 0 };
    write(slot, next, `Methode ${methodName(entry.method)} → ${methodName(method)}`);
  };
  const setParam = (slot: number, entry: EvoEntryDto, param: number) => {
    const kind = paramKind(entry.method);
    write(slot, { ...entry, param }, `${PARAM_LABEL[kind]} ${paramText(kind, entry.param)} → ${paramText(kind, param)}`);
  };
  const setTarget = (slot: number, entry: EvoEntryDto, target: number) => write(slot, { ...entry, target }, `Ziel ${speciesName(entry.target)} → ${speciesName(target)}`);

  const paramCell = (slot: number, entry: EvoEntryDto) => {
    const kind = paramKind(entry.method);
    const label = `${PARAM_LABEL[kind]}, Entwicklung ${slot + 1}`;
    const picked = PICKED[kind];
    if (kind === "none") return <span className={styles.muted}>kein Parameter</span>;
    if (picked) return <PickField label={label} text={paramText(kind, entry.param)} choice={picked} onPick={(v) => setParam(slot, entry, v)} disabled={off} />;
    const limits = kind === "level" ? { min: 1, max: 100 } : kind === "beauty" ? { bits: 8 as const } : { bits: 16 as const };
    return <NumberField label={label} value={entry.param} onCommit={(v) => setParam(slot, entry, v)} disabled={off} showBase={e.showBase} {...limits} />;
  };

  const rowsOf: TableRow[] = e.doc.evolution.map((entry, slot) => {
    const empty = entry.method === 0 && entry.target === 0;
    return {
      key: `e${slot}`,
      attrs: { "data-evo": String(slot) },
      disabled: off,
      cells: {
        method: (
          <div data-field={`method${slot}`}>
            <SelectField label={`Methode, Entwicklung ${slot + 1}`} value={entry.method} options={EVO_METHODS} onChange={(v) => setMethod(slot, entry, v)} disabled={off} />
          </div>
        ),
        param: <div data-field={`param${slot}`}>{paramCell(slot, entry)}</div>,
        target: empty ? (
          <span className={styles.muted}>keine Entwicklung</span>
        ) : (
          <div className={styles.inline} data-field={`target${slot}`}>
            <Reference area="pokemon" label={speciesName(entry.target)} tech={id4(entry.target)} onOpen={() => goTo({ kind: "species", index: entry.target })} dataAttrs={{ "data-species-ref": entry.target }} />
            <PickField label={`Ziel, Entwicklung ${slot + 1}`} text={speciesName(entry.target)} trigger="Ändern …" choice="species" onPick={(v) => setTarget(slot, entry, v)} disabled={off} />
          </div>
        ),
      },
    };
  });

  return (
    <div className={styles.tableTab}>
      <Table ariaLabel={`Entwicklungen von ${e.name}`} columns={COLUMNS} rows={rowsOf} selected={selected} onSelect={setSelected} footer="Sieben Einträge; das Spiel prüft sie nach jedem Kampf und beim Tausch." />
    </div>
  );
}
