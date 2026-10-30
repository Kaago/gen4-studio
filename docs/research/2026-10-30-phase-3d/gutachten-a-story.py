#!/usr/bin/env python3
"""Gutachten A (Phase 3d): Zahlen zu Flags, Variablen und Story-Fortschritt.

Liest die Befehlsdatenbank (metadata/scrcmd/*.json) und ein als Textprojekt eingerichtetes
Testprojekt (`g4k text adopt`): rotom-Quellen unter expanded/scripts, Level-Scripts (.json),
Eventdateien und Header unter data/. Gibt nur Zahlen und Namen von Befehlen und
Kategorien aus, keine ROM-Inhalte (keine Texte, keine Bytes, keine Flag-Namen aus pret).

Aufruf: python3 -I gutachten-a-story.py <projekt> <pt|hg> [--json]
"""
import collections
import glob
import json
import os
import re
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
DB = {"pt": "platinum_v2.json", "hg": "hgss_v2.json"}

# Erste Regel gewinnt.
FLAG_RULES = [
    ("konstante", re.compile(r"^(?!FLAG_)")),
    ("ungenutzt", re.compile(r"UNUSED|DUMMY")),
    ("temporaer", re.compile(r"MAP_LOCAL|MAPTEMP|TEMP")),
    ("trainer", re.compile(r"DEFEATED_TRAINER|TRAINER_FLAG")),
    ("verstecktes_item", re.compile(r"OBTAINED_HIDDEN|HIDDENITEM")),
    ("sichtbarkeit", re.compile(r"HIDE_")),
    ("taeglich", re.compile(r"DAILY")),
    ("system", re.compile(r"^FLAG_SYS_|FIRST_ARRIVAL")),
    ("unbekannt", re.compile(r"^FLAG_UNK|_UNK_|_0x[0-9A-Fa-f]+$|_x[0-9A-Fa-f]+$")),
]
VAR_RULES = [
    ("konstante", re.compile(r"^(?!VAR_)|_START$|_END$|_BASE$|^VAR_BASE$")),
    ("ungenutzt", re.compile(r"UNUSED|DUMMY")),
    ("temporaer", re.compile(r"MAP_LOCAL|TEMP|SPECIAL")),
    ("objekt_grafik", re.compile(r"OBJ_GFX|^VAR_OBJ_\d")),
    ("szene_zustand", re.compile(r"SCENE|_STATE$|_STATE_|TRIGGER")),
    ("unbekannt", re.compile(r"^VAR_UNK|_UNK_|_0x[0-9A-Fa-f]+$|_x[0-9A-Fa-f]+$")),
]

DIALOG = re.compile(r"(Message|Msg)")
DIALOG_NOT = re.compile(r"^(Close|Wait|Get|Buffer|Set|Hide|Show.*Menu)")
CHECK_FLAG = {"CheckFlag", "CheckTrainerFlag"}
SET_FLAG = {"SetFlag", "ClearFlag", "SetTrainerFlag", "ClearTrainerFlag"}
CMP_VAR = {"CompareVarToValue", "CompareVarToVar"}
SET_VAR = {"SetVarFromValue", "SetVarFromVar", "SetVar", "CopyVar", "AddVar", "SubVar",
           "AddVarFromValue", "SubVarFromValue"}
BRANCH = {"GoToIf", "CallIf"}
BATTLE = re.compile(r"Battle")
REWARD = re.compile(r"^(Give|AddItem|AddMoney|GiveMoney|ReceiveItem|AddPokemon|GivePokemon|"
                    r"GiveEgg|AddEgg|AddMon|GiveMon|AddDecoration|AddAccessory|GiveAccessory|"
                    r"AddMail|RegisterPoketchApp|GivePoketch|AddBadge|SetBadge|GiveBadge)")


def classify(name, rules):
    for cat, rx in rules:
        if rx.search(name):
            return cat
    return "benannt"


def load_db(game):
    with open(os.path.join(REPO, "crates/g4-kernel/metadata/scrcmd", DB[game])) as f:
        d = json.load(f)
    return d


def db_counts(d):
    out = {}
    fl = d["flags"]
    cats = collections.Counter(classify(k, FLAG_RULES) for k in fl)
    out["flags_namen"] = len(fl)
    out["flags_ids"] = len({v["id"] for v in fl.values()})
    out["flags_kategorien"] = dict(cats)
    out["flags_sprechend"] = sum(n for c, n in cats.items()
                                 if c not in ("ungenutzt", "unbekannt", "temporaer", "konstante"))
    vs = d["vars"]
    vcats = collections.Counter(classify(k, VAR_RULES) for k in vs)
    out["vars_namen"] = len(vs)
    out["vars_kategorien"] = dict(vcats)
    out["vars_sprechend"] = vcats.get("benannt", 0) + vcats.get("szene_zustand", 0)
    out["kommandos"] = len(d["commands"])
    return out


def id_names(table):
    m = {}
    for k, v in table.items():
        m.setdefault(v["id"], k)
    return m


def resolver(table):
    by_name = {k: v["id"] for k, v in table.items()}

    def to_id(arg):
        if arg in by_name:
            return by_name[arg]
        try:
            return int(arg, 0)
        except ValueError:
            return None
    return to_id


def parse_rotom(path):
    """Yield (command, args) per command line."""
    with open(path, encoding="utf-8") as f:
        for line in f:
            if not line.startswith((" ", "\t")):
                continue
            s = line.strip()
            if not s or s.startswith(("//", "#")):
                continue
            parts = s.split(None, 1)
            cmd = parts[0]
            args = [a.strip() for a in parts[1].split(",")] if len(parts) > 1 else []
            yield cmd, args


def main():
    root, game = sys.argv[1], sys.argv[2]
    as_json = "--json" in sys.argv
    d = load_db(game)
    res = {"spiel": game, "datenbank": db_counts(d)}
    flag_by_id = id_names(d["flags"])
    var_by_id = id_names(d["vars"])
    flag_cat = {k: classify(k, FLAG_RULES) for k in d["flags"]}
    var_cat = {k: classify(k, VAR_RULES) for k in d["vars"]}

    src = os.path.join(root, "expanded/scripts")
    rotoms = sorted(glob.glob(os.path.join(src, "*.rotom")))
    levels = sorted(glob.glob(os.path.join(src, "*.json")))

    # --- Scripts ---
    cmd_count = collections.Counter()
    node = collections.Counter()
    flag_set = collections.defaultdict(set)    # flag -> files
    flag_chk = collections.defaultdict(set)
    var_set = collections.defaultdict(set)
    var_cmp = collections.defaultdict(set)
    numeric_flag_args = 0
    numeric_var_args = 0
    scripts_declared = 0
    battle_cmds = collections.Counter()
    reward_cmds = collections.Counter()
    dialog_cmds = collections.Counter()
    trainer_numeric = 0
    files_with = collections.Counter()
    for p in rotoms:
        fid = int(os.path.basename(p)[:4])
        kinds = set()
        with open(p, encoding="utf-8") as f:
            scripts_declared += sum(1 for line in f if line.startswith("script "))
        for cmd, args in parse_rotom(p):
            cmd_count[cmd] += 1
            a0 = args[0] if args else ""
            if DIALOG.search(cmd) and not DIALOG_NOT.search(cmd):
                node["dialog"] += 1
                dialog_cmds[cmd] += 1
                kinds.add("dialog")
            if cmd in CHECK_FLAG or cmd in CMP_VAR or cmd in BRANCH:
                node["bedingung"] += 1
                kinds.add("bedingung")
            if BATTLE.search(cmd) and cmd.startswith(("Start", "Trainer", "Wild", "Battle")) \
                    and not cmd.startswith(("BattleTower", "BattleFrontier")):
                node["kampf"] += 1
                battle_cmds[cmd] += 1
                kinds.add("kampf")
                if cmd in ("StartTrainerBattle", "TrainerBattle") and a0[:1].isdigit():
                    trainer_numeric += 1
            if REWARD.search(cmd):
                node["belohnung"] += 1
                reward_cmds[cmd] += 1
                kinds.add("belohnung")
            if cmd in SET_FLAG:
                node["zustand_setzen"] += 1
                kinds.add("zustand")
                if a0.startswith("FLAG_"):
                    flag_set[a0].add(fid)
                elif a0[:1].isdigit():
                    numeric_flag_args += 1
            if cmd in CHECK_FLAG:
                if a0.startswith("FLAG_"):
                    flag_chk[a0].add(fid)
                elif a0[:1].isdigit():
                    numeric_flag_args += 1
            if cmd in SET_VAR:
                node["zustand_setzen"] += 1
                kinds.add("zustand")
                if a0.startswith("VAR_"):
                    var_set[a0].add(fid)
                elif a0[:1].isdigit():
                    numeric_var_args += 1
            if cmd in CMP_VAR:
                if a0.startswith("VAR_"):
                    var_cmp[a0].add(fid)
                elif a0[:1].isdigit():
                    numeric_var_args += 1
        for k in kinds:
            files_with[k] += 1

    # --- Level scripts ---
    lv_nonempty = 0
    entry_types = collections.Counter()
    var_conditions = 0
    lv_vars = collections.defaultdict(set)
    for p in levels:
        fid = int(os.path.basename(p)[:4])
        with open(p) as f:
            doc = json.load(f)
        he = doc.get("header_entries", [])
        vc = doc.get("var_conditions", [])
        if he or vc:
            lv_nonempty += 1
        for e in he:
            entry_types[e.get("type")] += 1
        for c in vc:
            var_conditions += 1
            name = var_by_id.get(c["var"], "#%d" % c["var"])
            lv_vars[name].add(fid)

    # --- Events ---
    obj = obj_hidden = obj_trainer = obj_script = 0
    hidden_kind = collections.Counter()
    hidden_flags = collections.defaultdict(set)
    script_ranges = collections.Counter()
    coord = 0
    coord_vars = collections.defaultdict(set)
    bg = 0
    bg_types = collections.Counter()
    warps = 0
    for p in sorted(glob.glob(os.path.join(root, "data/events/*.json"))):
        fid = int(os.path.basename(p)[:4])
        with open(p) as f:
            e = json.load(f)
        for o in e.get("object_events", []):
            obj += 1
            if o.get("hidden_flag"):
                obj_hidden += 1
                hidden_flags[flag_by_id.get(o["hidden_flag"], "#%d" % o["hidden_flag"])].add(fid)
                fname = flag_by_id.get(o["hidden_flag"], "")
                hidden_kind[("temporaer" if re.search("MAP_LOCAL|MAPTEMP|TEMP", fname) else
                             "gegenstand" if re.search("OBTAINED|ITEMBALL", fname) else
                             "hide" if "HIDE_" in fname else "sonst")] += 1
            if o.get("trainer_type"):
                obj_trainer += 1
            s = o.get("script", 0)
            if s:
                obj_script += 1
                script_ranges[("<2000" if s < 2000 else "2000-2999" if s < 3000 else
                               "3000-6999" if s < 7000 else "7000-7999" if s < 8000 else
                               "8000-9999" if s < 10000 else ">=10000")] += 1
        for c in e.get("coord_events", []):
            coord += 1
            name = var_by_id.get(c["var"], "#%d" % c["var"])
            coord_vars[name].add(fid)
        for b in e.get("bg_events", []):
            bg += 1
            bg_types[b.get("event_type")] += 1
        warps += len(e.get("warp_events", []))

    # Kategorien der benutzten Namen
    def cats(names, table):
        return dict(collections.Counter(table.get(n, "ohne_namen") for n in names))

    all_flags_used = set(flag_set) | set(flag_chk) | set(hidden_flags)
    all_vars_used = set(var_set) | set(var_cmp) | set(lv_vars) | set(coord_vars)

    # Header: Scriptdatei, Level-Script und Eventdatei je Header -> "Ort" der Datei
    script_to_hdr = collections.defaultdict(set)
    level_to_hdr = collections.defaultdict(set)
    event_to_hdr = collections.defaultdict(set)
    for p in glob.glob(os.path.join(root, "data/headers/*.json")):
        with open(p) as f:
            h = json.load(f)
        hid = int(os.path.basename(p)[:4])
        script_to_hdr[h.get("script_file_id")].add(hid)
        level_to_hdr[h.get("level_script_id")].add(hid)
        event_to_hdr[h.get("event_file_id")].add(hid)

    def places(files, mapping):
        out = set()
        for f in files:
            hdrs = mapping.get(f)
            out |= set(hdrs) if hdrs else {("datei", f)}
        return out

    # Fäden: ein Flag oder eine Variable, die in einer Datei gesetzt und in einer anderen
    # Datei geprüft bzw. sichtbar gemacht wird (über Ortsgrenzen).
    flag_cross = 0
    flag_set_only = flag_chk_only = 0
    for fl in all_flags_used:
        s = flag_set.get(fl, set())
        c = flag_chk.get(fl, set())
        h = hidden_flags.get(fl, set())
        if s and not (c or h):
            flag_set_only += 1
        if (c or h) and not s:
            flag_chk_only += 1
        src_places = places(s, script_to_hdr)
        use_places = places(c, script_to_hdr) | places(h, event_to_hdr)
        if s and use_places and (use_places - src_places):
            flag_cross += 1
    var_cross = 0
    for v in all_vars_used:
        s = var_set.get(v, set())
        use = places(var_cmp.get(v, set()), script_to_hdr) | places(lv_vars.get(v, set()), level_to_hdr) \
            | places(coord_vars.get(v, set()), event_to_hdr)
        if s and use and (use - places(s, script_to_hdr)):
            var_cross += 1
    fanout = sorted((len(f) for f in flag_set.values()), reverse=True)

    res["scripts"] = {
        "dateien_rotom": len(rotoms),
        "level_script_dateien": len(levels),
        "level_scripts_nicht_leer": lv_nonempty,
        "scripts_deklariert": scripts_declared,
        "befehle_gesamt": sum(cmd_count.values()),
        "befehle_verschieden": len(cmd_count),
        "knoten_befehle": dict(node),
        "dateien_mit": dict(files_with),
        "dialog_befehle": dict(dialog_cmds.most_common(8)),
        "kampf_befehle": dict(battle_cmds.most_common(10)),
        "belohnung_befehle": dict(reward_cmds.most_common(12)),
        "trainerkampf_mit_nummer": trainer_numeric,
        "flag_argumente_als_zahl": numeric_flag_args,
        "var_argumente_als_zahl": numeric_var_args,
    }
    res["level_scripts"] = {
        "eintraege": dict(entry_types),
        "var_conditions": var_conditions,
        "vars_verschieden": len(lv_vars),
        "vars_kategorien": cats(lv_vars, var_cat),
    }
    res["events"] = {
        "objekte": obj,
        "objekte_mit_flag": obj_hidden,
        "verschiedene_sichtbarkeitsflags": len(hidden_flags),
        "objekte_nach_flagart": dict(hidden_kind),
        "sichtbarkeitsflags_kategorien": cats(hidden_flags, flag_cat),
        "objekte_trainer": obj_trainer,
        "objekte_mit_script": obj_script,
        "script_bereiche": dict(script_ranges),
        "ausloeser": coord,
        "ausloeser_vars_verschieden": len(coord_vars),
        "ausloeser_vars_kategorien": cats(coord_vars, var_cat),
        "schilder_bg": bg,
        "warps": warps,
    }
    res["nutzung"] = {
        "flags_benutzt": len(all_flags_used),
        "flags_benutzt_kategorien": cats(all_flags_used, flag_cat),
        "flags_gesetzt": len(flag_set),
        "flags_geprueft_script": len(flag_chk),
        "flags_nur_gesetzt": flag_set_only,
        "flags_nur_geprueft_oder_sichtbarkeit": flag_chk_only,
        "flags_ueber_ortsgrenzen": flag_cross,
        "flag_setzer_dateien_max": fanout[:5],
        "vars_benutzt": len(all_vars_used),
        "vars_benutzt_kategorien": cats(all_vars_used, var_cat),
        "vars_gesetzt": len(var_set),
        "vars_verglichen_script": len(var_cmp),
        "vars_ueber_ortsgrenzen": var_cross,
        "db_flags_nie_benutzt_sprechend": sum(
            1 for k, c in flag_cat.items()
            if k not in all_flags_used and c not in ("ungenutzt", "unbekannt", "temporaer", "konstante")),
        "db_vars_nie_benutzt_sprechend": sum(
            1 for k, c in var_cat.items()
            if k not in all_vars_used and c in ("benannt", "szene_zustand")),
    }
    res["ids"] = id_usage(root, d)
    res["faeden"] = var_threads(root, d)
    res["dialog"] = dialog_paths(root)
    if as_json:
        print(json.dumps(res, indent=1, ensure_ascii=False))
    else:
        for sect, vals in res.items():
            print("==", sect)
            if isinstance(vals, dict):
                for k, v in vals.items():
                    print("  %s: %s" % (k, v))
            else:
                print("  ", vals)


def id_usage(root, d):
    """Flags und Variablen als Nummern (Name aus der Datenbank oder Zahl im Script), mit dem
    Pool, der für eine Geschichte frei wäre: nirgends in Scripts, Level-Scripts, Events
    benutzt, im dauerhaften Bereich (Flags < 0x4000 außerhalb der Trainer-Flags, Variablen
    0x4000 bis 0x7FFF)."""
    fid_of = resolver(d["flags"])
    vid_of = resolver(d["vars"])
    fnames = id_names(d["flags"])
    vnames = id_names(d["vars"])
    used_f, used_v = set(), set()
    unresolved = collections.Counter()
    src = os.path.join(root, "expanded/scripts")
    for p in glob.glob(os.path.join(src, "*.rotom")):
        for cmd, args in parse_rotom(p):
            if not args:
                continue
            if cmd in CHECK_FLAG | SET_FLAG and cmd not in ("SetTrainerFlag", "ClearTrainerFlag",
                                                             "CheckTrainerFlag"):
                i = fid_of(args[0])
                if i is None:
                    unresolved["flag"] += 1
                else:
                    used_f.add(i)
            if cmd in SET_VAR | CMP_VAR:
                for a in args[:2] if cmd in ("SetVarFromVar", "CompareVarToVar", "CopyVar") else args[:1]:
                    i = vid_of(a)
                    if i is None:
                        unresolved["var"] += 1
                    else:
                        used_v.add(i)
    for p in glob.glob(os.path.join(src, "*.json")):
        with open(p) as f:
            doc = json.load(f)
        for c in doc.get("var_conditions", []):
            used_v.add(c["var"])
    for p in glob.glob(os.path.join(root, "data/events/*.json")):
        with open(p) as f:
            e = json.load(f)
        for o in e.get("object_events", []):
            if o.get("hidden_flag"):
                used_f.add(o["hidden_flag"])
        for c in e.get("coord_events", []):
            used_v.add(c["var"])
    persistent_f = {i for i in used_f if i < 0x4000}
    saved_v = {i for i in used_v if 0x4000 <= i < 0x8000}
    trainer = range(0x550, 0x960)
    named_unused_f = {v["id"] for k, v in d["flags"].items() if re.search("UNUSED|DUMMY", k)}
    named_unused_v = {v["id"] for k, v in d["vars"].items() if re.search("UNUSED|DUMMY", k)}
    max_f = max(i for i in fnames if i < 0x4000)
    max_v = max(i for i in vnames if 0x4000 <= i < 0x8000)
    fcat = {v["id"]: classify(k, FLAG_RULES) for k, v in d["flags"].items()}
    vcat = {v["id"]: classify(k, VAR_RULES) for k, v in d["vars"].items()}
    loose = (None, "ungenutzt", "unbekannt")
    # Allgemeiner Bereich vor den Trainer-Flags, ohne temporäre Flags.
    free_f = [i for i in range(1, 0x550) if i not in used_f and fcat.get(i) in loose]
    # Systembereich hinter den Trainer-Flags: dort setzt die Engine selbst (riskanter).
    # HGSS: die täglichen Flags (DAILY_FLAG_BASE bis NUM_FLAGS) setzt die Engine jeden Tag zurück.
    daily = range(d["flags"]["DAILY_FLAG_BASE"]["id"], d["flags"]["NUM_FLAGS"]["id"]) \
        if "DAILY_FLAG_BASE" in d["flags"] else range(0)
    free_f_sys = [i for i in range(0x960, max_f + 1)
                  if i not in used_f and i not in daily and fcat.get(i) in loose]
    # Gespeicherte Variablen hinter den lokalen und den Grafik-Variablen.
    free_v = [i for i in range(0x4030, max_v + 1) if i not in used_v and vcat.get(i) in loose]
    return {
        "flags_benutzt_ids": len(persistent_f),
        "flags_benutzt_ohne_namen": sum(1 for i in persistent_f if i not in fnames),
        "flags_temp_benutzt": len(used_f) - len(persistent_f),
        "vars_benutzt_ids_gespeichert": len(saved_v),
        "vars_benutzt_ohne_namen": sum(1 for i in saved_v if i not in vnames),
        "vars_special_benutzt": sum(1 for i in used_v if i >= 0x8000),
        "argumente_nicht_aufgeloest": dict(unresolved),
        "flag_bereich_bis": hex(max_f),
        "var_bereich_bis": hex(max_v),
        "flags_frei_kandidaten_allgemein": len(free_f),
        "flags_frei_kandidaten_systembereich": len(free_f_sys),
        "flags_frei_und_als_ungenutzt_benannt": sum(1 for i in free_f if i in named_unused_f),
        "flags_ungenutzt_benannt_aber_benutzt": len(named_unused_f & used_f),
        "vars_frei_kandidaten": len(free_v),
        "vars_frei_und_als_ungenutzt_benannt": sum(1 for i in free_v if i in named_unused_v),
        "vars_ungenutzt_benannt_aber_benutzt": len(named_unused_v & used_v),
    }


def var_threads(root, d):
    """Je gespeicherter Variable: Werte (gesetzt, verglichen, Level-Script, Auslöser) und Orte
    (Header), an denen sie vorkommt. Eine Szenen-Variable mit mehreren Werten an mehreren Orten
    ist ein Faden im Sinne des Story-Graphs."""
    vid_of = resolver(d["vars"])
    vnames = id_names(d["vars"])
    vcat = {v["id"]: classify(k, VAR_RULES) for k, v in d["vars"].items()}
    values = collections.defaultdict(set)
    files = collections.defaultdict(set)
    src = os.path.join(root, "expanded/scripts")
    hdr_of_script = collections.defaultdict(set)
    hdr_of_level = collections.defaultdict(set)
    hdr_of_event = collections.defaultdict(set)
    for p in glob.glob(os.path.join(root, "data/headers/*.json")):
        with open(p) as f:
            h = json.load(f)
        hid = int(os.path.basename(p)[:4])
        hdr_of_script[h.get("script_file_id")].add(hid)
        hdr_of_level[h.get("level_script_id")].add(hid)
        hdr_of_event[h.get("event_file_id")].add(hid)
    for p in glob.glob(os.path.join(src, "*.rotom")):
        fid = int(os.path.basename(p)[:4])
        for cmd, args in parse_rotom(p):
            if cmd in ("SetVarFromValue", "CompareVarToValue", "SetVar") and len(args) >= 2:
                i = vid_of(args[0])
                try:
                    v = int(args[1], 0)
                except ValueError:
                    v = None
                if i is not None and 0x4000 <= i < 0x8000:
                    if v is not None:
                        values[i].add(v)
                    files[i] |= {("h", x) for x in hdr_of_script.get(fid, ())} or {("s", fid)}
    for p in glob.glob(os.path.join(src, "*.json")):
        fid = int(os.path.basename(p)[:4])
        with open(p) as f:
            doc = json.load(f)
        for c in doc.get("var_conditions", []):
            values[c["var"]].add(c["value"])
            files[c["var"]] |= {("h", x) for x in hdr_of_level.get(fid, ())} or {("l", fid)}
    for p in glob.glob(os.path.join(root, "data/events/*.json")):
        fid = int(os.path.basename(p)[:4])
        with open(p) as f:
            e = json.load(f)
        for c in e.get("coord_events", []):
            values[c["var"]].add(c["value"])
            files[c["var"]] |= {("h", x) for x in hdr_of_event.get(fid, ())} or {("e", fid)}
    story = [i for i in values if vcat.get(i) in ("szene_zustand", "benannt", "unbekannt", None)
             and 0x4030 <= i < 0x8000]
    nvals = sorted((len(values[i]) for i in story), reverse=True)
    nplaces = sorted((len(files[i]) for i in story), reverse=True)
    multi = [i for i in story if len(values[i]) >= 3 and len(files[i]) >= 2]
    med = lambda xs: xs[len(xs) // 2] if xs else 0
    return {
        "vars_mit_werten": len(story),
        "werte_je_var_max": nvals[:5],
        "werte_je_var_median": med(sorted(nvals)),
        "orte_je_var_max": nplaces[:5],
        "orte_je_var_median": med(sorted(nplaces)),
        "faeden_3plus_werte_2plus_orte": len(multi),
        "faeden_davon_szene_zustand_benannt": sum(1 for i in multi if vcat.get(i) == "szene_zustand"),
        "werte_summe_der_faeden": sum(len(values[i]) for i in multi),
    }


MSG_CMDS = {"Message", "NPCMsg", "MessageVar", "MessageInstant", "MessageNoSkip",
            "MessageSynchronized", "GenderMsgBox", "NonNPCMsg", "NonNPCMsgVar", "TrainerMessage",
            "DrawSignpostInstantMessage", "DrawSignpostScrollingMessage"}


def dialog_paths(root):
    """Weg Figur -> Script -> Nachricht: je Objekt mit lokalem Script (< 2000) die Nachrichten,
    die lexikalisch in seinem Script stehen (bis zur nächsten Script-Deklaration, wie
    `MessageUse.script` in checks/scripts.rs)."""
    src = os.path.join(root, "expanded/scripts")
    slot_msgs = {}          # (file, slot) -> [msg]
    slot_jumps = collections.Counter()
    for p in glob.glob(os.path.join(src, "*.rotom")):
        fid = int(os.path.basename(p)[:4])
        current = []
        pending_decl = False
        with open(p, encoding="utf-8") as f:
            for line in f:
                if line.startswith("script "):
                    m = re.search(r"#(\d+)", line)
                    slot = int(m.group(1)) if m else None
                    if not pending_decl:
                        current = []
                    current.append(slot)
                    slot_msgs.setdefault((fid, slot), [])
                    pending_decl = True
                    continue
                s2 = line.strip()
                if not s2:
                    continue
                pending_decl = False
                if not line.startswith((" ", "\t")):
                    continue
                parts = s2.split(None, 1)
                cmd = parts[0]
                if cmd in MSG_CMDS and len(parts) > 1:
                    a0 = parts[1].split(",")[0].strip()
                    for sl in current:
                        slot_msgs[(fid, sl)].append(a0)
                if cmd in ("GoTo", "Call", "GoToIf", "CallIf", "CallCommonScript", "CallStd"):
                    for sl in current:
                        slot_jumps[(fid, sl)] += 1
    hdr = {}
    for p in glob.glob(os.path.join(root, "data/headers/*.json")):
        with open(p) as f:
            h = json.load(f)
        hdr.setdefault(h.get("event_file_id"), h)
    objs = local = with_slot = with_msg = with_jump = 0
    msg_users = collections.Counter()
    for p in glob.glob(os.path.join(root, "data/events/*.json")):
        fid = int(os.path.basename(p)[:4])
        with open(p) as f:
            e = json.load(f)
        h = hdr.get(fid)
        for o in e.get("object_events", []):
            objs += 1
            sc = o.get("script", 0)
            if not h or not (0 < sc < 2000):
                continue
            local += 1
            key = (h.get("script_file_id"), sc)
            if key in slot_msgs:
                with_slot += 1
                if slot_msgs[key]:
                    with_msg += 1
                if slot_jumps.get(key):
                    with_jump += 1
                for m in set(slot_msgs[key]):
                    msg_users[(h.get("text_archive_id"), m)] += 1
    numeric = sum(1 for v in slot_msgs.values() for m in v if m[:1].isdigit())
    allm = sum(len(v) for v in slot_msgs.values())
    return {
        "objekte": objs,
        "objekte_lokales_script": local,
        "davon_slot_gefunden": with_slot,
        "davon_mit_nachricht": with_msg,
        "davon_mit_sprung_oder_aufruf": with_jump,
        "nachrichten_in_slots": allm,
        "davon_als_nummer": numeric,
        "nachrichten_von_mehreren_figuren": sum(1 for c in msg_users.values() if c > 1),
    }


if __name__ == "__main__":
    main()
