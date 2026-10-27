// ===== Gen4 Studio JSX-Bausteine, Stand der App (nach Phase 3, 25.10.2026) =====
// Gegenstück zu jsx-lib.js (Zielbild vom 23.10.). Maße und Tokens folgen den CSS-Modulen in
// apps/studio/src/ui/{primitives,frame} und workbench/. Für OpenPencil render: in eine IIFE kopieren.
const F = 'Atkinson Hyperlegible Next', M = 'Atkinson Hyperlegible Mono', Z = 'Zen Maru Gothic', PX = 'Pixelify Sans';
// Mit dunkel-map.js davor (const DARKMAP) liefert v() die Hex-Werte des dunklen Themas
const v = (t) => (typeof DARKMAP !== 'undefined' && DARKMAP[t]) || designVar(t);
// Symbolfarben als Hex (Thema Hell), weil OpenPencil Token an Vektoren nicht zuverlässig bindet
const HX = { p: '#182131', s: '#4A5263', d: '#848CA5', a: '#3152DE', at: '#2942B5', auf: '#FFFFFF', f: '#AD2131', w: '#8C5200', ok: '#186B42', info: '#2942B5', live: '#182131', sl: '#C6CEDE', st: '#E7EFF7', welt: '#186B42', geschichten: '#633994', pokemon: '#943121', gegenstaende: '#106363', texte: '#6B5229', technik: '#394252' };
const BER = { start: ['ph:house', 'Start'], welt: ['ph:map-trifold', 'Welt'], geschichten: ['ph:chats-circle', 'Geschichten'], pokemon: ['ph:paw-print', 'Pokémon'], gegenstaende: ['ph:backpack', 'Gegenstände'], texte: ['ph:text-aa', 'Texte'], technik: ['ph:cpu', 'Technik'] };
// Text: o.s Größe, o.w Gewicht, o.c Token, o.hex feste Farbe, o.f Familie, o.one einzeilig gekürzt
const T = (s, o = {}) => <Text font={o.f || F} size={o.s || 13} weight={o.w || 400} lineHeight={o.lh || ({ 10: 12, 12: 16, 13: 18, 15: 20, 18: 24, 24: 30 }[o.s || 13] || Math.round((o.s || 13) * 1.38))} color={o.hex || v(o.c || 'text/primaer')} w={o.width} grow={o.grow} textAlign={o.align} maxLines={o.one ? 1 : undefined} truncate={o.one ? true : undefined}>{s}</Text>;
const Mono = (s, o = {}) => T(s, { f: M, s: 12, c: 'text/sekundaer', ...o });
const I = (name, size = 18, c = 's') => <Icon name={name} size={size} color={HX[c] || c} />;
const Kbd = (k, dark) => <Frame flex="row" items="center" justify="center" px={4} minW={20} h={18} rounded={4} bg={v(dark ? 'scharnier/taste' : 'flaeche/deck')} stroke={v(dark ? 'scharnier/kante' : 'linie/standard')} strokeWidth={1}>{T(k, { f: M, s: 12, c: dark ? 'scharnier/text' : 'text/sekundaer' })}</Frame>;
// Taste (Button): art 'Primär' | 'Sekundär' | 'Leise' | 'Spielen' | 'Gefahr'; klein = 24 hoch
const Btn = ({ label, art = 'Sekundär', icon, kbd, small, off, grow }) => {
  const S = { 'Primär': ['akzent/grund', 'akzent/auf', 'auf'], 'Sekundär': ['flaeche/taste', 'text/primaer', 'p'], 'Leise': [null, 'text/primaer', 'p'], 'Spielen': ['live/grund', 'live/auf', 'live'], 'Gefahr': ['status/fehler', 'akzent/auf', 'auf'] }[art];
  if (off) return <Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 24 : 32} rounded={6} grow={grow} bg={art === 'Leise' ? undefined : v('flaeche/deck')} stroke={art === 'Leise' ? undefined : v('linie/leise')} strokeWidth={1}>{icon && I(icon, small ? 16 : 18, 'd')}{T(label, { w: 600, c: 'text/deaktiviert' })}</Frame>;
  // Sekundär: Kante (unten 2 px) als äußere Fläche, weil OpenPencil bei ungleichen Kanten die Ecken nicht rundet
  if (art === 'Sekundär') return <Frame flex="col" rounded={6} grow={grow} bg={v('linie/stark')} pt={1} pl={1} pr={1} pb={2}><Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 21 : 29} rounded={5} bg={v('flaeche/taste')}>{icon && I(icon, small ? 16 : 18, 'p')}{T(label, { w: 600 })}{kbd && Kbd(kbd)}</Frame></Frame>;
  return <Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 24 : 32} rounded={6} grow={grow} bg={S[0] ? v(S[0]) : undefined}>{icon && I(icon, small ? 16 : 18, S[2])}{T(label, { w: art === 'Spielen' ? 700 : 600, c: S[1] })}{kbd && Kbd(kbd)}</Frame>;
};
const IconBtn = (icon, o = {}) => <Frame w={32} h={32} rounded={6} bg={o.on ? v('auswahl/flaeche') : undefined} flex="row" items="center" justify="center">{I(o.on ? icon + '-fill' : icon, 18, o.on ? 'at' : o.off ? 'd' : (o.c || 'p'))}</Frame>;
// Referenz (Pille, Bereichsfarbe), Plakette (Badge, 16 hoch), Typ-Plakette (s 16 / m 22)
const Ref = (b, label, tech, icon) => <Frame flex="row" items="center" gap={6} px={8} h={24} rounded={12} bg={v('bereich/' + b + '-flaeche')}>{I(icon || BER[b][0], 18, b)}{T(label, { w: 600, c: 'bereich/' + b })}{tech && T(tech, { f: M, s: 12, c: 'bereich/' + b })}</Frame>;
const Badge = (label, art = 'neutral', icon) => { const m = { neutral: ['flaeche/deck', 'text/sekundaer', 's'], ungespeichert: ['auswahl/flaeche', 'akzent/text', 'at'], warn: ['status/warnung-flaeche', 'status/warnung', 'w'], fehler: ['status/fehler-flaeche', 'status/fehler', 'f'], ok: ['status/erfolg-flaeche', 'status/erfolg', 'ok'], info: ['status/info-flaeche', 'status/info', 'info'], live: ['live/flaeche', 'live/text', 'live'], code: ['flaeche/deck', 'text/sekundaer', 's'], gesperrt: ['flaeche/deck', 'text/sekundaer', 's'], unumkehrbar: ['status/warnung-flaeche', 'status/warnung', 'w'], aufplatte: ['status/warnung-flaeche', 'status/warnung', 'w'], nurpthgss: ['status/info-flaeche', 'status/info', 'info'] }[art]; return <Frame flex="row" items="center" gap={4} px={8} h={16} rounded={art === 'code' ? 4 : 8} bg={v(m[0])}>{icon && I(icon, 12, m[2])}{T(label, { s: 12, w: 600, c: m[1], f: art === 'code' ? M : F })}</Frame>; };
const TY = { normal: ['Normal', '#A5A57B', '#182131'], kampf: ['Kampf', '#BD3129', '#FFFFFF'], flug: ['Flug', '#A58CEF', '#182131'], gift: ['Gift', '#9C429C', '#FFFFFF'], boden: ['Boden', '#DEBD6B', '#182131'], gestein: ['Gestein', '#B59C39', '#182131'], kaefer: ['Käfer', '#A5B521', '#182131'], geist: ['Geist', '#735A94', '#FFFFFF'], stahl: ['Stahl', '#B5B5CE', '#182131'], fragezeichen: ['???', '#6B9C8C', '#182131'], feuer: ['Feuer', '#EF8431', '#182131'], wasser: ['Wasser', '#6B8CEF', '#182131'], pflanze: ['Pflanze', '#7BC652', '#182131'], elektro: ['Elektro', '#F7CE31', '#182131'], psycho: ['Psycho', '#F75A84', '#182131'], eis: ['Eis', '#94D6D6', '#182131'], drache: ['Drache', '#7339F7', '#FFFFFF'], unlicht: ['Unlicht', '#735A4A', '#FFFFFF'] };
const Typ = (k, m) => <Frame flex="row" items="center" justify="center" px={m ? 8 : 6} minW={m ? 64 : 52} h={m ? 22 : 16} rounded={11} bg={v('typ/' + k + '/grund')}>{T(TY[k][0], { s: m ? 13 : 12, w: 600, c: 'typ/' + k + '/schrift' })}</Frame>;
// Eingaben: Feld 32 hoch, Radius 4, Rand linie/standard, Fläche blatt
const Field = (text, o = {}) => <Frame flex="row" items="center" gap={8} px={8} h={32} w={o.w || 'fill'} grow={o.grow} rounded={4} bg={v(o.off ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(o.err ? 'status/fehler' : o.open ? 'akzent/grund' : o.off ? 'linie/leise' : 'linie/standard')} strokeWidth={1}>{o.icon && I(o.icon, 18)}{T(text, { grow: 1, one: true, c: o.ph ? 'text/sekundaer' : o.off ? 'text/deaktiviert' : 'text/primaer', f: o.mono ? M : F })}{o.select && I('ph:caret-down', 18)}</Frame>;
// Zahlfeld ohne sichtbares Label: Griffleiste links; hex: Segmente Dez | Hex rechts (Tüfteln)
const NumField = (val, o = {}) => <Frame flex="row" items="center" gap={8} grow={1} w="fill">{I('ph:dots-six-vertical', 18)}{Field(val, { mono: o.hexMode, grow: 1 })}{o.base && SegS(['Dez', 'Hex'], o.hexMode ? 'Hex' : 'Dez')}</Frame>;
const Toggle = (on, label) => <Frame flex="row" items="center" gap={8} minH={32}><Frame w={36} h={20} rounded={10} p={2} bg={v(on ? 'akzent/grund' : 'flaeche/deck')} stroke={on ? undefined : v('linie/stark')} strokeWidth={1} flex="row" items="center" justify={on ? 'end' : 'start'}><Frame w={14} h={14} rounded={7} bg={v(on ? 'akzent/auf' : 'text/sekundaer')} /></Frame>{label && T(label)}</Frame>;
const Check = (on, label, off) => <Frame flex="row" items="center" gap={8}><Frame w={18} h={18} rounded={4} bg={v(off ? 'flaeche/deck' : on ? 'akzent/grund' : 'flaeche/blatt')} stroke={on ? undefined : v(off ? 'linie/leise' : 'linie/stark')} strokeWidth={1} flex="row" items="center" justify="center">{on && I('ph:check-bold', 14, 'auf')}</Frame>{label && T(label, { c: off ? 'text/deaktiviert' : 'text/primaer' })}</Frame>;
// Segmente: Gruppe deck mit Rand, gewählt blatt + akzent/text + Innenkante auswahl/kante
const Seg = (opts, active, small) => <Frame flex="row" items="center" gap={2} p={2} h={small ? 28 : 32} rounded={6} bg={v('flaeche/deck')} stroke={v('linie/standard')} strokeWidth={1}>{opts.map((o) => { const [l, i] = Array.isArray(o) ? o : [o]; const on = (l || i) === active; return <Frame flex="row" items="center" gap={6} px={small ? 8 : 12} h="fill" rounded={4} bg={on ? v('flaeche/blatt') : undefined} stroke={on ? v('auswahl/kante') : undefined} strokeWidth={1}>{i && I(on ? i + '-fill' : i, 18, on ? 'at' : 's')}{l && T(l, { w: 600, c: on ? 'akzent/text' : 'text/primaer' })}</Frame>; })}</Frame>;
const SegS = (opts, active) => Seg(opts, active, true);
// Reiter (Primitive Tabs): aktiv blatt + 2 px Kobalt unten; Zähler als Pille
const Tabs = (items, active) => <Frame flex="row" items="end" gap={2} px={4} h={36} w="fill" bg={v('flaeche/deck')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>{items.map((it) => { const [l, n] = Array.isArray(it) ? it : [it]; const on = l === active; return <Frame flex="row" items="center" gap={6} px={12} h={32} bg={on ? v('flaeche/blatt') : undefined} strokeWeights={on ? { top: 0, right: 0, bottom: 2, left: 0 } : undefined} stroke={on ? v('akzent/grund') : undefined}>{T(l, { w: 600, c: on ? 'text/primaer' : 'text/sekundaer' })}{n !== undefined && <Frame px={6} h={16} rounded={8} bg={v(on ? 'auswahl/flaeche' : 'flaeche/deck')} flex="row" items="center">{T(String(n), { s: 12, c: on ? 'akzent/text' : 'text/sekundaer' })}</Frame>}</Frame>; })}</Frame>;
// Dockview-Reiterleiste (Schau und Hand): 36 hoch, gehaeuse; aktiv schau mit 2 px Kobalt oben
const DockTabs = (items, active, bg = 'flaeche/gehaeuse') => <Frame flex="row" items="end" h={36} w="fill" bg={v(bg)}>{items.map((l) => { const on = l === active; return <Frame flex="row" items="center" px={8} h={36} bg={on ? v('flaeche/schau') : undefined} strokeWeights={on ? { top: 2, right: 0, bottom: 0, left: 0 } : undefined} stroke={on ? v('akzent/grund') : undefined}>{T(l, { c: on ? 'text/primaer' : 'text/sekundaer' })}</Frame>; })}</Frame>;
// Inspector-Zeile: Raster 116 | Rest, min 32, links 2 px (Abweichung: akzent/grund)
const Row = ({ label, children, changed, hint }) => <Frame flex="row" items={hint ? 'start' : 'center'} gap={8} pl={8} pr={8} py={2} minH={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 0, left: 2 }} stroke={changed ? v('akzent/grund') : v('flaeche/blatt')}>{T(label, { c: 'text/sekundaer', width: 116 })}{hint ? <Frame flex="col" gap={2} grow={1}><Frame flex="row" items="center" gap={8} w="fill">{children}</Frame>{T(hint, { s: 12, c: 'text/sekundaer' })}</Frame> : <Frame flex="row" items="center" gap={8} grow={1}>{children}</Frame>}</Frame>;
// Abschnitt: Kopf 32 mit Pfeil, Titel 600, Zähler-Pille; Linie unten
const Section = ({ title, count, children, open = true }) => <Frame flex="col" w="fill"><Frame flex="row" items="center" gap={6} px={8} h={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{I(open ? 'ph:caret-down' : 'ph:caret-right', 18)}{T(title, { w: 600, grow: 1 })}{count !== undefined && <Frame px={6} h={16} rounded={8} bg={v('flaeche/deck')} flex="row" items="center">{T(String(count), { s: 12, c: 'text/sekundaer' })}</Frame>}</Frame>{open && <Frame flex="col" w="fill" pt={4} pb={8}>{children}</Frame>}</Frame>;
// Panelkopf: min 32, Titel 600, Untertitel Mono grau, Aktionen rechts; switcher = Titel mit Pfeil (Menü)
const PanelHead = ({ title, sub, actions, switcher }) => <Frame flex="row" items="center" gap={8} pl={12} pr={8} h={36} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}><Frame flex="row" items="center" gap={6}>{T(title, { w: 600 })}{switcher && I('ph:caret-down', 18)}</Frame>{sub && Mono(sub, { one: true, grow: 1 })}{!sub && <Frame grow={1} />}{actions}</Frame>;
const Search = (ph, w) => <Frame flex="row" items="center" gap={8} px={8} h={32} w={w || 'fill'} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{T(ph, { c: 'text/sekundaer', grow: 1, one: true })}</Frame>;
// Symbol (Sprite) als Platzhalter: Anfangsbuchstabe auf Bereichsfarbe (keine nachgebauten Sprites)
const Sprite = (letter, b = 'pokemon', size = 24) => <Frame w={size} h={size} rounded={4} bg={v('bereich/' + b + '-flaeche')} flex="row" items="center" justify="center">{T(letter, { s: size >= 48 ? 18 : 13, w: 700, c: 'bereich/' + b })}</Frame>;
// Listenzeile: min 32, gewählt auswahl/flaeche + akzent/text + 2 px links auswahl/kante
const ListRow = ({ title, tech, sub, right, icon, lead, sel, depth = 0, caret, mark }) => <Frame flex="row" items="center" gap={8} pl={12 + depth * 16} pr={12} minH={sub ? 40 : 32} w="fill" rounded={4} bg={sel ? v('auswahl/flaeche') : undefined} strokeWeights={sel ? { top: 0, right: 0, bottom: 0, left: 2 } : undefined} stroke={sel ? v('auswahl/kante') : undefined}>{caret && I(caret === 'open' ? 'ph:caret-down' : 'ph:caret-right', 18)}{lead}{icon && I(icon, 18, sel ? 'at' : 's')}<Frame flex="col" grow={1}><Frame flex="row" items="center" gap={6}>{T(title, { c: sel ? 'akzent/text' : 'text/primaer', one: true })}{tech && Mono(tech)}</Frame>{sub && T(sub, { s: 12, c: 'text/sekundaer' })}</Frame>{mark}{right && T(right, { s: 12, c: 'text/sekundaer', f: M })}</Frame>;
// Wertebalken (StatBar): 116 | Zahl | Spur | Urteil
const StatBar = ({ label, val, mean, verdict, changed, max = 255, track = 380 }) => <Frame flex="row" items="center" gap={8} pl={8} pr={8} minH={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 0, left: 2 }} stroke={changed ? v('akzent/grund') : v('flaeche/schau')}>{T(label, { c: 'text/sekundaer', width: 116 })}{T(String(val), { w: 600, align: 'right', width: 28 })}<Frame w={track} h={12}><Frame position="absolute" x={0} y={2} w={track} h={8} rounded={4} bg={v('flaeche/deck')} /><Frame position="absolute" x={0} y={2} w={Math.round(val / max * track)} h={8} rounded={4} bg={v('akzent/grund')} /><Frame position="absolute" x={Math.round(mean / max * track)} y={0} w={2} h={12} bg={v('linie/stark')} /></Frame>{T(verdict, { s: 12, c: 'text/sekundaer', width: 84 })}</Frame>;
// Pokémon-Karte: Radius 12, Rand linie/leise; gewählt Kobaltkante; leer gestrichelt
const MonCard = ({ name, lv, item, sel, empty, w = 200 }) => empty ? <Frame flex="row" items="center" justify="center" w={w} h={56} rounded={12} stroke={v('linie/standard')} strokeWidth={1} strokeDash={[4, 3]} dashPattern={[4, 3]}>{T(empty, { c: 'text/sekundaer' })}</Frame> : <Frame flex="row" items="center" gap={8} px={12} py={8} w={w} minH={56} rounded={12} bg={v('flaeche/blatt')} stroke={v(sel ? 'auswahl/kante' : 'linie/leise')} strokeWidth={sel ? 2 : 1}>{Sprite(name[0], 'pokemon', 32)}<Frame flex="col">{T(name, { w: 600 })}{T('Lv. ' + lv, { s: 12, c: 'text/sekundaer' })}{item && T(item, { s: 12, c: 'text/sekundaer' })}</Frame></Frame>;
// Meldung (Notice): ohne Rand, Statusfläche
const Notice = (text, art = 'info', title) => { const m = { info: ['status/info-flaeche', 'status/info', 'ph:info-fill', 'info'], warn: ['status/warnung-flaeche', 'status/warnung', 'ph:warning-fill', 'w'], fehler: ['status/fehler-flaeche', 'status/fehler', 'ph:x-circle-fill', 'f'], ok: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok'] }[art]; return <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[2], 18, m[3])}<Frame flex="col" grow={1}>{title && T(title, { w: 600, c: m[1] })}{T(text)}</Frame></Frame>; };
// Leerer Zustand: Titel 15/20 600 Atkinson, Grund 13/18 sekundär, Taste sekundär
const Empty = (what, why, action) => <Frame flex="col" items="center" justify="center" gap={8} px={16} py={32} w="fill" grow={1}>{T(what, { s: 15, w: 600 })}{T(why, { c: 'text/sekundaer', align: 'center', width: 320 })}{action && Btn({ label: action })}</Frame>;
// ===== Rahmen =====
// Titelleiste 40: macOS-Ampel im Inset 78, Projekt öffnen, Name 15/20 600, Spiel-Pille, Speicherstand; Suche mittig 360; rechts vier Symboltasten
const Titlebar = ({ project = 'Sinnoh-Remix', game = 'Platin, DE', unsaved = 0, dark } = {}) => (
  <Frame name="Titelleiste" flex="row" items="center" gap={16} pl={12} pr={12} h={40} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" gap={8} items="center" w={54}>{['#FF5F57', '#FEBC2E', '#28C840'].map((c) => <Ellipse w={12} h={12} fill={c} />)}</Frame>
    <Frame flex="row" items="center" gap={12} w={560}>{IconBtn('ph:folder-open', { c: dark ? '#E7EFF7' : 'p' })}{project ? T(project, { s: 15, w: 600 }) : T('Kein Projekt geöffnet', { s: 15, c: 'text/sekundaer' })}{project && <Frame flex="row" items="center" px={8} h={18} rounded={9} bg={v('flaeche/deck')}>{T(game, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>}{project && (unsaved ? <Frame flex="row" items="center" gap={6}><Ellipse w={8} h={8} fill={v('akzent/grund')} />{T(unsaved + (unsaved === 1 ? ' Änderung' : ' Änderungen') + ' nicht gespeichert', { s: 12, w: 600, c: 'akzent/text' })}</Frame> : <Frame flex="row" items="center" gap={4}>{I('ph:check', 12, 'ok')}{T('Alles gespeichert', { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    <Frame flex="row" items="center" gap={8} px={8} h={28} w={360} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:magnifying-glass', 18)}{T('Suchen oder Befehl eingeben', { c: 'text/sekundaer', grow: 1 })}{Kbd('⌘K')}</Frame>
    <Frame flex="row" items="center" justify="end" gap={8} grow={1}>{['ph:floppy-disk', 'ph:hammer', dark ? 'ph:sun' : 'ph:moon', 'ph:question'].map((i) => IconBtn(i, { c: dark ? '#E7EFF7' : 'p' }))}</Frame>
  </Frame>
);
// Atlas 88: Einträge 64 hoch, Symbol 24, Name 12/16; aktiv Bereichsfläche + 3 px Balken links + gefülltes Symbol; Geschichten gesperrt; unten Hilfe, Einstellungen
const ATL = ['start', 'welt', 'geschichten', 'pokemon', 'gegenstaende', 'texte', 'technik'];
const Atlas = (active, dark) => {
  const entry = (k, name, icon, on, off) => { const bg = k === 'start' || k === 'einstellungen' ? 'auswahl/flaeche' : 'bereich/' + k + '-flaeche'; const fg = k === 'start' || k === 'einstellungen' ? 'akzent/text' : 'bereich/' + k; return <Frame flex="col" items="center" justify="center" gap={4} w={88} h={64} bg={on ? v(bg) : undefined}>{on && <Frame position="absolute" x={0} y={8} w={3} h={48} rounded={2} bg={v(fg)} />}<Icon name={on ? icon + '-fill' : icon} size={24} color={on ? (HX[k] || HX.at) : off ? (dark ? '#5A637B' : HX.d) : (dark ? '#ADB5C6' : HX.s)} />{T(name, { s: 12, w: on ? 600 : 400, c: on ? fg : off ? 'text/deaktiviert' : 'text/sekundaer' })}</Frame>; };
  return <Frame name="Atlas" flex="col" pt={8} pb={8} w={88} h="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {ATL.map((k) => entry(k, BER[k][1], BER[k][0], k === active, k === 'geschichten'))}
    <Frame grow={1} />
    {entry('hilfe', 'Hilfe', 'ph:question', false)}{entry('einstellungen', 'Einstellungen', 'ph:gear-six', active === 'einstellungen')}
  </Frame>;
};
// Liste 272: Fläche blatt, Linie rechts
const ListPane = (children, w = 272) => <Frame name="Liste" flex="col" w={w} h="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')} overflow="hidden">{children}</Frame>;
// Prüflicht im Scharnier: Punkt mit Ring + drei Zähler (0 gedämpft); state 'befund' | 'stimmig' | 'pruefen' | 'ungeprueft'
const Light = ({ state = 'befund', e = 0, w = 101, i = 10 } = {}) => {
  const L = '#E7EFF7', LL = '#C6CEDE', D = '#6B7384';
  const st = state === 'stimmig' ? 'erfolg' : e ? 'fehler' : w ? 'warnung' : 'info';
  const cnt = (icon, n) => <Frame flex="row" items="center" gap={3}><Icon name={icon} size={14} color={n ? L : D} />{T(String(n), { c: n ? 'scharnier/text' : 'scharnier/leise' })}</Frame>;
  return <Frame flex="row" items="center" gap={6} px={8} h={32} rounded={6}>
    {state === 'ungeprueft' ? <Ellipse w={8} h={8} stroke={LL} strokeWidth={1.5} /> : state === 'pruefen' ? <Ellipse w={12} h={12} stroke={LL} strokeWidth={2} /> : <Ellipse w={12} h={12} fill={v('status/' + st + '-flaeche')} stroke={v('status/' + st)} strokeWidth={2} />}
    {state === 'stimmig' && T('Alles stimmig', { c: 'scharnier/text' })}
    {state === 'ungeprueft' && T('Nicht geprüft', { c: 'scharnier/text' })}
    {state === 'pruefen' && T('Prüft … Texte 312/724', { c: 'scharnier/text' })}
    {(state === 'befund' || state === 'pruefen') && <Frame flex="row" items="center" gap={8}>{cnt('ph:x-circle-fill', e)}{cnt('ph:warning-fill', w)}{cnt('ph:info-fill', i)}</Frame>}
  </Frame>;
};
// Scharnier 44 (Kanten 2 px oben/unten): Griff, Brotkrume (max. 3, letzte 600, Kennung grau), Tiefe Bauen | Tüfteln, Prüflicht, Verlauf, Wiederholen, Hier spielen
// crumbs: [['Zweiblattdorf'], ['Außen', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']]; menu: Index des Glieds mit Menüpfeil
const Scharnier = ({ crumbs = [], depth = 'Bauen', light = {}, undo = 0, menu } = {}) => {
  const L = '#E7EFF7', LL = '#C6CEDE';
  return (
    <Frame name="Scharnier" flex="row" items="center" gap={12} pl={4} pr={12} h={44} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('scharnier/kante')}>
      <Frame w={24} h={32} flex="row" items="center" justify="center"><Icon name="ph:dots-six-vertical" size={18} color={LL} /></Frame>
      <Frame flex="row" items="center" gap={8} grow={1}>{crumbs.map(([n, tech], idx) => { const last = idx === crumbs.length - 1; return <Frame flex="row" items="center" gap={8}>{idx > 0 && T('›', { c: 'scharnier/leise' })}{T(n, { w: last ? 600 : 400, c: last ? 'scharnier/text' : 'scharnier/leise' })}{tech && T(tech, { f: M, s: 12, c: 'scharnier/leise' })}{menu === idx && <Icon name="ph:caret-down" size={16} color={LL} />}</Frame>; })}</Frame>
      <Frame flex="row" items="center" gap={2} p={2} h={32} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWidth={1}>{[['Bauen', 'ph:hammer'], ['Tüfteln', 'ph:wrench']].map(([o, i]) => { const on = o === depth; return <Frame flex="row" items="center" gap={6} px={12} h="fill" rounded={4} bg={on ? v('scharnier/kante') : undefined} stroke={on ? v('scharnier/leise') : undefined} strokeWidth={1}><Icon name={on ? i + '-fill' : i} size={18} color={L} />{T(o, { w: 600, c: 'scharnier/text' })}</Frame>; })}</Frame>
      {Light(light)}
      <Frame flex="row" items="center" gap={6} px={8} h={32}><Icon name="ph:clock-counter-clockwise" size={18} color={L} />{T(String(undo), { w: 600, c: 'scharnier/text' })}</Frame>
      <Frame w={32} h={32} flex="row" items="center" justify="center"><Icon name="ph:arrow-clockwise" size={18} color={undo ? L : '#6B7384'} /></Frame>
      <Frame flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('live/grund')}>{I('ph:play-fill', 18, 'live')}{T('Hier spielen', { w: 700, c: 'live/auf' })}{Kbd('⌘R', true)}</Frame>
    </Frame>
  );
};
// Scharnier im Assistenten (WizardBar): Kante oben bereich/welt-punkt, Chip, Titel Zen Maru 15/20, Schritte als Pillen, rechts Abbrechen | Zurück ⌘[ | Weiter ↵
const WizardBar = ({ title, steps = [], step = 0, last = 'Anlegen', b = 'welt' } = {}) => {
  const L = '#E7EFF7', LL = '#C6CEDE';
  return (
    <Frame name="Scharnier (Assistent)" flex="row" items="center" gap={16} pl={4} pr={12} h={44} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('bereich/' + b + '-punkt')}>
      <Frame w={24} h={32} flex="row" items="center" justify="center"><Icon name="ph:dots-six-vertical" size={18} color={LL} /></Frame>
      <Frame flex="row" items="center" gap={8}><Frame w={26} h={26} rounded={6} bg={v('bereich/' + b + '-flaeche')} flex="row" items="center" justify="center">{I(BER[b][0] + '-fill', 16, b)}</Frame>{T(title, { f: Z, w: 700, s: 15, c: 'scharnier/text' })}</Frame>
      <Frame flex="row" items="center" gap={4} grow={1} justify="center">{steps.map((n, i) => { const s = i < step ? 'fertig' : i === step ? 'aktiv' : 'offen'; return <Frame flex="row" items="center" gap={4}>{i > 0 && <Frame w={16} h={2} rounded={1} bg={v(s === 'offen' ? 'scharnier/kante' : 'scharnier/leise')} />}<Frame flex="row" items="center" gap={6} pl={4} pr={8} h={28} rounded={14} bg={s === 'aktiv' ? v('scharnier/text') : undefined}><Frame w={20} h={20} rounded={10} flex="row" items="center" justify="center" bg={s === 'aktiv' ? v('scharnier/grund') : undefined} stroke={s === 'aktiv' ? undefined : v('scharnier/leise')} strokeWidth={1}>{s === 'fertig' ? <Icon name="ph:check-bold" size={12} color={L} /> : T(String(i + 1), { s: 12, w: 700, c: s === 'aktiv' ? 'scharnier/text' : 'scharnier/leise' })}</Frame>{T(n, { w: s === 'aktiv' ? 600 : 400, c: s === 'aktiv' ? 'scharnier/grund' : 'scharnier/leise' })}</Frame></Frame>; })}</Frame>
      <Frame flex="row" items="center" gap={8}><Frame flex="row" items="center" px={12} h={32}>{T('Abbrechen', { w: 600, c: 'scharnier/text' })}</Frame>{step > 0 && <Frame flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWeights={{ top: 1, right: 1, bottom: 2, left: 1 }}>{T('Zurück', { w: 600, c: 'scharnier/text' })}</Frame>}{step > 0 && Kbd('⌘[', true)}<Frame flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('akzent/grund')}>{T(step === steps.length - 1 ? last : 'Weiter', { w: 600, c: 'akzent/auf' })}</Frame>{Kbd('↵', true)}</Frame>
    </Frame>
  );
};
// Statusleiste 28: links Werkzeug + Kürzel, Mitte Kontext, rechts Befunde, Build-Stand, Dichte
const Statusbar = ({ tool, keys = [], info, problems = '101 Warnungen, 10 Hinweise', probArt = 'warn', build = 'Noch nicht gebaut', built } = {}) => (
  <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" items="center" gap={6}>{tool && T(tool, { s: 12, w: 600 })}{keys.map(([k, a], idx) => <Frame flex="row" items="center" gap={4}>{(tool || idx > 0) && T('·', { s: 12, c: 'text/sekundaer' })}{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    {info ? T(info, { s: 12, c: 'text/sekundaer', grow: 1, one: true }) : <Frame grow={1} />}
    {problems && T(problems, { s: 12, w: 600, c: probArt === 'fehler' ? 'status/fehler' : probArt === 'ok' ? 'text/sekundaer' : 'status/warnung' })}
    <Frame flex="row" items="center" gap={4}>{built && I('ph:check-circle-fill', 12, 'ok')}{T(build, { s: 12, c: 'text/sekundaer' })}</Frame>
    {T('Komfort', { s: 12, w: 600 })}
  </Frame>
);
// Fenster 1440 × 900: Titelleiste, Zeile [Atlas | Inhalt], Statusleiste
const Window = ({ name, active, children, status, title, dark }) => (
  <Frame name={name} w={1440} h={900} flex="col" bg={v('flaeche/gehaeuse')} overflow="hidden" rounded={10} stroke={v('linie/standard')} strokeWidth={1}>
    {Titlebar({ ...(title || {}), dark })}
    <Frame flex="row" w="fill" grow={1}>{Atlas(active, dark)}<Frame flex="row" grow={1} h="fill">{children}</Frame></Frame>
    {status || Statusbar()}
  </Frame>
);
// Doppeldeck: Schau (oben), Scharnier, Hand (unten, Standard 286; 200 bis 560)
const Deck = ({ schau, scharnier, hand, handH = 286, schauBg = 'flaeche/schau' }) => (
  <Frame flex="col" grow={1} h="fill">
    <Frame name="Schau" flex="col" w="fill" grow={1} bg={v(schauBg)} overflow="hidden">{schau}</Frame>
    {scharnier}
    <Frame name="Hand" flex="col" w="fill" h={handH} bg={v('flaeche/deck')} overflow="hidden">{hand}</Frame>
  </Frame>
);
// Hand-Spalte mit Kopf (Werkzeuge 180, An diesem Ort 260): flach, 1-px-Linien
const HandCol = (title, children, w, sub) => <Frame flex="col" w={w || 'fill'} grow={w ? undefined : 1} h="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')} overflow="hidden">{title && PanelHead({ title, sub })}{children}</Frame>;
// Werkzeug-Taste: min 40, Spalte, Rand linie/leise, Radius 6, Buchstabe oben rechts Mono 10
const Tool = (name, icon, key, on, off) => <Frame flex="col" items="center" justify="center" gap={2} w={78} h={56} rounded={6} bg={on ? v('auswahl/flaeche') : undefined} stroke={v(on ? 'auswahl/kante' : 'linie/leise')} strokeWidth={1}><Frame position="absolute" x={66} y={3}>{T(key, { f: M, s: 10, c: 'text/sekundaer' })}</Frame><Icon name={on ? icon + '-fill' : icon} size={18} color={on ? HX.at : off ? '#9CA5BD' : HX.p} />{T(name, { s: 12, c: on ? 'akzent/text' : off ? 'text/deaktiviert' : 'text/primaer', align: 'center', width: 70 })}</Frame>;
// DS-Bildschirm: 256 × 192 × Faktor, Grund szene/ds/grund, Box 248 × 54 (Pixelify 12/16 × Faktor)
const DSScreen = ({ scale = 2, lines = [], sign, children, bg }) => { const W = 256 * scale, H = 192 * scale; return (
  <Frame w={W} h={H} rounded={2} overflow="hidden" bg={bg || v('ds/grund')}>
    {children}
    {lines.length > 0 && <Frame position="absolute" x={4 * scale} y={H - 58 * scale} w={248 * scale} h={54 * scale} rounded={4 * scale} bg={v(sign ? 'ds/schild/flaeche' : 'ds/textbox/flaeche')} stroke={v(sign ? 'ds/schild/rahmen' : 'ds/textbox/rahmen')} strokeWidth={2 * scale} px={9 * scale} py={6 * scale} flex="col" gap={0}>{lines.map((l) => <Text font={PX} size={12 * scale} lineHeight={16 * scale} color={v(sign ? 'ds/schild/text' : 'ds/textbox/text')}>{l}</Text>)}<Frame position="absolute" x={234 * scale} y={40 * scale}><Icon name="ph:caret-down-fill" size={10 * scale} color="#4A5263" /></Frame></Frame>}
  </Frame>
); };
// Statusmarke für Zielbild-Seiten und Bretter: 'umgesetzt' | 'teilweise' | 'offen'
const Stand = (art, text) => { const m = { umgesetzt: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok', 'Umgesetzt'], teilweise: ['status/warnung-flaeche', 'status/warnung', 'ph:circle-half-fill', 'w', 'Teilweise umgesetzt'], offen: ['flaeche/deck', 'text/sekundaer', 'ph:circle-dashed', 's', 'Zielbild, noch offen'] }[art]; return <Frame flex="row" items="center" gap={6} px={10} h={28} rounded={14} bg={v(m[0])}>{I(m[2], 16, m[3])}{T(m[4], { w: 600, c: m[1] })}{text && T(text, { c: 'text/sekundaer' })}</Frame>; };
// ===== Werkbank (Phase 3e.4, 25.10.2026): ein Dock je Bereich, Scharnier oben, schwebende Teile =====
// Nur Zusätze: die Bausteine oben bleiben, wie sie sind (andere Screens und Zweige nutzen sie). Quellen: ui/frame/{Hinge,FloatingToolbar}.tsx,
// workbench/{SceneHierarchy,WorldExplorer,WorldTree,SceneBar,Legend}.tsx, workbench/inspector/InspectorHead.tsx, ui/primitives/{TreeRow,Menu},
// ui/tokens/dockview-theme.css. Vorlagen: beispiele/werkbank-figur.body.js, beispiele/werkbank-schwebend.body.js.
const DARK = typeof DARKMAP !== 'undefined';
// Symbolfarben des dunklen Themas aus DARKMAP; ein Body schaltet sie mit `if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);` ein (nur für seinen Screen)
const HX_DUNKEL = DARK ? { p: DARKMAP['text/primaer'], s: DARKMAP['text/sekundaer'], d: DARKMAP['text/deaktiviert'], a: DARKMAP['akzent/grund'], at: DARKMAP['akzent/text'], auf: DARKMAP['akzent/auf'], f: DARKMAP['status/fehler'], w: DARKMAP['status/warnung'], ok: DARKMAP['status/erfolg'], info: DARKMAP['status/info'], live: DARKMAP['live/auf'], welt: DARKMAP['bereich/welt'] } : null;
// schatten/schwebend: Platin 1000 (#101821) mit 24 % in Hell, 48 % in Dunkel, 0 8 24. OpenPencil bindet Schatten nicht an Variablen.
const SCHATTEN = DARK ? '0 8 24 #1018217A' : '0 8 24 #1018213D';
// Dichte in px: Komfort (Standard) und Kompakt; ein Body schaltet mit `Object.assign(DI, DI_KOMPAKT);` um. reiter = dichte/reiter, werkzeug = dichte/werkzeug,
// symbol = dichte/symbol, zeile = dichte/zeile, steuer = dichte/steuer, innen = dichte/zeile-innen, scharnier = dichte/scharnier.
const DI = { reiter: 28, werkzeug: 40, symbol: 18, zeile: 32, steuer: 32, innen: 12, scharnier: 44 };
const DI_KOMPAKT = { reiter: 24, werkzeug: 32, symbol: 16, zeile: 24, steuer: 24, innen: 8, scharnier: 36 };
// Taste „Schau maximieren“ im Scharnier (⤢, aria-pressed): gedrückt mit der Fläche des gewählten Segments
const MaxButton = (on) => <Frame name="Schau maximieren" w={32} h={32} rounded={6} flex="row" items="center" justify="center" bg={on ? v('scharnier/kante') : undefined} stroke={on ? v('scharnier/leise') : undefined} strokeWidth={1}><Icon name={on ? 'ph:arrows-in-simple' : 'ph:arrows-out-simple'} size={18} color="#E7EFF7" /></Frame>;
// Scharnier oben (3e.1, 3e.4): Kopfleiste über dem Dock, ohne Griff, nicht ziehbar; rechts neu „Schau maximieren“. crumbs wie bei Scharnier.
const ScharnierOben = ({ crumbs = [], depth = 'Bauen', light = {}, undo = 0, maximized = false } = {}) => {
  const L = '#E7EFF7';
  return (
    <Frame name="Scharnier" flex="row" items="center" gap={12} pl={12} pr={8} h={DI.scharnier} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('scharnier/kante')}>
      <Frame name="Brotkrume" flex="row" items="center" gap={8} grow={1} overflow="hidden">{crumbs.map(([n, tech], idx) => { const last = idx === crumbs.length - 1; return <Frame flex="row" items="center" gap={8}>{idx > 0 && T('›', { c: 'scharnier/leise' })}{T(n, { w: last ? 600 : 400, c: last ? 'scharnier/text' : 'scharnier/leise' })}{tech && T(tech, { f: M, s: 12, c: 'scharnier/leise', one: last })}</Frame>; })}</Frame>
      <Frame name="Tiefe" flex="row" items="center" gap={2} p={2} h={32} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWidth={1}>{[['Bauen', 'ph:hammer'], ['Tüfteln', 'ph:wrench']].map(([o, i]) => { const on = o === depth; return <Frame flex="row" items="center" gap={6} px={12} h="fill" rounded={4} bg={on ? v('scharnier/kante') : undefined} stroke={on ? v('scharnier/leise') : undefined} strokeWidth={1}><Icon name={on ? i + '-fill' : i} size={18} color={L} />{T(o, { w: 600, c: 'scharnier/text' })}</Frame>; })}</Frame>
      {Light(light)}
      <Frame name="Verlauf" flex="row" items="center" gap={6} px={8} h={32}><Icon name="ph:clock-counter-clockwise" size={18} color={L} />{T(String(undo), { w: 600, c: 'scharnier/text' })}</Frame>
      <Frame name="Wiederholen" w={32} h={32} flex="row" items="center" justify="center"><Icon name="ph:arrow-clockwise" size={18} color={undo ? L : '#6B7384'} /></Frame>
      <Frame name="Hier spielen" flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('live/grund')}>{I('ph:play-fill', 18, 'live')}{T('Hier spielen', { w: 700, c: 'live/auf' })}{Kbd('⌘R', true)}</Frame>
      {MaxButton(maximized)}
    </Frame>
  );
};
// Dock-Reiter (Dockview, dockview-theme-g4): Höhe dichte/reiter, Fläche gehaeuse, Text 13. Der sichtbare Reiter jeder Gruppe trägt 2 px Kobalt oben;
// in der Gruppe mit Fokus dazu flaeche/schau und text/primaer. state 'aktiv' | 'sichtbar' | 'ruhe'; o.dot: ungespeichert (Kobalt-Punkt, kein Schließen)
const DockTab = (label, state = 'ruhe', o = {}) => <Frame name={'Reiter ' + label} flex="row" items="center" gap={6} px={8} h={DI.reiter} bg={state === 'aktiv' ? v('flaeche/schau') : undefined} strokeWeights={state === 'ruhe' ? undefined : { top: 2, right: 0, bottom: 0, left: 0 }} stroke={state === 'ruhe' ? undefined : v('akzent/grund')}>{T(label, { c: state === 'aktiv' ? 'text/primaer' : 'text/sekundaer' })}{o.dot && <Ellipse w={8} h={8} fill={v('akzent/grund')} />}</Frame>;
// Reiterleiste einer Gruppe: tabs ['3D', 'Raster'] oder [['Scriptdatei 0412', true]] (zweiter Wert: ungespeichert); focused: die Gruppe hat den Fokus
const DockTabBar = (tabs, active, focused) => <Frame name="Reiterleiste" flex="row" items="end" h={DI.reiter} w="fill" bg={v('flaeche/gehaeuse')}>{tabs.map((t) => { const [l, dot] = Array.isArray(t) ? t : [t]; return DockTab(l, l === active ? (focused ? 'aktiv' : 'sichtbar') : 'ruhe', { dot }); })}</Frame>;
// Angedockte Gruppe: Reiterleiste und Inhalt, flach, ohne Rundung und Schatten. Grenze zur Nachbargruppe (Dockviews Sash) als 1-px-Linie linie/leise:
// edge 'left' oder 'top'. Ohne w/h füllt sie (grow).
const DockGroup = ({ name, tabs, active, focused, w, h: gh, edge, bg = 'flaeche/blatt', children }) => <Frame name={name} flex="col" w={w || 'fill'} h={gh || 'fill'} grow={w || gh ? undefined : 1} bg={v('flaeche/gehaeuse')} strokeWeights={edge ? { top: edge === 'top' ? 1 : 0, right: 0, bottom: 0, left: edge === 'left' ? 1 : 0 } : undefined} stroke={edge ? v('linie/leise') : undefined} overflow="hidden">{DockTabBar(tabs, active, focused)}<Frame name="Inhalt" flex="col" w="fill" grow={1} bg={v(bg)} overflow="hidden">{children}</Frame></Frame>;
// Schwebende Gruppe (Dockview floating, Ziehen an der Titelleiste): flaeche/gehaeuse unter den Reitern, Rand linie/standard, radius/fenster 12,
// schatten/schwebend; Titelleiste 22 hoch (Dockview) in flaeche/deck mit Linie linie/standard. x, y relativ zum Dock.
const FloatingGroup = ({ name, x, y, w = 340, h: fh = 480, tabs, active, focused = true, children }) => <Frame name={name} position="absolute" x={x} y={y} w={w} h={fh} flex="col" rounded={12} bg={v('flaeche/gehaeuse')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN} overflow="hidden">
  <Frame name="Titelleiste (Griff)" w="fill" h={22} bg={v('flaeche/deck')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')} />
  {DockTabBar(tabs, active, focused)}
  <Frame name="Inhalt" flex="col" w="fill" grow={1} bg={v('flaeche/blatt')} overflow="hidden">{children}</Frame>
</Frame>;
// Kontextmenü (Menü-Primitive): blatt, Rand linie/standard, radius/taste, Schatten; Einträge dichte/steuer hoch, 0 12 innen; '-' ist ein Trenner.
// Eintrag { label, off (gesperrt), hi (unter dem Zeiger: auswahl/flaeche, akzent/text), kbd }
const ContextMenu = ({ x, y, w = 220, items }) => <Frame name="Kontextmenü" position="absolute" x={x} y={y} w={w} flex="col" p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{items.map((it) => it === '-' ? <Frame name="Trenner" flex="col" w="fill" py={4}><Frame w="fill" h={1} bg={v('linie/leise')} /></Frame> : <Frame name={it.label} flex="row" items="center" gap={8} px={12} h={DI.steuer} w="fill" rounded={4} bg={it.hi ? v('auswahl/flaeche') : undefined}>{T(it.label, { c: it.off ? 'text/deaktiviert' : it.hi ? 'akzent/text' : 'text/primaer', grow: 1 })}{it.kbd && Kbd(it.kbd)}</Frame>)}</Frame>;
// Werkzeugleiste schwebend (FloatingToolbar): Griff und Pfeil des Menüs „Lage der Werkzeugleiste“ (je 24), Tasten dichte/werkzeug mit Buchstabe oben rechts
// (Mono 10), Gruppen durch Linien getrennt; aktiv auswahl/flaeche, Kante auswahl/kante, akzent/text. anchor 'links' (senkrecht) | 'oben' | 'unten' (waagrecht).
const WERKZEUGE = [[['Auswählen', 'ph:cursor', 'V']], [['Gelände malen', 'ph:paint-brush', 'B']], [['Figur', 'ph:person-simple-walk', 'F'], ['Tür', 'ph:door', 'T'], ['Auslöser', 'ph:lightning', 'A'], ['Schild', 'ph:signpost', 'S']], [['Gebäude', 'ph:cube', 'G']]];
const ToolKey = ([name, icon, key], on, off) => <Frame name={name} w={DI.werkzeug} h={DI.werkzeug} rounded={6} flex="row" items="center" justify="center" bg={on ? v('auswahl/flaeche') : undefined} stroke={on ? v('auswahl/kante') : undefined} strokeWidth={1}><Frame position="absolute" x={DI.werkzeug - 9} y={2}>{T(key, { f: M, s: 10, lh: 12, c: on ? 'akzent/text' : off ? 'text/deaktiviert' : 'text/sekundaer' })}</Frame>{I(icon, DI.symbol, on ? 'at' : off ? 'd' : 'p')}</Frame>;
const FloatingToolbar = ({ tool = 'Auswählen', anchor = 'links', x = 8, y = 56, blocked = false } = {}) => {
  const up = anchor === 'links';
  const sep = () => <Frame name="Trenner" w={up ? 48 : 1} h={up ? 1 : DI.werkzeug} bg={v('linie/leise')} />;
  const keys = WERKZEUGE.flatMap((g, gi) => [...(gi > 0 ? [sep()] : []), ...g.map((t) => ToolKey(t, t[0] === tool, blocked && gi === 2))]);
  return <Frame name="Werkzeugleiste" position="absolute" x={x} y={y} flex={up ? 'col' : 'row'} items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Griff und Lage" flex="row" h={up ? 24 : DI.werkzeug}><Frame w={24} h="fill" flex="row" items="center" justify="center">{I(up ? 'ph:dots-six' : 'ph:dots-six-vertical', DI.symbol, 's')}</Frame><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:caret-down', DI.symbol, 's')}</Frame></Frame>
    {keys}
  </Frame>;
};
// Ebenen der Schau (SceneBar, oben links) und Legende (unten links): schwebend, mit Schatten
const LayerBar = (x = 8, y = 8) => <Frame name="Ebenen der Schau" position="absolute" x={x} y={y} flex="row" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{IconBtn('ph:stack', { on: true })}{IconBtn('ph:map-pin', { on: true })}</Frame>;
const LegendBtn = (x, y) => <Frame name="Legende" position="absolute" x={x} y={y} flex="row" items="center" gap={8} px={12} h={32} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{I('ph:list-bullets', DI.symbol, 'p')}{T('Legende', { w: 600 })}</Frame>;
// 3D-Szene als Platzhalter (keine Spielgrafik): Kacheln in Grün und Ocker, Dächer, Wasser wie in welt-figur.body.js; Marker in den Szenen-Farben
// (ereignis/figur Kreis, ereignis/warp Quadrat), die gewählte Figur mit Halo auswahl/halo. sel: Index der gewählten Figur.
const Welt3D = ({ w, h: H, sel = 2, children }) => {
  const pal = ['#7B9C31', '#94AD42', '#C68C52', '#6B8C29', '#A5BD5A', '#B57B4A', '#5A7B21', '#8CAD39'];
  const cw = 58, chh = 46, ox = Math.round((w - 9 * cw) / 2) + 48, oy = Math.round((H - 7 * chh) / 2);
  const tiles = [];
  for (let r = 0; r < 7; r++) for (let c = 0; c < 9; c++) { if ((r === 0 && c > 6) || (r === 6 && c < 2) || (r === 2 && c === 4) || (r === 3 && c === 4)) continue; tiles.push(<Frame position="absolute" x={ox + c * cw - r * 14} y={oy + r * chh} w={56} h={44} bg={pal[(r * 3 + c * 5) % 8]} />); }
  const at = (c, r) => [ox + c * cw - r * 14 + 28, oy + r * chh + 22];
  const figs = [[1, 1], [3, 1], [2, 3], [5, 2], [6, 4], [3, 5], [7, 3], [5, 5]].map(([c, r], i) => { const [x, y] = at(c, r); return i === sel ? <Frame position="absolute" x={x - 12} y={y - 12} w={24} h={24} rounded={12} bg={v('ereignis/figur')} stroke={v('auswahl/halo')} strokeWidth={3} /> : <Frame position="absolute" x={x - 9} y={y - 9} w={18} h={18} rounded={9} bg={v('ereignis/figur')} stroke="#101821" strokeWidth={1} />; });
  const doors = [[6, 1], [7, 1], [2, 4], [4, 6]].map(([c, r]) => { const [x, y] = at(c, r); return <Frame position="absolute" x={x - 7} y={y - 7} w={14} h={14} rounded={2} bg={v('ereignis/warp')} stroke="#101821" strokeWidth={1} />; });
  return <Frame name="3D-Szene" w={w} h={H} bg={v('flaeche/welt')} overflow="hidden">
    {tiles}
    <Frame position="absolute" x={ox + 340} y={oy + 40} w={60} h={40} bg="#E7A57B" /><Frame position="absolute" x={ox + 420} y={oy + 30} w={46} h={60} bg="#E7A57B" /><Frame position="absolute" x={ox + 150} y={oy + 130} w={30} h={24} bg="#3173C6" /><Frame position="absolute" x={ox + 290} y={oy + 190} w={40} h={20} bg="#3173C6" />
    {doors}{figs}{children}
  </Frame>;
};
// Baumzeile (TreeRow): Einzug dichte/zeile-innen + 16 je Ebene, Pfeil 18 (leer bei Blättern), Symbol, Name, Kennung grau Mono 12, rechts Zähler und Zeichen.
// Feste Höhe (dichte/zeile, mit zweiter Zeile 40): mit overflow hidden und nur minH blieb die Zeile in OpenPencil 100 hoch.
// open true/false: Zweig offen/zu, undefined: Blatt. sel: gewählt (auswahl/flaeche, akzent/text, 2 px auswahl/kante links). sub: zweite Zeile (Tür: „→ Haus 1“).
// techW: Breite der Kennung (sie weicht zuerst, gekürzt mit …); count: Zähler rechts (Mono 12, sekundär); shown: Zeichen „Gezeigt“ (ShownMark, ph:grid-four in bereich/welt); end: weitere Zeichen rechts
const TreeRowX = ({ label, tech, techW, sub, icon, depth = 0, open, sel, count, shown, end }) => <Frame name={label} flex="row" items="center" gap={8} pl={DI.innen - 4 + depth * 16} pr={DI.innen} h={sub ? 40 : DI.zeile} w="fill" rounded={4} overflow="hidden" bg={sel ? v('auswahl/flaeche') : undefined} strokeWeights={sel ? { top: 0, right: 0, bottom: 0, left: 2 } : undefined} stroke={sel ? v('auswahl/kante') : undefined}>
  <Frame w={DI.symbol} h={DI.symbol} flex="row" items="center" justify="center">{open !== undefined && I(open ? 'ph:caret-down' : 'ph:caret-right', DI.symbol, 's')}</Frame>
  {icon && I(icon, DI.symbol, sel ? 'at' : 'p')}
  {sub ? <Frame flex="col" py={4}>{T(label, { c: sel ? 'akzent/text' : 'text/primaer', one: true })}{T(sub, { s: 12, c: sel ? 'akzent/text' : 'text/sekundaer', one: true })}</Frame> : T(label, { c: sel ? 'akzent/text' : 'text/primaer', one: true })}
  {tech && Mono(tech, { one: true, width: techW })}
  <Frame grow={1} />
  {end}{shown && <Frame name="Gezeigt" flex="row" items="center">{I('ph:grid-four', DI.symbol, 'welt')}</Frame>}{count !== undefined && Mono(String(count))}
</Frame>;
// Teiler (Splitter) waagrecht: 8 px Griff, Haarlinie linie/leise in der Mitte
const Teiler = (name = 'Teiler') => <Frame name={name} flex="col" justify="center" w="fill" h={8}><Frame w="fill" h={1} bg={v('linie/leise')} /></Frame>;
// Inspector-Kopf (InspectorHead): Augenbraue (Symbol 12 + Typ-Wort, text/xs sekundär), Titel titel/s (Zen Maru Gothic 24/30 700, eine Zeile),
// Kennung code/s grau; rechts Plaketten ([['Ungespeichert', 'ungespeichert']]) und das Menü „Aktionen“ (ph:dots-three); unten 1 px linie/leise
const InspectorHead = ({ word, icon, title, ident, badges = [], menu = true }) => <Frame name="Inspector-Kopf" flex="row" items="start" gap={8} pt={8} pb={8} pl={DI.innen} pr={8} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
  <Frame flex="col" gap={2} grow={1}>
    <Frame name="Augenbraue" flex="row" items="center" gap={4}>{I(icon, 12, 's')}{T(word, { s: 12, c: 'text/sekundaer' })}</Frame>
    {T(title, { f: Z, s: 24, lh: 30, w: 700, one: true })}
    {ident && Mono(ident, { one: true })}
  </Frame>
  {badges.length > 0 && <Frame name="Zustände" flex="row" gap={4}>{badges.map(([l, a]) => Badge(l, a))}</Frame>}
  {menu && <Frame name="Aktionen" flex="row" items="center" justify="center" px={8} h={DI.steuer} rounded={6}>{I('ph:dots-three', DI.symbol, 'p')}</Frame>}
</Frame>;
// ===== Weltmodell (Phase 3f, 26.10.2026): Weltkarte mit Umschalter, Raumplan, Ort, Patch-Zeile, Plakette „Geteilt“ =====
// Nur Zusätze: die Bausteine oben bleiben, wie sie sind. Quellen: workbench/{AreaGrid,WorldMapPanel,MoreAreas,WorldTree,SceneBar}.tsx,
// workbench/places/{PlacePicker,NewPlaceDialog}.tsx, workbench/wizards/{PlaceStep,LookPicker,PatchConsent,PlaceOnWorldMap,WizardFrame}.tsx,
// workbench/inspector/{HeaderEditor,MatrixCellEditor,SplitRoomDialog}.tsx. Vorlagen: beispiele/weltkarte-raumplan, assistent-ort,
// zusammenfassung-patch, inspector-geteilt (je .body.js); Anleitung NACHTRAG-3f.md.
// Zelle des Gitters (AreaGrid, Abstand 2): size 64 (Raumplan) oder 32 (Weltkarte, Assistent). map: Kartennummer ('0342'); pic: Platzhalter des
// Kartenbilds (Beispiel-Hex, keine Spielgrafik); tint: Ortsfarbe (Unterkante 4 px; ohne Bild auch die Fläche); sel: gewählt (2 px auswahl/kante);
// err: Block, der nicht dorthin darf (2 px status/fehler). Ohne map: leere Zelle („—“) oder mit plus: „+“ (beide gestrichelt linie/stark).
const GridCell = ({ size = 64, map, pic, tint, sel, plus, err } = {}) => {
  if (!map) return <Frame name={plus ? '+' : 'Leere Zelle'} w={size} h={size} flex="row" items="center" justify="center" stroke={v('linie/stark')} strokeWidth={1} strokeDash={[4, 3]} dashPattern={[4, 3]}>{T(plus ? '+' : '—', { s: plus ? 15 : 12, f: plus ? F : M, c: 'text/sekundaer' })}</Frame>;
  const tag = <Frame name="Kartennummer" px={4} h={14} rounded={6} bg={v('flaeche/blatt')} flex="row" items="center">{Mono(map, { c: 'text/primaer', lh: 14 })}</Frame>;
  return <Frame name={'Karte ' + map} w={size} h={size} bg={pic || tint || v('flaeche/deck')} stroke={v(err ? 'status/fehler' : sel ? 'auswahl/kante' : 'linie/standard')} strokeWidth={err || sel ? 2 : 1} overflow="hidden" flex="row" items="center" justify="center">
    {tint && <Frame position="absolute" x={0} y={size - 4} w={size} h={4} bg={tint} />}
    {size === 64 ? <Frame position="absolute" x={2} y={size - 18}>{tag}</Frame> : !pic && tag}
  </Frame>;
};
// Gitter aus Zeilen von GridCell (role="group", „Raumplan, 3 Zeilen, 4 Spalten“ bzw. „Weltkarte, …“)
const AreaGridX = (rows, name = 'Raumplan') => <Frame name={name} flex="col" gap={2}>{rows.map((r) => <Frame flex="row" gap={2}>{r}</Frame>)}</Frame>;
// Kopf des Reiters „Weltkarte“: PanelHead mit switcher (Name der Ansicht mit Pfeil), im Raumplan der Name als Untertitel
const WorldMapHead = (view, sub) => PanelHead({ title: view, sub, switcher: true });
// Taste des Pickers „Ort“ (button[data-place-picker]): sekundär, klein, 8-px-Punkt in der Ortsfarbe (Beispiel-Hex), Name; Tüfteln: tech grau in Mono
const PlaceBtn = (name, dot, tech) => <Frame name="Ort wählen" flex="col" rounded={6} bg={v('linie/stark')} pt={1} pl={1} pr={1} pb={2}><Frame flex="row" items="center" gap={6} px={8} h={21} rounded={5} bg={v('flaeche/taste')}><Ellipse w={8} h={8} fill={dot} />{T(name, { w: 600 })}{tech && Mono(tech)}</Frame></Frame>;
// Patch-Zeile (PatchConsent): Plakette „Nicht rückgängig“ und das Kästchen der einmaligen Zustimmung
const PATCH_TEXT = 'Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)';
const PatchLine = (checked) => <Frame name="Patch-Zeile" flex="row" items="center" gap={8} w="fill">{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}{Check(checked, PATCH_TEXT)}</Frame>;
// Plakette „Geteilt mit N Räumen“ (Art Info, Symbol ShareNetwork)
const SharedBadge = (n) => Badge(n === 1 ? 'Geteilt mit 1 Raum' : 'Geteilt mit ' + n + ' Räumen', 'info', 'ph:share-network');
// Inspector-Kopf mit Plaketten als Knoten (InspectorHead nimmt nur Text-Plaketten): sonst wie InspectorHead
const InspectorHeadB = ({ word, icon, title, ident, badges = [], menu = true }) => <Frame name="Inspector-Kopf" flex="row" items="start" gap={8} pt={8} pb={8} pl={DI.innen} pr={8} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
  <Frame flex="col" gap={2} grow={1}>
    <Frame name="Augenbraue" flex="row" items="center" gap={4}>{I(icon, 12, 's')}{T(word, { s: 12, c: 'text/sekundaer' })}</Frame>
    {T(title, { f: Z, s: 24, lh: 30, w: 700, one: true })}
    {ident && Mono(ident, { one: true })}
  </Frame>
  {badges.length > 0 && <Frame name="Zustände" flex="row" gap={4}>{badges}</Frame>}
  {menu && <Frame name="Aktionen" flex="row" items="center" justify="center" px={8} h={DI.steuer} rounded={6}>{I('ph:dots-three', DI.symbol, 'p')}</Frame>}
</Frame>;
// Liste „Welt“: „Orte“ (Weltbaum, orteH hoch; nicht „h“, das ist die JSX-Fabrik) über „In dieser Szene“, dazwischen der Teiler „Höhe der Orte“; orte und szene sind Baumzeilen (TreeRowX)
const WeltListe = ({ orte, szene, orteH = 248, count = '1 Weltkarte, 72 Gebiete, 492 Räume' }) => ListPane([PanelHead({ title: 'Welt', sub: 'Platin, DE' }), <Frame name="Spalte" flex="col" w="fill" grow={1}>
  <Frame name="Orte" flex="col" w="fill" h={orteH} overflow="hidden">
    {PanelHead({ title: 'Orte', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:sort-ascending')}{IconBtn('ph:plus')}{IconBtn('ph:caret-down')}</Frame> })}
    <Frame flex="col" gap={6} p={8} w="fill">{T(count, { s: 12, c: 'text/sekundaer', one: true })}{Search('Gebiet, Raum oder Kürzel suchen')}</Frame>
    <Frame name="Baum „Orte“" flex="col" px={4} w="fill">{orte}</Frame>
  </Frame>
  {Teiler('Teiler „Höhe der Orte“')}
  <Frame name="In dieser Szene" flex="col" w="fill" grow={1} overflow="hidden">
    {PanelHead({ title: 'In dieser Szene', actions: <Frame flex="row" items="center" gap={4}>{IconBtn('ph:arrow-left')}{IconBtn('ph:arrow-right', { off: true })}{IconBtn('ph:plus')}</Frame> })}
    <Frame flex="col" gap={6} p={8} w="fill">{Search('Name oder Nummer suchen')}</Frame>
    <Frame name="Baum „In dieser Szene“" flex="col" px={4} w="fill">{szene}</Frame>
  </Frame>
</Frame>]);
// Schritt des Assistenten unter dem Dock (data-wizard-hand, 286 hoch, Linie linie/leise oben): Spalten (cols) und der Fuß mit dem ersten Satz
// der Zusammenfassung und der Plakette „Nicht rückgängig“ (WizardFrame summary/irreversible)
const WizardCol = (children, first) => <Frame flex="col" gap={8} px={16} py={12} grow={1} h="fill" strokeWeights={first ? undefined : { top: 0, right: 0, bottom: 0, left: 1 }} stroke={first ? undefined : v('linie/leise')}>{children}</Frame>;
const WizardHand = ({ name, cols, foot }) => <Frame name={name} flex="col" w="fill" h={286} bg={v('flaeche/deck')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
  <Frame flex="row" w="fill" grow={1}>{cols}</Frame>
  {foot && <Frame name="Fuß: Zusammenfassung" flex="row" items="center" gap={8} px={16} py={8} w="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>{I('ph:list', 18, 's')}{T(foot, { grow: 1 })}{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}</Frame>}
</Frame>;

// ===== Agenten (Phase 3h.3, 26.10.2026): lokale Agenten über MCP =====
// Nur Zusätze. Quellen: einstellungen/AgentsSettings.tsx und .module.css, ui/frame/AgentStatus.tsx, app/AgentQuestion.tsx, ui/primitives/{Menu,Dialog}.module.css,
// ui/frame/Hinge.tsx (Verlaufsmenü). Vorlagen: beispiele/agenten-*.body.js.
// Menü (Menu-Primitive): blatt, Rand linie/standard, radius/taste, schatten/schwebend; Einträge dichte/steuer hoch, 0 12 innen. Eintrag
// { label, icon, detail (rechts, sekundär), check (true/false: Eintrag einer Wahl, die Haken-Spalte hält ihren Platz), hi (unter dem Zeiger), off } oder '-'
const MenuX = ({ x, y, w = 260, items }) => <Frame name="Menü" position="absolute" x={x} y={y} w={w} flex="col" p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{items.map((it) => it === '-' ? <Frame name="Trenner" flex="col" w="fill" py={4}><Frame w="fill" h={1} bg={v('linie/leise')} /></Frame> : <Frame name={it.label} flex="row" items="center" gap={8} px={12} h={DI.steuer} w="fill" rounded={4} bg={it.hi ? v('auswahl/flaeche') : undefined}>{it.check !== undefined && <Frame w={DI.symbol} h={DI.symbol} flex="row" items="center" justify="center">{it.check && I('ph:check', DI.symbol, it.hi ? 'at' : 'p')}</Frame>}{it.icon && I(it.icon, DI.symbol, it.off ? 'd' : it.hi ? 'at' : 'p')}{T(it.label, { c: it.off ? 'text/deaktiviert' : it.hi ? 'akzent/text' : 'text/primaer', grow: 1, one: true })}{it.detail && <Frame flex="row" pl={12}>{T(it.detail, { c: it.off ? 'text/deaktiviert' : it.hi ? 'akzent/text' : 'text/sekundaer', one: true })}</Frame>}</Frame>)}</Frame>;
// Agenten-Zeile der Statusleiste (AgentStatus): Taste 24 hoch, text/xs, Symbol agent (ph:robot); open: Menü offen (Fläche wie :hover)
const AgentItem = (text, open) => <Frame name="Agent verbunden" flex="row" items="center" gap={6} px={8} h={24} rounded={6} bg={open ? v('flaeche/deck') : undefined}>{I('ph:robot', 12, 'p')}{T(text, { s: 12 })}</Frame>;
// Statusleiste mit der Agenten-Zeile rechts vor dem Bauen-Stand (agent: Text, agentOpen: Menü offen); sonst wie Statusbar
const StatusbarAgent = ({ tool, keys = [], info, problems = '101 Warnungen, 10 Hinweise', build = 'Noch nicht gebaut', agent, agentOpen } = {}) => (
  <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" items="center" gap={6}>{tool && T(tool, { s: 12, w: 600 })}{keys.map(([k, a], idx) => <Frame flex="row" items="center" gap={4}>{(tool || idx > 0) && T('·', { s: 12, c: 'text/sekundaer' })}{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    {info ? T(info, { s: 12, c: 'text/sekundaer', grow: 1, one: true }) : <Frame grow={1} />}
    {problems && T(problems, { s: 12, w: 600, c: 'status/warnung' })}
    {agent && AgentItem(agent, agentOpen)}
    {T(build, { s: 12, c: 'text/sekundaer' })}
    {T('Komfort', { s: 12, w: 600 })}
  </Frame>
);
// Dialog (Dialog-Primitive): 480 breit, 20 innen, Abstand 12, blatt, Rand linie/standard, radius/fenster, schatten/schwebend; Titel text/l,
// Text text/s sekundär, Tasten rechts (gap 8), Kreuz oben rechts (dichte/steuer). actions: [{ label, art }]
const DialogX = ({ title, text, actions = [], w = 480 }) => <Frame name="Dialog" flex="col" gap={12} p={20} w={w} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
  <Frame name="Schließen" position="absolute" x={w - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
  {T(title, { s: 18, w: 600, width: w - 72 })}
  {T(text, { c: 'text/sekundaer', width: w - 40 })}
  <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{actions.map((a) => Btn(a))}</Frame>
</Frame>;
// Ein Screen mit Dialog: das Fenster, darüber die Abdunklung flaeche/ueberlagerung über das ganze Fenster, der Dialog in der Mitte
const MitDialog = (name, win, dialog) => <Frame name={name} w={1440} h={900} rounded={10} overflow="hidden">{win}<Frame name="Abdunklung" position="absolute" x={0} y={0} w={1440} h={900} flex="row" items="center" justify="center" bg={v('flaeche/ueberlagerung')}>{dialog}</Frame></Frame>;
// ===== Projektvorschau (Phase 4.0, 27.10.2026) =====
// Nur Zusätze. Quellen: ui/frame/{StartPane,ProjectPreview}.tsx und .module.css, technik/TechnikArea.tsx, ui/gallery/pages/examplePreviews.ts.
// Vorlagen: beispiele/start-vorschau.body.js, beispiele/projektkarte.body.js, beispiele/technik-projekt.body.js.
// Beispielbild 256 × 192 × s aus Szenenfarben (keine Spielgrafik, wie die Beispielbilder der Galerie), Kacheln zu 16 px.
// art 'eigen' (ein Ort von oben mit Häusern, Weg und Teich) | 'standard' (Kartenbild einer Startstadt, Meer rechts)
const VorschauBild = (art, s = 1) => {
  const k = 16 * s, R = (x, y, w, h, c, r) => <Frame position="absolute" x={x * k} y={y * k} w={w * k} h={h * k} rounded={r ? r * s : undefined} bg={v(c)} />;
  const P = (x, y, c) => <Frame position="absolute" x={x * k + 4 * s} y={y * k + 4 * s} w={8 * s} h={8 * s} rounded={4 * s} bg={v(c)} />;
  const teile = art === 'eigen'
    ? [R(0, 0, 16, 12, 'gelaende/hohes-gras'), R(0, 5, 16, 2, 'gelaende/unbekannt'), R(7, 0, 2, 12, 'gelaende/unbekannt'), R(11, 8, 4, 3, 'gelaende/wasser', 6),
       R(2, 1, 3, 3, 'gelaende/blockiert', 2), R(11, 1, 3, 3, 'gelaende/blockiert', 2), R(2, 8, 3, 3, 'gelaende/blockiert', 2),
       P(3, 4, 'ereignis/warp'), P(12, 4, 'ereignis/warp'), P(9, 6, 'ereignis/figur'), P(6, 9, 'ereignis/schild')]
    : [R(0, 0, 16, 12, 'gelaende/unbekannt'), R(0, 0, 2, 12, 'gelaende/hohes-gras'), R(13, 0, 3, 12, 'gelaende/wasser'), R(12, 0, 1, 12, 'gelaende/eis'),
       R(2, 0, 10, 2, 'gelaende/hohes-gras'), R(3, 3, 3, 3, 'gelaende/vorsprung', 2), R(8, 3, 3, 3, 'gelaende/vorsprung', 2), R(5, 8, 3, 2, 'gelaende/vorsprung', 2),
       P(4, 6, 'ereignis/warp'), P(9, 6, 'ereignis/warp'), P(6, 10, 'ereignis/warp'), P(10, 9, 'ereignis/figur')];
  return <Frame name={art === 'eigen' ? 'Vorschaubild (eigen)' : 'Vorschaubild (Standard)'} w={256 * s} h={192 * s} overflow="hidden" bg={v('ds/grund')}>{teile.map((t) => t)}</Frame>;
};
// Ohne Bild: der Name in der Spielschrift (16/20 × s) auf dem leeren DS-Bildschirm (ProjectPreview, Platzhalter)
const VorschauName = (name, s = 1) => <Frame name="Platzhalter mit Name" w={256 * s} h={192 * s} bg={v('ds/grund')} flex="row" items="center" justify="center" px={12 * s}>{T(name, { f: PX, s: 16 * s, lh: 20 * s, c: 'ds/textbox/flaeche', align: 'center' })}</Frame>;
// Projektkarte (StartPane, ProjectCard): blatt, Rand linie/standard, radius/fenster; oben 8 px eingerückt der DS-Rahmen 4 : 3 (radius/bildschirm),
// darin der Bildschirm 256 × 192; „Zuletzt“ (Info, Verlauf) oben links und „…“ oben rechts auf dem Rahmen; darunter Name, Spiel, Zeit.
// bild 'eigen' | 'standard' | null (Platzhalter); fehlt: „nicht gefunden“ (deck, grau, Bildschirm halb deckend, Taste „Aus der Liste nehmen“); menu: Einträge für MenuX
const ProjKarte = ({ name, game = 'Platin, DE', zeit = 'gerade eben', bild = null, zuletzt, fehlt, w = 309, menu, hover }) => {
  const fw = w - 16, fh = Math.round(fw * 3 / 4);
  return <Frame name={'Projektkarte: ' + name} flex="col" w={w} rounded={12} bg={v(fehlt ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(hover ? 'linie/stark' : 'linie/standard')} strokeWidth={1}>
    <Frame flex="col" w="fill" pt={8} px={8}>
      <Frame name="DS-Rahmen" w={fw} h={fh} rounded={8} bg={v('flaeche/bildschirm-rahmen')} flex="row" items="center" justify="center" opacity={fehlt ? 0.5 : undefined}>
        {bild && !fehlt ? VorschauBild(bild) : VorschauName(name)}
      </Frame>
      {zuletzt && <Frame position="absolute" x={16} y={16}>{Badge('Zuletzt', 'info', 'ph:clock-counter-clockwise')}</Frame>}
      <Frame name="Weitere Aktionen" position="absolute" x={w - 12 - 24} y={12} w={24} h={24} rounded={6} bg={v('flaeche/bildschirm-rahmen')} flex="row" items="center" justify="center"><Icon name="ph:dots-three-bold" size={16} color="#F7F7FF" /></Frame>
    </Frame>
    <Frame flex="col" gap={6} pt={8} px={8} pb={12} w="fill">
      {T(name, { s: 15, w: 600, c: fehlt ? 'text/sekundaer' : 'text/primaer', one: true, width: 'fill' })}
      <Frame flex="row" items="center" gap={8} w="fill">{Badge(game, 'neutral', 'ph:game-controller')}{fehlt ? T('nicht gefunden', { s: 12, w: 600, c: 'text/sekundaer' }) : T('geöffnet ' + zeit, { s: 12, c: 'text/sekundaer' })}</Frame>
    </Frame>
    {fehlt && <Frame flex="row" px={8} pb={12} w="fill">{Btn({ label: 'Aus der Liste nehmen', small: true })}</Frame>}
    {menu && MenuX({ x: w - 12 - 220, y: 12 + 28, w: 220, items: menu })}
  </Frame>;
};
const MENU_EIGEN = [{ label: 'Vorschaubild wählen…', hi: true }, { label: 'Vorschaubild entfernen' }, '-', { label: 'Aus der Liste nehmen' }];
const MENU_STANDARD = [{ label: 'Vorschaubild wählen…', hi: true }, '-', { label: 'Aus der Liste nehmen' }];
const MENU_FEHLT = [{ label: 'Aus der Liste nehmen', hi: true }];
// Toast unten rechts (ui/frame/toasts): blatt, Rand, Schatten; info mit Symbol Info, fehler mit Symbol Fehler
const Toast = (text, art = 'info') => <Frame name="Toast" flex="row" items="start" gap={8} px={12} py={10} w={360} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{I(art === 'fehler' ? 'ph:x-circle-fill' : 'ph:info-fill', 18, art === 'fehler' ? 'f' : 'info')}{T(text, { grow: 1, width: 'fill' })}</Frame>;
