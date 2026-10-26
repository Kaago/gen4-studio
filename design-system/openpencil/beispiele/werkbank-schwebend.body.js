  // App: Welt bauen, Inspector schwebend (Werkbank), Phase 3e.4: derselbe Moment wie werkbank-figur.body.js, der Inspector aber als schwebende Gruppe
  // über dem rechten Rand der Schau (Kontextmenü „Schweben lassen“ oder Palette „Inspector schweben lassen“: addFloatingGroup, 340 × 480), die Schau-Gruppe
  // und Script/Probleme füllen die Breite. Offen ist das Kontextmenü des Reiters „Inspector“ (Rechtsklick oder Umschalt+F10). Die Orte sind eingeklappt
  // (eine Zeile mit dem gezeigten Ort), so zeigt die Hierarchie mehr. Quellen wie dort, dazu Plan 3e.4, Entscheidungen „Schweben“ und „Kontextmenü (deutsch)“.
  // Dunkel: bash ../build-screen.sh werkbank-schwebend --dunkel.
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);

  // ----- Liste „Welt“: „Orte“ eingeklappt (Kopf und eine Zeile, die den Wähler „Ort oder Raum wählen“ öffnet), darunter „In dieser Szene“ ohne Teiler -----
  const orte = <Frame name="Orte (eingeklappt)" flex="col" w="fill">
    {PanelHead({ title: 'Orte', actions: <Frame flex="row" items="center" gap={8}>{IconBtn('ph:plus')}{IconBtn('ph:caret-right')}</Frame> })}
    <Frame flex="col" px={4} pt={4} pb={4} w="fill"><Frame name="Ort oder Raum wählen" flex="row" items="center" gap={6} px={DI.innen} h={DI.zeile} w="fill" rounded={6}>{I('ph:globe', DI.symbol, 's')}{T('Zweiblattdorf', { grow: 1, one: true })}{I('ph:caret-down', DI.symbol, 's')}</Frame></Frame>
  </Frame>;
  const fig = (n, x, z) => TreeRowX({ label: 'Objekt ' + n + ' · ID ' + n, tech: 'x ' + x + ', z ' + z, techW: 44, icon: 'ph:person-simple-walk', depth: 2, sel: n === 2 });
  const tuer = (n, ziel, x, z) => TreeRowX({ label: 'Warp ' + n, tech: 'x ' + x + ', z ' + z, sub: '→ ' + ziel, icon: 'ph:door', depth: 2 });
  const szene = <Frame name="In dieser Szene" flex="col" w="fill" grow={1} overflow="hidden" strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {PanelHead({ title: 'In dieser Szene', actions: <Frame flex="row" items="center" gap={4}>{IconBtn('ph:arrow-left')}{IconBtn('ph:arrow-right', { off: true })}{IconBtn('ph:plus')}</Frame> })}
    <Frame flex="col" gap={6} p={8} w="fill">{Search('Name oder Nummer suchen')}</Frame>
    <Frame name="Baum „In dieser Szene“" flex="col" px={4} w="fill">
      {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true })}
      {TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 })}
      {TreeRowX({ label: 'Karte 0411 · Zelle 0, 0', icon: 'ph:mountains', depth: 2, open: false })}
      {TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: true, count: 8 })}
      {fig(0, 119, 878)}{fig(1, 108, 889)}{fig(2, 105, 875)}{fig(3, 108, 869)}{fig(4, 112, 878)}{fig(5, 114, 885)}{fig(6, 109, 875)}{fig(7, 117, 870)}
      {TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: true, count: 4 })}
      {tuer(0, 'Haus 1', 107, 873)}{tuer(1, 'Haus 2', 115, 873)}{tuer(2, 'Haus 3', 107, 883)}{tuer(3, 'Haus 4', 116, 887)}
      {TreeRowX({ label: 'Auslöser', icon: 'ph:lightning', depth: 1, open: true, count: 2 })}
      {TreeRowX({ label: 'Trigger 0 · Script 5', tech: 'x 110, z 880', icon: 'ph:lightning', depth: 2 })}{TreeRowX({ label: 'Trigger 1 · Script 6', tech: 'x 111, z 880', icon: 'ph:lightning', depth: 2 })}
      {TreeRowX({ label: 'Schilder', icon: 'ph:signpost', depth: 1, open: true, count: 1 })}
      {TreeRowX({ label: 'Hintergrund-Event 0 · Script 12', tech: 'x 113, z 871', icon: 'ph:signpost', depth: 2 })}
      {TreeRowX({ label: 'Scripts', icon: 'ph:code', depth: 1, open: true, count: 2 })}
      {TreeRowX({ label: 'Scriptdatei 0412', icon: 'ph:code', depth: 2 })}{TreeRowX({ label: 'Level-Script 0009', icon: 'ph:code', depth: 2 })}
    </Frame>
  </Frame>;
  const liste = ListPane([PanelHead({ title: 'Welt', sub: 'Platin, DE' }), <Frame name="Spalte" flex="col" w="fill" grow={1}>{orte}{szene}</Frame>]);

  // ----- Schau-Gruppe über die volle Breite des Docks (1080): 3D vorn, Werkzeugleiste senkrecht links, Legende unten links -----
  const SW = 1080, SH = 548 - DI.reiter;
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Raster', 'Weltkarte'], active: '3D', focused: false, h: 548, bg: 'flaeche/welt',
    children: Welt3D({ w: SW, h: SH, sel: 2, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 }), LegendBtn(8, SH - 40)] }) });

  // ----- Script und Probleme unter der Schau (240), jetzt über die volle Breite -----
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });

  // ----- Inspector rechts, 340 breit, volle Höhe: Kopf und die Felder einer Figur (eventFieldSpecs „object“, Platin) -----
  const quiet = (label) => <Frame flex="row" items="center" px={8} h={24} rounded={6}>{T(label, { w: 600 })}</Frame>;
  const felder = <Frame name="Felder" flex="col" w="fill" pt={2}>
    {Row({ label: 'x (Kachel)', children: NumField('105') })}
    {Row({ label: 'z (Kachel)', children: NumField('875') })}
    {Row({ label: 'Lokale ID', children: NumField('2') })}
    {Row({ label: 'Grafik', children: NumField('148') })}
    {Row({ label: 'Bewegung', children: NumField('15') })}
    {Row({ label: 'Blickrichtung', children: Field('Unten', { select: true }) })}
    {Row({ label: 'Script', children: <Frame flex="col" gap={4} grow={1}><Frame flex="row" w="fill">{NumField('3')}</Frame><Frame flex="row" gap={6}>{quiet('Öffnen')}{quiet('Neues Script')}</Frame></Frame> })}
    {Row({ label: 'Trainer', children: [Mono('kein Trainer', { c: 'text/primaer', grow: 1 }), quiet('Ändern…')] })}
    {Row({ label: 'Sichtlinie', children: Field('Aus', { select: true }) })}
    {Row({ label: 'Sichtweite', children: NumField('0') })}
    {Row({ label: 'y', children: <Frame flex="col" gap={2} grow={1}>{Mono('0', { c: 'text/primaer' })}{T('Nur lesbar: Die Höhe lässt sich hier nicht ändern; sie bleibt unverändert erhalten.', { s: 12, c: 'text/sekundaer', width: 188 })}</Frame> })}
  </Frame>;
  // Schwebende Gruppe am rechten Rand der Schau (x, y relativ zum Dock unter dem Scharnier), mit Fokus: Reiter „Inspector“ aktiv
  const inspector = FloatingGroup({ name: 'Gruppe Inspector (schwebend)', x: 1080 - 340 - 12, y: DI.reiter + 12, w: 340, h: 480, tabs: ['Inspector'], active: 'Inspector', focused: true,
    children: [InspectorHead({ word: 'Figur', icon: 'ph:person-simple-walk', title: 'Objekt 2 · ID 2', ident: 'Objekt 2 · Eventdatei 0390' }), felder] });
  // Kontextmenü des Reiters „Inspector“ (getTabContextMenuItems): „Schweben lassen“ gesperrt, die Gruppe schwebt schon; „Zurück ins Dock“ nur schwebend
  const menue = ContextMenu({ x: 1080 - 340 - 12 + 48, y: DI.reiter + 12 + 22 + DI.reiter - 6, w: 220, items: [{ label: 'Maximieren' }, { label: 'Schweben lassen', off: true }, { label: 'Zurück ins Dock' }, '-', { label: 'Anordnung zurücksetzen' }] });

  const dock = <Frame name="Dock Welt" flex="col" w="fill" grow={1}>
    {schau}{unten}
    {inspector}{menue}
  </Frame>;
  return Window({ name: 'App: Welt bauen, Inspector schwebend (Werkbank)', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Zweiblattdorf'], ['Gebiet', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });
