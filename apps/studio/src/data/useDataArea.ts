import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { Context } from "react";
import type { AssetId, DataRowDto, HeaderSummary, ItemEntryDto, NameTablesDto, ProjectInfo } from "../api";
import type { EditBus } from "../bus/editBus";
import { useShell } from "../ui/frame/ShellContext";
import { POKEMON_SUBS } from "../ui/frame/shellState";
import type { PokemonSub } from "../ui/frame/shellState";
import type { PanelHeadSwitcher } from "../ui/primitives";
import { dataCrumbs } from "../workbench/breadcrumb";
import type { Crumb } from "../workbench/breadcrumb";
import type { DataNav, NavTarget } from "../workbench/navigation";
import type { ProblemsStore } from "../workbench/problems/problems";
import { NO_FILTER, SELECT_KEY, SUB_KIND, SUB_LABEL, SUB_LOCKED, SUB_SHORTCUT, areaOfSub, filterRows, isSubLocked, itemPocket, pocketLabel, rowLabel } from "./dataModel";
import type { DataArea, DataSub, ListFilter, ListKind } from "./dataModel";
import { useDataSnapshot } from "./dataStore";
import type { DataSnapshot, DataStore } from "./dataStore";

/** What a data area needs from App. */
export interface DataAreaProps {
  /** The open project; another object means another project: search and filter start over. */
  project: ProjectInfo | null;
  /** The lists of the project (shared by both data areas and the palette). */
  store: DataStore;
  /** Whether the area shows: it reads its lists from the first time it does. */
  active: boolean;
  /** The member a link asked for (a new object per request); null: none. */
  target: DataNav | null;
  goTo: (target: NavTarget) => void;
  /** Hears the breadcrumb of the area (the hinge shows it). Must keep its identity. */
  onTrail?: (crumbs: Crumb[]) => void;
  /** The assets with unsaved changes: the lists and editors mark their members. */
  dirty: readonly AssetId[];
  /** An editor wrote a member: the dirty count and the history catch up. */
  onEdited: () => void;
  /** The edit bus: an editor announces what it wrote (`dataChanged`) and hears what undo, redo and reload restored. */
  bus: EditBus;
  /** The header table: an encounter file is named by the place of a header that uses it. */
  headers: readonly HeaderSummary[];
  /** The Problems list: a document with rows of its own (an encounter file, a trainer) replaces them with every read. */
  problems?: ProblemsStore;
  /**
   * „In der Welt platzieren“ of a trainer that stands nowhere yet (the seam Task 8 of 3b.3b wires: the world with the figure tool and a
   * pending trainer binding). Without it the view's button is disabled.
   */
  onPlaceTrainer?: (id: number) => void;
}

/** The state of a data area for its list, view, hand and status parts (`useDataArea`). */
export interface DataAreaModel {
  area: DataArea;
  project: ProjectInfo | null;
  /** The part that shows (Gegenstände has one, `items`). */
  sub: DataSub;
  /** Shows another part of the Pokémon area; a locked part stays where it is. */
  setSub(sub: PokemonSub): void;
  /** The parts as the list head's menu and the breadcrumb show them; null in an area of one part. */
  switcher: PanelHeadSwitcher | null;
  /** The list of the part; null for a part without a list yet. */
  kind: ListKind | null;
  /** The selected member of the part (item id, or data member without the item table); null: none. */
  selected: number | null;
  select(index: number | null): void;
  /** The selected row; null when none is selected or the list does not have it (yet). */
  row: DataRowDto | null;
  /** Every row of the part's list (empty while it is read). */
  rows: readonly DataRowDto[];
  /** The name a row shows (`rowLabel`). */
  labelOf(row: DataRowDto): string;
  names: NameTablesDto;
  /** The arm9 item table: undefined while it is not read, null when the game's is not available. */
  itemTable: ItemEntryDto[] | null | undefined;
  /** The list is being read. */
  loading: boolean;
  /** Why the list could not be read. */
  error: string | null;
  /** Why the data cannot be read at all (Diamond and Pearl), as the backend says it. */
  locked: string | null;
  goTo(target: NavTarget): void;
  /** The lists of the project: an editor reads the other lists it offers (moves, items) from it. */
  store: DataStore;
  /** The member a link asked for last (its `tab` names the editor tab to show); null: none. */
  target: DataNav | null;
  dirty: readonly AssetId[];
  onEdited(): void;
  bus: EditBus;
  headers: readonly HeaderSummary[];
  /** The Problems list (null: none, in tests): `useDataDoc` hands it the rows a document carries (`DocSpec.problems`). */
  problems: ProblemsStore | null;
  /** Places a trainer in the world (`DataAreaProps.onPlaceTrainer`); null while nobody takes it. */
  placeTrainer: ((id: number) => void) | null;
}

/**
 * The search and the filter of the part that shows, and the rows they leave (`useDataSearch`). Apart from the model, so that typing in the
 * search renders the list and not the view and the hand, which read the model only (Spec §9: a list renders in under 50 ms).
 */
export interface DataSearchModel {
  /** The rows the search and the filter leave. */
  shown: readonly DataRowDto[];
  query: string;
  setQuery(query: string): void;
  filter: ListFilter;
  setFilter(filter: ListFilter): void;
  /** Clears the search and the filter. */
  clearFilter(): void;
}

/** The model of a data area and its search, provided apart (`dataAreaProvider`, `dataSearchProvider`). */
export interface DataAreaModels {
  model: DataAreaModel;
  search: DataSearchModel;
}

const NO_NAMES: NameTablesDto = {
  species: [],
  moves: [],
  items: [],
  abilities: [],
  types: [],
  classes: [],
  forms: [],
  archives: { species: null, moves: null, items: null, abilities: null, types: null, classes: null, forms: null },
};
const NO_ROWS: readonly DataRowDto[] = [];
const NO_DATA: DataSnapshot = { project: null, names: null, rows: {}, itemTable: undefined, locked: null, errors: {} };

type Search = { query: string; filter: ListFilter };
const NO_SEARCH: Search = { query: "", filter: NO_FILTER };

/**
 * Builds the model of `area`: the part and the selections live in the shell state (remembered between runs), search and filter here, per
 * part (gone with another project); the rows come from the store, read once the area is `active`.
 */
export function useDataAreaModel(area: DataArea, props: DataAreaProps): DataAreaModels {
  const { project, store, active, target, goTo, onTrail, dirty, onEdited, bus, headers } = props;
  const problems = props.problems ?? null;
  const placeTrainer = props.onPlaceTrainer ?? null;
  const shell = useShell();
  const { setPokemonSub, setSelected } = shell;
  const stored = shell.state.sub.pokemon;
  // A part remembered while it was open, locked now, shows the Pokédex.
  const sub: DataSub = area === "gegenstaende" ? "items" : isSubLocked(stored) ? "pokedex" : stored;
  const kind = SUB_KIND[sub];
  const selectKey = SELECT_KEY[sub];
  const selected = selectKey === null ? null : (shell.state.selected[selectKey] ?? null);

  const [searches, setSearches] = useState<Partial<Record<DataSub, Search>>>({});
  const [seenProject, setSeenProject] = useState(project);
  if (seenProject !== project) {
    // Another project: nothing typed or filtered in the old one stays. Reset while rendering, so no render shows the old search.
    setSeenProject(project);
    setSearches({});
  }
  const search = searches[sub] ?? NO_SEARCH;

  const snapshot = useDataSnapshot(store);
  const data = project !== null && snapshot.project === project ? snapshot : NO_DATA;
  useEffect(() => {
    if (active && project !== null && kind !== null) store.need(project, kind);
  }, [active, project, kind, store]);

  const hasTable = data.itemTable !== null && data.itemTable !== undefined;
  const rows = (kind === null ? undefined : data.rows[kind]) ?? NO_ROWS;
  const labels = useMemo(() => (kind === null ? [] : rows.map((row) => rowLabel(kind, row, hasTable))), [kind, rows, hasTable]);
  const labelByIndex = useMemo(() => new Map(rows.map((row, i) => [row.index, labels[i] ?? ""])), [rows, labels]);
  const labelOf = useCallback((row: DataRowDto) => labelByIndex.get(row.index) ?? (kind === null ? row.name : rowLabel(kind, row, hasTable)), [labelByIndex, kind, hasTable]);
  const shown = useMemo(() => (kind === null ? NO_ROWS : filterRows(kind, rows, labels, search.query, search.filter)), [kind, rows, labels, search]);
  const row = useMemo(() => (selected === null ? null : (rows.find((r) => r.index === selected) ?? null)), [rows, selected]);

  const setSub = useCallback((next: PokemonSub) => {
    if (!isSubLocked(next)) setPokemonSub(next);
  }, [setPokemonSub]);
  const select = useCallback((index: number | null) => {
    if (selectKey !== null) setSelected(selectKey, index);
  }, [selectKey, setSelected]);
  const setSearch = useCallback((part: DataSub, change: (s: Search) => Search) => setSearches((all) => ({ ...all, [part]: change(all[part] ?? NO_SEARCH) })), []);
  const setQuery = useCallback((query: string) => setSearch(sub, (s) => ({ ...s, query })), [setSearch, sub]);
  const setFilter = useCallback((filter: ListFilter) => setSearch(sub, (s) => ({ ...s, filter })), [setSearch, sub]);
  const clearFilter = useCallback(() => setSearch(sub, () => NO_SEARCH), [setSearch, sub]);

  // A link opens its part and selects its member; a search or filter that could hide it is cleared.
  useEffect(() => {
    if (target === null || areaOfSub(target.sub) !== area) return;
    if (target.sub !== "items") setPokemonSub(target.sub);
    setSelected(target.sub, target.index);
    setSearch(target.sub, () => NO_SEARCH);
  }, [target]);

  const counts = useMemo(() => {
    const of = (k: ListKind | null) => (k === null ? undefined : data.rows[k]?.length);
    return Object.fromEntries(POKEMON_SUBS.map((p) => [p, of(SUB_KIND[p])])) as Record<PokemonSub, number | undefined>;
  }, [data.rows]);
  const switcher = useMemo<PanelHeadSwitcher | null>(
    () =>
      area === "gegenstaende"
        ? null
        : {
            items: POKEMON_SUBS.map((p) => ({ id: p, label: SUB_LABEL[p], count: counts[p], shortcut: SUB_SHORTCUT[p], disabled: isSubLocked(p), hint: SUB_LOCKED[p] })),
            value: sub,
            onChange: (id: string) => setSub(id as PokemonSub),
          },
    [area, counts, sub, setSub],
  );

  // The breadcrumb: the area, the part (its switcher) or the selected item's pocket, and the selection.
  const pocket = row === null ? null : itemPocket(row);
  const crumbs = useMemo(() => {
    const to = row === null ? null : targetOf(sub, row.index);
    const selection = row === null || to === null ? null : { label: labelOf(row), target: to };
    if (switcher !== null) return dataCrumbs({ area: "pokemon", switcher, selection, goTo });
    // A click on the pocket shows that pocket's items (and drops a search that would hide them).
    const pocketCrumb =
      selection === null || pocket === null ? null : { label: pocketLabel(pocket), onClick: () => setSearch("items", () => ({ query: "", filter: { ...NO_FILTER, pocket } })) };
    return dataCrumbs({ area: "gegenstaende", pocket: pocketCrumb, selection, goTo });
  }, [row, labelOf, sub, switcher, pocket, goTo, setSearch]);
  useEffect(() => onTrail?.(crumbs), [crumbs, onTrail]);

  const error = kind === null ? null : (data.errors[kind] ?? data.errors.names ?? (kind === "item" ? data.errors.itemTable : undefined) ?? null);
  // Items wait for the item table too: without it they would show as data members ("Daten 0017") for a moment.
  const waiting = kind !== null && (data.rows[kind] === undefined || (kind === "item" && data.itemTable === undefined));
  const loading = project !== null && waiting && data.locked === null && error === null;

  const model = useMemo<DataAreaModel>(
    () => ({
      area,
      project,
      sub,
      setSub,
      switcher,
      kind,
      selected,
      select,
      row,
      rows,
      labelOf,
      names: data.names ?? NO_NAMES,
      itemTable: data.itemTable,
      loading,
      error,
      locked: data.locked,
      goTo,
      store,
      target,
      dirty,
      onEdited,
      bus,
      headers,
      problems,
      placeTrainer,
    }),
    [area, project, sub, setSub, switcher, kind, selected, select, row, rows, labelOf, data.names, data.itemTable, loading, error, data.locked, goTo, store, target, dirty, onEdited, bus, headers, problems, placeTrainer],
  );
  const searchModel = useMemo<DataSearchModel>(
    () => ({ shown, query: search.query, setQuery, filter: search.filter, setFilter, clearFilter }),
    [shown, search, setQuery, setFilter, clearFilter],
  );
  return { model, search: searchModel };
}

/** Where a member of a part leads (the breadcrumb's last crumb); null for a part without members yet. */
function targetOf(sub: DataSub, index: number): NavTarget | null {
  switch (sub) {
    case "pokedex":
      return { kind: "species", index };
    case "trainer":
      return { kind: "trainer", index };
    case "moves":
      return { kind: "move", index };
    case "items":
      return { kind: "item", id: index };
    default:
      return null;
  }
}

const CONTEXTS: Record<DataArea, Context<DataAreaModel | null>> = {
  pokemon: createContext<DataAreaModel | null>(null),
  gegenstaende: createContext<DataAreaModel | null>(null),
};

const SEARCHES: Record<DataArea, Context<DataSearchModel | null>> = {
  pokemon: createContext<DataSearchModel | null>(null),
  gegenstaende: createContext<DataSearchModel | null>(null),
};

/** The provider of an area's model (`PokemonArea`, `ItemsArea`). */
export const dataAreaProvider = (area: DataArea) => CONTEXTS[area].Provider;

/** The provider of an area's search (`PokemonArea`, `ItemsArea`), inside the model's. */
export const dataSearchProvider = (area: DataArea) => SEARCHES[area].Provider;

/** The model of the data area `area` around the caller. */
export function useDataArea(area: DataArea): DataAreaModel {
  const model = useContext(CONTEXTS[area]);
  if (model === null) throw new Error(`useDataArea outside the ${area} area`);
  return model;
}

/** The search of the data area `area` around the caller: only the lists read it. */
export function useDataSearch(area: DataArea): DataSearchModel {
  const search = useContext(SEARCHES[area]);
  if (search === null) throw new Error(`useDataSearch outside the ${area} area`);
  return search;
}
