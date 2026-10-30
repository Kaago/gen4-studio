#!/usr/bin/env python3
"""Gutachten B (3d): Lage von Flags und Variablen im RAM des laufenden Spiels (g4emu, melonDS).

Startet die ROM mit einer Kopie des Spielstands, `continue`, sucht den Bereich Variablen+Flags
des Spielstands im System-RAM, prüft den Zeiger aus games.yaml (`location.pointer`) als Basis,
verfolgt die Lage über einen Kartenwechsel und schreibt zur Probe ein Flag bzw. eine Variable.
Gibt nur Adressen, Zahlen und Flag-Nummern aus.

Aufruf (aus scripts/g4emu): uv run python <dieses Skript> <rom.nds> <save.sav> <out-dir> <modus>
Modi: find | flagtest | vartest | stable
"""
import json, shutil, struct, sys, time
from pathlib import Path

from g4emu import movement
from g4emu.emu import open_emu, game_code
from g4emu.games import load_games, read_location, RAM_BASE
from g4emu.paths import default_core

# Offsets im allgemeinen Block des Spielstands (PKHeX SAV4Pt/SAV4HGSS, aus der Erinnerung; hier
# gegen RAM und Spielstand geprüft): Variablen (u16 je Variable ab 0x4000), danach die Flags.
LAYOUT = {
    'CPUD': {'vars': 0xDAC, 'nvars': 288, 'flags': 0xFEC, 'nflagbytes': 364, 'slot': 0x40000, 'general': 0xCF2C, 'ctr': 0x14},
    'IPKE': {'vars': 0xDE4, 'nvars': 368, 'flags': 0x10C4, 'nflagbytes': 364, 'slot': 0x40000, 'general': 0xF628, 'ctr': 0x10},
    'IPKD': {'vars': 0xDE4, 'nvars': 368, 'flags': 0x10C4, 'nflagbytes': 364, 'slot': 0x40000, 'general': 0xF628, 'ctr': 0x10},
}

rom, save, out, mode = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4]
out.mkdir(parents=True, exist_ok=True)
code = game_code(rom)
game = load_games()[code]
lay = LAYOUT[code]
save_copy = out / 'probe.sav'
shutil.copyfile(save, save_copy)
sav = save_copy.read_bytes()

def u32(b, a): return struct.unpack_from('<I', b, a)[0]

def save_slot():
    # Neuere Hälfte nach dem Speicherzähler im Fuß des allgemeinen Blocks (u32 bei general-0x14)
    best = None
    for base in (0, lay['slot']):
        ctr = u32(sav, base + lay['general'] - lay['ctr'])
        if ctr != 0xFFFFFFFF and (best is None or ctr > best[0]):
            best = (ctr, base)
    return best

def region(buf, base):
    return bytes(buf[base + lay['vars']: base + lay['flags'] + lay['nflagbytes']])

def flag(buf, base, n):
    return (buf[base + lay['flags'] + n // 8] >> (n % 8)) & 1

def var(buf, base, n):
    return struct.unpack_from('<H', buf, base + lay['vars'] + 2 * (n - 0x4000))[0]

# Der Zeiger aus games.yaml (`location.pointer`) zeigt vor den allgemeinen Block: Platin 0x14 Byte,
# HeartGold 0x10 Byte (gemessen: dort beginnt der Bereich, der Byte für Byte dem Spielstand gleicht).
HDR = {'CPUD': 0x14, 'IPKD': 0x10, 'IPKE': 0x10}[code]

def ptr_of(ram):
    return u32(ram, game.pointer - RAM_BASE) + HDR

result = {'code': code, 'mode': mode}
with open_emu(default_core(), rom, save_copy, out / 'work') as emu:
    t = time.time()
    loc = movement.continue_game(emu, game)
    result['continue'] = {'header': loc.header, 'x': loc.x, 'y': loc.y, 'frame': emu.frame, 's': round(time.time() - t, 1)}
    ram = bytes(emu.ram())
    ctr, sbase = save_slot()
    result['save_slot'] = hex(sbase)
    sreg = region(sav, sbase)
    p = ptr_of(ram)
    result['pointer'] = hex(p)
    # 1. Suche: der ganze Bereich des Spielstands im RAM?
    hits = []
    i = ram.find(sreg)
    while i >= 0:
        hits.append(i); i = ram.find(sreg, i + 1)
    result['exact_hits'] = [hex(RAM_BASE + h) for h in hits][:5]
    # 2. Teilsuche (map-lokale Werte ändern sich beim Laden): 64-Byte-Fenster der Flags
    fl = sav[sbase + lay['flags'] + 16: sbase + lay['flags'] + 80]
    j = ram.find(fl)
    result['flag_window_hit'] = hex(RAM_BASE + j - 16 - (lay['flags'] - lay['vars'])) if j >= 0 else None
    hits_all = []
    k = ram.find(fl)
    while k >= 0 and len(hits_all) < 10:
        hits_all.append(hex(RAM_BASE + k - 16 - (lay['flags'] - lay['vars']))); k = ram.find(fl, k + 1)
    result['flag_window_hits'] = hits_all
    result['raw_pointer'] = hex(u32(ram, game.pointer - RAM_BASE))
    base_ram = p - RAM_BASE
    for b in (0, lay['slot']):
        result[f'slot_{b:#x}_counter'] = hex(u32(sav, b + lay['general'] - lay['ctr']))
        result[f'slot_{b:#x}_exact_hit'] = hex(RAM_BASE + ram.find(region(sav, b))) if ram.find(region(sav, b)) >= 0 else None
        result[f'slot_{b:#x}_diff_at_pointer'] = sum(1 for x, y in zip(region(ram, base_ram), region(sav, b)) if x != y)
    result['region_at_pointer_equals_save'] = region(ram, base_ram) == sreg
    diff = [k for k in range(len(sreg)) if region(ram, base_ram)[k] != sreg[k]]
    result['region_diff_bytes'] = len(diff)
    result['vars_addr'] = hex(p + lay['vars'])
    result['flags_addr'] = hex(p + lay['flags'])
    result['location_addr'] = hex(p - HDR + game.offset)
    set_flags = [n for n in range(lay['nflagbytes'] * 8) if flag(ram, base_ram, n)]
    result['flags_set'] = len(set_flags)
    result['flags_set_sample'] = [hex(n) for n in set_flags[:12]]
    result['vars_nonzero'] = sum(1 for n in range(0x4000, 0x4000 + lay['nvars']) if var(ram, base_ram, n))
    json.dump(sorted(set_flags), open(out / 'flags_set.json', 'w'))
    json.dump({hex(n): var(ram, base_ram, n) for n in range(0x4000, 0x4000 + lay['nvars']) if var(ram, base_ram, n)}, open(out / 'vars.json', 'w'))
    emu.screenshot(out / 'start.png')

    if mode == 'stable':
        # Zeiger und Bereich über einen Kartenwechsel und 3000 Bilder verfolgen
        seen = set(); moved = []
        def sample(tag):
            r = bytes(emu.ram()); q = ptr_of(r); seen.add(q)
            moved.append((tag, hex(q), emu.frame, read_location(r, game).header if read_location(r, game) else None))
        sample('start')
        if code == 'CPUD':
            movement.walk_to(emu, game, 8, 4); sample('walk_to'); movement.walk(emu, game, 'right', 1)
            movement.wait_for_map(emu, game, 414, 600); emu.step(150); sample('414')
            movement.walk(emu, game, 'left', 1); emu.step(150); sample('414b')
        else:
            movement.walk(emu, game, 'left', 1); movement.walk(emu, game, 'down', 1)
            movement.wait_for_map(emu, game, 63, 600); emu.step(150); sample('63')
        for k in range(10):
            emu.step(300); sample(f'+{(k+1)*300}')
        result['pointer_samples'] = moved
        result['pointer_distinct'] = sorted(hex(s) for s in seen)
        r = bytes(emu.ram())
        result['flag_mirror_hit_after'] = hex(RAM_BASE + r.find(fl)) if r.find(fl) >= 0 else None
        emu.screenshot(out / 'after.png')

    if mode in ('flagtest', 'control'):
        # Pt: Flag 0x173 versteckt das Objekt 0 im Zimmer 415 (x 10, z 4); HGSS: 0x2A9..0x2AB die
        # Objekte im Zimmer 64 (x 7..9, z 10). Flag löschen, Karte verlassen und wieder betreten.
        fl_n = [0x173] if code == 'CPUD' else [0x2A9, 0x2AA, 0x2AB]
        mem = emu.ram()
        result['before'] = {hex(n): flag(mem, base_ram, n) for n in fl_n}
        if mode == 'flagtest':
            for n in fl_n:
                a = base_ram + lay['flags'] + n // 8
                mem[a] = mem[a] & ~(1 << (n % 8))
        result['after_write'] = {hex(n): flag(emu.ram(), base_ram, n) for n in fl_n}
        if code == 'CPUD':
            movement.walk_to(emu, game, 8, 4); movement.walk(emu, game, 'right', 1)
            movement.wait_for_map(emu, game, 414, 600); emu.step(200)
            movement.walk(emu, game, 'left', 1); movement.wait_for_map(emu, game, 415, 600); emu.step(200)
        else:
            movement.walk(emu, game, 'left', 1); movement.walk(emu, game, 'down', 1)
            movement.wait_for_map(emu, game, 63, 600); emu.step(200)
            tried = []
            for d in ('left', 'down', 'up', 'right', 'left', 'left'):
                try:
                    movement.walk(emu, game, d, 1)
                except Exception:
                    pass
                emu.step(120)
                l = read_location(emu.ram(), game)
                tried.append((d, l.header, l.x, l.y))
                if l.header == 64:
                    break
            result['way_back'] = tried
            emu.step(200)
        r = bytes(emu.ram())
        result['flags_after_reentry'] = {hex(n): flag(r, ptr_of(r) - RAM_BASE, n) for n in fl_n}
        result['loc_after'] = str(read_location(r, game))
        emu.screenshot(out / f'{mode}.png')

    if mode == 'vartest':
        # Pt: Trigger im Zimmer 415 bei (4,5),(3,6),(5,6),(4,7) feuert bei Variable 0x40A5 == 0.
        mem = emu.ram()
        n = 0x40A5
        result['var_before'] = var(mem, base_ram, n)
        a = base_ram + lay['vars'] + 2 * (n - 0x4000)
        mem[a] = 0; mem[a + 1] = 0
        result['var_after_write'] = var(emu.ram(), base_ram, n)
        if sys.argv[5:] == ['control']:
            mem[a] = result['var_before'] & 0xFF; mem[a + 1] = result['var_before'] >> 8
        result['var_set'] = var(emu.ram(), base_ram, n)
        try:
            l2 = movement.walk(emu, game, 'down', 1)
            result['walk_down'] = str(l2)
            for i in range(3):   # nach links bis (4,5), eine Trigger-Kachel
                l2 = movement.walk(emu, game, 'left', 1)
                result[f'left_{i}'] = str(l2)
            emu.step(60)
            l2 = movement.walk(emu, game, 'down', 1)
            result['walk_down2'] = str(l2)
        except Exception as e:  # ein laufendes Script hält die Figur fest
            result['walk_error'] = str(e)
        emu.step(120)
        emu.screenshot(out / 'vartest.png')
        r = bytes(emu.ram())
        result['var_end'] = var(r, ptr_of(r) - RAM_BASE, n)
        result['loc_end'] = str(read_location(r, game))

print(json.dumps(result, indent=1))
json.dump(result, open(out / 'result.json', 'w'), indent=1)
