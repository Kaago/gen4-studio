"""Generate the English symbol tables of the text project from PokeAPI.

Writes `species.yaml`, `forms.yaml`, `moves.yaml`, `items.yaml`, `abilities.yaml`,
`types.yaml` and `LICENSE-PokeAPI.txt` into `--out` (normally
`crates/g4-kernel/metadata/symbols`). The CSV come from `data/v2/csv/` of PokeAPI at a fixed
commit: downloaded into a temporary folder, or read from `--csv-dir` (offline; the folder then
also holds `LICENSE.md`). Standard library only.

Call:
    python3 -I scripts/symbols/generate_symbols.py --commit <sha> \
        --out crates/g4-kernel/metadata/symbols [--csv-dir <folder>]

The tables are part of the schema of the text project: regenerate them only together with a
new `schemaVersion` and a migration.
"""

import argparse
import csv
import os
import re
import sys
import tempfile
import urllib.request

REPO = "https://github.com/PokeAPI/pokeapi"
RAW = "https://raw.githubusercontent.com/PokeAPI/pokeapi/{commit}/{path}"
CSV_DIR = "data/v2/csv"
CSV_FILES = [
    "pokemon_species.csv",
    "pokemon.csv",
    "moves.csv",
    "item_game_indices.csv",
    "items.csv",
    "abilities.csv",
    "type_game_indices.csv",
    "types.csv",
]
LICENSE = "LICENSE.md"

# Personal members 496-507 in the order of `crates/g4-kernel/metadata/species_forms.yaml`.
FORMS = [
    "deoxys-attack",
    "deoxys-defense",
    "deoxys-speed",
    "wormadam-sandy",
    "wormadam-trash",
    "giratina-origin",
    "shaymin-sky",
    "rotom-heat",
    "rotom-wash",
    "rotom-frost",
    "rotom-fan",
    "rotom-mow",
]
FIRST_FORM = 496
GEN = 4
# Ranges that must be complete (inclusive); items have gaps in the game.
COMPLETE = {"species": 507, "moves": 467, "abilities": 123, "types": 17}


class GenError(Exception):
    pass


def fetch(commit, dest):
    for path in [LICENSE] + [f"{CSV_DIR}/{name}" for name in CSV_FILES]:
        url = RAW.format(commit=commit, path=path)
        target = os.path.join(dest, os.path.basename(path))
        with urllib.request.urlopen(url, timeout=60) as resp, open(target, "wb") as out:
            out.write(resp.read())


def rows(csv_dir, name):
    with open(os.path.join(csv_dir, name), newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def ident(prefix, identifier):
    body = re.sub(r"[^A-Za-z0-9]+", "_", identifier).strip("_").upper()
    if not body:
        raise GenError(f"{prefix}: empty identifier from {identifier!r}")
    return f"{prefix}_{body}"


def species(csv_dir):
    table = {0: "SPECIES_NONE"}
    for r in rows(csv_dir, "pokemon_species.csv"):
        n = int(r["id"])
        if int(r["generation_id"]) <= GEN and n < 10000:
            table[n] = ident("SPECIES", r["identifier"])
    table[494] = "SPECIES_EGG"
    table[495] = "SPECIES_BAD_EGG"
    return table


def forms(csv_dir):
    by_identifier = {r["identifier"]: r for r in rows(csv_dir, "pokemon.csv")}
    table = {}
    for i, name in enumerate(FORMS):
        if name not in by_identifier:
            raise GenError(f"form {name} missing in pokemon.csv")
        table[FIRST_FORM + i] = ident("SPECIES", name)
    return table


def moves(csv_dir):
    table = {0: "MOVE_NONE"}
    for r in rows(csv_dir, "moves.csv"):
        n = int(r["id"])
        if int(r["generation_id"]) <= GEN and n < 10000:
            table[n] = ident("MOVE", r["identifier"])
    return table


def abilities(csv_dir):
    table = {0: "ABILITY_NONE"}
    for r in rows(csv_dir, "abilities.csv"):
        n = int(r["id"])
        if int(r["generation_id"]) <= GEN and r["is_main_series"] == "1" and n < 10000:
            table[n] = ident("ABILITY", r["identifier"])
    return table


def by_game_index(csv_dir, index_file, key, names_file, prefix, none):
    names = {r["id"]: r["identifier"] for r in rows(csv_dir, names_file)}
    table = {0: f"{prefix}_NONE"} if none else {}
    for r in rows(csv_dir, index_file):
        if int(r["generation_id"]) != GEN:
            continue
        n = int(r["game_index"])
        if n in table:
            raise GenError(f"{prefix}: game index {n} twice")
        table[n] = ident(prefix, names[r[key]])
    return table


def items(csv_dir):
    return by_game_index(csv_dir, "item_game_indices.csv", "item_id", "items.csv", "ITEM", True)


def types(csv_dir):
    # Type 0 is Normal in the game; there is no "none" type.
    table = by_game_index(csv_dir, "type_game_indices.csv", "type_id", "types.csv", "TYPE", False)
    return table, 9 not in table


def check(name, prefix, table, start=0):
    seen = {}
    for n, sym in table.items():
        if re.fullmatch(rf"{prefix}_[0-9]+", sym):
            raise GenError(f"{name}: {sym} looks like a fallback id")
        if sym in seen:
            raise GenError(f"{name}: {sym} for {seen[sym]} and {n}")
        seen[sym] = n
    last = COMPLETE.get(name)
    if last is not None:
        missing = [n for n in range(start, last + 1) if n not in table]
        if missing:
            raise GenError(f"{name}: gaps at {missing[:10]}")
        extra = [n for n in table if n > last]
        if extra:
            raise GenError(f"{name}: numbers past {last}: {extra[:10]}")


def write_yaml(path, commit, files, prefix, table):
    lines = [
        "# Generated by scripts/symbols/generate_symbols.py, do not edit by hand.",
        f"# Source: PokeAPI ({REPO}) commit {commit}, "
        + ", ".join(f"{CSV_DIR}/{name}" for name in files)
        + "; BSD-3-Clause, see LICENSE-PokeAPI.txt.",
        f"prefix: {prefix}",
        "ids:",
    ]
    lines += [f"  {n}: {table[n]}" for n in sorted(table)]
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def generate(commit, csv_dir, out):
    sp = species(csv_dir)
    fo = forms(csv_dir)
    mv = moves(csv_dir)
    it = items(csv_dir)
    ab = abilities(csv_dir)
    ty, mystery = types(csv_dir)
    if mystery:
        ty[9] = "TYPE_MYSTERY"
    check("species", "SPECIES", {**sp, **fo})
    check("moves", "MOVE", mv)
    check("items", "ITEM", it)
    check("abilities", "ABILITY", ab)
    check("types", "TYPE", ty)
    os.makedirs(out, exist_ok=True)
    write_yaml(os.path.join(out, "species.yaml"), commit, ["pokemon_species.csv"], "SPECIES", sp)
    write_yaml(os.path.join(out, "forms.yaml"), commit, ["pokemon.csv"], "SPECIES", fo)
    write_yaml(os.path.join(out, "moves.yaml"), commit, ["moves.csv"], "MOVE", mv)
    write_yaml(
        os.path.join(out, "items.yaml"),
        commit,
        ["item_game_indices.csv", "items.csv"],
        "ITEM",
        it,
    )
    write_yaml(os.path.join(out, "abilities.yaml"), commit, ["abilities.csv"], "ABILITY", ab)
    write_yaml(
        os.path.join(out, "types.yaml"),
        commit,
        ["type_game_indices.csv", "types.csv"],
        "TYPE",
        ty,
    )
    with open(os.path.join(csv_dir, LICENSE), encoding="utf-8") as f:
        license_text = f.read()
    if "Redistribution and use in source and binary forms" not in license_text:
        raise GenError("LICENSE.md is not the expected BSD-3-Clause text")
    with open(os.path.join(out, "LICENSE-PokeAPI.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write(
            f"The symbol tables in this folder are generated from PokeAPI ({REPO}),\n"
            f"commit {commit}, files {CSV_DIR}/{{{','.join(CSV_FILES)}}}.\n"
            "PokeAPI is licensed under the BSD 3-Clause License; its LICENSE.md follows.\n\n"
        )
        f.write(license_text.rstrip("\n") + "\n")
    print(
        f"species {len(sp)}, forms {len(fo)}, moves {len(mv)}, items {len(it)}, "
        f"abilities {len(ab)}, types {len(ty)}" + (" (own TYPE_MYSTERY 9)" if mystery else "")
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--commit", required=True, help="PokeAPI commit SHA (40 hex digits)")
    ap.add_argument("--out", required=True, help="output folder")
    ap.add_argument("--csv-dir", help="folder with LICENSE.md and the CSV (no download)")
    args = ap.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.commit):
        sys.exit("--commit must be a full 40-digit SHA")
    try:
        if args.csv_dir:
            generate(args.commit, args.csv_dir, args.out)
        else:
            with tempfile.TemporaryDirectory() as tmp:
                fetch(args.commit, tmp)
                generate(args.commit, tmp, args.out)
    except GenError as e:
        sys.exit(f"generate_symbols: {e}")


if __name__ == "__main__":
    main()
