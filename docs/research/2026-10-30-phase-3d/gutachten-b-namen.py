#!/usr/bin/env python3
"""Gutachten B (3d): Bereiche und freie Nummern der Flag- und Variablennamen aus den
scrcmd-Datenbanken (metadata/scrcmd/*.json). Liest nur die gebündelten JSON-Dateien.

Aufruf: python3 gutachten-b-namen.py <repo-root>
"""
import json, re, sys, collections
root = sys.argv[1] if len(sys.argv) > 1 else '.'

def runs(ids):
    ids = sorted(ids); out = []
    for i in ids:
        if out and out[-1][1] == i - 1: out[-1][1] = i
        else: out.append([i, i])
    return out

def group(name):
    m = re.match(r'(FLAG|VAR)_([A-Z]+(?:_[A-Z]+)?)', name)
    return m.group(2) if m else name

for f in ['platinum_v2.json', 'hgss_v2.json']:
    d = json.load(open(f'{root}/crates/g4-kernel/metadata/scrcmd/{f}'))
    for tab in ['flags', 'vars']:
        t = {k: v['id'] for k, v in d[tab].items()}
        pref = 'FLAG_' if tab == 'flags' else 'VAR_'
        named = {k: v for k, v in t.items() if k.startswith(pref)}
        print(f'== {f} {tab}: {len(t)} Einträge, {len(set(named.values()))} Nummern mit {pref}-Namen')
        # Bereiche: Nummern nach Gruppe des Namens, als Läufe
        by = collections.defaultdict(set)
        for k, v in named.items():
            g = k.split('_')[1]
            if g in ('DEFEATED',) and 'TRAINER' in k: g = 'DEFEATED_TRAINER'
            if g == 'OBTAINED' and 'HIDDEN' in k: g = 'OBTAINED_HIDDEN'
            if g == 'MAP' and 'LOCAL' in k: g = 'MAP_LOCAL'
            by[g].add(v)
        for g, ids in sorted(by.items(), key=lambda x: min(x[1])):
            r = runs(ids)
            if len(ids) >= 5:
                print(f'   {g:18s} n={len(ids):5d}  {min(ids):#06x}..{max(ids):#06x}  Läufe={len(r)}')
        unused = sorted(v for k, v in named.items() if 'UNUSED' in k)
        r = runs(unused)
        big = sorted(r, key=lambda x: x[0] - x[1])[:8]
        print(f'   UNUSED-Namen: {len(unused)} Nummern, {len(r)} Läufe, längste: ' +
              ', '.join(f'{a:#06x}..{b:#06x} ({b-a+1})' for a, b in big))
        # Lücken: Nummern ohne Namen im genutzten Bereich unterhalb der Grenzen
        lo = 0 if tab == 'flags' else 0x4000
        hi = max(v for v in named.values() if v < (0x4000 if tab == 'flags' else 0x8000))
        gaps = sorted(set(range(lo, hi + 1)) - set(named.values()))
        print(f'   ohne Namen in {lo:#06x}..{hi:#06x}: {len(gaps)}; Läufe ≥4: ' +
              ', '.join(f'{a:#06x}..{b:#06x}' for a, b in runs(gaps) if b - a >= 3)[:400])
        other = sorted((v, k) for k, v in t.items() if not k.startswith(pref))
        print('   Grenzen/Konstanten:', ', '.join(f'{k}={v:#x}' for v, k in other))
