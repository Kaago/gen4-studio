# Symbol tables

The fixed English ids of the text project (`SPECIES_PIKACHU`, `MOVE_THUNDER_SHOCK`, …) for
the game's numbers of species and forms, moves, items, abilities and types. The kernel embeds
them (`textproject::symbols`); a number without an entry is written as `PREFIX_NNNN`.

| File | Numbers | Source |
|---|---|---|
| `species.yaml` | 0–495 | `pokemon_species.csv`, generation ≤ 4 (national dex = game index) |
| `forms.yaml` | 496–507 | `pokemon.csv`, in the order of `../species_forms.yaml` |
| `moves.yaml` | 0–467 | `moves.csv`, generation ≤ 4 |
| `items.yaml` | 0–536, with gaps | `item_game_indices.csv` (generation 4) and `items.csv` |
| `abilities.yaml` | 0–123 | `abilities.csv`, generation ≤ 4, main series |
| `types.yaml` | 0–17 | `type_game_indices.csv` (generation 4) and `types.csv` |

## Source and license

Generated from [PokeAPI](https://github.com/PokeAPI/pokeapi) at commit
`2bf639e5572812a67d2b94903fdcb9d095f5aeb2`, files under `data/v2/csv/`. PokeAPI is licensed
under the BSD 3-Clause License (copyright Paul Hallett and PokéAPI contributors); its license
text with the copyright line is in `LICENSE-PokeAPI.txt`, as the license requires for
redistribution. Pokémon and Pokémon character names are trademarks of Nintendo.

The names are not ROM data: they are the English identifiers of PokeAPI, not read from any
game. Nothing from pret is copied.

## Regenerating

From the repository root, with network (the CSV and `LICENSE.md` are downloaded into a
temporary folder):

```sh
python3 -I scripts/symbols/generate_symbols.py --commit 2bf639e5572812a67d2b94903fdcb9d095f5aeb2 --out crates/g4-kernel/metadata/symbols
```

Offline, from a folder holding `LICENSE.md` and the eight CSV (`pokemon_species.csv`,
`pokemon.csv`, `moves.csv`, `item_game_indices.csv`, `items.csv`, `abilities.csv`,
`type_game_indices.csv`, `types.csv`) of that commit:

```sh
python3 -I scripts/symbols/generate_symbols.py --commit 2bf639e5572812a67d2b94903fdcb9d095f5aeb2 --out crates/g4-kernel/metadata/symbols --csv-dir <folder>
```

Ids are the PokeAPI identifier with `-` and other non-alphanumerics as `_`, upper case, with
the prefix. The generator stops on a duplicate id, on an id of the form `PREFIX_<digits>`
(that is the fallback form) and on gaps in species 0–507, moves 0–467, abilities 0–123 and
types 0–17.

## Own entries

Not from PokeAPI, added by the generator:

- `SPECIES_NONE` (0), `MOVE_NONE` (0), `ITEM_NONE` (0), `ABILITY_NONE` (0).
- `SPECIES_EGG` (494) and `SPECIES_BAD_EGG` (495).

Types have no "none": type 0 is Normal. Type 9 (the "???" type) has a generation-4 game index
in PokeAPI (`unknown`), so it is `TYPE_UNKNOWN` and no own `TYPE_MYSTERY` entry is needed.

## Changing the tables

The tables are part of the schema of the text project: an id written into a project must keep
meaning the same number. They change only together with a new `schemaVersion` and a migration
that renames the ids in existing projects.
