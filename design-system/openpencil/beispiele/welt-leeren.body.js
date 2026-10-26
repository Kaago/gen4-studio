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
  // Hinweis mit Aktionsknopf (Notice mit actions): Warnung, Satz, Fortschrittsbalken eines laufenden Übernehmens, Knopf unter dem Satz (wie Notice.tsx: actions im Body)
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
