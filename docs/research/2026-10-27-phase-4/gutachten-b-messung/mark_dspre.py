"""Writes a .g4/base.json with origin "dspre" into a DSPRE folder from its .nds (test copies only).
Usage: python3 -I mark_dspre.py <project> <rom.nds>"""
import hashlib, json, os, sys, struct
proj, rom = sys.argv[1], sys.argv[2]
b = open(rom, 'rb').read()
mark = {
  "version": 1, "origin": "dspre", "sha1": hashlib.sha1(b).hexdigest(),
  "game_code": b[0x0C:0x10].decode(), "title": b[0:12].rstrip(b'\0').decode(),
  "revision": b[0x1E], "size": len(b), "template": "original",
  "created": "2026-10-26T00:00:00Z", "studio_version": "gutachten-b",
  "secure_area_crc": struct.unpack_from('<H', b, 0x6C)[0],
  "secure_area_sha1": hashlib.sha1(b[0x4000:0x8000]).hexdigest(),
}
os.makedirs(os.path.join(proj, '.g4'), exist_ok=True)
open(os.path.join(proj, '.g4', 'base.json'), 'w').write(json.dumps(mark, indent=2) + "\n")
print(mark["sha1"], mark["game_code"], mark["secure_area_crc"])
