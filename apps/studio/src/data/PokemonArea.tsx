import { memo } from "react";
import type { ReactNode } from "react";
import { dataStatus } from "../app/statusContext";
import { DataListPane, DataPanel } from "./DataParts";
import type { DataPart } from "./DataParts";
import type { DataSub } from "./dataModel";
import { EncounterHand } from "./encounters/EncounterHand";
import { EncounterList } from "./encounters/EncounterList";
import { EncounterView } from "./encounters/EncounterView";
import { MoveHand } from "./moves/MoveHand";
import { MovesList } from "./moves/MovesList";
import { MoveView } from "./moves/MoveView";
import { PokedexHand } from "./pokedex/PokedexHand";
import { PokedexList } from "./pokedex/PokedexList";
import { PokedexView } from "./pokedex/PokedexView";
import { speciesCount } from "./pokedex/pokedexModel";
import { TrainerHand } from "./trainer/TrainerHand";
import { TrainerHandNarrate } from "./trainer/TrainerHandNarrate";
import { TrainerList } from "./trainer/TrainerList";
import { TrainerView } from "./trainer/TrainerView";
import { dataAreaProvider, dataSearchProvider, useDataAreaModel, useDataArea } from "./useDataArea";
import type { DataAreaProps } from "./useDataArea";

const Provider = dataAreaProvider("pokemon");
const SearchProvider = dataSearchProvider("pokemon");

/**
 * The Pokémon area's model for its list, view, hand and status parts (`useDataArea("pokemon")`). Memoised: it renders when `context`
 * changes (the owner keeps it) or its children do, not for every render of the app.
 */
export const PokemonArea = memo(function PokemonArea({ context, children }: { context: DataAreaProps; children?: ReactNode }) {
  const { model, search } = useDataAreaModel("pokemon", context);
  return (
    <Provider value={model}>
      <SearchProvider value={search}>{children}</SearchProvider>
    </Provider>
  );
});

/** The parts of the Pokémon area by their switcher entry: the Pokédex, the trainers, the encounter files and the moves. */
export const POKEMON_PARTS: Partial<Record<DataSub, DataPart>> = {
  pokedex: { List: PokedexList, View: PokedexView, Hand: PokedexHand, count: speciesCount },
  trainer: { List: TrainerList, View: TrainerView, Hand: TrainerHand, ViewNarrate: TrainerView, HandNarrate: TrainerHandNarrate },
  encounters: { List: EncounterList, View: EncounterView, Hand: EncounterHand },
  moves: { List: MovesList, View: MoveView, Hand: MoveHand },
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
