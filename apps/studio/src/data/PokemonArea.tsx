import { memo } from "react";
import type { ReactNode } from "react";
import { dataStatus } from "../app/statusContext";
import { DataList } from "./DataList";
import { DataListPane, DataPanel, chosenHead } from "./DataParts";
import type { DataPart } from "./DataParts";
import type { DataSub } from "./dataModel";
import { PokedexHand } from "./pokedex/PokedexHand";
import { PokedexList } from "./pokedex/PokedexList";
import { PokedexView } from "./pokedex/PokedexView";
import { speciesCount } from "./pokedex/pokedexModel";
import { dataAreaProvider, useDataAreaModel, useDataArea } from "./useDataArea";
import type { DataAreaProps } from "./useDataArea";

const Provider = dataAreaProvider("pokemon");

/**
 * The Pokémon area's model for its list, view, hand and status parts (`useDataArea("pokemon")`). Memoised: it renders when `context`
 * changes (the owner keeps it) or its children do, not for every render of the app.
 */
export const PokemonArea = memo(function PokemonArea({ context, children }: { context: DataAreaProps; children?: ReactNode }) {
  const model = useDataAreaModel("pokemon", context);
  return <Provider value={model}>{children}</Provider>;
});

function MovesList() {
  return <DataList area="pokemon" label="Attacken" attr="data-move" />;
}

const Chosen = chosenHead("pokemon");

/**
 * The parts of the Pokémon area by their switcher entry. The mount points of the editors: the Pokédex (Task 14) and the moves (Task 15)
 * replace their entries; Trainer and Begegnungen come with Phase 3b.3b (their entries are locked in the switcher).
 */
export const POKEMON_PARTS: Partial<Record<DataSub, DataPart>> = {
  pokedex: { List: PokedexList, View: PokedexView, Hand: PokedexHand, count: speciesCount },
  moves: { List: MovesList, View: Chosen, Hand: Chosen },
};

/** The list column of the Pokémon area: the switcher of its parts and the list of the part that shows. */
export function PokemonList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return <DataListPane area="pokemon" parts={POKEMON_PARTS} open={open} onToggle={onToggle} />;
}

/** The view (Schau) of the Pokémon area: the chosen member of the part that shows. */
export function PokemonView() {
  return <DataPanel area="pokemon" parts={POKEMON_PARTS} place="View" />;
}

/** The hand of the Pokémon area: the editor of the chosen member. */
export function PokemonHand() {
  return <DataPanel area="pokemon" parts={POKEMON_PARTS} place="Hand" />;
}

/** The middle of the status line while Pokémon shows: "Staravia, 0397". */
export function PokemonStatus() {
  const { row, labelOf } = useDataArea("pokemon");
  return <>{dataStatus(row === null ? null : { label: labelOf(row), tech: row.tech })}</>;
}
