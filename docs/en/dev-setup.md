*Deutsch: [docs/dev-setup.md](../dev-setup.md)*

# Development setup

## apicula (NSBMD to GLB)

The Matrix shows the map models and the building models (NSBMD) with their textures (NSBTX) as GLB. The conversion is done by
[apicula](https://github.com/scurest/apicula). There is no prebuilt release for macOS, so it is built from source:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
source scripts/tool-versions.env
cargo install --git https://github.com/scurest/apicula --rev "$APICULA_REV" --locked --root tools/apicula
```

The result is `tools/apicula/bin/apicula` (`tools/` is excluded by `.gitignore`).
The version is in `scripts/tool-versions.env` (`APICULA_REV`, commit `3d4e91e14045392a49c89e86dab8cb936225588c`, `apicula 0.1.1-dev`);
`pnpm bundle` builds apicula with the same version if `tools/apicula/bin/apicula` is missing.

### Where the app looks for apicula

1. The environment variable `G4_APICULA` (full path to the binary). If it is set, it always applies, even if the path is wrong.
2. A file `apicula` next to the Studio binary.
3. Debug builds only: `tools/apicula/bin/apicula` in the repository.

In the app bundle, apicula sits as a sidecar in `Contents/MacOS/` (entry 2), see `docs/en/release.md`. An empty
`apicula` file (placeholder from `build.rs`) does not count as found.

If apicula is missing, the app reports `tool apicula unavailable: ...`; permissions and building boxes are still shown.

```sh
G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev
```

### Model cache

Converted models (maps and buildings) are stored under `<project>/.g4/models/`. The key is the xxh3-64 hash over
the salt `g4glb-v3`, the xxh3-64 hash of the apicula binary (tool hash, computed once per path and process), the
NSBMD and the NSBTX texture set (for maps `AreaData[area_data_id].map_tileset`, for buildings `buildings_tileset`), as
16 hex characters. There are three kinds of files:

- `<hash>.glb`: the converted model.
- `<hash>.fail`: marker of a failed *textured* conversion; its content is the error text. Whoever finds the marker
  skips the textured attempt and shows the model without textures (a `.glb` for that exists under the hash of the
  untextured NSBMD) instead of starting apicula again on every load.
- `last-tool`: the tool hash (16 hex characters) of the apicula used last. If apicula is missing, hits are looked up under this
  hash, so that models that have already been converted and their markers stay visible even without apicula.

If the model, the tileset or the apicula binary changes, a new key results and with it a fresh
attempt; old entries stay where they are. If the area data or tileset entry does not exist, the model is converted
without textures.

apicula runs on a worker thread (`spawn_blocking`) and for at most `G4_APICULA_TIMEOUT_SECS` seconds per call
(default 60). After that the process is terminated together with its child processes. If the textured attempt hangs, no
`.fail` marker is written (apicula may just have been slow), the model appears without textures, and until the program ends
this tileset is not tried again (remembered in memory only; after a restart there is a fresh attempt). Only
if the conversion without textures also runs into the time limit does the message `apicula did not finish within <n> s` appear:

```sh
G4_APICULA_TIMEOUT_SECS=120 G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev
```

Consequences:

- The files at the top level of `.g4/` are versioned project metadata (`base.json`, `preview.png`) and stay. Only the
  subfolders `cache/`, `tmp/`, `build/`, `backup/` and `models/` are local and may be deleted at any time, so `.g4/models/`
  as well.
- An apicula update invalidates `.glb` and `.fail` automatically (new tool hash), so deleting the markers is
  no longer necessary. The only way to get rid of a marker without an update is by hand: `rm <project>/.g4/models/*.fail`.
  A faulty `.glb` can only be renewed by deleting the file.
- When deleting, the app must not be loading models. Intermediate states (`apicula-<pid>-<n>` folders) are harmless.

## rotom and rotom-lsp (scripts)

Scripts are compiled with [rotom](https://github.com/KalaayPT/rotom); the library is a Cargo dependency of `g4-kernel`
(Git tag `v0.7.0`, with a comment in the code where which API is used). Completion, hover and diagnostics in the script editor come from the
language server `rotom-lsp`, a separate process (sidecar) from the **same tag `v0.7.0`**. Without it the editor keeps working (open,
edit, save, compile); only the language services are missing, and the header bar says that rotom-lsp is missing and that completion and diagnostics are off.

Build (once; `tools/` is excluded by `.gitignore`):

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
CFLAGS="-D_POSIX_C_SOURCE=200809L" \
  cargo install --git https://github.com/KalaayPT/rotom --tag v0.7.0 rotom-lsp --root tools/rotom
```

- `CFLAGS` is needed on macOS SDKs from 15.4: otherwise nitroarc's `strchrnul` collides with the SDK declaration (the same value is in
  `.cargo/config.toml`; `cargo install` did not pick it up from there).
- **Network while building:** on **every clean build** (also of `g4-kernel`, which includes rotom as a library), rotom's `build.rs` downloads `db-latest.zip` from the network, and the first time `cargo` also downloads the Git dependency. At run time Studio needs no network: it always uses the bundled command database in `crates/g4-kernel/metadata/scrcmd/` (revision 33, origin in `SOURCE.md` there), never a downloaded one and never `rotom init`. The warning "stripping debug info with `rust-objcopy` failed" is harmless.
- The binary needs `libnitroarc_ffi.dylib` (LGPL, dynamically linked, as already with uxie in the app) and has **no rpath**. For
  development, put the dylib next to it and set an rpath (`cargo build --release -p studio` produces it in `target/release/`):

  ```sh
  cp target/release/libnitroarc_ffi.dylib tools/rotom/bin/
  install_name_tool -add_rpath @loader_path tools/rotom/bin/rotom-lsp
  codesign --force --sign - tools/rotom/bin/rotom-lsp
  echo | tools/rotom/bin/rotom-lsp     # answers with a parse error, no dyld error
  ```

  `apps/studio/scripts/prepare-bundle.sh` does the same (and builds `rotom-lsp` if it is missing); for the bundle, the copy in
  `src-tauri/binaries/` additionally gets the rpath `@executable_path/../Frameworks` (dylib in `Contents/Frameworks`).

### Where the app looks for rotom-lsp

1. The environment variable `G4_ROTOM_LSP` (full path). If it is set, it always applies, even if the path is wrong.
2. A file `rotom-lsp` next to the Studio binary (in the bundle `Contents/MacOS/`). An empty file (placeholder from `build.rs`) does not count.
3. Debug builds only: `tools/rotom/bin/rotom-lsp` in the repository.

```sh
G4_ROTOM_LSP="$PWD/tools/rotom/bin/rotom-lsp" pnpm tauri dev
```

### Lifecycle

One process per open project, working directory = project root, started by the script panel through `lsp_start(channel)`. The
messages travel as `Content-Length`-framed JSON-RPC over stdio; a reader thread pushes every message into the Tauri channel,
`lsp_send` queues the message for a writer thread (it never blocks; if the server does not read, the queue fills up and `lsp_send` reports an error). It is ended on project change, `lsp_stop` and app exit (the whole process group). If it dies, the
next `lsp_send` restarts it, but at most once per minute; a faster death results in the error that rotom-lsp keeps exiting, and it
stays off until the next `lsp_start`. The restart requires a new `initialize` from the client.

## Tests

Commands from the root of the repository, with `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`:

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
cargo deny --locked check licenses advisories
bash scripts/check-licenses-npm.sh
cd apps/studio && pnpm test && pnpm build
```

### cargo-deny (licences and advisories)

`cargo deny` checks the dependency tree with all features against `deny.toml` in the repository root: licences only from the
allowlist, no open RustSec advisory (vulnerability, unmaintained, yanked). Install once per machine (it ends up
in `~/.cargo/bin`):

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo install --locked cargo-deny@0.20.2
```

The first run downloads the RustSec database to `~/.cargo/advisory-dbs`, every later one fetches it again; a new advisory can therefore
turn the gate red without any change to the repository. In that case first try `cargo update -p <crate>@<version>` (semver-compatible) or a
compatible update of the direct parent; only if none brings the fix, add one exception per RUSTSEC ID under
`[advisories] ignore` with a reason (crate and version, chain, why no update works, why it is acceptable). Blanket
switches like `unmaintained = "none"` do not exist.

`--all-features` is needed as soon as `g4-kernel` is tested on its own: the integration tests `store`, `cli`, `project`, `build`
and `headers` as well as the vanilla test in `export` are guarded by `#![cfg(feature = "test-support")]` or
`#[cfg(feature = "test-support")]`. `cargo test -p g4-kernel` without the feature does not compile them and reports `0 tests`
without an error. `cargo test -p g4-kernel --features test-support` is enough too. With `--workspace` the
Studio crate switches the feature on for `g4-kernel`; `--all-features` makes that explicit and is the standard command for the check
before every commit.

**In a Git worktree**, build and test only at workspace level: `cargo test --workspace --all-features <name filter>`
(several filters after `--` work too). Commands per package (`-p g4-kernel`, `-p g4-formats`) and `--test <name>` change the
feature set of rotom; its `build.rs` then runs again and downloads `db-latest.zip` from the network (see "rotom and
rotom-lsp"). In the 3b.3a worktrees, whose `target/` had taken over the rotom build output from the main checkout,
exactly this download failed ("headers' total size surpassed max_headers_size"). The name filter runs the same tests.

### npm licences

`scripts/check-licenses-npm.sh` checks the production packages of `apps/studio` (`pnpm licenses list --prod --json`) against the
allowlist `MIT`, `Apache-2.0`, `ISC`, `BSD-2-Clause`, `BSD-3-Clause`, `0BSD`, `CC0-1.0`, `Unlicense`, `OFL-1.1`, `MPL-2.0` and
`Zlib`. An SPDX expression `A OR B` is accepted if one side is allowed, `A AND B` only if both are allowed; parentheses group.
Every other licence is printed with package and version on stderr and the script exits with 1; otherwise it prints `npm licenses: <n> packages ok`.
A new dependency goes into `package.json` with an exact version (without `^`); a licence outside the list (such as EPL-2.0 of
`elkjs`) needs a decision, not an entry in the script on the side.

### Gated tests

Tests with ROM data run only if the environment variables are set; otherwise they report `skipped: ...` on stderr and
count as passed. With `-- --nocapture` you can see what really ran. The project is only read; tests that write
work on a copy in a temp directory (`g4_kernel::test_support::vanilla_copy()`). The originals are never
written.

| Variable | Meaning | Needed by |
|---|---|---|
| `G4_TEST_PROJECT` | ds-rom project folder (`..._DSPRE_contents`) | the vanilla tests in `g4-formats`, `g4-kernel` and `studio`; `crates/g4-kernel/tests/base.rs` (4.1a; plus `publish_after_a_map_edit_writes_a_small_patch_that_applies`, 4.5), the gated part of `cli_publish_prints_the_files_and_the_count` in `crates/g4-kernel/tests/cli.rs` (4.5) and `crates/g4-kernel/tests/original.rs` (4.1b), `crates/g4-kernel/tests/battle_sprites.rs` and `crates/g4-kernel/tests/egg_moves.rs` (3d.3), `crates/g4-kernel/tests/textproject_source.rs` (4.3b; since 3d.3 plus `text_project_egg_move_edit_builds_and_check_data_finds_it`), `crates/g4-kernel/tests/git.rs` and `crates/g4-kernel/tests/diff.rs` (4.4; in `git.rs` plus `publish_then_commit_all_leaves_status_empty_and_no_remote`, 4.5), `crates/g4-kernel/tests/recipes.rs` (3d.2: `vanilla_room_recipe_matches_the_wizard_scenario`, Platinum DE only, otherwise `skipped`; "Clear world" with undo of the scripts is checked by `vanilla_empty_region_undo_restores_the_world` in `tests/templates.rs`), `crates/g4-kernel/tests/dialog.rs` (3d.5: `vanilla_dialogs_of_a_town_resolve`, the dialogs of Twinleaf Town and New Bark Town respectively and of every character in the game), `crates/g4-kernel/tests/refs_messages.rs` (3d.5: `vanilla_message_refs_count_the_script_messages`, the index of the messages from the scripts) and `crates/g4-kernel/tests/recipes_erzaehlen.rs` (3d.5: `vanilla_new_figure_message_equals_the_input`, `vanilla_doors_round_trip_in_the_town`, `vanilla_trainer_here_in_the_town`; all three work on `vanilla_copy()`, Platinum DE and HeartGold DE, otherwise `skipped`) as well as `reset_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/original.rs`, `battle_sprites_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/data/tests.rs` (3d.3), `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` in `apps/studio/src-tauri/src/mcp/tools.rs` (4.3b), `git_and_diff_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/git.rs` (4.4) and `publish_and_commit_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/publish.rs` (4.5, also needs `git`) as well as `crates/g4-kernel/tests/flags.rs` (3d.1; also `<folder>.nds` next to a test copy `<folder>`), `crates/g4-kernel/tests/threads.rs` and `crates/g4-kernel/tests/stories.rs` (3d.6, likewise; `stories.rs` also needs `git`) need the `.nds` directly next to the folder (base ROM through `suggest_base`, only read); `flags_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/flags.rs` (3d.1) needs only `G4_TEST_PROJECT` (it works on `vanilla_copy()` without a base ROM); deleting (3d.4): `vanilla_world_edges_match_the_measured_counts` in `crates/g4-kernel/tests/refs.rs` (counts of the world edges; the script → header part sets up the text project on a copy under `target/tmp/refs-<folder>` and needs the `.nds` next to the folder), `textproject_round_trip_with_tombstone` and `textproject_export_idempotent_with_tombstone` in `crates/g4-kernel/tests/textproject.rs`, `a_branch_without_the_freed_file_builds_after_git_switch` in `crates/g4-kernel/tests/git.rs` (needs `git` and the `.nds` next to the folder) and `delete_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/delete.rs` (needs the `.nds`); `crates/g4-kernel/tests/delete.rs` itself runs without ROM data (synthetic text project, without `G4_TEST_PROJECT`); BDHC and terrain (6.2): `crates/g4-formats/tests/vanilla_bdhc.rs`, `crates/g4-kernel/tests/terrain.rs` (reads `unpacked/maps`, the writing test `set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen` works on `vanilla_copy()`) and `terrain_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/terrain.rs` (on `vanilla_copy()`) need only `G4_TEST_PROJECT`, no base ROM; world format (6.3a): `crates/g4-kernel/tests/world.rs` (`vanilla_tilesets_have_one_ground_tile_per_texture`, `vanilla_vote_shares_match_task0`; each derives the tileset from the map texture pack, only read, need only `G4_TEST_PROJECT`), plus, with a world map, `textproject_round_trip_with_world` and `textproject_export_idempotent_with_world` in `crates/g4-kernel/tests/textproject.rs` as well as `vanilla_migration_v2_to_v3_changes_only_the_manifest` in `crates/g4-kernel/tests/textproject_migrate.rs` (on `vanilla_copy()`, need the `.nds` next to `<name>_DSPRE_contents`); baking and build (6.3b): `crates/g4-kernel/tests/world_bake.rs` (`vanilla_maps_adopted_form1_bake_byte_equal`, `vanilla_adopted_project_builds_byte_equal`, `painted_map_on_a_vanilla_pack_bakes_within_limits`, `world_step_is_incremental_per_map_on_the_copy`, `member_changed_both_ways_on_the_copy`; each sets up the text project on `vanilla_copy()` and needs the `.nds` next to `<name>_DSPRE_contents`, the comparison with apicula also needs `G4_APICULA`, without it the test reports `apicula=skipped`); painting (6.4a): `crates/g4-kernel/tests/world_paint.rs` (`painted_map_beside_the_start_town_bakes_and_builds`, `undo_to_the_start_leaves_the_files_as_they_were` set up the text project on `vanilla_copy()` and need the `.nds` next to `<name>_DSPRE_contents`; `palette_textures_of_every_map_package` needs only `G4_TEST_PROJECT`; they print `paint-gate …`, `paint-undo …`, `palette …`); freeform and exchange (6.5b): `crates/g4-kernel/tests/world_freeform.rs` (`freeform_on_a_painted_map_bakes_and_apicula_reads_it`, `piece_model_on_the_copy`, `form2_of_a_vanilla_map_bakes_apicula_readable`, `form2_sweep_over_every_map`, `tmj_round_trip_of_the_painted_map_and_every_adopted_map`; each sets up the text project on `vanilla_copy()` and needs the `.nds` next to `<name>_DSPRE_contents`; form 2 needs `G4_APICULA`, otherwise `skipped`, the freeform test reports `apicula=skipped` without it; the sweep over every map runs only with `G4_FORM2_SWEEP=1`); Smart Drawing (6.4b): `crates/g4-kernel/tests/world_smart.rs` (`edge_set_rings_the_plus_without_a_wrong_corner`, `cliff_sets_walls_and_collision_by_the_edge_rule`; set up the text project on `vanilla_copy()`, paint the map "Kantenweg" ("edge path") next to the start town and need the `.nds` next to `<name>_DSPRE_contents`; they print `smart-ring …`, `smart-cliff …`, `smart-cliff-lower …`) |
| `G4_APICULA` | full path to `tools/apicula/bin/apicula` | the model tests (`export`, building GLB in `studio`) |
| `G4_DSROM_CLI` | full path to the ds-rom CLI `dsrom` | `vanilla_rebuild_matches_dsrom_cli` in `crates/g4-kernel/tests/build.rs` |
| `G4_ROTOM_LSP` | full path to `tools/rotom/bin/rotom-lsp` | only the gate script passes it on; the unit tests of `lsp.rs` do not need it (fake server) |
| `G4_TEST_ROM` | an `.nds` file (only read) | `extract_rom_creates_openable_project`, `project_start_inspects_the_test_rom`, `project_start_create_then_build_matches_the_rom`, `project_start_incremental_build_matches_the_rom`, `project_start_empty_region_from_the_rom`, `project_start_text_project_builds_byte_equal` (4.3b) |
| `G4_BASE_ROM_DIR` | folder with base ROMs (`*.nds`, not recursive, only read) | no gated test (the tests in `tests/base.rs` and `tests/original.rs` take the neighbouring `.nds` of the project); `g4k patch create`, `g4k base check`, `studio mcp --project` and the bridge look there for the SHA-1 of the mark; in the gate `Roms/_testkopien`, never the originals |
| `git` (CLI) | `find_git`: `G4_GIT`, otherwise `PATH` and the usual places | only the tests that write a repository (`git::cli`, `create_with_git_reports_the_phase_and_commits`, `cli_git_init_and_status`, `tests/git.rs`, `tests/diff.rs`, `git_and_diff_over_the_command_path_on_the_vanilla_copy`, `commit_all_with_git_commits_and_reads_the_status`, `publish_and_commit_over_the_command_path_on_the_vanilla_copy`); without `git` they report `skipped: git not found` and stay green. Tests that only read need no `git` (`gix`, `test_support::GitFixture`) |
| `G4_GIT` | full (absolute) path to a `git` | optional: only this path applies, no further search; a relative path (even `git` alone) counts as not found, never a search in `PATH` (`cli_git_status_honours_g4_git` checks both through a child process) |
| `xdelta3` in `PATH` | xdelta3 CLI, optional | cross-check in `patch_after_map_and_after_map_script_text_is_small_and_applies`; if it is missing, the test reports `xdelta3 cross-check skipped: …` and stays green |

`G4_TEST_PROJECT` is set to each of the three projects in turn (Platinum DE, HeartGold DE, HeartGold US); the
tests run once per project. The paths must be absolute, because `cargo test` starts the tests in the folder of the respective crate:

```sh
G4_TEST_PROJECT="$PWD/../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
G4_APICULA="$PWD/tools/apicula/bin/apicula" \
G4_DSROM_CLI="$PWD/../Roms/_tools/dsrom" \
  cargo test --workspace --all-features -- --nocapture
```

`vanilla_rebuild_matches_dsrom_cli` builds the same project with the reference CLI and with `g4_kernel::build::build` and demands
byte-identical ROMs. For changes to build or pack code this run is mandatory. Since 3b.3a the build also packs the
data folders (encounters, trainers, Pokémon base data, learnsets, evolutions, moves, items, trainer text tables,
item and Pokémon icons); the test checks that the ten kinds of data archives are packed with their full member count and the
ROM nevertheless stays byte-identical. From a
Git worktree, `$PWD/../Roms` points nowhere: set `G4_DSROM_CLI=/<absolute path to Roms>/_tools/dsrom` there (and likewise
`G4_TEST_PROJECT` and `G4_ROMS_DIR` as absolute paths).

#### Data archives and the uxie oracle (3b.3a)

The codecs of the data archives in `g4-formats/src/data/` are our own (raw bytes with a field map); uxie 0.9.7 is only
a test oracle now (dev dependency of `g4-formats`, still a regular one in `g4-kernel`). Reason: uxie rejects 172 of 471 vanilla moves
and writes them with 14 instead of 16 bytes, and its encounter writer is unusable; personal data, items and trainer properties,
on the other hand, it reads correctly field by field. The gated tests for this:

| Test | File | Checks |
|---|---|---|
| `vanilla_fixed_records_round_trip`, `vanilla_variable_records_round_trip`, `vanilla_trainer_parties_round_trip_with_their_properties` | `crates/g4-formats/tests/vanilla_data.rs` | `write(read(x)) == x` for every member of the ten archive kinds; the member count must match (Platinum / HGSS: encounters 183/142, trainerProperties and trainerParty 928/738, personalPokeData, learnsets, evolutions 508 each, moveData 471, itemData 446/514, trainerTextTable and trainerTextOffset 1 each). A folder without members or with files other than `NNNN` makes the test fail |
| `vanilla_personal_matches_uxie`, `vanilla_items_match_uxie`, `vanilla_trainer_properties_match_uxie` | same | the uxie oracle: every field of our own codecs against the getters of uxie (for items also the individual "effect in party" flags) |
| `vanilla_pt_water_slots_have_min_le_max` | same | Platinum only (HGSS reports "skipped: not a Platinum project"): 1080 occupied water slots, all with min ≤ max |
| `vanilla_arm9_tables_are_plausible` | `crates/g4-kernel/tests/arm9_tables.rs` | item table, TM/HM list and palettes of the Pokémon icons from `metadata/arm9_tables.yaml` are plausible (CPUD, IPKD, IPKE) |
| `vanilla_data_members_load_unlocked_and_byte_identical` | `crates/g4-kernel/tests/store.rs` | every data member loads unlocked and byte-identical into the store |
| `vanilla_name_counts_match_task0`, `every_vanilla_trainer_name_round_trips` | `crates/g4-kernel/tests/names.rs` | name counts of the text archives, trainer names there and back |
| `vanilla_index_builds_under_a_second`, `vanilla_dirty_stays_fast_after_index_build`, `vanilla_route_201_encounters_reference_known_species` | `crates/g4-kernel/tests/refs.rs` | reference index |
| `vanilla_flag_names_count_the_metadata`, `vanilla_usage_matches_a_recount_of_the_sources`, `vanilla_flag_index_is_fast`, `vanilla_pool_counts_the_free_numbers`, `vanilla_own_name_compiles_byte_equal` | `crates/g4-kernel/tests/flags.rs` | flags and variables (3d.1): pret names (Platinum 4,097 flags / 307 variables, HGSS 2,244 / 386), usage index against a recount of the sources without the index (objects, hidden items, triggers, trainers, level scripts, `SetFlag`/`ClearFlag`/`CheckFlag`), timings (`G4_TIMING_STRICT=1`: warm ≤ 1,000 ms, 200 queries ≤ 50 ms, `usage` ≤ 50 ms), free pool (80–170 flags, 10–100 variables; Platinum DE was at 101, hence the lower bound 80), an own name compiles byte-equal and follows `rename_id`; prints `names …`, `usage …`, `index …`, `pool …`, `own …`. Adopts the scripts once per run on a copy under `target/tmp/flags-<folder>` (replaced on the next run); the writing test copies it once more into the temp directory. Needs the base ROM next to the folder (`suggest_base`, or `<folder>.nds` next to a test copy `<folder>`) |
| `vanilla_threads_count_at_least_the_measured`, `vanilla_thread_graph_of_the_first_thread_has_a_node_per_value` | `crates/g4-kernel/tests/threads.rs` | threads (3d.6): at least 59 (Platinum) or 62 (HGSS) variables with three values in two places, every thread with ≥ 3 values and ≥ 2 places; the graph of the first thread has one value node per value and only edges between existing nodes; timings (`G4_TIMING_STRICT=1`: `threads()` warm ≤ 100 ms, `thread_graph` ≤ 20 ms); prints `threads …` (count, sum of the values, highest number of values and places, cold with index build) and `graph …`. Adopts the scripts once per run on a copy under `target/tmp/threads-<folder>` (replaced on the next run). Needs the base ROM next to the folder like `tests/flags.rs` |
| `text_project_with_a_story_builds_from_base_byte_equal` | `crates/g4-kernel/tests/stories.rs` | stories (3d.6): a vanilla copy as a text project in Git with a story builds the base ROM byte-equal, register, story and side file are sources and in the Git index; prints `story-git …`. Works on `vanilla_copy()`, needs the base ROM next to the folder and `git` |
| `vanilla_item_icon_of_potion_decodes_to_a_non_empty_png`, `vanilla_every_item_with_an_icon_decodes`, `vanilla_mon_icon_of_bulbasaur_uses_palette_one`, `vanilla_every_species_and_form_icon_decodes` | `crates/g4-kernel/tests/icons.rs` | item and Pokémon icons (NCGR/NCLR) |
| `flags_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/flags.rs` | flags and variables (3d.1) over the command path on `vanilla_copy()` (scripts not adopted): `list_flags` with at least 4,097 (Platinum) or 2,244 (HGSS) entries, `flag_usage` of a visibility flag names event file and header, the free entries lie in the general range, `free_flag` refuses with `scripts_not_adopted` (`scripts_indexed=false`), `name_flag` in the DSPRE layout with `not_text_project`; prints `flags …` (timings `list_ms`, `usage_ms`, `cold_ms`, the number of variables, the pools without script references) |
| `vanilla_data_commands_on_copy` | `apps/studio/src-tauri/src/commands/data/tests.rs` | the data commands on `vanilla_copy()` |
| `vanilla_every_battle_sprite_decodes_with_the_measured_counts`, `vanilla_every_form_sprite_decodes` | `crates/g4-kernel/tests/battle_sprites.rs` | battle sprites (3d.3): all 1,812 Pokémon images, 157 / 161 form images and 105 / 129 trainer images can be read; the counts match, the mean of the connected areas is at most 3, at least 1,778 images have a transparent corner, every form image differs from form 0. Prints `sprites …`, `forms …`; release build, a few seconds per game (up to 17 s under load) |
| `battle_sprites_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/data/tests.rs` | `get_battle_sprite` and `get_trainer_sprite` over the command path on `vanilla_copy()`; with `G4_TIMING_STRICT=1` at most 50 ms per first request, **in a debug build** (without `--release`); prints `sprite … first_ms=… cached_ms=… trainer_ms=…` |
| `vanilla_egg_moves_read_with_the_measured_counts`, `vanilla_egg_moves_are_in_ascending_species_order` | `crates/g4-kernel/tests/egg_moves.rs` | egg moves (3d.3 part B): 198 species, in HeartGold 1,847 moves (at most 14 per species, member 4,092 bytes), in Platinum 1,711 moves in 1,910 words with the budget 1,910 (excerpt in `ov005.bin`); `write()` byte-equal to the member or to the excerpt. The second only prints whether the marks ascend. Prints `egg_moves <game code> source=… species=… moves=… words=… ms=…` and `egg_moves <game code> ascending=… first=… last=…`; release build, reading the table takes 0 to 11 ms |
| `text_project_egg_move_edit_builds_and_check_data_finds_it` | `crates/g4-kernel/tests/textproject_source.rs` | text project (3d.3 part B): egg moves 1 and 2 of species 1 swapped through the store and saved, only `data/species/0001.json` changes, the comparison finds nothing open, the build carries the swap (HeartGold through `g4k check-data egg_moves`, Platinum in the excerpt of `ov005.bin` of the unpacked ROM); needs the base ROM next to the project like the other tests of this file; prints `egg-moves-text <game code> count=<n> move0=<move>`; release build, several minutes (export of the whole project) |
| `vanilla_world_edges_match_the_measured_counts` | `crates/g4-kernel/tests/refs.rs` | world edges of the declarative index (3d.4, `metadata/entity_refs.yaml`): header → matrix / matrix → map / warp → header per game within ±1 % (Platinum 593 / 1,172 / 1,213, HeartGold DE 540 / 958 / 1,317), plus script → header (Platinum 67, HeartGold DE **114**; the assessment counted 154, but the first parameter of HGSS `MoveWarp` is a warp index); prints `<game code> <edge>: n / distinct` for the 13 edges and `script → header`/`script → warp index`. The growth is about 12,300 (Platinum) and 11,300 (HeartGold) references; `vanilla_index_builds_under_a_second` still holds (with `G4_TIMING_STRICT=1`; cannot be judged under load, measure on a quiet machine) |
| `textproject_round_trip_with_tombstone`, `textproject_export_idempotent_with_tombstone` | `crates/g4-kernel/tests/textproject.rs` | the tombstone (`deleted: true` on the last trainer) survives export, import and rebuild: the import ignores the envelope, the member comes back, the ROM builds byte-equal, the file keeps the envelope; the second export writes nothing (the envelope of the file on disk is kept) |
| `a_branch_without_the_freed_file_builds_after_git_switch` | `crates/g4-kernel/tests/git.rs` | gate 3d.4: a new trainer in the text project in Git, deleted and freed on the branch `ohne` ("without") and committed; `git switch` there and back: the build appends the members again or removes them by the trimming rule (`textproject.member-removed`), both builds work, `git status` stays empty. Needs `git` (identity through `GIT_AUTHOR_*`/`GIT_COMMITTER_*`) and the `.nds` next to the folder |
| `delete_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/delete.rs` | deleting (3d.4) over the command path on `vanilla_copy()` as a text project with a base ROM: a new trainer, `plan_delete`, `delete_entity` as one undo step, undo, then "free the number" (the JSON goes away); prints `text project set up: …`, `delete over the command path: … ms` and the time of the trainer list with the tombstone flags |
| `stories_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/stories/tests.rs` | stories (3d.6) over the command path on `vanilla_copy()` as a text project with adopted scripts: `list_threads` with at least 59 (Platinum) or 62 (HGSS) threads, `thread_graph` of the first with one value node per value, `story_create` changes exactly `data/ids/stories.json`, `data/stories/0000.json` and `0000.layout.json` among all sources, the thread names the story, `undo` restores all sources; prints `stories <game code> threads_ms=… graph_ms=… create_ms=… threads=… first=… values=… edges=…`. Needs the base ROM next to the folder |
| `vanilla_bdhc_round_trip_byte_identical_and_valid` | `crates/g4-formats/tests/vanilla_bdhc.rs` | BDHC (6.2): every vanilla BDHC is read, written back byte-equal and passes the structure check (`Bdhc::validate` 0 errors); Platinum 666 maps, 0 warnings, HGSS 676 maps, 1 warning (map 0464, vertical plate); prints payload min / median / p90 / max with map (Platinum 21,452 B on 188, HGSS 10,778 B on 658) |
| `vanilla_map_100_floor_is_two_tiles_high`, `vanilla_heights_cover_the_walkable_tiles`, `vanilla_free_steps_are_counted`, `set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen` | `crates/g4-kernel/tests/terrain.rs` (feature `test-support`) | terrain (6.2): Platinum map 100 has the floor at 32 units (2 tiles) and an edge at 96 units; heights per tile centre over all maps (share of free tiles without height printed); free steps counted over all maps and printed (Platinum 152, HGSS 52; the fixed target is held by `vanilla_checks_match_the_fixed_counts` for `map.step-unblocked`); `set_map_terrain` with the scenario grid on `vanilla_copy()` survives saving and reopening |
| `terrain_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/terrain.rs` | `get_map_heights`, `get_map_budget` and `set_map_terrain` (6.2) over the command path on `vanilla_copy()`: Platinum map 100 most frequent height 32 units, map 188 `bdhc.used` 21,452 quiet, HGSS map 658 10,778 quiet; setting and undo restore the bytes; prints `terrain <game code> …` with budget and VRAM estimate of a few maps |

The gated tests find the base ROM as `<name>.nds` next to `<name>_DSPRE_contents`; the tests of 3d.1 (`tests/flags.rs`) and 3d.6 (`tests/threads.rs`, `tests/stories.rs`, `stories_over_the_command_path_on_the_vanilla_copy`) also take `<folder>.nds` next to a folder without the suffix (the test copies of a phase, for example `_testkopien/3d-6-pt` and `3d-6-pt.nds`). The other tests with a base ROM (for example `delete_over_the_command_path_on_the_vanilla_copy`) report `skipped: …` on such a folder and count as passed; for them, `G4_TEST_PROJECT` points with an absolute path (cargo starts the tests of `-p studio` in `apps/studio/src-tauri`) to a symlink `<name>_DSPRE_contents` next to a symlink `<name>.nds` (in 3d.6 under `target/agent/t10-vanilla/`).

For example only the round trips and the oracle, per project:

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_fixed_records vanilla_variable_records vanilla_trainer_parties \
  vanilla_personal_matches_uxie vanilla_items_match_uxie vanilla_trainer_properties_match_uxie vanilla_pt_water_slots --nocapture
```

#### Trainers and encounters (3b.3b)

Since 3b.3b, trainers are a composite asset (properties and party in two files, name and battle texts in
text archives), encounters have rules in the kernel (`data/encounter_rules.rs`), and `create_trainer` extends the
battle script file ([docs/formats/trainer_scripts.md](../formats/trainer_scripts.md), German with an English lead). The gated tests for this:

| Test | File | Checks |
|---|---|---|
| `vanilla_trainers_decode_and_encode_identically` | `crates/g4-formats/tests/vanilla_data.rs` | every trainer (Platinum 928, HGSS 738 pairs) is read as one document and both files are written back byte-equal |
| `vanilla_trainers_load_unlocked_through_the_store` | `crates/g4-kernel/tests/store.rs` | every vanilla trainer loads through the store as one asset, unlocked and byte-equal |
| `vanilla_encounter_files_raise_no_warnings` | `crates/g4-kernel/src/data/encounter_rules.rs` (unit test, gated) | no vanilla encounter file breaks a rule; so every warning in the editor points to a change |
| `vanilla_creates_trainers_behind_the_sight_slot` | `crates/g4-kernel/tests/trainer_create.rs` | on `vanilla_copy()` with adopted scripts: Platinum 928, placeholder 929, 930; HGSS 738, 739, placeholder 740, 741; the battle script file compiles cleanly, a newly opened store shows the placeholder locked (adopts the scripts of the copy for this) |

Each project on its own:

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_trainers_decode vanilla_trainers_load vanilla_encounter_files_raise \
  vanilla_creates_trainers_behind --nocapture
```

#### proptest

`proptest` 1 is a dev dependency of `g4-formats` and `g4-kernel`; the tests run without a project in the normal
`cargo test --workspace --all-features`:

- `g4-formats/src/data/`: round trip over random bytes per codec (personal, evolution, move, item, encounter,
  trainer text table), learnsets from arbitrary terminated bytes there and back (a changed length always yields a
  terminated multiple of 4), `read(write(d)) == d` for the party in every format, setter isolation in the record core
  (`setter_changes_only_its_own_bytes`, `set_bits_changes_only_the_masked_bits`: a setter changes only its own bytes or bits).
- `g4-kernel/tests/transaction.rs`: `undo_redo_are_inverse_over_mixed_kinds` (64 random sequences of edits, undo and redo over
  mixed kinds).
- `g4-kernel/tests/refs.rs`: `incremental_equals_full_rebuild` (48 sequences: the incremental reference index equals the one
  built in full).
- `g4-kernel/tests/names.rs`: `trainer_name_wrap_unwrap_round_trip`.

The two kernel tests with their own `TestRunner` do not write `proptest-regressions` files; a failure appears only in
the output (with the shrunk case).

#### Checking data in the built ROM: `g4k check-data`

`g4k check-data <rom.nds> --expect KIND:MEMBER:FIELD=VALUE …` reads the data NARCs from a built ROM (paths from
`archives.yaml` for the game code in the ROM header) and compares individual fields:

- KIND `species` and `item` are read by uxie (independent of our own codecs), `move`, `learnset` and `encounter` by our own
  codecs (uxie rejects moves, its encounter reader is unusable, see above; for `encounter` this deviates from the
  specification, which says "via uxie"). The field names are those of the codecs (`attack`, `power`, `price` …); a learnset has
  only `len` (number of entries); an encounter file has `grass_rate`, `grassN` (species in grass slot N, 0 to 11; HGSS: the
  tag column), `grassN_level` and in HGSS `grassN_morning`, `grassN_night`.
- KIND `map` (since 3b.4b) reads a map file of the ROM with `g4_formats::map` and has two fields: `model_sha1` (SHA-1 of the model as it stands in the map) and `per_sha1` (SHA-1 of the collision, the same 2048 bytes as an exported `.per`), both as 40 hex digits. This is how a scenario checks that an import arrived in the built ROM: `--expect map:666:model_sha1=<SHA-1 of the exported .nsbmd>`.
- KIND `egg_moves` (since 3d.3) reads the one member of the egg move table (HeartGold `a/2/2/9`, Platinum only with the
  patch NARC of DSPRE; it does not read the table in overlay 5, in which case "CPUD has no egg_moves archive"); MEMBER here is
  the species number, the fields are `count` (number of egg moves, 0 without an entry) and `move0` to `move15`:
  `--expect egg_moves:1:count=14 --expect egg_moves:1:move0=113` (Bulbasaur in HeartGold DE).
- MEMBER is the member in the archive, so for `item` the data member, not the item ID (in Platinum, Potion is both 17).
- Exit 0 if everything matches (stdout "species 397 attack: 85" per expectation and "N expectation(s) met"); exit 1 on a
  deviation or an unreadable member (stderr "species 397 attack: expected 75, found 85"); exit 2 on a misspelled
  `--expect`, before the ROM is read.

```sh
cargo build --workspace --all-features --bin g4k
target/debug/g4k check-data <copy>/out.nds --expect species:397:attack=85 --expect learnset:397:len=13
target/debug/g4k check-data <copy>/out.nds --expect encounter:140:grass0=150 --expect encounter:183:grass_rate=100
```

In a worktree, build the whole workspace instead of `--bin g4k` (`cargo build --workspace --all-features`), otherwise
rotom's build script runs again (see above).

Scenarios call it in the `check:` section (example `scripts/agent/scenarios/pt-daten.yaml`, description in
[docs/agent-gate.md](../agent-gate.md), "Szenario"; German).

#### Project checks (3b.4a)

Since 3b.4a, the module `crates/g4-kernel/src/checks/` checks the project with the catalog of specification 3b.4 §4 and the two
place checks from 3f.2 (41 codes, `Registry::all`; limits and exceptions with their source in `crates/g4-kernel/metadata/check_limits.yaml`, described in
[docs/formats/check_limits.md](../formats/check_limits.md)). Hard rule: a check is an "error" only if it has 0 hits on
Platinum DE, HeartGold DE and HeartGold US each; the gated tests prove this. The gated tests for this:

| Test | File | Checks |
|---|---|---|
| `vanilla_checks_match_the_fixed_counts` | `crates/g4-kernel/tests/vanilla_checks.rs` | all groups on the vanilla project (read only, scripts not adopted): the count per code equals the fixed target in `SOLL` (columns Platinum DE, HeartGold DE, HeartGold US), every check with severity error has 0 findings; prints target, actual and time per code |
| `vanilla_script_checks_after_adopt` | same (feature `test-support`) | on `vanilla_copy()` before and after `scripts::adopt`: the slot checks find the same from the sources as from the members, `script.message-missing` runs and leaves out only the common scripts without a header (Platinum 28, HeartGold 25 files) |
| `overlay_under_lock_takes_under_5_ms_on_hgss` | `crates/g4-kernel/tests/checks_runner.rs` | on `vanilla_copy()` with all event files and text archives loaded and a few changed: the copy under the store lock (`snapshot_overlay`) stays under 5 ms |
| `full_run_on_vanilla_under_three_seconds_warm` | same | a full run over all groups, twice: the second one, with the cache of the first, under 3 s; prints both times |

Each project on its own (the timings with `--nocapture`; `--test-threads 1` so that the time measurements do not run against each other):

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_checks_match_the_fixed_counts vanilla_script_checks_after_adopt \
  full_run_on_vanilla overlay_under_lock --nocapture --test-threads 1
```

Measured on 2026-10-25 (dev build, per project Platinum DE / HeartGold DE / HeartGold US): 111 / 145 / 153 findings, all
error codes 0; all groups on a new snapshot 1047 / 1083 / 1004 ms, the checks alone 158 / 164 / 135 ms;
full run cold 3.46 / 1.18 / 3.60 s (the HeartGold DE value with a warm file cache of the system; in task 6 cold 3.8 s),
warm 287 / 280 / 247 ms; the copy under the lock 0.8 to 3.1 ms. The four runs take 90 to 100 s per project.

**When checks run.** The UI (`apps/studio/src/checks/useChecks.ts`) starts a full run over all groups
(`run_checks { scope: "all" }`) when a project is opened, after a fix, on "Check again" and 300 ms after the last
change (bus topic `edited`, which `useProjectSession.refreshDirty` sends after every write and `useScripts` after
saving a script or level script, "add script" and adopting; the first read after opening and the
read after a fix do not send it, so that exactly one run happens there); the same in Tauri and in browser mode, there is
no timer in the backend. Until a report is there, the check light, panel and Technical never say "All good" or "No
problems": during the first run "Checking …", without a report and without a run (the first run failed or came without a
report) "Not checked" with "Check again" (`data-light="unchecked"`). Only one run goes at a time; requests in the meantime are merged into
one further run, and while it runs the UI asks `checks_status` for progress every 250 ms.
The full run after every change is fast when warm because the snapshot caches the text and script results per file
(key: text generation and file stamp or file stamp of the source, that is length and modification time; on a
file system with coarse modification times such as HFS+ or exFAT, a change of the same length within the same second would go unnoticed,
APFS has nanoseconds): warm it costs the cheap part plus the changed files, measured
261 ms on HeartGold DE (task 6) and 250 to 290 ms on all three projects (above); cold, on opening, 1.2 to 3.8 s with
progress in the check light ("Checking … texts 312/724"). Every report replaces the findings as a whole; a report without
groups (`groups: []`), with only some groups or from an older store generation replaces nothing. If the store changes
during a run, the backend discards the result and checks again. The same applies to writes that bypass the store
and that the checks read from disk: saving a script or level script, `add_script`, `new_script_file`,
`adopt_scripts`, `apply_fix create_script` and `create_trainer` count up an epoch of the disk
(`ChecksState::disk_written` through `Session::files_written`, `apps/studio/src-tauri/src/state.rs`); a report is valid only
as long as store generation and epoch are those of its run (`finish_checks`, `current_report`), otherwise
`checks_status` returns none and `run_checks` checks again. The scope `scope: "cheap"` (only the cheap
group) remains for the bridge and scenarios; the UI does not use it. In the bridge `run_checks` is synchronous;
a fresh session (scenario batch) checks cold, on Platinum DE with adopted scripts 1.5 to 1.6 s.

**`g4-checks.json`.** Hidden findings and disabled checks are stored in the project in `<project>/g4-checks.json`
(not in `.g4/`, which is a deletable cache): `{"version": 1, "ignored": [{"code", "subject": {"kind", "index"}, "key",
"fingerprint"}], "disabled": [code]}`, the fingerprint as 16 hex digits. Since 4.3b, `version` (1) is always in the file;
a file without the field (3b.4 to 4.3a) counts as version 1, a higher one is refused by Studio ("g4-checks.json: version 2
comes from a newer version of Gen4 Studio."). `subject.index` is an ID or a
number for `header` and `trainer` (`"MAP_T01"` or `3`; the ID is what gets written in the text project), a number for all other kinds; format in
[docs/formats/text_project.md](../formats/text_project.md), "`g4-checks.json`". `ignore_finding`, `unignore_finding`, `accept_baseline` and
`set_check_enabled` write the file immediately (atomically); `open_project` reads it. A broken file (edited by hand,
a merge conflict, from a newer Studio with an unknown subject kind) does not lock the project (ruling R15): it opens
without hidden findings, every report names the file as a `skipped` row with `code` "g4-checks.json" and the
parse error (in the panel "g4-checks.json is not readable (…): nothing is hidden or disabled …"), and the four
commands refuse every write with "g4-checks.json is not readable (…). Repair or delete the file …" so that it is
never overwritten unread. Before every write attempt Studio reads it again: if it has been repaired or deleted, it applies
from then on. `g4k check` keeps refusing a broken file with "g4-checks.json: …". Errors can never be hidden or
disabled. The
fingerprint covers the checked bytes of the subject: if the place changes, it no longer matches and the finding
reports again; unchanged ones stay hidden.

**`g4k check`.** `g4k check <project> [--json] [--expect CODE=N …]` runs all checks on the saved project
and, like Studio, reads `<project>/g4-checks.json`: hidden findings count as "ignored", disabled
checks do not run (errors are never left out). Without `--json` one line per finding and a total, with `--json`
the whole report (`findings`, `ignored`, `skipped` as `{code, why}`, subjects as `{kind, index}`, fingerprints as
16 hex digits). `--expect warp.dest-warp=1` compares the number of non-hidden findings per code: exit 1 with
"warp.dest-warp: expected 1, found 0" on stderr; an unknown code ends before the run with exit 2. Findings alone
do not change the exit code. Example after the scenario `pt-pruefungen.yaml`:

```sh
target/debug/g4k check <copy> --expect warp.dest-warp=0 --expect event.script-missing=0 \
  --expect event.trainer-missing=0 --expect text.too-wide=0
# 0 finding(s): 0 error(s), 0 warning(s), 0 info(s); 111 ignored; 1553 ms … 4 expectation(s) met
```

Scenarios check the answers of the bridge directly with expectations on the changes (`{expect: {path, equals}}`,
description in [docs/agent-gate.md](../agent-gate.md), "Szenario"; German).

Measurement run, not part of the normal pass (`#[ignore]`): `G4_APICULA=... cargo test -p studio --release -- --ignored
--nocapture bench_matrix_models` loads a large matrix (default: header 3, Platinum matrix 0) and prints the timings;
`G4_BENCH_HEADER` selects a different header.

#### Importing map parts (3b.4b)

Since 3b.4b, `crates/g4-kernel/src/map_import.rs` imports finished binary parts into existing maps (`inspect`, `import`, `export`; model, collision, buildings, BDHC, BGS, whole map file, "take parts from map N"), through `Store::edit_map` as one step in the history; the NSBMD header is read by `crates/g4-formats/src/nsbmd_header.rs` ([docs/formats/nsbmd_header.md](../formats/nsbmd_header.md)). All checks run before writing, a refusal changes nothing (`import_refuses_on_error_and_changes_nothing` in the kernel, `export_then_import_from_files_and_a_refusal_changes_nothing` in Studio).

**Error codes.** Every error message, warning and note of a part check carries a code from `PartCode` (`map_import.rs`, `PartCode::ALL`, `as_str`, for example `model.too-big`, `permissions.size`, `whole.foreign`) with arguments and an English sentence (`PartMessage { code, args, message }`; on the wire `state: {kind, code, args, message}` and `notes`). The UI composes the user-facing sentence with a way out from code and arguments: `apps/studio/src/workbench/inspector/importParts.ts` (`PART_TEXTS`, `partText`, `partView`). Two tests hold this together: `every_error_and_warning_of_a_part_carries_its_code` (kernel: every path delivers its code, the codes reached and the named unreachable ones add up to `PartCode::ALL`) and `commands::map_import::tests::every_part_code_has_its_german_sentence` (Studio: the keys of `PART_TEXTS` are `PartCode::ALL` in the same order). A new code therefore needs both places. The refusal of `import_map_parts` stays the English sentence ("import refused: model: model is 65536 bytes; the game holds at most 61440 (0xF000)").

**Bridge paths.** `Roots::read_path` admits import files only under the allowed folders (`--allow-root`, for `studio-web.sh` `$G4_ROMS_DIR/_testkopien`) and under `G4_IMPORT_ROOT` (folders separated by `:`), never from an original or `_saves`; `Roots::write_path` treats an export target like a ROM target (`apps/studio/src-tauri/src/bridge/roots.rs`).

The gated tests for this (`crates/g4-kernel/tests/map_import.rs`, on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `every_vanilla_model_reimported_into_itself_is_byte_identical` | every map model of the project, exported and imported into its own map, gives the same bytes |
| `map_a_into_b_and_back_restores_b` | all parts of map 5 into map 6 ("take parts from map N") as one step, then back from the previously exported files: map 6 has its bytes again, also after undoing twice |
| `hgss_bin_into_pt_and_back` | a whole map file of the other family into map 6: Platinum leaves out the background sound with `whole.bgs-dropped`, HeartGold/SoulSilver creates an empty one, foreign buildings stay out; back through the exported `.bin` |
| `per_round_trip` | `.per` of map 5 into map 6 and back |

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- every_vanilla_model_reimported map_a_into_b hgss_bin_into_pt per_round_trip --nocapture
```

**Scenarios.** Since 3b.4b, `g4emu scenario` can do: `${sha1-of:PATH}` (SHA-1 of a file of the copy, read only when substituted, for example in `check:` after building), `{expect: {error: substring}}` (the bridge must refuse the command, the scenario carries on), a command without arguments with an expectation (`[dirty, {}, {expect: …}]`) and the section `prepare:` with the step `make-oversize-nsbmd {path, bytes?}` (a synthetic model over 0xF000 bytes for the counter case; `scripts/g4emu/g4emu/fixtures.py`, `make_oversize_nsbmd`; there is no command of its own `g4emu make-oversize-nsbmd`). Description in [docs/agent-gate.md](../agent-gate.md), "Szenario" (German). The overall gate scenarios `pt-kleiner-hack.yaml` and `hg-kleiner-hack.yaml` each take under a minute (measured 47 to 49 s, 23 bridge sessions) plus building the tools; the counter-tests `pt-kleiner-hack-ohne-import.yaml` and `hg-kleiner-hack-ohne-import.yaml` belong to them and must fail on the way. Recipe "Gesamtgate" (overall gate) in [docs/agent-gate.md](../agent-gate.md).

#### Creating a project from a ROM (3g.1)

`crates/g4-kernel/src/start.rs` creates a project from an `.nds` file; layout of the mark and of the CRC restoration in [docs/formats/project_base.md](../formats/project_base.md).

```sh
cargo build --workspace --all-features
target/debug/g4k inspect-rom <rom.nds>            # JSON: title, game code, family, language, SHA-1, known_base, support
target/debug/g4k create <rom.nds> <new-folder>    # [--template original|data-only|empty-region]; prints root=… and the phases with ms
```

`inspect-rom` only reads. `create` unpacks into `<parent>/.<name>.g4-new`, writes `.g4/base.json` and only then renames to `<new-folder>`; the target must be missing or empty. It takes the name literally: a folder ending in `_DSPRE_contents` remains a name like any other (the bridge blocks such names, `g4k` does not). It refuses Diamond/Pearl ("Diamond/Pearl: not released without a test project"), as well as a file without a Gen 4 game code. Success-path tests without a ROM use a fake extractor (`create_project_with` in `start.rs`), because ds-rom cannot unpack the synthetic header ROM (`test_support::synthetic_rom`).

The gated test `project_start_create_then_build_matches_the_rom` (`G4_TEST_ROM`) creates the project in a temp folder, builds it and compares the SHA-1 of the built ROM with that of the input (without `restore_header_crcs`, `0x6C` and `0x15E` differ):

```sh
G4_TEST_ROM="$PWD/../Roms/_testkopien/3g-probe/pt.nds" \
  cargo test --workspace --all-features -- project_start_ --nocapture
```

In the gate script this runs as the step "gated tests (project start)", once per `*.nds` directly under `$G4_ROMS_DIR` (only read); without an `.nds` the line "skipped (keine ROM)" ("no ROM") appears. The filter `project_start_` selects only these tests; per ROM the step takes about 15 to 25 s plus building the tests.

#### Empty region with a start town (3g.2)

`crates/g4-kernel/src/templates.rs` plans and applies the world template "Empty region with a start town" (recipe `metadata/templates.yaml`, start scripts `metadata/templates/start_pt.rotom` and `start_hgss.rotom`); procedure, preconditions and start script in [docs/formats/templates.md](../formats/templates.md). The kernel writes the edge map, level script and start script immediately (not undoable), everything else is one step in the history ("Template: Empty region with a start town").

```sh
target/debug/g4k template <project> empty-region   # adopts the scripts if needed; applies, saves, prints the report
target/debug/g4k create <rom.nds> <new-folder> --template empty-region
```

`g4k template` writes into the project; apply it only to copies under `_testkopien`. The gated tests (`crates/g4-kernel/tests/templates.rs`, on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `vanilla_plan_empty_region` | the plan: start town, kept rooms, edge cells, empty level script |
| `vanilla_empty_region_has_no_check_errors` | adopt scripts, apply, save: all three check groups without errors, no `text.too-wide` in the text archive of the town, a new plan sees `already_empty` |
| `vanilla_empty_region_undo_restores_the_world` | one ⌘Z restores header table, matrix 0 and the event file of the town byte-equal, since 3d.2 also the start script, the level script file, the touched text archives and `compile-state.json` (the project is in Git in the test: after ⌘Z `git status` names only the edge map); ⌘⇧Z brings the result back |
| `vanilla_empty_region_builds` | the saved result builds without errors |

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/_testkopien/3g-b-pt" \
  cargo test --workspace --all-features --test templates -- vanilla_ --nocapture --test-threads 1
G4_TEST_ROM="/<absolute path to Roms>/_testkopien/3g-probe/pt.nds" \
  cargo test --workspace --all-features -- project_start_ --nocapture
```

`project_start_empty_region_from_the_rom` (`G4_TEST_ROM`) creates a project with `Template::EmptyRegion` and demands the mark `empty_region`, progress up to `done = total` and 0 errors in all check groups. Measured: the four tests in `templates.rs` take about 50 s on Platinum DE and about 40 s on HeartGold DE (three times adopting scripts); the template phase of `create_project` 4 to 7 s.

#### Base ROM and patch (4.1a)

`crates/g4-kernel/src/start.rs` checks a base ROM and assigns it (mark v2 with `origin: "dspre"`), `base.rs` looks it up by SHA-1 and reads from it, `patch.rs` writes the xdelta patch (xdelta3 3.0.12 in the crate `crates/g4-xdelta`); formats in [docs/formats/project_base.md](../formats/project_base.md) and [docs/formats/xdelta_patch.md](../formats/xdelta_patch.md).

```sh
target/debug/g4k base check <project>                    # mark=…, sha1=…, suggestion=… (without a mark) or base=… (origin, for example sibling)
target/debug/g4k base check <project> --rom <rom.nds>   # checks the .nds as a base ROM, one line warning: per warning
target/debug/g4k base assign <project> <rom.nds>        # writes .g4/base.json; wrong game: exit 1
target/debug/g4k patch create <project> [--base-rom <rom.nds>] [--out <patch>] [--json]
target/debug/g4k publish <project> --version <v> [--notes <text> | --notes-file <path>] [--dir <folder>] [--file <name>] [--readme] [--overwrite] [--base-rom <rom.nds>] [--json]
```

`patch create` builds to `build/<name>.nds` and writes `build/<name>.xdelta` with `<name>.xdelta.txt` (`<name>` = `<project>-<game code>`); the base ROM comes from `--base-rom`, otherwise the neighbouring `.nds`, otherwise `G4_BASE_ROM_DIR`. Without a mark it ends with exit 1 and "No base ROM is assigned to this project. Assign one with `g4k base assign <project> <nds>`."; an unreadable or newer mark it refuses with the same sentences as the app and MCP. `publish` (4.5) builds in the same way, writes the release `<project>-v<version>.xdelta` with a sidecar file and optionally `README.txt` to `--dir` (default `build/`) and records it in `.g4/releases.json`; output and refusals in [docs/formats/xdelta_patch.md](../formats/xdelta_patch.md#veröffentlichen). `base assign`, `patch create` and `publish` write into the project: only on copies under `_testkopien`.

The gated tests in `crates/g4-kernel/tests/base.rs` (`G4_TEST_PROJECT`; the base ROM is the `.nds` next to the original, found through `suggest_base` and only read; whatever writes works on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds` | assigning without a warning; the build of the copy with a mark has the SHA-1 of the `.nds` |
| `base_rom_members_equal_the_vanilla_folders` | the archives from the ROM equal the folders of the project |
| `base_rom_header_table_equals_the_vanilla_project` | the header table from the ARM9 of the ROM equals that of the project |
| `patch_after_map_and_after_map_script_text_is_small_and_applies` | patch after a map change ≤ 1 KiB, after map, script and text in one place ≤ 16 KiB, creating ≤ 1 s, applied with our own decoder and (if in `PATH`) with `xdelta3 -d` equal; text archive 0 as a stress line without a limit |
| `publish_after_a_map_edit_writes_a_small_patch_that_applies` (4.5) | `build_and_publish` after one map byte (`0.1.0`, with `README.txt`): phases `build`, `encode`, `verify`, `write`, `record` in this order, patch ≤ 1 KiB, applied with our own decoder and (if in `PATH`) with `xdelta3 -d` equal to the built ROM, sidecar file with `Version:` and `Änderungen:` (the sidecar text is German), one entry in `.g4/releases.json`; `0.1.1` after that gives two entries; prints `publish <game code> size=… build_ms=… encode_ms=… verify_ms=…` |

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test base -- --nocapture --test-threads 1
```

That is how it ran in the 4.1a gate; in a worktree, where `-p` and `--test` trigger the download of rotom (see above), the same with `cargo test --workspace --all-features --test base -- …`. On stderr appear `rebuild <game code> sha1=… equal`, `patch-sizes <game code> map=… all=… encode_ms=…/… verify_ms=…/…`, `patch-stress …`, `xdelta3 cross-check: map=… all=… stress=…` and (4.5) `publish <game code> size=… build_ms=… encode_ms=… verify_ms=…`. Measured on 2026-10-27 (debug build): 132 s (Platinum DE), 232 s (HeartGold DE), 139 s (HeartGold US) for the four tests; values in [docs/phase-4-1a-gate.md](../phase-4-1a-gate.md) (German). In `scripts/gate.sh` they run along in the step of the gated tests per project.

#### Reset to original (4.1b)

`crates/g4-kernel/src/original.rs` resets an entity to the state of the base ROM (`plan_reset`, `reset`; store kinds through `Tx::replace`, headers field by field, scripts, textures and models as a file step), `original/changed.rs` lists what deviates from the original (xxh3 per base member against the fingerprints from `.g4/build/state.json`), `original/base_hashes.rs` keeps the base hashes in the app cache; format in [docs/formats/project_base.md](../formats/project_base.md), section "Auf Original zurücksetzen" ("Reset to original"), tools in [docs/en/mcp.md](mcp.md). There is no `g4k` subcommand; headless it works through `target/debug/studio mcp --project <copy> --base-rom <nds>` and the tools `changed_from_original`, `plan_reset`, `reset_to_original`.

The gated tests in `crates/g4-kernel/tests/original.rs` (`G4_TEST_PROJECT`; the base ROM is the `.nds` next to the project, found through `test_support::vanilla_base_rom()` and only read; all of them write to `vanilla_copy()`):

| Test | Checks |
|---|---|
| `reset_of_each_store_kind_gives_the_base_member_back` | map, event file, whole text archive, message, trainer, Pokémon (personal, learnset, evolution), encounter file: change, save, reset, save; every member byte-equal to the base member, every reset ≤ 200 ms (from opening the base ROM to the recorded step), ⌘Z restores the change |
| `header_reset_field_by_field_gives_the_base_record_back` | header first one field, then the rest, also with the dynamic-headers patch (base in the ARM9, project in the NARC): the saved record equals the base, ⌘Z to the half state |
| `script_texture_and_model_reset_give_the_base_member_back` | after `scripts::adopt` script file, map texture and building model: member byte-equal, ⌘Z writes the old bytes back |
| `changed_list_after_edits_names_them_and_after_reset_not` | after a build the list is empty (second run ≤ 2 s, hashes only racy entries), after changes to a map and a trainer it names exactly these, after resetting the map only the trainer |
| `decompile_member_gives_the_source_rotom_writes_for_every_script_file` | `decompile_member` gives the same text as rotom's own decompiling for every script file with global calls |

Also the gated Studio test `reset_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/original.rs`): list, plan and reset over the command path of the app; `ms` includes finding and opening the base ROM.

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test original -- --nocapture --test-threads 1
G4_TEST_PROJECT="/<absolute path to Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p studio --all-features --lib reset_over_the_command_path -- --nocapture --test-threads 1
```

That is how it ran in the 4.1b gate, on fresh copies with the `.nds` next to them (`4-1b-pt.nds` and so on); in a worktree, where `-p` and `--test` trigger the download of rotom (see above), through `cargo test --workspace --all-features` with a name filter. On stderr appear `reset <game code> <target> ms=… (kernel … ms)`, `changed <game code> first_ms=… second_ms=… files=… recorded=… racy=…` and `decompile <game code> <n> files with global calls ms=…`. Measured on 2026-10-27 (debug build): 205 s (Platinum DE), 210 s (HeartGold DE, red first run; afterwards `changed_list…` alone green), 236 s (HeartGold US) for the five tests; every reset 9 to 108 ms, the list first 10 to 17 s, afterwards 0.1 to 0.6 s; values in [docs/phase-4-1b-gate.md](../phase-4-1b-gate.md) (German). In `scripts/gate.sh` they run along in the step of the gated tests per project.

#### World model (3e.2)

`apps/studio/src/workbench/worldModel.ts` (`buildWorld`) builds the world tree of the Workbench from headers, matrices, warps and the world file of the project (world maps, areas, rooms). `worldModel.vanilla.test.ts` checks the model against the three vanilla projects. The test reads no ROMs but JSON dumps that `scripts/agent/world-dump.sh` writes through the bridge; without `G4_WORLD_DUMP` it reports a skipped test with a hint.

The dumps (one folder per project: `project.json`, `headers.json`, `matrices.json`, `matrix-<n>.json` for every matrix with a section, `warps.json`, `meta.json`) are made from fresh copies under `_testkopien`, with the bridge running (`scripts/agent/studio-web.sh`; `call.sh` reads port and token from `target/agent/`):

```sh
for p in 3e-pt 3e-hgd 3e-hgu; do
  scripts/agent/world-dump.sh "/<absolute path to Roms>/_testkopien/$p" "$PWD/target/agent/world-dump/$p"
done
G4_WORLD_DUMP="$PWD/target/agent/world-dump" \
  pnpm --dir apps/studio exec vitest run src/workbench/worldModel.vanilla.test.ts
```

The path in `G4_WORLD_DUMP` must be absolute; the folders are named `3e-pt` (Platinum DE, `CPUD`), `3e-hgd` (HeartGold DE, `IPKD`) and `3e-hgu` (HeartGold US, `IPKE`). Per project the test checks: the world maps (Platinum exactly "Sinnoh", HeartGold/SoulSilver "Johto" and "Kanto"), at least 90 % of the rooms assigned to an area, every header exactly once in the tree, `buildWorld` under 200 ms; in addition, for Platinum, the Great Marsh and the Battle Zone under "More areas" and all rooms of Mt. Coronet under one area (at least one under a different place, its entrance), for HeartGold/SoulSilver the Safari Zone under "More areas". Measured (bridge, fresh copies): Platinum 482 of 492 rooms assigned, HeartGold/SoulSilver 427 of 448 each, `buildWorld` under 5 ms in every case.

#### Build graph (4.2)

The build runs only the steps whose inputs have changed (`rotom`, `chatot`, per archive `pack:<folder>`, `watch:<folder>`, `rom`; the ROM step yields `blz:arm9`, `blz:overlays`, `rom`, `crc`). It keeps its state under `<project>/.g4/build/`: `state.json` (the fingerprints of the inputs and outputs per step and the text reconciliation JSON against member, format in [docs/formats/build_state.md](../formats/build_state.md)) and `blz/` (the compressed ARM9 and overlay files). Without the folder the build builds everything in full and byte-equal. The text reconciliation (`texts` in `state.json`), however, is not a cache but decides whether the JSON or the member of a text archive applies: "Rebuild (no cache)" (`--no-cache`, `fresh: true`) therefore deletes everything under `.g4/build/` except it and rewrites `state.json` with `texts` only. All steps then run in full, and a member that DSPRE has changed since the last build keeps applying. Only with no state at all (fresh clone, deleted folder) does the JSON apply for every archive, and the build reports this as the note "N text archives without build state: the JSON version applies."

```sh
cargo build --workspace --all-features
target/debug/g4k build <copy> --out <copy>/out.nds --timings    # one JSON line: {"ms", "rom", "steps": [{id, status, ms, done, total, cached, reason}]}
target/debug/g4k build <copy> --out <copy>/out.nds --no-cache   # deletes .g4/build/ first (except the text reconciliation), all steps run in full
```

`--timings` writes exactly one JSON line to stdout instead of the text lines (problems still go to stderr); `status` is `skipped`, `incremental` or `full`, and `reason` names the cause of a full run (`no-state`, `build.cache-stale` after a tool change, `output-changed`, `removed`; for `crc` `no-base-mark`). Without `--timings`, the line `packed` names the kinds whose pack step ran (skipped ones are missing). In the app, `--no-cache` corresponds to the command `build_rom` with `fresh: true`. A text conflict (JSON and member both changed) makes `g4k build` abort with the message "text archives changed as JSON and as member since the last build: …"; in the app and through MCP this is the error `text_conflict`, resolved with `resolve_text_conflicts`. `g4k` itself cannot resolve a text conflict; without the app it works through `studio mcp --project <copy> --allow-irreversible` and the tool `resolve_text_conflicts` (irreversible).

The gated test `build_timings_prints_one_json_line` (`crates/g4-kernel/tests/cli.rs`, on `vanilla_copy()`) builds the copy twice with `--timings`: the output is one JSON line each with the step `rom`, which runs in full the first time and is skipped the second time. `build_no_cache_removes_the_state_first` checks `--no-cache` without ROM data.

The gated tests in `crates/g4-kernel/tests/build_graph.rs` (all on `vanilla_copy()`) check that the incremental build yields the same bytes as a build without a cache to a different path:

- `incremental_equals_full_after_nothing_changed`, `a_new_out_path_runs_the_rom_step`, `map_edit_incremental_equals_full` (map through the store), `text_edit_incremental_equals_full` (text through the store: `chatot` checks only this archive and finds the member up to date (the store wrote it when saving), `pack:textArchives` inserts one member), `script_edit_incremental_equals_full` (after `scripts::adopt` one more script through `write_source`: `rotom` translates exactly this file);
- `simulated_dspre_edit_is_detected`: a map member overwritten in place with the same size, the mtime reset (same size, mtime and inode, only the ctime changes); `pack:maps` runs incrementally, the ROM equals the full build;
- `salt_change_builds_fully`: the salts of `pack:maps` and `rom` replaced in `state.json`; both run in full with `build.cache-stale`, the other steps do not, the ROM stays the same;
- `dspre_mark_build_matches_the_base`: runs only if the copy has a base mark (after 4.1a `origin: "dspre"`) and compares the SHA-1 of the built ROM with the mark; otherwise "skipped: no base mark";
- `build_graph_timings`: the times of "Play here" (one output path). Per case one warm-up run and three runs whose median counts: `full` (without a cache), map, text and script changed, no change; plus the first build without a state as `cold` (without a limit). The line on stderr reads `build-timings: {"game":"CPUD","full_ms":…,"map_ms":…,"text_ms":…,"script_ms":…,"none_ms":…,"cold_ms":…}`. With `G4_TIMING_STRICT=1`, map, text and script must be ≤ 500 ms and no change ≤ 200 ms; otherwise the test only prints.

Also `project_start_incremental_build_matches_the_rom` (`G4_TEST_ROM`): a new project built three times to the same path (fresh, without a change, after a map that was changed and written back); every time the SHA-1 of the ROM is the one from `read_base_mark`.

The time limits apply to the app running in the release profile. In the debug profile the build takes three to four times as long (Platinum DE: map 1.3 s, no change 0.6 s), so the gate step measures with `--release`:

```sh
env G4_TEST_PROJECT="$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test build_graph -- build_graph_timings --nocapture --test-threads 1
```

Measured (release, 2026-10-27, median in ms, two runs each, load 3 to 9): Platinum DE map 244–247, text 273–290, script 306–314, no change 95–96, without cache 1543–1566; HeartGold DE 265–284, 281–289, 287–341, 95–103, 6336–6985; HeartGold US 260–274, 254–266, 290–306, 94–97, 6506–7191. Before task 9b these were map 284–349, text 300–404, script 454–508 and no change 149–183 ms. Three things create the distance to the limits (see [build_state.md](../formats/build_state.md)): `state.json` is held in the process as long as its `stat` stays the same (before, it was read seven to thirteen times per build, about 10 ms each), the ROM and the NARCs just written are recorded with the hash from memory, and rotom's translation takes the hashes of unchanged sources from memory (before, about 150 ms per build for reading all sources). The rest is carried by the ROM itself (ds-rom loads, builds and writes about 128 MB, roughly 110 to 130 ms) and the fingerprints of the roughly 40 steps. Under load the times scatter; the gate repeats a failed run once and writes "ok (2nd run)" into the table.

#### Text project (4.3a)

`crates/g4-kernel/src/textproject/` writes a project as a text project (manifest `g4-project.json`, one JSON per entity under `data/<kind>/NNNN.json`, registers `data/ids/`, blobs `data/raw/`, `data/rotom-quirks.json`) and reads it back; format in [docs/formats/text_project.md](../formats/text_project.md). Headless:

```sh
target/release/g4k text export <copy>    # needs a base mark and base ROM (--base-rom, otherwise the neighbouring .nds, otherwise G4_BASE_ROM_DIR); adopts scripts not yet adopted; refuses while JSON changes not yet built exist (--force discards them)
target/release/g4k text check <copy>     # schema=…, problems, differs=N; never writes
target/release/g4k text import <copy>    # checks everything before it writes the first member; refuses while members are newer than their JSON (--force discards them)
target/release/g4k text sources <copy>   # the files of the text project, one per line (for git add)
target/release/g4k text migrate <copy>   # older schema version with a snapshot under .g4/backup/
target/release/g4k text adopt <copy>     # set up as a text project (scripts, export, rotom.toml, recording); phase=… ms=… per phase
target/release/g4k text sync <copy>      # adopt one-sided changes; conflict/broken <key> <label>, exit 1 while conflicts or broken files remain; --keep studio|dspre <key>
target/release/g4k text revert <copy>    # take back: manifest, data/, ID files to .g4/backup/textproject-<time>/
```

**Base layer:** The export compares with the base ROM, extracted and unpacked with all archives under `<project>/.g4/cache/base-layer/<sha1>/` (about 150 MB and 18,500 files for Platinum, 182 MB and 20,000 files for HeartGold), created once per base (6 to 8 s under light load) and deletable at any time; it is never in Git. A folder next to `data/` in the project root (such as a backup `data.prev/`) counts as an unmapped change and becomes a blob under `data/raw/`; backups belong outside the project.

The gated tests (`G4_TEST_PROJECT`; the base ROM is the `.nds` next to the original, found through `suggest_base` and only read; everything that writes works on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `textproject_round_trip` (`tests/textproject.rs`) | export, delete all members with an entity as well as text and script members with a source (header table and internal names zeroed), import: every file under `unpacked/`, `arm9.bin` and `mapname.bin` byte-equal, the build has the SHA-1 of the base; 0 blobs, 0 side files |
| `textproject_export_idempotent` | a second export writes and deletes nothing, no source changes content or mtime |
| `textproject_from_base` | `extract_rom` of the base ROM plus the files from `sources`, import, build: SHA-1 equal to the base |
| `vanilla_base_layer_has_no_raw_blobs` (`tests/textproject_vanilla_probe.rs`) | the base layer equals the DSPRE folder outside the mapped members (no blobs) |
| `vanilla_shared_item_members_are_counted` | how many item IDs share a member (`same_member_as`) |

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject --test textproject_vanilla_probe \
  -- textproject_round_trip textproject_export_idempotent textproject_from_base vanilla_ --nocapture --test-threads 1
```

On stderr appear `textproject-stats <game code> files=… bytes=… json=… side=… raw=… base_layer_ms=… export_ms=… import_ms=… build_ms=…`, `textproject-second-export … ms=…`, `textproject-from-base … extract_ms=… import_ms=… build_ms=… equal`, `base-layer … ms=… diff_ms=… files=… bytes=… not_built=…` and `items … ids=… members=… shared=… unreferenced=…`, plus `textproject-art …` per kind and `textproject-locked …` per locked member (none in vanilla). Measured on 2026-10-27 (release, load 4 to 14): 347 s (Platinum DE), 684 s (HeartGold DE), 554 s (HeartGold US) for the five tests including the build; values and times in [docs/phase-4-3a-gate.md](../phase-4-3a-gate.md) (German). In `scripts/gate.sh` they run along in the step of the gated tests per project (there in the debug profile). The diff of two exported projects with the IDs as numbers is shown by `python3 -I scripts/agent/textproject_diff.py <export A> <export B>` (see [docs/agent-gate.md](../agent-gate.md), "Phase 4.3a"; German).

#### Text project as source (4.3b)

Since 4.3b, in a project with `g4-project.json` the store reads and writes the entity JSON (`textproject::source`), the reconciliation with DSPRE is in `.g4/build/state.json` (`sync`, `textproject::sync`), switching over and taking back are in `textproject::adopt`, the ID files for rotom are in `.rotom/command_database/.g4-ids-<kind>.json` (`textproject::include`), "Rename ID" is in `textproject::rename`; format in [docs/formats/text_project.md](../formats/text_project.md). Headless: `g4k text adopt`, `g4k text sync [--keep studio|dspre <key>]` and `g4k text revert` (short description under "Text project (4.3a)"), plus `g4k text rename-id <copy> <old> <new>` (one line `<file> <count>` per file, then `renamed … files=… refs=… ms=…`; without undo, a refusal writes nothing).

The gated tests (`G4_TEST_PROJECT` with the `.nds` next to it, only read; everything that writes works on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `store_on_json_edits_only_the_entity_file_and_builds_like_dspre` (`tests/textproject_source.rs`) | Pikachu changed through the store: only `data/species/0025.json` changes, text project and DSPRE folder build the same ROM; prints `store <game code> save_ms=… load_ms=…` |
| `simulated_dspre_change_is_adopted_one_sided` | full reconciliation, focus reconciliation without a change, one member byte adopted, `g4k check-data` on the ROM; prints `sync-full …` and `sync <game code> stat_files=… stat_ms=… one_ms=…` |
| `simulated_change_on_both_sides_is_a_conflict_and_both_choices_work` | trainer 1 on both sides, diff, both choices |
| `adopt_dspre_folder_builds_byte_equal_and_reverts` | switching over and taking back, each time a build with the SHA-1 of the base; prints `adopt <game code> phases …` and `revert …` |
| `vanilla_checks_on_the_text_project_have_no_errors` | all checks on the text project, 0 errors |
| `recipes_with_ids_apply_on_the_vanilla_project` | the template recipes with IDs |
| `script_refs_on_the_vanilla_project_are_counted`, `a_script_using_an_id_constant_compiles_to_the_number` | script references in the reference index, ID files for rotom; prints `refs …` and `ids …` |
| `rename_id_updates_every_reference_on_the_vanilla_project` | "Rename ID" of a trainer across all references; prints `rename …` |
| `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` (`apps/studio/src-tauri/src/mcp/tools.rs`) | MCP `adopt_text_project`, then `data_edit` with `SPECIES_PIKACHU` as with `25` |
| `project_start_text_project_builds_byte_equal` (`tests/project_start.rs`, `G4_TEST_ROM`) | the wizard with "as a text project" builds the ROM byte-equal |

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/_testkopien/4-3b-pt_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject_source -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p studio --all-features --lib mcp_with_ids_acts_like_numbers -- --nocapture --test-threads 1
G4_TEST_ROM="/<absolute path to Roms>/_testkopien/4-3b-pt.nds" \
  cargo test --release -p g4-kernel --all-features --test project_start -- project_start_text_project_builds_byte_equal --nocapture
```

`G4_TIMING_STRICT=1` turns the times into limits (saving ≤ 200 ms, focus reconciliation ≤ 500 ms, a change ≤ 1,000 ms). Measured on 2026-10-29 (release, load 3 to 18): `textproject_source` 798 s (Platinum DE), 938 s (HeartGold DE), 845 s (HeartGold US), the Studio test 77 to 94 s, the project start 53 to 57 s; values in [docs/phase-4-3b-gate.md](../phase-4-3b-gate.md) (German). In `scripts/gate.sh` they run along without a change: the kernel and Studio tests in the step of the gated tests per project (debug), the project start in the step `project_start_` per `.nds`. The full run of `scripts/gate.sh` thus took 8,324 s (gated tests per project 2,168 to 2,340 s in the debug profile). The gated Studio tests must also run with `G4_ROMS_DIR` set when `G4_TEST_PROJECT` is the original (this is how the gate script calls them): a base ROM for the `UserPolicy` path belongs as a copy in the temp folder, never the folder of the originals as `--allow-dir`.

#### Minimal Git (4.4)

"Set up Git" (`git::cli::git_init`, format in [docs/formats/project_git.md](../formats/project_git.md)) calls the
`git` CLI. The tests that need it run only if `find_git()` finds something, and always in an environment of their own
(`test_support::GitFixture::cli(home)`: `GIT_CONFIG_GLOBAL` pointing to an empty file in a temp folder,
`GIT_CONFIG_NOSYSTEM=1`, `HOME` pointing to the temp folder, so without an identity and without the developer's settings);
further variables are added through `GitCli::env`, never through `std::env::set_var`. Repositories only in temp folders, never a
remote, never `git clone`.

The gated tests in `crates/g4-kernel/tests/git.rs` (`G4_TEST_PROJECT` and `git`; identity through `GIT_AUTHOR_*` and
`GIT_COMMITTER_*` in the environment, everything on `vanilla_copy()`):

| Test | Checks |
|---|---|
| `git_setup_on_the_dspre_folder_keeps_status_empty_after_build` | set up, status empty; `build::build` to `build/`, status empty; `git repack -a -d`, `git count-objects -v`: `packed_kib` ≤ 65 MiB, `files` ≤ 20,027, no `.nds` and under `.g4/` only `base.json`/`preview.png` in the index; prints `git <game code> dspre files=… ignored_archives=… unpack_ms=… add_ms=… commit_ms=… status_ms=… packed_kib=…` |
| `no_game_data_file_falls_under_an_eol_rule` | after unpacking, no file under `files/` and `unpacked/` has an extension with `eol=lf` (`.json`, `.rotom`, `.yaml`, `.yml`, `.toml`, `.md`); under `arm9/`, `arm9_overlays/`, `arm7/` only the `.yaml` of ds-rom, without CR; prints `eol …` |
| `a_clone_with_autocrlf_builds_byte_equal_dspre` | clone through `git init`, `config core.autocrlf true`, `fetch <copy> HEAD`, `checkout FETCH_HEAD`: builds the same SHA-1, status empty, no `[remote` in both `.git/config`; prints `clone <game code> dspre build_ms=… equal` |
| `text_project_in_git_builds_from_base_byte_equal` | gate of phase 4: assign base, set up as a text project (`adopt`), set up, status empty; index ⊇ `textproject::sources()`, otherwise only the templates and `expanded/`; `build::build` = SHA-1 of the base ROM, status empty; `packed_kib` ≤ 10 MiB; clone with `core.autocrlf=true`, `needs_restore`, `restore_from_base`, reconciliation without anything open, build = SHA-1 of the base ROM, status empty; a second clone through `g4k text restore`; prints `git <game code> text files=… add_ms=… commit_ms=… packed_kib=…`, `clone <game code> text restore_ms=… build_ms=… equal`, `restore_phases <game code> extract=… merge=… import=… sync=…` and `text <game code> adopt_ms=… moved=… members=… sync_files=…` |
| `trainer_and_text_changes_show_with_field_names_against_head_and_base` | trainer 1 level +1 and message 0 in text archive 1 + " Studio", saved: the list against `HEAD` names "Trainer 0001" and "Text archive 0001"; the diff shows `party[0].level` and the word span against `HEAD` and against the base with the same lines, against saved `same`; after "Reset to original" and saving, the trainer is `same` against the base; prints `entries …` and `diff <game code> status_ms=… entries_ms=… head_ms=… base_ms=…`; with `G4_TIMING_STRICT=1` status ≤ 500 ms |

Also `crates/g4-kernel/tests/diff.rs` `one_entity_diffs_fast_in_the_dspre_layout` (after changes to trainer 1, Pokémon 25, map 0 and header 0, these are changed against `HEAD`, move 1, text archive 433 and map texture 0 equal; prints per diff `diff <game code> <name> <against> state=… ms=…` and `open_for_diff_ms=… slowest_ms=…`; with `G4_TIMING_STRICT=1` every diff < 200 ms) and the Studio test `git_and_diff_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/git.rs` ("Set up Git", status, list and diff over the command path of the app; prints `git init on the vanilla copy: …`, `status after init: … ms`, `diff against HEAD: … ms`).

```sh
G4_TEST_PROJECT="/<absolute path to Roms>/_testkopien/4-4-pt_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p g4-kernel --all-features --test diff -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p studio --all-features --lib git_and_diff_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Measured on 2026-10-29 in the gate (release, `git version 2.54.0 (Apple Git-157)`, load 3.6 to 41.8): `--test git` 433 s
(Platinum DE), 550 s (HeartGold DE), 414 s (HeartGold US), `--test diff` 31 to 34 s, the Studio test 32 to 36 s; packed
46,311 / 56,792 / 56,289 KiB (DSPRE) and 3,120 / 3,232 / 3,107 KiB (text project), status 172 to 215 ms, clone of the
text project completed in 43.7 to 75.3 s (depending on load), build from the clone SHA-1-equal; values in
[docs/phase-4-4-gate.md](../phase-4-4-gate.md) (German). In `scripts/gate.sh` they run along in the step of the gated tests per project
(debug), without a change to the script.

## Design system and tokens

The design system "Doppeldeck" lives in the repository under `design-system/` (description: [design-system/README.md](../../design-system/README.md)). The values are W3C DTCG tokens in `design-system/tokens/`; the app reads them from three **generated and checked-in** files in `apps/studio/src/ui/tokens/`: `tokens.css` (CSS variables `--g4-*`), `dockview-theme.css` (theme `dockview-theme-g4`) and `tokens.ts` (numbers and hex values for three.js, CodeMirror and canvas). `pnpm build` therefore needs no Python.

To change a token:

```sh
python3 design-system/scripts/check_tokens.py     # grid (RGB555) and contrasts in both themes
cd apps/studio && pnpm tokens                     # rewrites the three files (python3 export_tokens.py <root> <output folder>)
cd ../.. && ./scripts/check-tokens.sh             # check_tokens.py plus comparing the checked-in files with a fresh export
```

After that, load the changed token files into OpenPencil ("Import design tokens", see the README of the design system). Never change the three files by hand: the gate step `tokens` reports every deviation from the export.

Style rules for `apps/studio/src` and the gate step `styles` (`scripts/check-styles.sh`, same `grep` principle, exit 1 with `file:line` per finding):

- Styles live in CSS Modules (`Name.module.css`) and use only `--g4-*` variables (theme, room, density, type), never primitives like `--g4-platin-500`.
- No `style={` in `.tsx` (neither an object in the attribute nor a style from a variable), no hex colour (`#rgb` to `#rrggbbaa`) and no primitive variable (`--g4-platin-500`, `--g4-kobalt-600` …) in `.tsx`, `.ts` and `.css`. Excepted are `ui/tokens/`, `world/colors.ts` (converts the numbers from `tokens.ts` for three.js), test files and lines with `/* dyn */` (values that only arise at run time, such as the position of a marker; the mark is on the same line). Custom CSS variables that a component sets at run time are typed once in `apps/studio/src/css-custom-properties.d.ts`.
- `scripts/check-styles.allow` names files and folders (one per line, glob allowed) that may still be in the old style. Since 3c.2 the list is empty and stays so; new files never go on it.

Depth rule for `apps/studio/src` and the gate step `depth` (`scripts/check-depth.sh`, since 3d.2, same `grep` principle, exit 1 with `file:line [depth]` per finding): the depth (Tell, Build, Tinker) is never compared directly with a value but asked through `depthAtLeast(depth, min)`; what an area shows is told by `viewDepth(area, depth)` (Tell only in the areas from `NARRATING_AREAS`, otherwise Build; all of it in `ui/frame/shellState.ts`). A line fails with a name on `depth`/`Depth` before `===`/`!==` and a string, with `===`/`!==` before `"erzaehlen"` or `"tuefteln"`, or with `case "erzaehlen"`/`case "tuefteln"`. `"bauen"` alone does not count (the Technical entry "Build" is also called that). Excepted are test files and `ui/frame/shellState.ts`.

To maintain the `.fig`: after saving in OpenPencil, `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` (from the repository root; repairs text glyphs and snaps colours to RGB555), then check the file in.

## Gallery

The gallery shows the building blocks from `apps/studio/src/ui/primitives/` with their states, one page per group (buttons, inputs, content and status, floating elements, selection, screen with the DS screen, frame with the Hinge in assistant mode, start cards, palette and tabs); at the top, "Theme" (Light, Dark, System) and "Density" (Comfortable, Compact) switch. It runs only in browser mode, never in the Tauri window, and needs no project: `http://127.0.0.1:1430/?gallery`, after `scripts/agent/studio-web.sh` has been started (or with `pnpm dev` in `apps/studio` on the Vite port). There are no automatic tests for the gallery; for the gate the agent takes screenshots in Light, Dark and Compact and watches for unreadable spots. A new building block gets a line on a page under `ui/gallery/pages/` (`pages/index.ts` lists the pages).

## Gate script

`.github/workflows/ci.yml` runs the hermetic steps (without ROM, save game and tools, `G4_GATE_HERMETIC=1`, see below) on every pull request against `main` and on every push to `main`; by hand, you start the workflow with `gh workflow run ci.yml` (with `-f windows=true` also on Windows; see `docs/en/release.md`). For public repositories the standard runners of GitHub Actions are free, so manual starts are no longer needed. Before a merge, the full `scripts/gate.sh` (repository root, bash) additionally runs locally, with the ROM-bound steps; it is the acceptance. Besides Rust and pnpm, the prerequisite is `uv` (the step `g4emu unit` starts `uv run`; the first run downloads the Python packages from PyPI). It runs in order:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo deny (licenses, advisories)`: `cargo deny --locked check licenses advisories` with `deny.toml` (see "cargo-deny (licences and advisories)" under "Tests"); if `cargo-deny` is missing, the script aborts with the line "FAILED (cargo-deny fehlt)" (the script's message is German: "cargo-deny is missing") and the install command; directly afterwards `npm licenses`: `scripts/check-licenses-npm.sh` (the licences of the production packages of `apps/studio` against the allowlist, see "npm licences" under "Tests")
4. `cargo test --workspace --all-features` (without ROM data: `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_DSROM_CLI`, `G4_APICULA` and `G4_ROTOM_LSP` from the calling shell are removed for this step, the gated tests skip themselves)
5. `mcp`: `cargo test -p studio --test mcp_stdio` (the product binary without Cargo features: `studio mcp` over stdio on the synthetic project, including writing, a batch as one undo step, irreversible actions only with `--allow-irreversible` and the test that `g4_call` is missing without `--expert`; hermetic like the step before, the same `G4_*` variables are removed; about 30 s with a warm cache, the tests themselves 8 s; the gated tests run along per project in the last step)
6. `pnpm test` in `apps/studio`
7. `tokens`: `scripts/check-tokens.sh` (grid and contrasts with `check_tokens.py`, then the comparison of the checked-in token files with a fresh export; see "Design system and tokens")
8. `styles`: `scripts/check-styles.sh` (no inline styles and no hex colours outside the exceptions; see "Design system and tokens")
9. `depth`: `scripts/check-depth.sh` (no direct comparison of the depth, only `depthAtLeast`/`viewDepth`; see "Design system and tokens")
10. `pnpm build` in `apps/studio`, then the check that `apps/studio/dist` does not contain the text `__bridge` (the dev bridge must not get into the production bundle)
11. `g4emu unit`: `uv run pytest -m "not integration" -q` in `scripts/g4emu`
12. `g4emu integration`: `uv run pytest -m integration -q` in `scripts/g4emu` (Platinum DE with a save game up to the room, stairs and header ID in the real emulator). Runs only if the core (`tools/g4emu/core/melondsds_libretro.dylib` or `G4EMU_CORE`, relative paths are allowed), `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` (the gate always looks for save games there; it does not evaluate `G4EMU_SAVES`) and `$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds` exist; otherwise the line "skipped (kein Kern)", "skipped (kein Spielstand)" or "skipped (keine ROM)" (no core / no save game / no ROM) appears in the table, without aborting
13. per test project under `$G4_ROMS_DIR` (pattern `*_DSPRE_contents`) `cargo test --workspace --all-features` with `G4_TEST_PROJECT`, `G4_APICULA`, `G4_DSROM_CLI` and `G4_ROTOM_LSP` (all absolute), that is the gated tests from the section "Gated tests" against Platinum DE, HeartGold DE and HeartGold US; without `build_graph_timings` (`--skip`, which step 14 measures in the release profile), and `G4_TIMING_STRICT` from the calling shell is removed for this step
14. `build graph (timings)`: per test project `cargo test --release -p g4-kernel --all-features --test build_graph -- build_graph_timings --nocapture --test-threads 1` with `G4_TEST_PROJECT` and `G4_TIMING_STRICT=1` (time limits of "Play here", see "Build graph (4.2)"), then likewise `--test refs -- vanilla_index_builds_under_a_second` (warm build of the reference index ≤ 1,000 ms; in the parallel run of step 13 it only measures, because the other tests there read the same files); if the run fails, the script repeats it once (line "ok (2nd run)"), otherwise it aborts
15. per `*.nds` under `$G4_ROMS_DIR` (only read) `cargo test --workspace --all-features -- project_start_` with `G4_TEST_ROM` (create a project, build, compare SHA-1, see "Creating a project from a ROM (3g.1)"); without an `.nds` a line "skipped (keine ROM)"

Since "Release and CI", five more hermetic steps sit between step 3 and 4 (without a number, so that the references to steps 13 to 15 stay right): `script tests` (`scripts/tests/run.sh`, the tests of the shell scripts), `actions` (`scripts/check-actions.sh`: every `uses:` line pinned to a SHA and in `.github/actions-allowlist.txt`, licence from the list of `deny.toml`), `no rom data` (`scripts/check-no-rom-data.sh`: no ROM, save game or patch data under `git ls-files`), `links` (`scripts/check-links.sh`: relative links in `README.md`, `README.de.md`, `docs/README.md` and `CONTRIBUTING.md` point to existing files, with exactly matching upper and lower case) and `version` (`scripts/version.sh check`). The versions of the tools (apicula, rotom, cargo-deny, pnpm, Node, cargo-about) are in `scripts/tool-versions.env`; CI, release workflow and scripts read them there. With `G4_GATE_HERMETIC=1` the gate ends after step 11 (`g4emu unit`): that is exactly what `gate-macos` in `ci.yml` runs (on every pull request and every push to `main`). `gate-windows` does not run the gate, only `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features --no-fail-fast`.

```sh
scripts/gate.sh
```

The script finds the Rust toolchain itself (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, if present; otherwise the `PATH` of the shell applies) and appends `~/.cargo/bin` (where `cargo-deny` lives) to the end of the `PATH`, so it needs no `export PATH`. Variables, all optional:

| Variable | Default | Meaning |
|---|---|---|
| `G4_GATE_HERMETIC` | not set | `1`: the gate ends after the step `g4emu unit` and the table closes with "Gate (hermetisch): ok" (the script's message: "Gate (hermetic): ok"); this is what CI runs (no other `G4_*` needed). Not set: the full local gate |
| `G4_ROMS_DIR` | `../Roms` (next to the repository) | folder with the `*_DSPRE_contents` projects; set it absolute from a Git worktree, the default hits nothing there |
| `G4_APICULA` | `tools/apicula/bin/apicula` | apicula binary for the model tests |
| `G4_DSROM_CLI` | `$G4_ROMS_DIR/_tools/dsrom` | ds-rom reference CLI for the rebuild comparison |
| `G4EMU_CORE` | `tools/g4emu/core/melondsds_libretro.dylib` | emulator core for the step `g4emu integration` |
| `G4_ROTOM_LSP` | `tools/rotom/bin/rotom-lsp` | rotom-lsp binary (preflight; passed on to the gated tests) |

The script stops at the first error and prints a table step / project / result / seconds at the end, also on abort; the exit code is non-zero as soon as a step fails. A folder without `header.yaml` does not count as a project and appears as a line "skipped (no header.yaml)". If `G4_ROMS_DIR`, `G4_APICULA`, `G4_DSROM_CLI` or `G4_ROTOM_LSP` is missing, or if it finds no project, it aborts with a message (otherwise the gated tests would only report "skipped" and still count as passed).

Duration (the step `mcp` joined with about 30 seconds since 3h.2): about 30 to 40 minutes with a warm build cache (measured 1704 s in 3c, 1781 s in 3b.3a, 1798 s in 3b.3b, 2452 s in 3b.4a; before 3b.2 it was 1217 s, plus about a minute for the g4emu steps; since 3b.2 the gated script and text tests are added, since 3b.3a the data tests from "Data archives and the uxie oracle"; since 3b.3b the trainer and encounter tests from "Trainers and encounters (3b.3b)", since 3b.4a the check tests from "Project checks (3b.4a)"; measured runs are in [docs/phase-3c-gate.md](../phase-3c-gate.md) (1704 s), [docs/phase-3b3-gate.md](../phase-3b3-gate.md) (part A 1781 s, part B 1798 s) and [docs/phase-3b4-gate.md](../phase-3b4-gate.md) (part A 2452 s); the three projects take 380 to 830 s each, the steps before them together under a minute); a cold build comes on top. The projects under `$G4_ROMS_DIR` are only read: tests that write work on a copy in the temp directory (`vanilla_copy()`), and the script does not touch `_testkopien`.

## Agent test environment

A Claude agent operates the app and the game itself; the instructions for that are in [docs/agent-gate.md](../agent-gate.md) (German). The building blocks:

- **Bridge `studio-bridge`** (Cargo feature `dev-bridge`, dev build only, never in the app bundle): a small HTTP server on `127.0.0.1:1431` that calls all Tauri commands of the app. Since 3h.1 the command layer `dispatch` (`commands/dispatch.rs`, 122 commands) is product code without a feature; the bridge is only the HTTP frame around it. Which paths a call may touch is decided by a `Policy` (`commands/policy.rs`): `DevPolicy` (the former struct `Roots` in `bridge/roots.rs`, feature `dev-bridge`) for the bridge, `UserPolicy` for `studio mcp` (see "MCP (`studio mcp`)"). The bridge accepts only projects and build targets under `$G4_ROMS_DIR/_testkopien` (never under `*_DSPRE_contents`) and requires a token (`X-G4-Bridge-Token`, found in `target/agent/bridge.token`). `scripts/agent/studio-web.sh` starts bridge and Vite (`127.0.0.1:1430`); `.claude/launch.json` knows it as `studio-web`. As long as the bridge runs, every local user reaches it through the Vite proxy on `127.0.0.1:1430` (the token is ready for the proxy); a development machine with a single user is assumed. The page then runs in the browser window of the Claude desktop app at `http://127.0.0.1:1430/?project=<copy>` and shows "Browser mode (bridge)" in the status bar.
- **`scripts/agent/call.sh <command> ['<json>']`** calls a command of the running bridge; `studio-bridge batch <file.json> --allow-root <dir>` runs a list of commands without a server.
- **`g4emu`** (`scripts/g4emu`, Python with `uv`): a windowless emulator (libretro.py with the melonDS DS core 1.4.0). Set up with `bash scripts/g4emu/setup.sh` (downloads the core from the pinned GitHub release, SHA-256 checked, to `tools/g4emu/core/`). Call: `uv run g4emu run --rom … --save … --out … <steps>` (`continue`, `walk-to`, `expect-map`, `where`, `shot` …), `uv run g4emu scenario <yaml> --project <copy> --set NAME=VALUE …` (change the project, build, play; examples in `scripts/agent/scenarios/`; a change can bind the answer of the bridge to a name that later steps use as `${name}`, and since 3b.4a make expectations about the answer, `{expect: {path, equals}}`, missed with return code 2; since 3b.4b also expected refusals `{expect: {error: …}}`, file hashes `${sha1-of:PATH}` and the section `prepare:`, see "Importing map parts (3b.4b)"), `uv run g4emu find-u16` for searching addresses. Since 3b.3b `g4emu` knows battle steps (`expect-battle`, `walk-until-battle`, `end-battle`, `win-battle`, `mash`): a battle is recognised by the BattleMon records in RAM, the values per game code are in `scripts/g4emu/g4emu/games.yaml` (`battle`, `battle_pointer`, `home_header`; IPKE without a measured pointer); description in [docs/agent-gate.md](../agent-gate.md), "Kämpfe" (battles; German). The tests: `uv run pytest -m "not integration"` (unit) and `-m integration` (real core). Since 3d.9 there is `uv run g4emu repl --rom … --save … --out … --gdb-port P`: the game keeps running, every line on stdin is exactly one step (answer as a JSON line, `quit` ends, `expect-halted`/`expect-running` ask the stub), and the first line `{"ready": true, "gdb_port": N, "game": …}` names the port. On this port a GDB stub modelled on melonDS 1.1 listens (`g4emu/gdbstub.py`: one client, halt on connect until `c`, `\x03`, `m`/`M` on the 4 MiB of working memory, `D`), so that `g4k live` and play mode can be tested against the running game without melonDS. The gated test `tests/test_live.py` (`-m integration -k live`, port 3390) takes the ROMs from `G4_LIVE_ROM_CPUD`, `G4_LIVE_ROM_IPKD` and `G4_LIVE_ROM_IPKE` and `g4k` from `G4K_BIN` (otherwise `cargo build -p g4-kernel --bin g4k`).
- **Play mode live (3d.9)** reads and writes the save state in the working memory of the running game through the GDB stub of **melonDS 1.1** or newer (Emu settings › Devtools; older versions do not know it, and the app then reports "melonDS 1.1 or newer required"). The stub listens only with `[Instance0.Gdb] Enabled = true` in `~/Library/Preferences/melonDS/melonDS.toml`, serves exactly one client (port 3333 for the ARM9, 3334 for the ARM7; Studio uses only 3333); "Play here" switches it on, backs the file up beforehand as `melonDS.toml.g4-backup` and changes it only if no melonDS is running (melonDS writes it back on exit). The **JIT** must be off (`[JIT] Enable = false`), otherwise the stub does not work, and `BreakOnStartup` must be off so that the game does not halt at startup; the app sets both along with the stub, and the line "Play mode live" in Technical › Emulator shows what is missing. Without melonDS the same runs against the stub of `g4emu repl` (port 3390, never touch 3333 as long as a melonDS of the user might be running); starting and configuring melonDS exists only on macOS; Windows and Linux start the game as before without live values.
- **Save games** live outside the repository in `$G4_ROMS_DIR/_testkopien/_saves/<GAMECODE>-zimmer.sav` (CPUD Platinum DE, IPKD HeartGold DE, IPKE HeartGold US) with an `.md` file that describes the way there. `g4emu` copies them before every run and never writes back.
- **Ports:** Vite 1430, bridge 1431; `G4_VITE_PORT` and `G4_BRIDGE_PORT` choose others (for a second instance from a worktree; `call.sh` reads the bridge port from `target/agent/bridge.port`). The app itself (`pnpm tauri dev`) stays on 1420/1421.
- **Environment variables:** `G4_ROMS_DIR` (folder `Roms`, default `../Roms`), `G4_APICULA`, `G4EMU_CORE`, `G4EMU_SAVES` (a different save game folder), `G4_BRIDGE_BIN` and `G4K_BIN` (a finished `studio-bridge` or `g4k` for scenarios; build from a worktree with `cargo build --workspace --all-features`, `cargo test --no-run` does not always renew `studio-bridge`). In a Git worktree `tools/` is missing (not versioned): link `tools/apicula` or set `G4_APICULA`. The default of `G4_ROMS_DIR` is also wrong there (`../Roms` lies next to the main checkout): `scripts/agent/studio-web.sh` and `scripts/gate.sh` need `G4_ROMS_DIR=<absolute path to Roms>` from a worktree.

## MCP (`studio mcp`)

Since 3h.1 there is an MCP server for coding agents (since 3h.2 it reads and edits headless), in the program file of the app itself: `studio mcp` (a subcommand before Tauri starts, `src/main.rs` calls `studio_lib::mcp::main`). It is product code without a Cargo feature, uses the same command layer `dispatch` as the bridge and the protocol over stdio with `rmcp` (3.7.0). The instructions for users are in [docs/en/mcp.md](mcp.md); this section says what developers need. Modules in `apps/studio/src-tauri/src/mcp/`: `mod.rs` (command line, protection of stdout), `catalog.rs` (the 41 tools with schemas, class per command in `access.rs`), `tools.rs` (`Toolbox`: call, `g4_batch`, resources), `server.rs` (mapping to rmcp), plus `commands/policy.rs` (`UserPolicy`).

**Building and calling:**

```sh
cargo build -p studio                                   # target/debug/studio (no feature needed)
target/debug/studio mcp --project <copy>                # waits for JSON-RPC (one message per line) on stdin
target/debug/studio mcp --help
```

Switches: `--project <DIR>` (required), `--allow-dir <DIR>` (repeatable), `--read-only`, `--allow-irreversible` (not with `--read-only`), `--emulator <PATH>` (for `run_rom`), `--expert` (`g4_call`, `g4_commands`). The access is `{write: !read_only, irreversible: allow_irreversible, expert}`; the policy is `UserPolicy::new(project, allow_dirs, emulator)`. When stdin is closed, the server waits for running calls (`server::DRAIN_LIMIT`, 600 s); in debug builds `G4_MCP_TEST_DELAY_MS=<n>` holds every call for `n` ms on its worker so that `closing_stdin_waits_for_a_running_write` can check this. These two hooks (`G4_MCP_TEST_DELAY_MS`, `G4_MCP_TEST_NOISE`) exist only with `debug_assertions`; the tests that need them (the two tests on closing stdin and `stray_stdout_does_not_break_the_protocol`) are excluded with `#[cfg(debug_assertions)]`, so in a `--release` run of `mcp_stdio` 22 instead of 25 tests run. Return code 0, 1 (error in the protocol) or 2 (nothing served: wrong arguments, project not readable, original locked; message on stderr). stdout belongs to the protocol: on Unix the server redirects fd 1 to stderr (`libc::fcntl` with `F_DUPFD_CLOEXEC`, `dup2`) and writes the protocol to the copy; on Windows this does not exist. In debug builds `G4_MCP_TEST_NOISE=1` prints a line to stdout (it ends up on stderr) so that tests can prove the protection.

**Writing:** The default is read and edit (`--read-only` switches it off). The class of every command (reading, writing, irreversible, depending on the argument as with `build_rom` and `export_map_part`) is in `mcp/access.rs`; `admit` returns `read_only` or `needs_confirmation`. Irreversible tools stay in `tools/list` and answer with `needs_confirmation` without `--allow-irreversible`. `g4_batch` checks all calls in advance, runs them in one session and merges them with `squash_since` into one undo step; a `RollbackGuard` also rolls the batch back on a panic (details in [docs/en/mcp.md](mcp.md), "Batch"). To try it on a copy: `target/debug/studio mcp --project "$G4_ROMS_DIR/_testkopien/<name>"` and with `--allow-irreversible` for `create_trainer` or `apply_header_patch`.

**Scenarios over MCP:** `scripts/g4emu` also plays scenarios through `studio mcp` (YAML key `mcp:`, see [docs/agent-gate.md](../agent-gate.md), "Szenarien über MCP"; German). The client chooses the program like this: if `G4_STUDIO_BIN` is set, it counts as the path of the program file `studio`; otherwise it builds with `cargo build -p studio --bin studio` and takes `target/debug/studio`. A prebuilt binary saves the build, for example `G4_STUDIO_BIN=$PWD/target/debug/studio uv run g4emu scenario …`.

**Originals in development:** If `G4_ROMS_DIR` is set, `UserPolicy` refuses a project (and every `--allow-dir` folder) that lies under `$G4_ROMS_DIR` but not under `$G4_ROMS_DIR/_testkopien`, and everything under `_testkopien/_saves`: return code 2, "Originals are locked, only copies under _testkopien". An `--allow-dir` folder (or project) that contains the originals is refused as well, that is `$G4_ROMS_DIR` itself or a folder above it ("Originals are locked, the folder contains G4_ROMS_DIR"). Every path that a command reads or writes is additionally checked individually against the same rule. If `G4_ROMS_DIR` cannot be resolved (folder missing), `studio mcp` does not start: "G4_ROMS_DIR is set but not readable: …", return code 2. The name block `*_DSPRE_contents` of the bridge does not apply to users (3g creates user projects that way). So always point to a copy:

```sh
cp -R "$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" "$G4_ROMS_DIR/_testkopien/<name>"   # the target must not exist
```

**Tests:**

```sh
cargo test -p studio --all-features                     # library (350 since 3h.2) and tests/mcp_stdio.rs (25, 3 of them gated)
cargo test -p studio --test mcp_stdio                   # only the client test over stdio (the gate step `mcp`), about 8 s
G4_TEST_PROJECT="$G4_ROMS_DIR/<project>_DSPRE_contents" \
  cargo test -p studio --test mcp_stdio vanilla          # gated: header, checks, texts on vanilla_copy(), about 80 s
G4_TEST_PROJECT=… G4_DSROM_CLI=… cargo test -p studio vanilla_build_rom_defaults_into_build   # gated, about 85 s
```

`tests/mcp_stdio.rs` starts `CARGO_BIN_EXE_studio mcp` with an rmcp client (`TokioChildProcess`) on the synthetic project (`synthetic_project()`); the gated test runs on a copy in the temp directory (`vanilla_copy()`), the original is only read. The rmcp client skips lines on stdout that are not JSON; whoever checks the protection of stdout uses a raw line (`initialize_line_with_2025_11_25`).

**Manual probe** (without an MCP client, with protocol version 2025-11-25, as a Python client speaks it): a small script in the scratchpad starts `studio mcp --project <copy>` with `subprocess.Popen` and sends lines:

```python
import json, subprocess
p = subprocess.Popen(["target/debug/studio", "mcp", "--project", COPY], stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
def rpc(method, params=None, id=None):
    msg = {"jsonrpc": "2.0", "method": method, **({"params": params} if params is not None else {}), **({"id": id} if id is not None else {})}
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    return json.loads(p.stdout.readline()) if id is not None else None
print(rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "probe", "version": "0"}}, 1)["result"]["serverInfo"])
rpc("notifications/initialized")
print([t["name"] for t in rpc("tools/list", {}, 2)["result"]["tools"]])
print(rpc("tools/call", {"name": "get_header", "arguments": {"id": 0}}, 3)["result"]["structuredContent"])
p.stdin.close()
```

One answer per request does not necessarily arrive in the order of the requests (the server works concurrently); with several open requests, match them by `id`. A command error is `result.isError: true`, an unknown tool a JSON-RPC answer with `error` (-32602). The manual probe of 2026-10-25 on `_testkopien/3h-lesen` is in the gate document [docs/phase-3h-gate.md](../phase-3h-gate.md) (German). Setting up Claude Code itself is the by-hand item of the gate: `claude mcp add gen4-studio -- <repo>/target/debug/studio mcp --project <copy>` (see [docs/en/mcp.md](mcp.md)); the agent does not change the user's Claude configuration.

**Gate script:** Since 3h.2, `scripts/gate.sh` has the step `mcp` (`cargo test -p studio --test mcp_stdio`, directly after `cargo test` without ROM data); the gated tests (`vanilla_*`) run along per project in the last step. Scenarios do not run in the gate script; their run and the images are in the gate document ([docs/phase-3h-gate.md](../phase-3h-gate.md), German).

## App bundle

A runnable `.app` (with `libnitroarc_ffi.dylib` and the sidecars apicula and rotom-lsp) is made with:

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, then tauri build (through scripts/with-rust.sh, which finds cargo itself)
bash scripts/check-bundle.sh     # output: bundle ok
```

Prerequisites, procedure, check commands and known limits (not signed, Gatekeeper) are in [docs/en/release.md](release.md). The smoke test of the packaged app (open a script, completion, error on saving) is in the gate document [docs/phase-3b2-gate.md](../phase-3b2-gate.md) (German), part (H).
`pnpm tauri dev` and normal `cargo` builds do not need this.
