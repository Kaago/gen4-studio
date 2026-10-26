{(() => {
const F = 'Atkinson Hyperlegible Next', M = 'Atkinson Hyperlegible Mono', Z = 'Zen Maru Gothic', PX = 'Pixelify Sans';
const v = (t) => (typeof DARKMAP !== 'undefined' && DARKMAP[t]) || designVar(t);
const HX = { p: '#182131', s: '#4A5263', d: '#848CA5', a: '#3152DE', at: '#2942B5', auf: '#FFFFFF', f: '#AD2131', w: '#8C5200', ok: '#186B42', info: '#2942B5', live: '#182131', sl: '#C6CEDE', st: '#E7EFF7', welt: '#186B42', geschichten: '#633994', pokemon: '#943121', gegenstaende: '#106363', texte: '#6B5229', technik: '#394252' };
const BER = { start: ['ph:house', 'Start'], welt: ['ph:map-trifold', 'Welt'], geschichten: ['ph:chats-circle', 'Geschichten'], pokemon: ['ph:paw-print', 'Pokémon'], gegenstaende: ['ph:backpack', 'Gegenstände'], texte: ['ph:text-aa', 'Texte'], technik: ['ph:cpu', 'Technik'] };
const T = (s, o = {}) => <Text font={o.f || F} size={o.s || 13} weight={o.w || 400} lineHeight={o.lh || ({ 10: 12, 12: 16, 13: 18, 15: 20, 18: 24, 24: 30 }[o.s || 13] || Math.round((o.s || 13) * 1.38))} color={o.hex || v(o.c || 'text/primaer')} w={o.width} grow={o.grow} textAlign={o.align} maxLines={o.one ? 1 : undefined} truncate={o.one ? true : undefined}>{s}</Text>;
const Mono = (s, o = {}) => T(s, { f: M, s: 12, c: 'text/sekundaer', ...o });
const I = (name, size = 18, c = 's') => <Icon name={name} size={size} color={HX[c] || c} />;
const Kbd = (k, dark) => <Frame flex="row" items="center" justify="center" px={4} minW={20} h={18} rounded={4} bg={v(dark ? 'scharnier/taste' : 'flaeche/deck')} stroke={v(dark ? 'scharnier/kante' : 'linie/standard')} strokeWidth={1}>{T(k, { f: M, s: 12, c: dark ? 'scharnier/text' : 'text/sekundaer' })}</Frame>;
const Btn = ({ label, art = 'Sekundär', icon, kbd, small, off, grow }) => {
  const S = { 'Primär': ['akzent/grund', 'akzent/auf', 'auf'], 'Sekundär': ['flaeche/taste', 'text/primaer', 'p'], 'Leise': [null, 'text/primaer', 'p'], 'Spielen': ['live/grund', 'live/auf', 'live'], 'Gefahr': ['status/fehler', 'akzent/auf', 'auf'] }[art];
  if (off) return <Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 24 : 32} rounded={6} grow={grow} bg={art === 'Leise' ? undefined : v('flaeche/deck')} stroke={art === 'Leise' ? undefined : v('linie/leise')} strokeWidth={1}>{icon && I(icon, small ? 16 : 18, 'd')}{T(label, { w: 600, c: 'text/deaktiviert' })}</Frame>;
  // Sekundär: Kante (unten 2 px) als äußere Fläche, weil OpenPencil bei ungleichen Kanten die Ecken nicht rundet
  if (art === 'Sekundär') return <Frame flex="col" rounded={6} grow={grow} bg={v('linie/stark')} pt={1} pl={1} pr={1} pb={2}><Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 21 : 29} rounded={5} bg={v('flaeche/taste')}>{icon && I(icon, small ? 16 : 18, 'p')}{T(label, { w: 600 })}{kbd && Kbd(kbd)}</Frame></Frame>;
  return <Frame flex="row" items="center" justify="center" gap={6} px={small ? 8 : 12} h={small ? 24 : 32} rounded={6} grow={grow} bg={S[0] ? v(S[0]) : undefined}>{icon && I(icon, small ? 16 : 18, S[2])}{T(label, { w: art === 'Spielen' ? 700 : 600, c: S[1] })}{kbd && Kbd(kbd)}</Frame>;
};
const IconBtn = (icon, o = {}) => <Frame w={32} h={32} rounded={6} bg={o.on ? v('auswahl/flaeche') : undefined} flex="row" items="center" justify="center">{I(o.on ? icon + '-fill' : icon, 18, o.on ? 'at' : o.off ? 'd' : (o.c || 'p'))}</Frame>;
const Ref = (b, label, tech, icon) => <Frame flex="row" items="center" gap={6} px={8} h={24} rounded={12} bg={v('bereich/' + b + '-flaeche')}>{I(icon || BER[b][0], 18, b)}{T(label, { w: 600, c: 'bereich/' + b })}{tech && T(tech, { f: M, s: 12, c: 'bereich/' + b })}</Frame>;
const Badge = (label, art = 'neutral', icon) => { const m = { neutral: ['flaeche/deck', 'text/sekundaer', 's'], ungespeichert: ['auswahl/flaeche', 'akzent/text', 'at'], warn: ['status/warnung-flaeche', 'status/warnung', 'w'], fehler: ['status/fehler-flaeche', 'status/fehler', 'f'], ok: ['status/erfolg-flaeche', 'status/erfolg', 'ok'], info: ['status/info-flaeche', 'status/info', 'info'], live: ['live/flaeche', 'live/text', 'live'], code: ['flaeche/deck', 'text/sekundaer', 's'], gesperrt: ['flaeche/deck', 'text/sekundaer', 's'], unumkehrbar: ['status/warnung-flaeche', 'status/warnung', 'w'], aufplatte: ['status/warnung-flaeche', 'status/warnung', 'w'], nurpthgss: ['status/info-flaeche', 'status/info', 'info'] }[art]; return <Frame flex="row" items="center" gap={4} px={8} h={16} rounded={art === 'code' ? 4 : 8} bg={v(m[0])}>{icon && I(icon, 12, m[2])}{T(label, { s: 12, w: 600, c: m[1], f: art === 'code' ? M : F })}</Frame>; };
const TY = { normal: ['Normal', '#A5A57B', '#182131'], kampf: ['Kampf', '#BD3129', '#FFFFFF'], flug: ['Flug', '#A58CEF', '#182131'], gift: ['Gift', '#9C429C', '#FFFFFF'], boden: ['Boden', '#DEBD6B', '#182131'], gestein: ['Gestein', '#B59C39', '#182131'], kaefer: ['Käfer', '#A5B521', '#182131'], geist: ['Geist', '#735A94', '#FFFFFF'], stahl: ['Stahl', '#B5B5CE', '#182131'], fragezeichen: ['???', '#6B9C8C', '#182131'], feuer: ['Feuer', '#EF8431', '#182131'], wasser: ['Wasser', '#6B8CEF', '#182131'], pflanze: ['Pflanze', '#7BC652', '#182131'], elektro: ['Elektro', '#F7CE31', '#182131'], psycho: ['Psycho', '#F75A84', '#182131'], eis: ['Eis', '#94D6D6', '#182131'], drache: ['Drache', '#7339F7', '#FFFFFF'], unlicht: ['Unlicht', '#735A4A', '#FFFFFF'] };
const Typ = (k, m) => <Frame flex="row" items="center" justify="center" px={m ? 8 : 6} minW={m ? 64 : 52} h={m ? 22 : 16} rounded={11} bg={v('typ/' + k + '/grund')}>{T(TY[k][0], { s: m ? 13 : 12, w: 600, c: 'typ/' + k + '/schrift' })}</Frame>;
const Field = (text, o = {}) => <Frame flex="row" items="center" gap={8} px={8} h={32} w={o.w || 'fill'} grow={o.grow} rounded={4} bg={v(o.off ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(o.err ? 'status/fehler' : o.open ? 'akzent/grund' : o.off ? 'linie/leise' : 'linie/standard')} strokeWidth={1}>{o.icon && I(o.icon, 18)}{T(text, { grow: 1, one: true, c: o.ph ? 'text/sekundaer' : o.off ? 'text/deaktiviert' : 'text/primaer', f: o.mono ? M : F })}{o.select && I('ph:caret-down', 18)}</Frame>;
const NumField = (val, o = {}) => <Frame flex="row" items="center" gap={8} grow={1} w="fill">{I('ph:dots-six-vertical', 18)}{Field(val, { mono: o.hexMode, grow: 1 })}{o.base && SegS(['Dez', 'Hex'], o.hexMode ? 'Hex' : 'Dez')}</Frame>;
const Toggle = (on, label) => <Frame flex="row" items="center" gap={8} minH={32}><Frame w={36} h={20} rounded={10} p={2} bg={v(on ? 'akzent/grund' : 'flaeche/deck')} stroke={on ? undefined : v('linie/stark')} strokeWidth={1} flex="row" items="center" justify={on ? 'end' : 'start'}><Frame w={14} h={14} rounded={7} bg={v(on ? 'akzent/auf' : 'text/sekundaer')} /></Frame>{label && T(label)}</Frame>;
const Check = (on, label, off) => <Frame flex="row" items="center" gap={8}><Frame w={18} h={18} rounded={4} bg={v(off ? 'flaeche/deck' : on ? 'akzent/grund' : 'flaeche/blatt')} stroke={on ? undefined : v(off ? 'linie/leise' : 'linie/stark')} strokeWidth={1} flex="row" items="center" justify="center">{on && I('ph:check-bold', 14, 'auf')}</Frame>{label && T(label, { c: off ? 'text/deaktiviert' : 'text/primaer' })}</Frame>;
const Seg = (opts, active, small) => <Frame flex="row" items="center" gap={2} p={2} h={small ? 28 : 32} rounded={6} bg={v('flaeche/deck')} stroke={v('linie/standard')} strokeWidth={1}>{opts.map((o) => { const [l, i] = Array.isArray(o) ? o : [o]; const on = (l || i) === active; return <Frame flex="row" items="center" gap={6} px={small ? 8 : 12} h="fill" rounded={4} bg={on ? v('flaeche/blatt') : undefined} stroke={on ? v('auswahl/kante') : undefined} strokeWidth={1}>{i && I(on ? i + '-fill' : i, 18, on ? 'at' : 's')}{l && T(l, { w: 600, c: on ? 'akzent/text' : 'text/primaer' })}</Frame>; })}</Frame>;
const SegS = (opts, active) => Seg(opts, active, true);
const Tabs = (items, active) => <Frame flex="row" items="end" gap={2} px={4} h={36} w="fill" bg={v('flaeche/deck')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>{items.map((it) => { const [l, n] = Array.isArray(it) ? it : [it]; const on = l === active; return <Frame flex="row" items="center" gap={6} px={12} h={32} bg={on ? v('flaeche/blatt') : undefined} strokeWeights={on ? { top: 0, right: 0, bottom: 2, left: 0 } : undefined} stroke={on ? v('akzent/grund') : undefined}>{T(l, { w: 600, c: on ? 'text/primaer' : 'text/sekundaer' })}{n !== undefined && <Frame px={6} h={16} rounded={8} bg={v(on ? 'auswahl/flaeche' : 'flaeche/deck')} flex="row" items="center">{T(String(n), { s: 12, c: on ? 'akzent/text' : 'text/sekundaer' })}</Frame>}</Frame>; })}</Frame>;
const DockTabs = (items, active, bg = 'flaeche/gehaeuse') => <Frame flex="row" items="end" h={36} w="fill" bg={v(bg)}>{items.map((l) => { const on = l === active; return <Frame flex="row" items="center" px={8} h={36} bg={on ? v('flaeche/schau') : undefined} strokeWeights={on ? { top: 2, right: 0, bottom: 0, left: 0 } : undefined} stroke={on ? v('akzent/grund') : undefined}>{T(l, { c: on ? 'text/primaer' : 'text/sekundaer' })}</Frame>; })}</Frame>;
const Row = ({ label, children, changed, hint }) => <Frame flex="row" items={hint ? 'start' : 'center'} gap={8} pl={8} pr={8} py={2} minH={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 0, left: 2 }} stroke={changed ? v('akzent/grund') : v('flaeche/blatt')}>{T(label, { c: 'text/sekundaer', width: 116 })}{hint ? <Frame flex="col" gap={2} grow={1}><Frame flex="row" items="center" gap={8} w="fill">{children}</Frame>{T(hint, { s: 12, c: 'text/sekundaer' })}</Frame> : <Frame flex="row" items="center" gap={8} grow={1}>{children}</Frame>}</Frame>;
const Section = ({ title, count, children, open = true }) => <Frame flex="col" w="fill"><Frame flex="row" items="center" gap={6} px={8} h={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{I(open ? 'ph:caret-down' : 'ph:caret-right', 18)}{T(title, { w: 600, grow: 1 })}{count !== undefined && <Frame px={6} h={16} rounded={8} bg={v('flaeche/deck')} flex="row" items="center">{T(String(count), { s: 12, c: 'text/sekundaer' })}</Frame>}</Frame>{open && <Frame flex="col" w="fill" pt={4} pb={8}>{children}</Frame>}</Frame>;
const PanelHead = ({ title, sub, actions, switcher }) => <Frame flex="row" items="center" gap={8} pl={12} pr={8} h={36} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}><Frame flex="row" items="center" gap={6}>{T(title, { w: 600 })}{switcher && I('ph:caret-down', 18)}</Frame>{sub && Mono(sub, { one: true, grow: 1 })}{!sub && <Frame grow={1} />}{actions}</Frame>;
const Search = (ph, w) => <Frame flex="row" items="center" gap={8} px={8} h={32} w={w || 'fill'} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{T(ph, { c: 'text/sekundaer', grow: 1, one: true })}</Frame>;
const Sprite = (letter, b = 'pokemon', size = 24) => <Frame w={size} h={size} rounded={4} bg={v('bereich/' + b + '-flaeche')} flex="row" items="center" justify="center">{T(letter, { s: size >= 48 ? 18 : 13, w: 700, c: 'bereich/' + b })}</Frame>;
const ListRow = ({ title, tech, sub, right, icon, lead, sel, depth = 0, caret, mark }) => <Frame flex="row" items="center" gap={8} pl={12 + depth * 16} pr={12} minH={sub ? 40 : 32} w="fill" rounded={4} bg={sel ? v('auswahl/flaeche') : undefined} strokeWeights={sel ? { top: 0, right: 0, bottom: 0, left: 2 } : undefined} stroke={sel ? v('auswahl/kante') : undefined}>{caret && I(caret === 'open' ? 'ph:caret-down' : 'ph:caret-right', 18)}{lead}{icon && I(icon, 18, sel ? 'at' : 's')}<Frame flex="col" grow={1}><Frame flex="row" items="center" gap={6}>{T(title, { c: sel ? 'akzent/text' : 'text/primaer', one: true })}{tech && Mono(tech)}</Frame>{sub && T(sub, { s: 12, c: 'text/sekundaer' })}</Frame>{mark}{right && T(right, { s: 12, c: 'text/sekundaer', f: M })}</Frame>;
const StatBar = ({ label, val, mean, verdict, changed, max = 255, track = 380 }) => <Frame flex="row" items="center" gap={8} pl={8} pr={8} minH={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 0, left: 2 }} stroke={changed ? v('akzent/grund') : v('flaeche/schau')}>{T(label, { c: 'text/sekundaer', width: 116 })}{T(String(val), { w: 600, align: 'right', width: 28 })}<Frame w={track} h={12}><Frame position="absolute" x={0} y={2} w={track} h={8} rounded={4} bg={v('flaeche/deck')} /><Frame position="absolute" x={0} y={2} w={Math.round(val / max * track)} h={8} rounded={4} bg={v('akzent/grund')} /><Frame position="absolute" x={Math.round(mean / max * track)} y={0} w={2} h={12} bg={v('linie/stark')} /></Frame>{T(verdict, { s: 12, c: 'text/sekundaer', width: 84 })}</Frame>;
const MonCard = ({ name, lv, item, sel, empty, w = 200 }) => empty ? <Frame flex="row" items="center" justify="center" w={w} h={56} rounded={12} stroke={v('linie/standard')} strokeWidth={1} strokeDash={[4, 3]} dashPattern={[4, 3]}>{T(empty, { c: 'text/sekundaer' })}</Frame> : <Frame flex="row" items="center" gap={8} px={12} py={8} w={w} minH={56} rounded={12} bg={v('flaeche/blatt')} stroke={v(sel ? 'auswahl/kante' : 'linie/leise')} strokeWidth={sel ? 2 : 1}>{Sprite(name[0], 'pokemon', 32)}<Frame flex="col">{T(name, { w: 600 })}{T('Lv. ' + lv, { s: 12, c: 'text/sekundaer' })}{item && T(item, { s: 12, c: 'text/sekundaer' })}</Frame></Frame>;
const Notice = (text, art = 'info', title) => { const m = { info: ['status/info-flaeche', 'status/info', 'ph:info-fill', 'info'], warn: ['status/warnung-flaeche', 'status/warnung', 'ph:warning-fill', 'w'], fehler: ['status/fehler-flaeche', 'status/fehler', 'ph:x-circle-fill', 'f'], ok: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok'] }[art]; return <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[2], 18, m[3])}<Frame flex="col" grow={1}>{title && T(title, { w: 600, c: m[1] })}{T(text)}</Frame></Frame>; };
const Empty = (what, why, action) => <Frame flex="col" items="center" justify="center" gap={8} px={16} py={32} w="fill" grow={1}>{T(what, { s: 15, w: 600 })}{T(why, { c: 'text/sekundaer', align: 'center', width: 320 })}{action && Btn({ label: action })}</Frame>;
const Titlebar = ({ project = 'Sinnoh-Remix', game = 'Platin, DE', unsaved = 0, dark } = {}) => (
  <Frame name="Titelleiste" flex="row" items="center" gap={16} pl={12} pr={12} h={40} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" gap={8} items="center" w={54}>{['#FF5F57', '#FEBC2E', '#28C840'].map((c) => <Ellipse w={12} h={12} fill={c} />)}</Frame>
    <Frame flex="row" items="center" gap={12} w={560}>{IconBtn('ph:folder-open', { c: dark ? '#E7EFF7' : 'p' })}{project ? T(project, { s: 15, w: 600 }) : T('Kein Projekt geöffnet', { s: 15, c: 'text/sekundaer' })}{project && <Frame flex="row" items="center" px={8} h={18} rounded={9} bg={v('flaeche/deck')}>{T(game, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>}{project && (unsaved ? <Frame flex="row" items="center" gap={6}><Ellipse w={8} h={8} fill={v('akzent/grund')} />{T(unsaved + (unsaved === 1 ? ' Änderung' : ' Änderungen') + ' nicht gespeichert', { s: 12, w: 600, c: 'akzent/text' })}</Frame> : <Frame flex="row" items="center" gap={4}>{I('ph:check', 12, 'ok')}{T('Alles gespeichert', { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    <Frame flex="row" items="center" gap={8} px={8} h={28} w={360} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:magnifying-glass', 18)}{T('Suchen oder Befehl eingeben', { c: 'text/sekundaer', grow: 1 })}{Kbd('⌘K')}</Frame>
    <Frame flex="row" items="center" justify="end" gap={8} grow={1}>{['ph:floppy-disk', 'ph:hammer', dark ? 'ph:sun' : 'ph:moon', 'ph:question'].map((i) => IconBtn(i, { c: dark ? '#E7EFF7' : 'p' }))}</Frame>
  </Frame>
);
const ATL = ['start', 'welt', 'geschichten', 'pokemon', 'gegenstaende', 'texte', 'technik'];
const Atlas = (active, dark) => {
  const entry = (k, name, icon, on, off) => { const bg = k === 'start' || k === 'einstellungen' ? 'auswahl/flaeche' : 'bereich/' + k + '-flaeche'; const fg = k === 'start' || k === 'einstellungen' ? 'akzent/text' : 'bereich/' + k; return <Frame flex="col" items="center" justify="center" gap={4} w={88} h={64} bg={on ? v(bg) : undefined}>{on && <Frame position="absolute" x={0} y={8} w={3} h={48} rounded={2} bg={v(fg)} />}<Icon name={on ? icon + '-fill' : icon} size={24} color={on ? (HX[k] || HX.at) : off ? (dark ? '#5A637B' : HX.d) : (dark ? '#ADB5C6' : HX.s)} />{T(name, { s: 12, w: on ? 600 : 400, c: on ? fg : off ? 'text/deaktiviert' : 'text/sekundaer' })}</Frame>; };
  return <Frame name="Atlas" flex="col" pt={8} pb={8} w={88} h="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {ATL.map((k) => entry(k, BER[k][1], BER[k][0], k === active, k === 'geschichten'))}
    <Frame grow={1} />
    {entry('hilfe', 'Hilfe', 'ph:question', false)}{entry('einstellungen', 'Einstellungen', 'ph:gear-six', active === 'einstellungen')}
  </Frame>;
};
const ListPane = (children, w = 272) => <Frame name="Liste" flex="col" w={w} h="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')} overflow="hidden">{children}</Frame>;
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
const Statusbar = ({ tool, keys = [], info, problems = '101 Warnungen, 10 Hinweise', probArt = 'warn', build = 'Noch nicht gebaut', built } = {}) => (
  <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="row" items="center" gap={6}>{tool && T(tool, { s: 12, w: 600 })}{keys.map(([k, a], idx) => <Frame flex="row" items="center" gap={4}>{(tool || idx > 0) && T('·', { s: 12, c: 'text/sekundaer' })}{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>
    {info ? T(info, { s: 12, c: 'text/sekundaer', grow: 1, one: true }) : <Frame grow={1} />}
    {problems && T(problems, { s: 12, w: 600, c: probArt === 'fehler' ? 'status/fehler' : probArt === 'ok' ? 'text/sekundaer' : 'status/warnung' })}
    <Frame flex="row" items="center" gap={4}>{built && I('ph:check-circle-fill', 12, 'ok')}{T(build, { s: 12, c: 'text/sekundaer' })}</Frame>
    {T('Komfort', { s: 12, w: 600 })}
  </Frame>
);
const Window = ({ name, active, children, status, title, dark }) => (
  <Frame name={name} w={1440} h={900} flex="col" bg={v('flaeche/gehaeuse')} overflow="hidden" rounded={10} stroke={v('linie/standard')} strokeWidth={1}>
    {Titlebar({ ...(title || {}), dark })}
    <Frame flex="row" w="fill" grow={1}>{Atlas(active, dark)}<Frame flex="row" grow={1} h="fill">{children}</Frame></Frame>
    {status || Statusbar()}
  </Frame>
);
const Deck = ({ schau, scharnier, hand, handH = 286, schauBg = 'flaeche/schau' }) => (
  <Frame flex="col" grow={1} h="fill">
    <Frame name="Schau" flex="col" w="fill" grow={1} bg={v(schauBg)} overflow="hidden">{schau}</Frame>
    {scharnier}
    <Frame name="Hand" flex="col" w="fill" h={handH} bg={v('flaeche/deck')} overflow="hidden">{hand}</Frame>
  </Frame>
);
const HandCol = (title, children, w, sub) => <Frame flex="col" w={w || 'fill'} grow={w ? undefined : 1} h="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')} overflow="hidden">{title && PanelHead({ title, sub })}{children}</Frame>;
const Tool = (name, icon, key, on, off) => <Frame flex="col" items="center" justify="center" gap={2} w={78} h={56} rounded={6} bg={on ? v('auswahl/flaeche') : undefined} stroke={v(on ? 'auswahl/kante' : 'linie/leise')} strokeWidth={1}><Frame position="absolute" x={66} y={3}>{T(key, { f: M, s: 10, c: 'text/sekundaer' })}</Frame><Icon name={on ? icon + '-fill' : icon} size={18} color={on ? HX.at : off ? '#9CA5BD' : HX.p} />{T(name, { s: 12, c: on ? 'akzent/text' : off ? 'text/deaktiviert' : 'text/primaer', align: 'center', width: 70 })}</Frame>;
const DSScreen = ({ scale = 2, lines = [], sign, children, bg }) => { const W = 256 * scale, H = 192 * scale; return (
  <Frame w={W} h={H} rounded={2} overflow="hidden" bg={bg || v('ds/grund')}>
    {children}
    {lines.length > 0 && <Frame position="absolute" x={4 * scale} y={H - 58 * scale} w={248 * scale} h={54 * scale} rounded={4 * scale} bg={v(sign ? 'ds/schild/flaeche' : 'ds/textbox/flaeche')} stroke={v(sign ? 'ds/schild/rahmen' : 'ds/textbox/rahmen')} strokeWidth={2 * scale} px={9 * scale} py={6 * scale} flex="col" gap={0}>{lines.map((l) => <Text font={PX} size={12 * scale} lineHeight={16 * scale} color={v(sign ? 'ds/schild/text' : 'ds/textbox/text')}>{l}</Text>)}<Frame position="absolute" x={234 * scale} y={40 * scale}><Icon name="ph:caret-down-fill" size={10 * scale} color="#4A5263" /></Frame></Frame>}
  </Frame>
); };
const Stand = (art, text) => { const m = { umgesetzt: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok', 'Umgesetzt'], teilweise: ['status/warnung-flaeche', 'status/warnung', 'ph:circle-half-fill', 'w', 'Teilweise umgesetzt'], offen: ['flaeche/deck', 'text/sekundaer', 'ph:circle-dashed', 's', 'Zielbild, noch offen'] }[art]; return <Frame flex="row" items="center" gap={6} px={10} h={28} rounded={14} bg={v(m[0])}>{I(m[2], 16, m[3])}{T(m[4], { w: 600, c: m[1] })}{text && T(text, { c: 'text/sekundaer' })}</Frame>; };
  // App: Technik, Welt leeren (Phase 3g.2): der Assistent „Welt leeren…“ (Technik › Werkzeuge) öffnet sich im Scharnier des Welt-Decks,
  // die Welt bleibt in der Schau (Atlas auf Welt). Schritt 1 „Was bleibt“ mit offener Voraussetzung „Scripts übernehmen“.
  // Quellen: apps/studio/src/project/ClearWorldWizard.tsx und .module.css, technik/TechnikArea.tsx (Werkzeug „Welt leeren“, data-tool="welt-leeren"),
  // workbench/wizards/WizardFrame.tsx. Zahlen der Platin-DE-Testkopie. Abweichung: Die Orte-Liste zeigt wie welt-figur.body.js nur eine Auswahl.
  const place = (n, c, cnt, o = {}) => ListRow({ title: n, caret: o.open ? 'open' : 'closed', icon: o.icon || 'ph:map-trifold', depth: o.d || 0, sel: o.sel, tech: o.tech, right: undefined, mark: cnt !== undefined ? <Frame flex="row" items="center" gap={6}><Frame w={8} h={8} rounded={1} bg={c} />{T(String(cnt), { s: 12, f: M, c: 'text/sekundaer' })}</Frame> : undefined });
  const liste = ListPane([
    PanelHead({ title: 'Orte', sub: 'Platin, DE', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:plus')}</Frame> }),
    <Frame flex="col" gap={8} px={8} pt={8} pb={4} w="fill">{T('125 Orte, 593 Header', { s: 12, c: 'text/sekundaer' })}{Search('Ort, Raum oder Kürzel suchen')}</Frame>,
    <Frame flex="col" px={4} w="fill">
      {place('Route 216', '#E7A5C6', 2)}{place('Route 217', '#E7A5C6', 3)}{place('Route 218', '#E7A5C6', 3)}{place('Route 219', '#E7A5C6', 1)}{place('Route 221', '#E7A5C6', 2)}{place('Route 222', '#E7A5C6', 4)}
      {place('Zweiblattdorf', '#DEE79C', 7, { open: true })}
      {['T01R0101', 'T01R0102', 'T01R0201', 'T01R0202', 'T01R0301', 'T01R0401'].map((r) => ListRow({ title: r, tech: '1 Karte', caret: 'closed', icon: 'ph:house', depth: 1 }))}
      {place('Sandgemme', '#E7A5C6', 9)}{place('Flori', '#A5DEA5', 7)}{place('Blumenladen', '#E7A5C6', 1)}{place('Trostu', '#A5DEC6', 9)}
    </Frame>
  ]);
  const tiles = []; const pal = ['#7B9C31', '#94AD42', '#C68C52', '#6B8C29', '#A5BD5A', '#B57B4A', '#5A7B21', '#8CAD39'];
  for (let r = 0; r < 7; r++) for (let c = 0; c < 9; c++) { if ((r === 0 && c > 6) || (r === 6 && c < 2) || (r === 2 && c === 4) || (r === 3 && c === 4)) continue; tiles.push(<Frame position="absolute" x={250 + c * 58 - r * 14} y={95 + r * 46} w={56} h={44} bg={pal[(r * 3 + c * 5) % 8]} />); }
  const schau = [
    DockTabs(['3D', 'Raster', 'Weltkarte'], '3D'),
    <Frame name="3D-Szene" w="fill" grow={1} bg={v('flaeche/welt')}>
      {tiles}
      <Frame position="absolute" x={612} y={150} w={60} h={40} bg="#E7A57B" /><Frame position="absolute" x={690} y={140} w={46} h={60} bg="#E7A57B" /><Frame position="absolute" x={420} y={240} w={30} h={24} bg="#3173C6" />
    </Frame>
  ];
  // Spalte der Hand: Überschrift 13/600, Sätze Primärtext (parts.column / parts.heading / parts.text)
  const Spalte = (children, o = {}) => <Frame flex="col" gap={8} px={16} py={12} grow={1} h="fill" strokeWeights={o.first ? undefined : { top: 0, right: 0, bottom: 0, left: 1 }} stroke={o.first ? undefined : v('linie/leise')}>{children}</Frame>;
  // Hinweis mit Aktionsknopf (Notice mit actions): Warnung, Satz, Fortschrittsbalken eines laufenden Übernehmens, Knopf rechts
  const Bar = (n, total) => <Frame flex="col" gap={4} w="fill">
    <Frame w="fill" h={8} rounded={4} bg={v('flaeche/deck')}><Frame w={Math.max(4, Math.round(n / total * 360))} h={8} rounded={4} bg={v('akzent/grund')} /></Frame>
    {T(n + ' / ' + total + ' Scriptdateien', { s: 12, c: 'text/sekundaer' })}
  </Frame>;
  const Hinweis = (text, action, extra) => <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v('status/warnung-flaeche')}>
    {I('ph:warning-fill', 18, 'w')}
    <Frame flex="col" gap={8} grow={1}>{T(text, { width: 270 })}{extra}{action && Btn({ label: action, art: 'Sekundär', small: true })}</Frame>
  </Frame>;
  const links = Spalte([
    T('Bleibt', { w: 600 }),
    T('Zweiblattdorf bleibt mit 6 Räumen.', { width: 330 }),
    T('Ein Start-Script gibt dir CHELAST auf Level 5, den Pokédex und die Laufschuhe.', { width: 330 }),
  ], { first: true });
  const mitte = Spalte([
    T('Wird geleert', { w: 600 }),
    T('536 Orte werden geleert: ihre Figuren, Türen, Auslöser und Level-Scripts verschwinden. Scripts und Texte bleiben liegen.', { width: 330 }),
    T('467 Felder der Weltkarte werden zu einer gesperrten Randkarte.', { width: 330 }),
  ]);
  const rechts = Spalte([Hinweis('Dafür müssen die Scripts übernommen sein.', 'Scripts übernehmen')]);
  // Fuß: Listensymbol, der Satz, rechts die Plakette „Nicht rückgängig“ (Warnung mit Symbol)
  const fuss = <Frame name="Fuß: Zusammenfassung" flex="row" items="center" gap={8} px={16} py={8} w="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {I('ph:list', 18, 's')}
    {T('Leert die Welt bis auf Zweiblattdorf. Die Welt holst du mit ⌘Z zurück; Randkarte, Level-Script und Start-Script bleiben als Dateien.', { grow: 1 })}
    {Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}
  </Frame>;
  const hand = <Frame name="Assistent: Was bleibt" flex="col" w="fill" h="fill" bg={v('flaeche/deck')}>
    <Frame flex="row" w="fill" grow={1}>{links}{mitte}{rechts}</Frame>
    {fuss}
  </Frame>;
  return Window({ name: 'App: Technik, Welt leeren', active: 'welt', title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 1 von 2: Was bleibt' }),
    children: [liste, Deck({ schau, scharnier: WizardBar({ title: 'Welt leeren', steps: ['Was bleibt', 'Zusammenfassung'], step: 0, last: 'Welt leeren', b: 'welt' }), hand })] });
})()}
