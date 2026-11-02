"""Prüft die Gen4-Studio-Tokens: RGB555-Treue aller Farben und WCAG-2.2-Kontraste der wichtigsten Paare in Hell und Dunkel.

Aufruf: python3 design-system/scripts/check_tokens.py   (aus der Repo-Wurzel; der Pfad ist beliebig)
Exit-Code 1 bei Verstoß.
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(__file__), '..', 'tokens')
DS = {(v << 3) | (v >> 2) for v in range(32)}


def flat(d, pre=''):
    out = {}
    for k, v in d.items():
        if isinstance(v, dict) and '$value' in v:
            # Only strings are colours; a shadow's value is an object.
            if isinstance(v['$value'], str):
                out[pre + k] = v['$value']
        elif isinstance(v, dict):
            out.update(flat(v, pre + k + '/'))
    return out


def load(p):
    return flat(json.load(open(os.path.join(ROOT, p))))


prim = load('Primitiv/Wert.tokens.json')
szene = load('Szene/Wert.tokens.json')
themes = {'Hell': load('Thema/Hell.tokens.json'), 'Dunkel': load('Thema/Dunkel.tokens.json')}


def res(v):
    return prim[v[1:-1].replace('.', '/')] if v.startswith('{') else v


def lum(h):
    def f(c):
        c /= 255
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (f(int(h[i:i + 2], 16)) for i in (1, 3, 5))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


fail = 0
for name, src in [('Primitiv', prim), ('Szene', szene)] + [(k, {kk: res(vv) for kk, vv in t.items()}) for k, t in themes.items()]:
    for k, h in src.items():
        if not h.startswith('#'):
            continue
        ch = [int(h[i:i + 2], 16) for i in (1, 3, 5)]
        if any(c not in DS for c in ch):
            print(f'RGB555 {name} {k} {h}')
            fail += 1

TEXT = ['text/primaer', 'text/sekundaer', 'akzent/text', 'status/fehler', 'status/warnung', 'status/erfolg', 'status/info']
SURF = ['flaeche/gehaeuse', 'flaeche/deck', 'flaeche/taste', 'flaeche/blatt']
PAIRS = [(t, s, 4.5) for t in TEXT for s in SURF]
PAIRS += [('akzent/auf', 'akzent/grund', 4.5), ('live/auf', 'live/grund', 4.5), ('live/auf', 'live/taste', 4.5), ('scharnier/text', 'scharnier/grund', 4.5), ('scharnier/leise', 'scharnier/grund', 4.5), ('text/primaer', 'auswahl/flaeche', 4.5), ('linie/stark', 'flaeche/taste', 3.0), ('linie/stark', 'flaeche/blatt', 3.0), ('fokus/ring', 'flaeche/gehaeuse', 3.0)]
for b in ['welt', 'geschichten', 'pokemon', 'gegenstaende', 'texte', 'technik']:
    PAIRS.append((f'bereich/{b}', f'bereich/{b}-flaeche', 4.5))
for s in ['fehler', 'warnung', 'erfolg', 'info']:
    PAIRS.append((f'status/{s}', f'status/{s}-flaeche', 4.5))
for mode, t in themes.items():
    r = {k: res(v)[:7] for k, v in t.items()}
    for fg, bg, need in PAIRS:
        c = ratio(r[fg], r[bg])
        if c < need:
            print(f'Kontrast {mode}: {fg} auf {bg} = {c:.2f} (mindestens {need})')
            fail += 1
# The DS boxes: dialog box, sign and the battle status box carry game text on their surface.
for box in ['textbox', 'schild', 'status']:
    c = ratio(szene[f'ds/{box}/text'], szene[f'ds/{box}/flaeche'])
    if c < 4.5:
        print(f'Kontrast Szene: ds/{box}/text auf ds/{box}/flaeche = {c:.2f} (mindestens 4.5)')
        fail += 1
# Pokémon types: the name on the type colour, the same in both themes.
for typ in sorted({k.split('/')[1] for k in szene if k.startswith('typ/')}):
    c = ratio(szene[f'typ/{typ}/schrift'], szene[f'typ/{typ}/grund'])
    if c < 4.5:
        print(f'Kontrast Szene: typ/{typ}/schrift auf typ/{typ}/grund = {c:.2f} (mindestens 4.5)')
        fail += 1
print('Alles stimmig.' if not fail else f'{fail} Verstöße.')
sys.exit(1 if fail else 0)
