#!/usr/bin/env python3
"""Gutachten C, Phase 6: grobe Lage einer PDSMS-Tileset-Datei (`.pdsts`) messen.

Liest nur, gibt nur Zahlen aus (keine Namen, keine Geometrie). Aufruf:
    python3 -I gutachten-c-pdsts.py <datei.pdsts> [...]

Kein Code aus PDSMS (das Repo trägt keine Lizenz); die Messung schaut nur auf die Bytes:
erstes Wort, Zahl der eingebetteten Dateinamen `.png` (Texturen) und `.obj` (Kachel-Geometrie),
Anteil druckbarer Bytes und ob die Datei mit einem bekannten Container (ZIP, gzip, Java-Serialisierung,
JSON/XML) beginnt.
"""
import re
import struct
import sys

MAGIC = {b"PK\x03\x04": "zip", b"\x1f\x8b": "gzip", b"\xac\xed": "java-serialisierung",
         b"{": "json", b"<": "xml"}


def probe(path):
    d = open(path, "rb").read()
    kind = next((v for k, v in MAGIC.items() if d.startswith(k)), "eigenes binärformat")
    printable = sum(1 for c in d if 32 <= c < 127) / max(1, len(d))
    pngs = len(re.findall(rb"[\x20-\x7e]{1,64}\.png", d))
    objs = len(re.findall(rb"[\x20-\x7e]{1,64}\.obj", d))
    first = struct.unpack_from("<I", d, 0)[0] if len(d) >= 4 else None
    print(f"bytes {len(d)}  container {kind}  erstes u32 (LE) {first}  druckbar {100 * printable:.0f} %  "
          f".png-namen {pngs}  .obj-namen {objs}")


if __name__ == "__main__":
    for p in sys.argv[1:]:
        probe(p)
