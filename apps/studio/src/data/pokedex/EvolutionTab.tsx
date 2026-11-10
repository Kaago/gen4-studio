import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { EvoEntryDto } from "../../api";
import { NumberField, Reference, SelectField, Table } from "../../ui/primitives";
import type { TableRow } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { useDataArea } from "../useDataArea";
import { PickField } from "./pickers";
import type { Choice } from "./pickers";
import type { Editing } from "./PokedexHand";
import { EVO_METHODS, PARAM_LABEL, isEvolutionTarget, itemName, moveName, paramKind } from "./pokedexModel";
import type { ParamKind } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/** The picker of a parameter that names an item, a move or a species. */
const PICKED: Partial<Record<ParamKind, Choice>> = { item: "item", move: "move", species: "species" };

/** The parameter a method starts with when the one before meant something else: level 1 for a level, else 0. */
const startParam = (kind: ParamKind) => (kind === "level" ? 1 : 0);

/**
 * Entwicklung: the seven entries of the species (the game reads them on the species that evolves), each with its method, the parameter
 * the method asks for (a level, an item, a move, a species, the beauty, or none) and the target with a jump to it („Ziel wählen“ while
 * there is none). A change is built when its turn comes, from the entry as the changes before it left it.
 */
export function EvolutionTab({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const columns = [
    { key: "method", label: t("evolutionTab.method"), width: 260 },
    { key: "param", label: t("evolutionTab.param"), width: 200 },
    { key: "target", label: t("evolutionTab.target") },
  ];
  const { rows, labelOf, goTo } = useDataArea("pokemon");
  const [selected, setSelected] = useState<string | undefined>(undefined);
  const off = e.locked.evolution;
  const speciesName = (index: number) => {
    const row = rows.find((r) => r.index === index);
    return row ? labelOf(row) : t("speciesFallback", { id: id4(index) });
  };
  const methodName = (method: number) => EVO_METHODS[method]?.label ?? t("evolutionTab.methodNumber", { method });
  const paramText = (kind: ParamKind, param: number) =>
    kind === "item" ? itemName(e.names.items, param) : kind === "move" ? moveName(e.names.moves, param) : kind === "species" ? speciesName(param) : String(param);

  /** Writes entry `slot` as `make` changes it, from the entry as it is when the write comes; `make` returns null for no change. */
  const change = (slot: number, make: (entry: EvoEntryDto) => { next: EvoEntryDto; detail: string } | null) =>
    void e.update((doc) => {
      const entry = doc.evolution[slot];
      const made = entry === undefined ? null : make(entry);
      if (made === null) return null;
      const { method, param, target } = made.next;
      return { patch: { Evolution: { slot, method, param, target } }, label: t("evolutionTab.changeLabel", { name: e.name, slot: slot + 1, detail: made.detail }) };
    });

  const setMethod = (slot: number, method: number) =>
    change(slot, (entry) => {
      if (entry.method === method) return null;
      // "Keine" empties the entry; another method keeps the parameter only when it means the same kind of thing.
      const kind = paramKind(method);
      const next = method === 0 ? { method, param: 0, target: 0 } : { ...entry, method, param: kind === paramKind(entry.method) ? entry.param : startParam(kind) };
      return { next, detail: t("evolutionTab.methodChange", { from: methodName(entry.method), to: methodName(method) }) };
    });
  const setParam = (slot: number, param: number) =>
    change(slot, (entry) => {
      if (entry.param === param) return null;
      const kind = paramKind(entry.method);
      return { next: { ...entry, param }, detail: `${PARAM_LABEL[kind]} ${paramText(kind, entry.param)} → ${paramText(kind, param)}` };
    });
  const setTarget = (slot: number, target: number) =>
    change(slot, (entry) =>
      entry.target === target ? null : { next: { ...entry, target }, detail: entry.target === 0 ? t("evolutionTab.targetSet", { name: speciesName(target) }) : t("evolutionTab.targetChange", { from: speciesName(entry.target), to: speciesName(target) }) },
    );

  const paramCell = (slot: number, entry: EvoEntryDto) => {
    const kind = paramKind(entry.method);
    const label = t("evolutionTab.paramLabel", { param: PARAM_LABEL[kind], slot: slot + 1 });
    const picked = PICKED[kind];
    if (kind === "none") return <span className={styles.muted}>{t("evolutionTab.noParam")}</span>;
    if (picked) {
      return (
        <PickField label={label} text={paramText(kind, entry.param)} choice={picked} only={picked === "species" ? isEvolutionTarget : undefined} onPick={(v) => setParam(slot, v)} disabled={off} />
      );
    }
    const limits = kind === "level" ? { min: 1, max: 100 } : kind === "beauty" ? { bits: 8 as const } : { bits: 16 as const };
    return <NumberField label={label} value={entry.param} onCommit={(v) => setParam(slot, v)} disabled={off} showBase={e.showBase} {...limits} />;
  };

  const targetCell = (slot: number, entry: EvoEntryDto) => {
    if (entry.method === 0 && entry.target === 0) return <span className={styles.muted}>{t("evolutionTab.noEvolution")}</span>;
    const label = t("evolutionTab.targetLabel", { slot: slot + 1 });
    if (entry.target === 0) {
      return <PickField label={label} text={t("evolutionTab.noTarget")} trigger={t("evolutionTab.chooseTarget")} choice="species" only={isEvolutionTarget} onPick={(v) => setTarget(slot, v)} disabled={off} />;
    }
    return (
      <div className={styles.inline}>
        <Reference area="pokemon" label={speciesName(entry.target)} tech={id4(entry.target)} onOpen={() => goTo({ kind: "species", index: entry.target })} dataAttrs={{ "data-species-ref": entry.target }} />
        <PickField label={label} text={speciesName(entry.target)} trigger={t("evolutionTab.change")} choice="species" only={isEvolutionTarget} onPick={(v) => setTarget(slot, v)} disabled={off} />
      </div>
    );
  };

  const rowsOf: TableRow[] = e.doc.evolution.map((entry, slot) => ({
    key: `e${slot}`,
    attrs: { "data-evo": String(slot) },
    disabled: off,
    cells: {
      method: (
        <div data-field={`method${slot}`}>
          <SelectField label={t("evolutionTab.methodLabel", { slot: slot + 1 })} value={entry.method} options={EVO_METHODS} onChange={(v) => setMethod(slot, v)} disabled={off} />
        </div>
      ),
      param: <div data-field={`param${slot}`}>{paramCell(slot, entry)}</div>,
      target: <div data-field={`target${slot}`}>{targetCell(slot, entry)}</div>,
    },
  }));

  return (
    <div className={styles.tableTab}>
      <Table
        ariaLabel={t("evolutionTab.tableLabel", { name: e.name })}
        columns={columns}
        rows={rowsOf}
        selected={selected}
        onSelect={setSelected}
        footer={t("evolutionTab.footer")}
      />
    </div>
  );
}
