#!/usr/bin/env python3
"""Gutachten C (Phase 4): inventory of a DSPRE folder for the Git question.

Usage: python3 -I gutachten-c-inventar.py <project> <archives.yaml> <family: pt|hgss>

Prints (JSON on stdout) per top-level entry the file count and the apparent bytes, and for
every folder below `unpacked/` the NARC below `files/` whose members are byte-identical
(found by parsing every file under `files/` that starts with the magic `NARC`), with the
note whether the studio's build packs that folder (`archives.yaml`). Reads only; writes
nothing. Prints no file contents, only counts, sizes and paths.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import struct
import sys
from pathlib import Path


def tree_stats(path: Path) -> tuple[int, int]:
    if path.is_file():
        return 1, path.stat().st_size
    files = size = 0
    for root, _dirs, names in os.walk(path):
        for n in names:
            files += 1
            size += os.path.getsize(os.path.join(root, n))
    return files, size


def narc_members(data: bytes) -> list[bytes] | None:
    if len(data) < 0x1C or data[:4] != b"NARC":
        return None
    header_size = struct.unpack_from("<H", data, 0x0C)[0]
    off = header_size
    if data[off : off + 4] != b"BTAF":
        return None
    btaf_size, count = struct.unpack_from("<IH", data, off + 4)
    entries = [struct.unpack_from("<II", data, off + 12 + 8 * i) for i in range(count)]
    off += btaf_size
    if data[off : off + 4] != b"BTNF":
        return None
    off += struct.unpack_from("<I", data, off + 4)[0]
    if data[off : off + 4] != b"GMIF":
        return None
    base = off + 8
    return [data[base + s : base + e] for s, e in entries]


def digest(chunks: list[bytes]) -> str:
    h = hashlib.sha1()
    for c in chunks:
        h.update(hashlib.sha1(c).digest())
    return h.hexdigest()


def studio_unpacked(archives_yaml: Path, family: str) -> dict[str, str]:
    """unpacked name -> packed path for one family (simple line parser, no YAML dependency)."""
    out: dict[str, str] = {}
    current = None
    for line in archives_yaml.read_text().splitlines():
        if re.match(r"^[a-z]+:\s*$", line):
            current = line.split(":")[0]
            continue
        if current != family:
            continue
        m = re.search(r'packed: "([^"]+)".*unpacked: "([^"]+)"', line)
        if m:
            out[m.group(2)] = m.group(1)
    return out


def main() -> int:
    root, archives_yaml, family = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
    top = {}
    for entry in sorted(root.iterdir()):
        files, size = tree_stats(entry)
        top[entry.name] = {"files": files, "bytes": size}

    narcs: dict[str, dict] = {}
    for p in sorted((root / "files").rglob("*")):
        if not p.is_file():
            continue
        data = p.read_bytes()
        members = narc_members(data)
        if members is None:
            continue
        narcs[str(p.relative_to(root))] = {
            "bytes": len(data),
            "members": len(members),
            "digest": digest(members),
        }

    by_digest: dict[str, list[str]] = {}
    for path, info in narcs.items():
        by_digest.setdefault(info["digest"], []).append(path)

    known = studio_unpacked(archives_yaml, family)
    dirs = []
    for d in sorted((root / "unpacked").iterdir()):
        if not d.is_dir():
            continue
        names = sorted(n for n in os.listdir(d) if not n.startswith("."))
        chunks = [(d / n).read_bytes() for n in names]
        dg = digest(chunks)
        twins = by_digest.get(dg, [])
        dirs.append(
            {
                "unpacked": d.name,
                "members": len(names),
                "bytes": sum(len(c) for c in chunks),
                "identical_narc": twins,
                "narc_bytes": narcs[twins[0]]["bytes"] if twins else None,
                "studio_packs": d.name in known,
                "studio_packed_path": known.get(d.name),
            }
        )

    files_total = tree_stats(root / "files")
    narc_total = sum(i["bytes"] for i in narcs.values())
    dup_bytes = sum(x["narc_bytes"] or 0 for x in dirs if x["identical_narc"])
    dup_studio = sum(x["narc_bytes"] or 0 for x in dirs if x["identical_narc"] and x["studio_packs"])
    summary = {
        "files_dir": {"files": files_total[0], "bytes": files_total[1]},
        "narcs_in_files": {"count": len(narcs), "bytes": narc_total},
        "unpacked_dirs": len(dirs),
        "unpacked_with_identical_narc": sum(1 for x in dirs if x["identical_narc"]),
        "narc_bytes_duplicated_by_unpacked": dup_bytes,
        "narc_bytes_duplicated_and_packed_by_studio": dup_studio,
        "studio_kinds_in_archives_yaml": len(known),
        "studio_kinds_present_as_unpacked": sum(1 for x in dirs if x["studio_packs"]),
    }
    json.dump({"top": top, "summary": summary, "unpacked": dirs}, sys.stdout, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
