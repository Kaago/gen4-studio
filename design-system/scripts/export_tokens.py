"""Erzeugt aus den DTCG-Tokens (design-system/tokens) CSS- und TS-Artefakte für Gen4 Studio.

Aufruf: python3 export_tokens.py <design-system-ordner> [<ausgabeordner>]
Mit Ausgabeordner (Repo: apps/studio/src/ui/tokens) entstehen dort flach tokens.css, dockview-theme.css und tokens.ts;
ohne Ausgabeordner landen sie wie bisher in <design-system-ordner>/css und <design-system-ordner>/ts.
"""
import json, os, sys

ROOT = sys.argv[1]
OUT = sys.argv[2] if len(sys.argv) > 2 else None
TOK = os.path.join(ROOT, 'tokens')


def load(p):
    return json.load(open(os.path.join(TOK, p)))


def flat(d, pre=''):
    out = {}
    for k, v in d.items():
        if isinstance(v, dict) and '$value' in v:
            out[pre + k] = v
        elif isinstance(v, dict):
            out.update(flat(v, pre + k + '/'))
    return out


def css_name(k):
    return '--g4-' + k.replace('/', '-')


prim = flat(load('Primitiv/Wert.tokens.json'))
szene = flat(load('Szene/Wert.tokens.json'))
hell = flat(load('Thema/Hell.tokens.json'))
dunkel = flat(load('Thema/Dunkel.tokens.json'))
raum = flat(load('Raum/Wert.tokens.json'))
komfort = flat(load('Dichte/Komfort.tokens.json'))
kompakt = flat(load('Dichte/Kompakt.tokens.json'))
bewegung = flat(load('Bewegung/Wert.tokens.json'))
schrift = flat(load('Schrift/Wert.tokens.json'))
typo = flat(load('Typografie.tokens.json'))


def css_value(v):
    val = v['$value']
    if isinstance(val, str) and val.startswith('{'):
        return 'var(' + css_name(val[1:-1].replace('.', '/')) + ')'
    if v['$type'] == 'duration':
        return f"{val['value']}{val['unit']}"
    if v['$type'] == 'fontFamily':
        fallback = 'ui-monospace, monospace' if 'Mono' in val else 'system-ui, sans-serif'
        return f'"{val}", {fallback}'
    return val


def resolve(v):
    val = v['$value']
    if isinstance(val, str) and val.startswith('{'):
        return prim[val[1:-1].replace('.', '/')]['$value']
    return val


def block(sel, tokens, indent='  '):
    lines = [f'{sel} {{']
    for k, v in tokens.items():
        lines.append(f'{indent}{css_name(k)}: {css_value(v)};')
    lines.append('}')
    return '\n'.join(lines)


fonts = []
for k, v in typo.items():
    t = v['$value']
    fam = t['fontFamily']
    fb = 'ui-monospace, monospace' if 'Mono' in fam else 'system-ui, sans-serif'
    fonts.append(f'  {css_name("font/" + k)}: {t["fontWeight"]} {t["fontSize"]}/{t["lineHeight"]} "{fam}", {fb};')

css = []
css.append('/* Gen4 Studio Design-System: Tokens. Generiert aus design-system/tokens (DTCG). Nicht von Hand ändern. */')
css.append('/* Alle Farben sind RGB555-treu: jeder Kanal liegt im 15-Bit-Raster des Nintendo DS. */\n')
css.append(':root {')
css.append('  /* Primitive */')
for k, v in prim.items():
    css.append(f'  {css_name(k)}: {v["$value"]};')
css.append('  /* Szene (gleich in beiden Themen) */')
for k, v in szene.items():
    css.append(f'  {css_name("szene/" + k)}: {v["$value"]};')
css.append('  /* Raum, Form, Schriftgrößen */')
for k, v in raum.items():
    css.append(f'  {css_name(k)}: {css_value(v)};')
css.append('  /* Schriftfamilien und Textstile */')
for k, v in schrift.items():
    css.append(f'  {css_name("schrift/" + k)}: {css_value(v)};')
css.extend(fonts)
css.append('  /* Bewegung */')
for k, v in bewegung.items():
    css.append(f'  {css_name(k)}: {css_value(v)};')
css.append('  /* Dichte: Komfort (Standard) */')
for k, v in komfort.items():
    css.append(f'  {css_name(k)}: {css_value(v)};')
css.append('  /* Thema: Hell (Standard) */')
for k, v in hell.items():
    css.append(f'  {css_name(k)}: {css_value(v)};')
css.append('  color-scheme: light;')
css.append('}\n')
dk = block(':root[data-theme="dunkel"]', dunkel).replace('\n}', '\n  color-scheme: dark;\n}')
css.append(dk + '\n')
css.append('@media (prefers-color-scheme: dark) {')
css.append(block('  :root:not([data-theme="hell"])', dunkel, '    ').replace('\n}', '\n    color-scheme: dark;\n  }'))
css.append('}\n')
css.append(block(':root[data-dichte="kompakt"]', kompakt) + '\n')
css.append('@media (prefers-reduced-motion: reduce) {\n  :root {\n    --g4-dauer-kurz: 0ms;\n    --g4-dauer-mittel: 0ms;\n    --g4-dauer-scharnier: 0ms;\n  }\n}\n')
css.append('@media (forced-colors: active) {\n  :root {\n    --g4-fokus-ring: Highlight;\n    --g4-akzent-grund: Highlight;\n    --g4-akzent-auf: HighlightText;\n    --g4-linie-stark: CanvasText;\n  }\n}\n')
css.append('''/* Basis */
:where(html) {
  font: var(--g4-font-text-s);
  color: var(--g4-text-primaer);
  background: var(--g4-flaeche-gehaeuse);
  font-variant-numeric: tabular-nums;
}
:where(:focus-visible) {
  outline: var(--g4-strich-fokus) solid var(--g4-fokus-ring);
  outline-offset: 2px;
}
''')

dockview = '''/* Dockview-Thema für Gen4 Studio: class "dockview-theme-g4" auf den Dockview-Container setzen. */
.dockview-theme-g4 {
  --dv-background-color: var(--g4-flaeche-gehaeuse);
  --dv-group-view-background-color: var(--g4-flaeche-gehaeuse);
  --dv-paneview-header-border-color: var(--g4-linie-leise);
  --dv-tabs-and-actions-container-background-color: var(--g4-flaeche-gehaeuse);
  --dv-tabs-and-actions-container-height: 36px;
  --dv-tabs-and-actions-container-font-size: 13px;
  --dv-activegroup-visiblepanel-tab-background-color: var(--g4-flaeche-schau);
  --dv-activegroup-hiddenpanel-tab-background-color: var(--g4-flaeche-gehaeuse);
  --dv-inactivegroup-visiblepanel-tab-background-color: var(--g4-flaeche-gehaeuse);
  --dv-inactivegroup-hiddenpanel-tab-background-color: var(--g4-flaeche-gehaeuse);
  --dv-activegroup-visiblepanel-tab-color: var(--g4-text-primaer);
  --dv-activegroup-hiddenpanel-tab-color: var(--g4-text-sekundaer);
  --dv-inactivegroup-visiblepanel-tab-color: var(--g4-text-sekundaer);
  --dv-inactivegroup-hiddenpanel-tab-color: var(--g4-text-sekundaer);
  --dv-tab-divider-color: var(--g4-linie-leise);
  --dv-separator-border: var(--g4-linie-leise);
  --dv-paneview-active-outline-color: var(--g4-fokus-ring);
  --dv-drag-over-background-color: color-mix(in srgb, var(--g4-akzent-grund) 18%, transparent);
  --dv-drag-over-border-color: var(--g4-akzent-grund);
  --dv-icon-hover-background-color: var(--g4-flaeche-deck);
  font-family: var(--g4-schrift-familie-ui);
}
/* Aktiver Reiter: 2-px-Kobaltkante oben, wie die Komponente "Reiter" */
.dockview-theme-g4 .dv-tab.dv-active-tab { box-shadow: inset 0 2px 0 var(--g4-akzent-grund); }
/* Das Scharnier ist kein Dockview-Panel, sondern ein eigener Trenner zwischen Schau- und Hand-Gruppe. */
'''


def hexnum(h):
    return '0x' + h[1:7].upper()


ts = []
ts.append('// Gen4 Studio Design-System: Werte für Code, der keine CSS-Variablen lesen kann (three.js, Monaco, CodeMirror, Canvas).')
ts.append('// Generiert aus design-system/tokens (DTCG). Nicht von Hand ändern.\n')
ts.append('/** Szenenfarben als Zahl für THREE.Color. Material mit toneMapped: false, depthWrite: false. Gleich in Hell und Dunkel. */')
ts.append('export const szene = {')
groups = {}
for k, v in szene.items():
    g, n = k.split('/')
    groups.setdefault(g, {})[n] = v['$value']
for g, items in groups.items():
    key = g.replace('-', '_')
    ts.append(f'  {key}: {{')
    for n, h in items.items():
        ts.append(f"    '{n}': {hexnum(h)},")
    ts.append('  },')
ts.append('} as const;\n')
ts.append('/** Muster je Geländeklasse (Shader-Parameter uPattern): 0 voll, 1 diagonal, 2 kreuz, 3 punkte, 4 wellen/linien, 5 schachbrett. */')
ts.append("export const gelaendeMuster = { blockiert: 1, wasser: 4, 'hohes-gras': 3, vorsprung: 0, eis: 2, spezial: 5, unbekannt: 0 } as const;\n")
ts.append('/** Form je Ereignisart für Marker und Legende. */')
ts.append("export const ereignisForm = { figur: 'kreis', trainer: 'kreis-sichtlinie', warp: 'raute', ausloeser: 'zone', schild: 'etikett', gegenstand: 'quadrat' } as const;\n")
for name, src in (('hell', hell), ('dunkel', dunkel)):
    ts.append(f'/** Thema {name.capitalize()}: aufgelöste Hex-Werte (für Monaco defineTheme, Canvas, Export). */')
    ts.append(f'export const {name} = {{')
    for k, v in src.items():
        ts.append(f"  '{k}': '{resolve(v)}',")
    ts.append('} as const;\n')
ts.append('''/** Syntaxfarben für rotom-Skripte (CodeMirror HighlightStyle oder Monaco-Regeln). Kontrast >= 4.5:1 auf flaeche/blatt. */
export const syntax = {
  hell: { befehl: '#2942B5', schluesselwort: '#633994', zeichenkette: '#186B42', zahl: '#943121', flag: '#6B5229', kommentar: '#4A5263', fehler: '#C62939' },
  dunkel: { befehl: '#94ADF7', schluesselwort: '#D6C6F7', zeichenkette: '#B5E7C6', zahl: '#FFC6B5', flag: '#E7D6B5', kommentar: '#ADB5C6', fehler: '#F7949C' },
} as const;
''')
ts.append('/** Pokémon-Typen der 4. Generation, RGB555-gerastert, mit lesbarer Schriftfarbe. */')
types = [('Normal', '#A5A57B', '#182131'), ('Feuer', '#EF8431', '#182131'), ('Wasser', '#6B8CEF', '#182131'), ('Pflanze', '#7BC652', '#182131'), ('Elektro', '#F7CE31', '#182131'), ('Eis', '#94D6D6', '#182131'), ('Kampf', '#BD3129', '#FFFFFF'), ('Gift', '#9C429C', '#FFFFFF'), ('Boden', '#DEBD6B', '#182131'), ('Flug', '#A58CEF', '#182131'), ('Psycho', '#F75A84', '#182131'), ('Käfer', '#A5B521', '#182131'), ('Gestein', '#B59C39', '#182131'), ('Geist', '#735A94', '#FFFFFF'), ('Drache', '#7339F7', '#FFFFFF'), ('Unlicht', '#735A4A', '#FFFFFF'), ('Stahl', '#B5B5CE', '#182131'), ('???', '#6B9C8C', '#182131')]
ts.append('export const typen = {')
for n, bg, fg in types:
    ts.append(f"  '{n}': {{ grund: '{bg}', schrift: '{fg}' }},")
ts.append('} as const;\n')
ts.append('/** Prüft, ob eine Farbe im DS-Farbraum liegt (RGB555), und rastet sie sonst ein. */')
ts.append('''export function rgb555(hex: string): string {
  const steps = Array.from({ length: 32 }, (_, i) => (i << 3) | (i >> 2));
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return '#' + c.map((v) => steps.reduce((a, b) => (Math.abs(b - v) < Math.abs(a - v) ? b : a))).map((v) => v.toString(16).padStart(2, '0').toUpperCase()).join('');
}
''')

if OUT:
    css_dir = ts_dir = OUT
else:
    css_dir = os.path.join(ROOT, 'css')
    ts_dir = os.path.join(ROOT, 'ts')
os.makedirs(css_dir, exist_ok=True)
os.makedirs(ts_dir, exist_ok=True)
open(os.path.join(css_dir, 'tokens.css'), 'w').write('\n'.join(css))
open(os.path.join(css_dir, 'dockview-theme.css'), 'w').write(dockview)
open(os.path.join(ts_dir, 'tokens.ts'), 'w').write('\n'.join(ts))
print('css', len(css), 'ts', len(ts), '->', OUT or f'{css_dir}, {ts_dir}')
