#!/usr/bin/env python3
"""Compare the text projects of two exported projects (Gate 4.3a).

    python3 -I scripts/agent/textproject_diff.py <export A> <export B>

Compares `data/` of both project roots without `data/ids/`. Trainers, trainer classes, places
and headers are baptised in the language of each project, so before comparing, every string
that a register of the project's own `data/ids/` knows (`MAP_*`, `TRAINER_*`,
`TRAINER_CLASS_*`, `PLACE_*`) is replaced by its number, in each project with its own
registers. JSON files are compared as values, every other file (side files, blobs) byte by
byte. Then the files under `expanded/textArchives/` are compared byte by byte and only counted.

Prints every differing path under `data/` (also a file that exists on one side only, marked
`only in A` or `only in B`), then a summary line for `data/` and one for
`expanded/textArchives/`. Exit code 0 after a comparison (whatever it found), 2 on a usage
error or an unreadable project. Standard library only.
"""

import json
import os
import sys

REGISTERS = ("maps", "trainers", "trainer_classes", "places")


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(2)


def load_registers(root):
    """ID -> number over every register of `root`; an ID in two registers is an error."""
    numbers = {}
    for name in REGISTERS:
        path = os.path.join(root, "data", "ids", f"{name}.json")
        if not os.path.isfile(path):
            fail(f"{path}: missing (is this an exported project?)")
        with open(path, "rb") as f:
            try:
                register = json.load(f)
            except ValueError as e:
                fail(f"{path}: {e}")
        if not isinstance(register, dict):
            fail(f"{path}: not an object of ID -> number")
        for ident, number in register.items():
            if not isinstance(number, int):
                fail(f"{path}: {ident} has no number")
            if ident in numbers:
                fail(f"{path}: {ident} is in two registers")
            numbers[ident] = number
    return numbers


def replace_ids(value, numbers):
    """`value` with every string that `numbers` knows replaced by its number."""
    if isinstance(value, str):
        return numbers.get(value, value)
    if isinstance(value, list):
        return [replace_ids(v, numbers) for v in value]
    if isinstance(value, dict):
        return {k: replace_ids(v, numbers) for k, v in value.items()}
    return value


def files_below(root, rel, skip=()):
    """Relative paths (with `/`) of every file below `root/rel`, without the folders in `skip`
    and without hidden entries (`.DS_Store` and the like, which the export ignores too)."""
    out = set()
    base = os.path.join(root, rel)
    if not os.path.isdir(base):
        return out
    for dirpath, dirnames, filenames in os.walk(base):
        here = os.path.relpath(dirpath, root).replace(os.sep, "/")
        dirnames[:] = [
            d for d in dirnames if not d.startswith(".") and f"{here}/{d}" not in skip
        ]
        for name in filenames:
            if not name.startswith("."):
                out.add(f"{here}/{name}")
    return out


def read(root, rel):
    with open(os.path.join(root, rel), "rb") as f:
        return f.read()


def same_data_file(a, b, rel, numbers_a, numbers_b):
    bytes_a, bytes_b = read(a, rel), read(b, rel)
    if not rel.endswith(".json"):
        return bytes_a == bytes_b
    try:
        value_a = json.loads(bytes_a)
        value_b = json.loads(bytes_b)
    except ValueError as e:
        fail(f"{rel}: {e}")
    return replace_ids(value_a, numbers_a) == replace_ids(value_b, numbers_b)


def main(argv):
    if len(argv) != 3 or argv[1].startswith("-"):
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        sys.exit(2)
    a, b = os.path.abspath(argv[1]), os.path.abspath(argv[2])
    for root in (a, b):
        if not os.path.isdir(os.path.join(root, "data")):
            fail(f"{root}: no data/ (export the text project first)")
    numbers_a, numbers_b = load_registers(a), load_registers(b)

    skip = {"data/ids"}
    data_a, data_b = files_below(a, "data", skip), files_below(b, "data", skip)
    differing = 0
    for rel in sorted(data_a | data_b):
        if rel not in data_b:
            print(f"{rel} (only in A)")
        elif rel not in data_a:
            print(f"{rel} (only in B)")
        elif same_data_file(a, b, rel, numbers_a, numbers_b):
            continue
        else:
            print(rel)
        differing += 1

    texts = "expanded/textArchives"
    texts_a, texts_b = files_below(a, texts), files_below(b, texts)
    differing_texts = sum(
        1
        for rel in texts_a | texts_b
        if rel not in texts_a or rel not in texts_b or read(a, rel) != read(b, rel)
    )
    print(
        f"data: {differing} of {len(data_a | data_b)} files differ "
        f"(A {len(data_a)}, B {len(data_b)}; data/ids not compared)"
    )
    print(
        f"{texts}: {differing_texts} of {len(texts_a | texts_b)} files differ "
        f"(A {len(texts_a)}, B {len(texts_b)})"
    )


if __name__ == "__main__":
    main(sys.argv)
