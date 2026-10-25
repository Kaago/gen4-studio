  const place = (n, c, cnt, o = {}) => ListRow({ title: n, caret: o.open ? 'open' : 'closed', icon: o.icon || 'ph:map-trifold', depth: o.d || 0, sel: o.sel, tech: o.tech, right: undefined, mark: cnt !== undefined ? <Frame flex="row" items="center" gap={6}><Frame w={8} h={8} rounded={1} bg={c} />{T(String(cnt), { s: 12, f: M, c: 'text/sekundaer' })}</Frame> : (o.grid ? I('ph:grid-four', 18, 'ok') : undefined) });
  const liste = ListPane([
    PanelHead({ title: 'Orte', sub: 'Platin, DE', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:plus')}</Frame> }),
    <Frame flex="col" gap={8} px={8} pt={8} pb={4} w="fill">{T('125 Orte, 593 Header', { s: 12, c: 'text/sekundaer' })}{Search('Ort, Raum oder Kürzel suchen')}</Frame>,
    <Frame flex="col" px={4} w="fill">
      {place('Route 216', '#E7A5C6', 2)}{place('Route 217', '#E7A5C6', 3)}{place('Route 218', '#E7A5C6', 3)}{place('Route 219', '#E7A5C6', 1)}{place('Route 221', '#E7A5C6', 2)}{place('Route 222', '#E7A5C6', 4)}{place('Route 224', '#E7A5C6', 1)}{place('Route 227', '#E7A5C6', 2)}{place('Route 228', '#E7A5C6', 5)}{place('Route 229', '#E7A5C6', 1)}
      {place('Zweiblattdorf', '#DEE79C', 7, { open: true })}
      {place('Zweiblattdorf', null, undefined, { d: 1, icon: 'ph:globe', sel: true, grid: true })}
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
      <Frame position="absolute" x={612} y={150} w={60} h={40} bg="#E7A57B" /><Frame position="absolute" x={690} y={140} w={46} h={60} bg="#E7A57B" /><Frame position="absolute" x={420} y={240} w={30} h={24} bg="#3173C6" /><Frame position="absolute" x={560} y={300} w={40} h={20} bg="#3173C6" />
      <Frame position="absolute" x={8} y={8} flex="row" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow="0 8 24 #1821313D">{IconBtn('ph:stack', { on: true })}{IconBtn('ph:map-pin', { on: true })}</Frame>
      <Frame position="absolute" x={8} y={424} flex="row" items="center" gap={8} px={12} h={32} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:list', 18, 'p')}{T('Legende', { w: 600 })}</Frame>
    </Frame>
  ];
  const tools = <Frame flex="col" gap={4} p={8} w="fill">{[[['Auswählen', 'ph:cursor', 'V', true], ['Gelände malen', 'ph:paint-brush', 'B']], [['Figur', 'ph:person-simple-walk', 'F'], ['Tür', 'ph:door', 'T']], [['Auslöser', 'ph:lightning', 'A'], ['Schild', 'ph:signpost', 'S']], [['Gebäude', 'ph:cube', 'G']]].map((row) => <Frame flex="row" gap={4}>{row.map(([n, i, k, on]) => Tool(n, i, k, on))}</Frame>)}{T('Wählt Marker und Gebäude in der Szene.', { s: 12, c: 'text/sekundaer', width: 160 })}</Frame>;
  const insp = <Frame flex="col" grow={1} h="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 0, right: 1, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {DockTabs(['Inspector', 'Script', 'Probleme'], 'Inspector', 'flaeche/deck')}
    <Frame flex="row" items="center" gap={8} px={12} h={32} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{T('Objekt 2 · ID 2', { w: 600 })}{Mono('Figur 3 von 8 in Zweiblattdorf')}</Frame>
    <Frame flex="col" w="fill" pt={2}>
      {Row({ label: 'x (Kachel)', children: NumField('105') })}{Row({ label: 'z (Kachel)', children: NumField('875') })}{Row({ label: 'Lokale ID', children: NumField('2') })}{Row({ label: 'Grafik', children: NumField('148') })}{Row({ label: 'Bewegung', children: NumField('15') })}{Row({ label: 'Blickrichtung', children: Field('Unten', { select: true }) })}
    </Frame>
  </Frame>;
  const ort = HandCol('An diesem Ort', <Frame flex="col" w="fill" px={4} pt={4}>
    <Frame flex="row" items="center" gap={6} px={8} h={28}>{T('Figuren', { w: 600 })}{T('8', { s: 12, c: 'text/sekundaer' })}</Frame>
    {[['0', '119, z 878'], ['1', '108, z 889'], ['2', '105, z 875'], ['3', '108, z 869'], ['4', '112, z 878'], ['5', '114, z 885'], ['6', '109, z 875']].map(([n, p]) => ListRow({ title: 'Objekt ' + n + ' · ID ' + n, icon: 'ph:person-simple-walk', right: 'x ' + p, sel: n === '2' }))}
  </Frame>, 260, '8 Figuren, 4 Türen');
  return Window({ name: 'App: Welt bauen, Figur gewählt', active: 'welt', title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf, Raster 0' }),
    children: [liste, Deck({ schau, scharnier: Scharnier({ crumbs: [['Zweiblattdorf'], ['Außen', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 }), hand: <Frame flex="row" w="fill" h="fill">{HandCol('Werkzeuge', tools, 180)}{insp}{ort}</Frame> })] });
