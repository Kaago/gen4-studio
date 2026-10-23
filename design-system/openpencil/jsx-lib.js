// ===== Gen4 Studio JSX-Bausteine (für OpenPencil render) =====
const F = 'Atkinson Hyperlegible Next', M = 'Atkinson Hyperlegible Mono', Z = 'Zen Maru Gothic', PX = 'Pixelify Sans';
const v = (t) => designVar(t);
// Symbolfarben als Hex (Thema Hell), weil OpenPencil Token an Vektoren nicht zuverlässig bindet
const HX = { p: '#182131', s: '#4A5263', d: '#848CA5', a: '#3152DE', auf: '#FFFFFF', f: '#C62939', w: '#9C5A00', ok: '#186B42', info: '#2942B5', live: '#182131', sl: '#9CA5BD', st: '#E7EFF7', welt: '#186B42', geschichten: '#633994', pokemon: '#943121', gegenstaende: '#106363', texte: '#6B5229', technik: '#394252' };
const BER = { welt: ['ph:map-trifold', 'Welt'], geschichten: ['ph:chats-circle', 'Geschichten'], pokemon: ['ph:paw-print', 'Pokémon'], gegenstaende: ['ph:backpack', 'Gegenstände'], texte: ['ph:text-aa', 'Texte'], technik: ['ph:cpu', 'Technik'] };
// o.one = true: einzeilig mit Auslassung (für Felder, Listen, Leisten)
const T = (s, o = {}) => <Text font={o.f || F} size={o.s || 13} weight={o.w || 400} lineHeight={o.lh || Math.round((o.s || 13) * 1.38)} color={o.hex || v(o.c || 'text/primaer')} w={o.width} grow={o.grow} textAlign={o.align} maxLines={o.one ? 1 : undefined} truncate={o.one ? true : undefined}>{s}</Text>;
const I = (name, size = 16, c = 's') => <Icon name={name} size={size} color={HX[c] || c} />;
const Kbd = (k, dark) => <Frame flex="row" items="center" px={5} h={20} rounded={4} bg={v(dark ? 'scharnier/taste' : 'flaeche/blatt')} stroke={v(dark ? 'scharnier/kante' : 'linie/standard')} strokeWeights={{ top: 1, right: 1, bottom: 2, left: 1 }}>{T(k, { s: 12, w: 600, c: dark ? 'scharnier/text' : 'text/primaer' })}</Frame>;
// Taste: art = 'Primär' | 'Sekundär' | 'Leise' | 'Spielen' | 'Gefahr'
const Btn = ({ label, art = 'Sekundär', icon, kbd, grow }) => {
  const S = { 'Primär': ['akzent/grund', 'akzent/auf', 'auf'], 'Sekundär': ['flaeche/taste', 'text/primaer', 'p'], 'Leise': [null, 'text/primaer', 'p'], 'Spielen': ['live/grund', 'live/auf', 'live'], 'Gefahr': ['status/fehler', 'akzent/auf', 'auf'] }[art];
  return <Frame flex="row" items="center" justify="center" gap={6} px={art === 'Leise' ? 10 : 14} h={32} rounded={6} grow={grow} bg={S[0] ? v(S[0]) : undefined} stroke={art === 'Sekundär' ? v('linie/stark') : undefined} strokeWeights={art === 'Sekundär' ? { top: 1, right: 1, bottom: 2, left: 1 } : undefined}>{icon && I(icon, 16, S[2])}{T(label, { w: 600, c: S[1] })}{kbd && Kbd(kbd)}</Frame>;
};
const IconBtn = (icon, active) => <Frame w={32} h={32} rounded={6} bg={active ? v('auswahl/flaeche') : undefined} flex="row" items="center" justify="center">{I(icon, 18, active ? 'a' : 's')}</Frame>;
// Referenz-Chip (klickbar) und Plaketten
const Ref = (b, label, icon) => <Frame flex="row" items="center" gap={6} px={8} h={24} rounded={6} bg={v('bereich/' + b + '-flaeche')}>{I(icon || BER[b][0], 14, b)}{T(label, { w: 600, c: 'bereich/' + b })}</Frame>;
const Badge = (label, art = 'neutral') => { const m = { neutral: ['flaeche/deck', 'text/primaer'], warn: ['status/warnung-flaeche', 'status/warnung'], fehler: ['status/fehler-flaeche', 'status/fehler'], ok: ['status/erfolg-flaeche', 'status/erfolg'], info: ['status/info-flaeche', 'status/info'], live: ['live/flaeche', 'live/text'], technik: ['bereich/technik-flaeche', 'bereich/technik'] }[art]; return <Frame flex="row" items="center" gap={6} px={8} h={22} rounded={11} bg={v(m[0])}>{T(label, { s: 12, w: 600, c: m[1] })}</Frame>; };
const TY = { Normal: ['#A5A57B', '#182131'], Feuer: ['#EF8431', '#182131'], Wasser: ['#6B8CEF', '#182131'], Pflanze: ['#7BC652', '#182131'], Elektro: ['#F7CE31', '#182131'], Eis: ['#94D6D6', '#182131'], Kampf: ['#BD3129', '#FFFFFF'], Gift: ['#9C429C', '#FFFFFF'], Boden: ['#DEBD6B', '#182131'], Flug: ['#A58CEF', '#182131'], Psycho: ['#F75A84', '#182131'], 'Käfer': ['#A5B521', '#182131'], Gestein: ['#B59C39', '#182131'], Geist: ['#735A94', '#FFFFFF'], Drache: ['#7339F7', '#FFFFFF'], Unlicht: ['#735A4A', '#FFFFFF'], Stahl: ['#B5B5CE', '#182131'], '???': ['#6B9C8C', '#182131'] };
const Typ = (t, w) => <Frame flex="row" items="center" justify="center" px={8} w={w} h={22} rounded={11} bg={TY[t][0]}>{T(t, { s: 12, w: 700, hex: TY[t][1] })}</Frame>;
// Eingaben
const Field = (text, o = {}) => <Frame flex="row" items="center" gap={6} px={8} h={28} w={o.w || 'fill'} grow={o.grow} rounded={4} bg={v(o.off ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(o.focus ? 'fokus/ring' : o.err ? 'status/fehler' : 'linie/stark')} strokeWidth={o.focus || o.err ? 2 : 1}>{o.icon && I(o.icon, 14)}{T(text, { grow: 1, one: true, c: o.off ? 'text/deaktiviert' : 'text/primaer', f: o.mono ? M : F, align: o.right ? 'right' : undefined })}{o.select && I('ph:caret-down', 12)}</Frame>;
const Toggle = (on, label) => <Frame flex="row" items="center" gap={8}><Frame w={36} h={20} rounded={10} bg={v(on ? 'akzent/grund' : 'linie/standard')} flex="row" items="center" px={2} justify={on ? 'end' : 'start'}><Frame w={16} h={16} rounded={8} bg={v('flaeche/blatt')} /></Frame>{label && T(label)}</Frame>;
const Check = (on, label) => <Frame flex="row" items="center" gap={8}><Frame w={18} h={18} rounded={4} bg={v(on ? 'akzent/grund' : 'flaeche/blatt')} stroke={on ? undefined : v('linie/stark')} strokeWidth={1} flex="row" items="center" justify="center">{on && I('ph:check-bold', 12, 'auf')}</Frame>{label && T(label)}</Frame>;
const Seg = (opts, active) => <Frame flex="row" items="center" gap={2} p={2} h={32} rounded={6} bg={v('flaeche/deck')}>{opts.map((o) => { const [l, i] = Array.isArray(o) ? o : [o]; return <Frame flex="row" items="center" gap={6} px={10} h="fill" rounded={4} bg={l === active ? v('flaeche/blatt') : undefined} stroke={l === active ? v('linie/standard') : undefined} strokeWidth={1}>{i && I(i, 14, l === active ? 'a' : 's')}{T(l, { w: l === active ? 600 : 400, c: l === active ? 'text/primaer' : 'text/sekundaer' })}</Frame>; })}</Frame>;
// Inspector
const Row = ({ label, children, changed }) => <Frame flex="row" items="center" gap={8} pl={10} pr={4} minH={32} w="fill">{changed && <Frame position="absolute" x={0} y={6} w={3} h={20} rounded={2} bg={v('akzent/grund')} />}{T(label, { width: 116, w: 600, c: 'text/sekundaer' })}<Frame flex="row" items="center" gap={6} grow={1}>{children}</Frame></Frame>;
const Section = ({ title, children, action, open = true }) => <Frame flex="col" gap={4} w="fill"><Frame flex="row" items="center" gap={6} h={28} w="fill">{I(open ? 'ph:caret-down' : 'ph:caret-right', 12)}{T(title, { w: 700, grow: 1 })}{action}</Frame>{open && children}</Frame>;
const PanelHead = ({ title, sub, actions, icon, b }) => <Frame flex="row" items="center" gap={8} px={12} h={40} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{icon && I(icon, 16, b || 's')}{T(title, { w: 700 })}{sub && T(sub, { s: 12, c: 'text/sekundaer' })}<Frame grow={1} />{actions}</Frame>;
const Search = (ph, w) => <Frame flex="row" items="center" gap={8} px={10} h={30} w={w || 'fill'} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:magnifying-glass', 14)}{T(ph, { c: 'text/sekundaer', grow: 1 })}</Frame>;
const Thumb = (b, icon, size = 28) => <Frame w={size} h={size} rounded={6} bg={v('bereich/' + b + '-flaeche')} flex="row" items="center" justify="center">{I(icon || BER[b][0], Math.round(size * 0.58), b)}</Frame>;
const ListRow = ({ title, sub, right, b = 'pokemon', icon, sel }) => <Frame flex="row" items="center" gap={10} px={10} h={44} w="fill" rounded={6} bg={sel ? v('auswahl/flaeche') : undefined} strokeWeights={sel ? { top: 0, right: 0, bottom: 0, left: 3 } : undefined} stroke={sel ? v('auswahl/kante') : undefined}>{Thumb(b, icon)}<Frame flex="col" grow={1}>{T(title, { w: 600 })}{sub && T(sub, { f: M, s: 11, c: 'text/sekundaer' })}</Frame>{right && T(right, { s: 12, c: 'text/sekundaer' })}</Frame>;
// Fensterrahmen
const Titlebar = ({ project = 'Sinnoh-Remix', game = 'Platin, DE', unsaved = 3 } = {}) => (
  <Frame name="Titelleiste" flex="row" items="center" gap={16} pl={14} pr={10} h={40} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" gap={8} items="center">{['#FF5F57', '#FEBC2E', '#28C840'].map((c) => <Ellipse w={12} h={12} fill={c} />)}</Frame>
    <Frame flex="row" items="center" gap={10} grow={1}>{T(project, { w: 700 })}<Frame flex="row" items="center" gap={4} px={8} h={22} rounded={11} bg={v('bereich/technik-flaeche')}>{I('ph:game-controller', 12, 'technik')}{T(game, { s: 12, w: 600, c: 'bereich/technik' })}</Frame>{unsaved ? <Frame flex="row" items="center" gap={6}><Ellipse w={8} h={8} fill={v('akzent/grund')} />{T(unsaved + ' Änderungen nicht gespeichert', { s: 12, c: 'text/sekundaer' })}</Frame> : <Frame flex="row" items="center" gap={6}>{I('ph:check', 12, 'ok')}{T('Alles gespeichert', { s: 12, c: 'text/sekundaer' })}</Frame>}</Frame>
    <Frame flex="row" items="center" gap={8} px={10} h={28} w={380} rounded={6} bg={v('flaeche/taste')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:magnifying-glass', 14)}{T('Suchen oder Befehl eingeben', { c: 'text/sekundaer', grow: 1 })}{Kbd('⌘K')}</Frame>
    <Frame flex="row" items="center" justify="end" gap={2} grow={1}>{['ph:floppy-disk', 'ph:hammer', 'ph:upload-simple', 'ph:moon', 'ph:question'].map((i) => IconBtn(i))}</Frame>
  </Frame>
);
const ATL = [['start', 'Start', 'ph:house-simple'], ['welt'], ['geschichten'], ['pokemon'], ['gegenstaende'], ['texte'], ['technik']];
const Atlas = (active) => (
  <Frame name="Atlas-Leiste" flex="col" items="center" gap={4} pt={8} pb={8} w={88} h="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {ATL.map(([k, n, i]) => { const on = k === active; const name = n || BER[k][1]; const icon = i || BER[k][0]; const bg = k === 'start' ? 'auswahl/flaeche' : 'bereich/' + k + '-flaeche'; const dot = k === 'start' ? 'akzent/grund' : 'bereich/' + k + '-punkt'; return <Frame flex="col" items="center" justify="center" gap={4} w={80} h={58} rounded={8} bg={on ? v(bg) : undefined}>{on && <Frame position="absolute" x={0} y={15} w={3} h={28} rounded={2} bg={v(dot)} />}{I(icon, 22, on ? (k === 'start' ? 'a' : k) : 's')}{T(name, { s: 12, w: on ? 700 : 400, c: on ? 'text/primaer' : 'text/sekundaer' })}</Frame>; })}
    <Frame grow={1} />
    {[['ph:lifebuoy', 'Hilfe'], ['ph:gear-six', 'Einstellungen']].map(([i, n]) => <Frame flex="col" items="center" gap={2} w={80} h={52} justify="center">{I(i, 20)}{T(n, { s: 12, c: 'text/sekundaer' })}</Frame>)}
  </Frame>
);
// Scharnier: mode 'Bearbeiten' | 'Assistent' | 'Spielen'
const Scharnier = ({ mode = 'Bearbeiten', crumbs = [], depth = 'Bauen', hints = 2, undo = 14, title, b = 'geschichten', icon, steps = [], step = 0, playText } = {}) => {
  const L = '#E7EFF7', LL = '#9CA5BD';
  if (mode === 'Spielen') return (
    <Frame name="Scharnier" flex="row" items="center" gap={16} pl={12} pr={10} h={44} w="fill" bg={v('live/grund')} strokeWeights={{ top: 2, right: 0, bottom: 0, left: 0 }} stroke="#DE8C08">
      <Frame flex="row" items="center" gap={8} grow={1}><Frame w={10} h={10} rounded={5} bg="#182131" />{T('Spielt in melonDS', { w: 700, c: 'live/auf' })}{T(playText || 'ab Zweiblattdorf, Kachel 8, 5', { c: 'live/auf' })}</Frame>
      <Frame flex="row" items="center" gap={8}><Frame flex="row" items="center" gap={6} px={12} h={30} rounded={6} bg="#FFD66B">{I('ph:arrows-clockwise', 14, 'live')}{T('Neu laden', { w: 600, c: 'live/auf' })}</Frame><Frame flex="row" items="center" gap={6} px={12} h={30} rounded={6} bg="#182131"><Icon name="ph:stop-fill" size={14} color="#F7A510" />{T('Stoppen', { w: 700, hex: '#F7A510' })}</Frame></Frame>
    </Frame>
  );
  if (mode === 'Assistent') return (
    <Frame name="Scharnier" flex="row" items="center" gap={16} pl={12} pr={10} h={44} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 0, left: 0 }} stroke={v('bereich/' + b + '-punkt')}>
      <Frame flex="row" items="center" gap={8}><Frame w={26} h={26} rounded={6} bg={v('bereich/' + b + '-flaeche')} flex="row" items="center" justify="center">{I(icon || BER[b][0], 16, b)}</Frame>{T(title, { f: Z, w: 700, s: 16, c: 'scharnier/text' })}</Frame>
      <Frame flex="row" items="center" gap={4} grow={1} justify="center">{steps.map((n, i) => { const s = i < step ? 'fertig' : i === step ? 'aktiv' : 'offen'; return <Frame flex="row" items="center" gap={4}>{i > 0 && <Frame w={16} h={2} rounded={1} bg={v(s === 'offen' ? 'scharnier/kante' : 'scharnier/leise')} />}<Frame flex="row" items="center" gap={6} pl={4} pr={10} h={28} rounded={14} bg={s === 'aktiv' ? v('scharnier/text') : undefined}><Frame w={20} h={20} rounded={10} flex="row" items="center" justify="center" bg={s === 'fertig' ? v('scharnier/taste') : s === 'aktiv' ? v('scharnier/grund') : undefined} stroke={s === 'offen' ? v('scharnier/leise') : undefined} strokeWidth={1}>{s === 'fertig' ? I('ph:check-bold', 11, L) : T(String(i + 1), { s: 12, w: 700, c: s === 'aktiv' ? 'scharnier/text' : 'scharnier/leise' })}</Frame>{T(n, { w: s === 'aktiv' ? 700 : 400, c: s === 'aktiv' ? 'scharnier/grund' : s === 'fertig' ? 'scharnier/text' : 'scharnier/leise' })}</Frame></Frame>; })}</Frame>
      <Frame flex="row" items="center" gap={8}><Frame flex="row" items="center" px={10} h={30}>{T('Abbrechen', { w: 600, c: 'scharnier/leise' })}</Frame>{step > 0 && <Frame flex="row" items="center" px={12} h={30} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWeights={{ top: 1, right: 1, bottom: 2, left: 1 }}>{T('Zurück', { w: 600, c: 'scharnier/text' })}</Frame>}<Frame flex="row" items="center" gap={6} px={14} h={30} rounded={6} bg={v('akzent/grund')}>{T(step === steps.length - 1 ? 'Anlegen' : 'Weiter', { w: 700, c: 'akzent/auf' })}{Kbd('↵', true)}</Frame></Frame>
    </Frame>
  );
  return (
    <Frame name="Scharnier" flex="row" items="center" gap={16} pl={8} pr={10} h={44} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 0, left: 0 }} stroke={v('scharnier/kante')}>
      <Icon name="ph:dots-six-vertical" size={16} color={LL} />
      <Frame flex="row" items="center" gap={6} grow={1}>{crumbs.map(([i, n], idx) => { const last = idx === crumbs.length - 1; return <Frame flex="row" items="center" gap={6}>{idx > 0 && <Icon name="ph:caret-right" size={12} color={LL} />}<Icon name={i} size={14} color={last ? L : LL} />{T(n, { w: last ? 600 : 400, c: last ? 'scharnier/text' : 'scharnier/leise' })}</Frame>; })}</Frame>
      <Frame flex="row" items="center" gap={2} p={2} h={30} rounded={6} bg={v('scharnier/taste')}>{[['Erzählen', 'ph:book-open-text'], ['Bauen', 'ph:hammer'], ['Tüfteln', 'ph:wrench']].map(([o, i]) => <Frame flex="row" items="center" gap={6} px={10} h="fill" rounded={4} bg={o === depth ? v('scharnier/text') : undefined}><Icon name={i} size={14} color={o === depth ? '#293142' : LL} />{T(o, { w: o === depth ? 600 : 400, c: o === depth ? 'scharnier/grund' : 'scharnier/leise' })}</Frame>)}</Frame>
      <Frame flex="row" items="center" justify="end" gap={12} grow={1}>
        <Frame flex="row" items="center" gap={6}><Ellipse w={8} h={8} fill={hints ? '#F7BD31' : '#73D69C'} />{T(hints ? hints + ' Hinweise' : 'Alles stimmig', { c: 'scharnier/text' })}</Frame>
        <Frame w={1} h={20} bg={v('scharnier/kante')} />
        <Frame flex="row" items="center" gap={4}><Icon name="ph:arrow-counter-clockwise" size={16} color={L} />{T(String(undo), { c: 'scharnier/text' })}</Frame>
        <Icon name="ph:arrow-clockwise" size={16} color="#5A6B94" />
        <Frame w={1} h={20} bg={v('scharnier/kante')} />
        <Frame flex="row" items="center" gap={6} pl={12} pr={6} h={30} rounded={6} bg={v('live/grund')}>{I('ph:play-fill', 14, 'live')}{T('Hier spielen', { w: 700, c: 'live/auf' })}<Frame flex="row" items="center" px={5} h={18} rounded={4} bg="#FFD66B">{T('⌘R', { s: 12, w: 600, c: 'live/auf' })}</Frame></Frame>
      </Frame>
    </Frame>
  );
};
const Statusbar = ({ tool, toolIcon = 'ph:cursor', keys = [], info, build = 'Gebaut vor 4 Minuten' } = {}) => (
  <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" items="center" gap={8} grow={1}>{tool && I(toolIcon, 14, 'a')}{tool && T(tool, { s: 12, w: 600 })}{keys.map(([k, a]) => <Frame flex="row" items="center" gap={4}>{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    {info && T(info, { s: 12, c: 'text/sekundaer' })}
    <Frame w={1} h={14} bg={v('linie/standard')} />
    <Frame flex="row" items="center" gap={6}>{I('ph:check-circle', 14, 'ok')}{T(build, { s: 12, c: 'text/sekundaer' })}</Frame>
    <Frame w={1} h={14} bg={v('linie/standard')} />
    {T('Komfort', { s: 12, c: 'text/sekundaer' })}
  </Frame>
);
// Fenster 1440 × 900: Titelleiste, dann Zeile [Atlas | Inhalt], dann Statusleiste
const Window = ({ name, active, children, status, title }) => (
  <Frame name={name} w={1440} h={900} flex="col" bg={v('flaeche/gehaeuse')} overflow="hidden" rounded={10} stroke={v('linie/standard')} strokeWidth={1} shadow="0 24 64 #18213133">
    {Titlebar(title)}
    <Frame flex="row" w="fill" grow={1}>{Atlas(active)}<Frame flex="row" grow={1} h="fill">{children}</Frame></Frame>
    {status || Statusbar()}
  </Frame>
);
// Doppeldeck: Schau oben, Scharnier, Hand unten (Hand-Höhe in px)
const Deck = ({ schau, scharnier, hand, handH = 300 }) => (
  <Frame flex="col" grow={1} h="fill">
    <Frame name="Schau" flex="col" w="fill" grow={1} bg={v('flaeche/schau')} overflow="hidden">{schau}</Frame>
    {scharnier}
    <Frame name="Hand" flex="row" w="fill" h={handH} bg={v('flaeche/deck')}>{hand}</Frame>
  </Frame>
);
// DS-Bildschirm-Vorschau (scale 1 = 256×192)
const DSScreen = ({ scale = 1.5, lines = [], sign, children }) => { const W = 256 * scale, H = 192 * scale; return (
  <Frame w={W + 20} h={H + 20} p={10} rounded={8} bg={v('flaeche/bildschirm-rahmen')}>
    <Frame w={W} h={H} rounded={2} overflow="hidden" bg="#7BA563">
      {children}
      {lines.length > 0 && <Frame position="absolute" x={4 * scale} y={H - 58 * scale} w={W - 8 * scale} h={54 * scale} rounded={4 * scale} bg={sign ? '#E7D6B5' : '#F7F7FF'} stroke={sign ? '#6B5229' : '#4A5263'} strokeWidth={2 * scale} px={9 * scale} py={6 * scale} flex="col" gap={2 * scale}>{lines.map((l) => <Text font={PX} size={12.5 * scale} lineHeight={20 * scale} color={sign ? '#423921' : '#394252'}>{l}</Text>)}</Frame>}
    </Frame>
  </Frame>
); };
