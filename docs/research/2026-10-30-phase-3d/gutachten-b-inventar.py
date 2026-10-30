#!/usr/bin/env python3
"""Gutachten B (3d): Inventar der Flag- und Variablen-Verwendungen eines übernommenen
Projekts (frische Kopie nach `g4k text adopt`). Liest nur: die scrcmd-Datenbank des Repos,
die rotom-Quellen `expanded/scripts/*.rotom`, die Level-Scripts `expanded/scripts/*.json`
und die Events `data/events/*.json`. Gibt nur Zahlen und Namen aus, keine ROM-Inhalte.

Aufruf: python3 gutachten-b-inventar.py <repo-root> <projekt> pt|hgss [--json out.json]
"""
import collections, glob, json, os, re, sys, time

repo, proj, fam = sys.argv[1], sys.argv[2], sys.argv[3]
out_json = sys.argv[5] if len(sys.argv) > 5 and sys.argv[4] == '--json' else None
db = json.load(open(f"{repo}/crates/g4-kernel/metadata/scrcmd/{'platinum_v2' if fam == 'pt' else 'hgss_v2'}.json"))
FIRST_VAR = 0x4000
TRAINER_FLAG_BASE = 0x550  # aus hgss_v2.json; in Pt aus den Namen FLAG_DEFEATED_TRAINER_* abgeleitet
HIDDEN_BASE = 0x320 if fam == 'hgss' else 0x2DA  # HGSS: HIDDEN_ITEMS_FLAG_BASE; Pt: erster FLAG_OBTAINED_HIDDEN_*

# rotom v0.7.0 src/decompiler/ir_to_source.rs param_semantics (Namen, klein geschrieben)
FLAG_NAMES = {'flag', 'flagid', 'flag_id'}
VAR_NAMES = {'var', 'variable', 'var_0', 'var_1', 'var_2', 'var_3', 'var_4', 'var_5', 'var_6',
             'varid', 'var_id', 'destvarid', 'dest_var_id', 'variable_1', 'variable_2', 'retvar',
             'var_dest', 'var_result', 'var_or_addend', 'var_or_trno', 'countdown_variable',
             'var_flag'}
WRITE_FLAG_CMDS = {'SetFlag', 'ClearFlag'}
TRAINER_FLAG_CMDS = {'SetTrainerFlag': 'w', 'ClearTrainerFlag': 'w', 'CheckTrainerFlag': 'r'}

def param_kind(cmd, p):
    """('flag'|'var'|'flex'|'flagvar'|None, access) eines Parameters."""
    n, t, acc = p['name'].lower(), p['type'], p.get('access')
    if cmd in ('CheckFlagFromVar', 'SetFlagFromVar', 'SetFlagVar', 'ClearFlagVar', 'CheckFlagVar') and n in ('flagid', 'varid', 'flag', 'var_flag'):
        return 'flagvar', 'r'   # Flag-Nummer steht in einer Variable: dynamisch
    if t == 'flag' or (n in FLAG_NAMES and t in ('u16', 'flag')):
        a = 'w' if cmd in WRITE_FLAG_CMDS else 'r'
        return 'flag', a
    if t == 'var' or n in VAR_NAMES:
        a = {'read': 'r', 'must_write': 'w', 'may_write': 'w', 'read_must_write': 'rw',
             'read_may_write': 'rw', 'none': '-'}.get(acc, '?')
        return 'var', a
    if t == 'flex':
        return 'flex', 'r'
    return None, None

pt_db = json.load(open(f"{repo}/crates/g4-kernel/metadata/scrcmd/platinum_v2.json")) if fam == 'hgss' else None
HGSS_WRITE_FIRST = {'SetVar': 'w', 'CopyVar': 'w', 'SetOrCopyVar': 'w', 'AddVar': 'rw', 'SubVar': 'rw'}

def borrow(name, i, kind):
    # HGSS hat keine access-Angaben: Name des Befehls wie in Pt, sonst Zuweisungsbefehle
    if kind[0] not in ('var', 'flex') or kind[1] != '?':
        return kind
    if name in HGSS_WRITE_FIRST:
        return (kind[0], HGSS_WRITE_FIRST[name] if i == 0 else 'r')
    if name.startswith('Compare') or name.startswith('Check') and i == 0 and False:
        return (kind[0], 'r')
    pc = pt_db['commands'].get(name) if pt_db else None
    if pc and i < len(pc.get('params', [])):
        a = pc['params'][i].get('access')
        m = {'read': 'r', 'must_write': 'w', 'may_write': 'w', 'read_must_write': 'rw', 'read_may_write': 'rw', 'none': '-'}
        if a in m:
            return (kind[0], m[a])
    return kind

cmds = {}
for name, c in db['commands'].items():
    if c['type'] == 'movement':
        continue
    kinds = [param_kind(name, p) for p in c.get('params', [])]
    if fam == 'hgss':
        kinds = [borrow(name, i, k) for i, k in enumerate(kinds)]
    cmds[name] = (c, kinds)
    if c.get('legacy_name') and c['legacy_name'] not in db['commands']:
        cmds.setdefault(c['legacy_name'], (c, kinds))

# Befehle zählen
cnt = collections.Counter()
acc_known = collections.Counter()
for name, c in db['commands'].items():
    if c['type'] == 'movement':
        continue
    ks = {k for k, _ in cmds[name][1] if k}
    for k in ks:
        cnt[(c['type'], k)] += 1
    if any(k in ('var', 'flex') for k in ks):
        acc_known['mit access' if any(p.get('access') for p in c.get('params', [])) else 'ohne access'] += 1
print(f'[{fam}] Befehle mit Flag/Var-Parametern (Typ, Art): ' + ', '.join(f'{a}/{b}={n}' for (a, b), n in sorted(cnt.items())))
print(f'[{fam}] davon mit access-Angabe: {dict(acc_known)}')

names = {}
for tab in ('flags', 'vars'):
    for k, v in db[tab].items():
        names.setdefault(k, v['id'])
names.setdefault('VARS_START', 0x4000)
flag_name = {}
for k, v in db['flags'].items():
    if k.startswith('FLAG_'):
        flag_name.setdefault(v['id'], k)

num_re = re.compile(r'^(0x[0-9A-Fa-f]+|\d+)$')
line_re = re.compile(r'^\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*)$')
alias_re = re.compile(r'^\s*alias\s+(\S+)\s+as\s+(\S+)')

def val(tok, aliases):
    tok = tok.strip()
    if tok in aliases:
        tok = aliases[tok]
    if num_re.match(tok):
        return int(tok, 0)
    return names.get(tok)

uses = collections.defaultdict(list)   # ('F'|'V', n) -> [(file, line, cmd, access)]
dyn = 0; unresolved = collections.Counter(); trainer_dyn = 0
t0 = time.time()
files = sorted(glob.glob(f'{proj}/expanded/scripts/*.rotom'))
for path in files:
    fno = int(os.path.basename(path)[:4])
    aliases = {}
    in_movement = False
    for ln, line in enumerate(open(path, encoding='utf-8', errors='replace'), 1):
        s = line.split('//')[0].rstrip()
        m = alias_re.match(s)
        if m:
            aliases[m.group(2)] = m.group(1); continue
        if re.match(r'^(movement|actions)\b', s):
            in_movement = True; continue
        if re.match(r'^\S', s):
            in_movement = False; continue
        m = line_re.match(s)
        if not m or in_movement:
            continue
        cmd, rest = m.group(1), m.group(2)
        if cmd not in cmds:
            continue
        c, kinds = cmds[cmd]
        args = [a for a in rest.split(',')] if rest else []
        canon = cmd
        for i, (k, a) in enumerate(kinds):
            if not k or i >= len(args):
                continue
            v = val(args[i], aliases)
            if v is None:
                unresolved[k] += 1; continue
            if k == 'flag':
                uses[('F', v)].append((fno, ln, canon, a))
            elif k == 'flagvar':
                dyn += 1
                if v >= FIRST_VAR: uses[('V', v)].append((fno, ln, canon, 'r'))
            elif k in ('var', 'flex'):
                if v >= FIRST_VAR:
                    uses[('V', v)].append((fno, ln, canon, a))
        if canon in TRAINER_FLAG_CMDS and args:
            v = val(args[0], aliases)
            if v is not None and v < FIRST_VAR:
                uses[('F', TRAINER_FLAG_BASE + v)].append((fno, ln, canon, TRAINER_FLAG_CMDS[canon]))
            else:
                trainer_dyn += 1
took = time.time() - t0

def summary(kind, rng=None):
    keys = [k for k in uses if k[0] == kind and (rng is None or rng(k[1]))]
    occ = sum(len(uses[k]) for k in keys)
    w = sum(1 for k in keys for u in uses[k] if 'w' in u[3])
    r = sum(1 for k in keys for u in uses[k] if 'r' in u[3])
    q = sum(1 for k in keys for u in uses[k] if u[3] == '?')
    return len(keys), occ, r, w, q

print(f'[{fam}] Scripts: {len(files)} Quellen, gelesen in {took*1000:.0f} ms (Python)')
for label, kind, rng in [
    ('Flags gesamt', 'F', None),
    ('  Map-lokal 0x0001..0x003F/0x0040', 'F', lambda n: 1 <= n <= 0x40),
    ('  Trainer-Flags 0x0550..0x095F', 'F', lambda n: 0x550 <= n < 0x960),
    ('  Temp/Sonder ≥0x4000', 'F', lambda n: n >= 0x4000),
    ('Variablen gesamt', 'V', None),
    ('  Map-lokal/Temp 0x4000..0x401F', 'V', lambda n: 0x4000 <= n <= 0x401F),
    ('  Obj-Grafik 0x4020..0x402F', 'V', lambda n: 0x4020 <= n <= 0x402F),
    ('  gespeichert 0x4030..', 'V', lambda n: 0x4030 <= n < 0x8000),
    ('  Sonder/Script-lokal ≥0x8000', 'V', lambda n: n >= 0x8000),
]:
    d, o, r, w, q = summary(kind, rng)
    print(f'[{fam}] {label:34s} verschiedene={d:5d} Vorkommen={o:6d} lesend={r:6d} schreibend={w:6d} unbekannt={q:5d}')
print(f'[{fam}] Flag über Variable (dynamisch): {dyn}; Trainer-Flag über Variable: {trainer_dyn}; nicht aufgelöst: {dict(unresolved)}')
top = sorted(((len(v), k) for k, v in uses.items()), reverse=True)[:5]
print(f'[{fam}] häufigste: ' + ', '.join(f"{'Flag' if k[0]=='F' else 'Var'} {k[1]:#06x}×{n}" for n, k in top))
only_set = [k for k, v in uses.items() if k[0] == 'F' and all('w' in u[3] for u in v)]
only_check = [k for k, v in uses.items() if k[0] == 'F' and all('w' not in u[3] for u in v)]
print(f'[{fam}] Flags nur gesetzt: {len(only_set)}, nur geprüft: {len(only_check)}')

# Events
ev = collections.Counter(); ev_flags = collections.defaultdict(int); trig_vars = collections.defaultdict(int)
hidden = set(); trainer_obj = set()
for path in glob.glob(f'{proj}/data/events/*.json'):
    d = json.load(open(path))
    for o in d.get('object_events', []):
        ev['objects'] += 1
        if o['hidden_flag']:
            ev['objects_flag'] += 1; ev_flags[o['hidden_flag']] += 1
        if 3000 <= o['script'] <= 4999:
            trainer_obj.add(o['script'] - 3000 + 1)
        if 5000 <= o['script'] <= 6999:
            trainer_obj.add(o['script'] - 5000 + 1)
    for b in d.get('bg_events', []):
        if b['event_type'] == 2 and b['script'] >= 8000:
            ev['hidden'] += 1; hidden.add(b['script'] - 8000)
    for c in d.get('coord_events', []):
        ev['triggers'] += 1; trig_vars[c['var']] += 1
print(f'[{fam}] Events: Objekte={ev["objects"]} mit Flag={ev["objects_flag"]} (verschiedene Flags={len(ev_flags)}, '
      f'davon map-lokal ≤0x40: {sum(1 for f in ev_flags if f <= 0x40)}); versteckte Items={ev["hidden"]} '
      f'(verschiedene={len(hidden)}, Flags {HIDDEN_BASE + min(hidden):#06x}..{HIDDEN_BASE + max(hidden):#06x}); '
      f'Trigger={ev["triggers"]} (verschiedene Variablen={len(trig_vars)}, < 0x4000: {sum(1 for v in trig_vars if v < 0x4000)})')
print(f'[{fam}] Trainer hinter Objekt-Events: {len(trainer_obj)}')
# Level-Scripts
lv = 0; lv_vars = collections.Counter(); lv_files = 0
for path in glob.glob(f'{proj}/expanded/scripts/*.json'):
    d = json.load(open(path)); lv_files += 1
    for c in d.get('var_conditions', []):
        lv += 1; lv_vars[c['var']] += 1
print(f'[{fam}] Level-Scripts: {lv_files} Dateien, var_conditions={lv} (verschiedene Variablen={len(lv_vars)})')

# Vereinigung
all_flags = {k[1] for k in uses if k[0] == 'F'} | set(ev_flags) | {HIDDEN_BASE + h for h in hidden}
all_vars = {k[1] for k in uses if k[0] == 'V'} | set(trig_vars) | set(lv_vars)
print(f'[{fam}] Vereinigung: verschiedene Flags={len(all_flags)} (ohne Trainer-Flags {len([f for f in all_flags if not 0x550 <= f < 0x960])}), '
      f'verschiedene Variablen={len(all_vars)}')
unused_named_but_used = [f for f in all_flags if 'UNUSED' in flag_name.get(f, '')]
src = lambda f: ('S' if ('F', f) in uses else '') + ('E' if f in ev_flags else '')
print(f'[{fam}] als UNUSED benannt, aber benutzt: {len(unused_named_but_used)} ' + str([hex(f) + ':' + src(f) for f in sorted(unused_named_but_used)][:20]))
# Lesend/schreibend über alle Quellen: Event-Flags werden gelesen (Sichtbarkeit), versteckte Items
# und Trainer hinter Objekt-Events liest und setzt das Spiel selbst
readers = {k[1] for k, v in uses.items() if k[0] == 'F' and any('r' in u[3] for u in v)} | set(ev_flags) \
    | {HIDDEN_BASE + h for h in hidden} | {TRAINER_FLAG_BASE + t for t in trainer_obj}
writers = {k[1] for k, v in uses.items() if k[0] == 'F' and any('w' in u[3] for u in v)} \
    | {HIDDEN_BASE + h for h in hidden} | {TRAINER_FLAG_BASE + t for t in trainer_obj}
print(f'[{fam}] Flags gesetzt, nirgends gelesen: {len(writers - readers)}; gelesen, nirgends gesetzt: {len(readers - writers)}')
vr = {k[1] for k, v in uses.items() if k[0] == 'V' and any('r' in u[3] for u in v)} | set(trig_vars) | set(lv_vars)
vw = {k[1] for k, v in uses.items() if k[0] == 'V' and any('w' in u[3] for u in v)}
vs = lambda x: {v for v in x if 0x4030 <= v < 0x8000}
print(f'[{fam}] gespeicherte Variablen (0x4030..): gelesen={len(vs(vr))} geschrieben={len(vs(vw))} nur gelesen={len(vs(vr)-vs(vw))} nur geschrieben={len(vs(vw)-vs(vr))}')
free_unused = sorted(v for v, n in flag_name.items() if 'UNUSED' in n and v not in all_flags and v < 0x960)
def runs(ids):
    out = []
    for i in ids:
        if out and out[-1][1] == i - 1: out[-1][1] = i
        else: out.append([i, i])
    return out
r = sorted(runs(free_unused), key=lambda x: x[0] - x[1])
print(f'[{fam}] freie Flags (UNUSED-Name, nicht benutzt, < 0x960): {len(free_unused)}, Läufe {len(r)}, längste: ' + ', '.join(f'{a:#06x}..{b:#06x}({b-a+1})' for a, b in r[:6]))
var_name = {}
for k, v in db['vars'].items():
    if k.startswith('VAR_'): var_name.setdefault(v['id'], k)
free_v = sorted(v for v, n in var_name.items() if 'UNUSED' in n and v not in all_vars)
print(f'[{fam}] freie Variablen (UNUSED-Name, nicht benutzt): {len(free_v)} ' + ', '.join(f'{a:#06x}..{b:#06x}' for a, b in runs(free_v)))
# Kein Name in der Datenbank, im gespeicherten Bereich, nicht benutzt
vend = 0x4120 if fam == 'pt' else 0x4170
nameless_v = [v for v in range(0x4030, vend) if v not in var_name and v not in all_vars]
unk_v = [v for v in range(0x4030, vend) if 'UNK' in var_name.get(v, '') and v not in all_vars]
print(f'[{fam}] Variablen 0x4030..{vend-1:#06x}: ohne Namen und unbenutzt={len(nameless_v)}, UNK-Name und unbenutzt={len(unk_v)}')
unk_f = [v for v, n in flag_name.items() if 'UNK' in n and v not in all_flags and 0x41 <= v < 0x550]
print(f'[{fam}] Flags 0x0041..0x054F mit UNK-Name und unbenutzt: {len(unk_f)}')
if out_json:
    json.dump({'flags': sorted(all_flags), 'vars': sorted(all_vars)}, open(out_json, 'w'))
