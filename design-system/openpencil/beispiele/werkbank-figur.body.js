  // App: Welt bauen, Figur bearbeiten (Werkbank), Phase 3e.4: ein Dock je Bereich in der Anordnung „Werkbank“. Nachfolger von welt-figur.body.js
  // (dort noch zwei Decks, Schau oben und Hand unten). Quellen: apps/studio/src/workbench/{WorldList,WorldExplorer,WorldTree,SceneHierarchy,sceneModel,
  // layout}.ts(x), workbench/inspector/{InspectorHead,EventEditor,eventFields,TrainerField}.tsx, ui/frame/{Hinge,FloatingToolbar}.tsx,
  // ui/tokens/dockview-theme.css; Plan docs/superpowers/plans/2026-10-25-phase-3e4-ein-dock-je-bereich.md, Entscheidung „Vorlage „Werkbank““.
  // Raster bei 1440 × 900: Atlas 88, Liste „Welt“ 272, Dock 1080 × 832; darin oben das Scharnier 44, darunter die Schau-Gruppe (3D, Raster, Weltkarte;
  // Reiter dichte/reiter 28) und die Gruppe Script/Probleme 240 hoch unter der Schau, rechts der Inspector 340 breit über die volle Höhe (788 px).
  // Dunkel: bash ../build-screen.sh werkbank-figur --dunkel (dieselbe Datei; Symbolfarben schaltet die nächste Zeile um).
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);

  // ----- Liste „Welt“: „Orte“ (Weltbaum) über „In dieser Szene“ (Hierarchie), dazwischen der Teiler „Höhe der Orte“ (Anteil hier 0,31) -----
  const orte = <Frame name="Orte" flex="col" w="fill" h={248} overflow="hidden">
    {PanelHead({ title: 'Orte', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:sort-ascending')}{IconBtn('ph:plus')}{IconBtn('ph:caret-down')}</Frame> })}
    <Frame flex="col" gap={6} p={8} w="fill">{T('1 Weltkarte, 72 Gebiete, 492 Räume', { s: 12, c: 'text/sekundaer', one: true })}{Search('Gebiet, Raum oder Kürzel suchen')}</Frame>
    <Frame name="Baum „Orte“" flex="col" px={4} w="fill">
      {TreeRowX({ label: 'Scripts', icon: 'ph:code', open: false })}
      {TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 })}
      {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, shown: true, count: 6 })}
      {TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, count: 0 })}
      {TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 })}
      {TreeRowX({ label: 'Route 202', icon: 'ph:globe', depth: 1, count: 0 })}
    </Frame>
  </Frame>;
  const fig = (n, x, z) => TreeRowX({ label: 'Objekt ' + n + ' · ID ' + n, tech: 'x ' + x + ', z ' + z, techW: 44, icon: 'ph:person-simple-walk', depth: 2, sel: n === 2 });
  const tuer = (n, ziel, x, z) => TreeRowX({ label: 'Warp ' + n, tech: 'x ' + x + ', z ' + z, sub: '→ ' + ziel, icon: 'ph:door', depth: 2 });
  const szene = <Frame name="In dieser Szene" flex="col" w="fill" grow={1} overflow="hidden">
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
  const liste = ListPane([PanelHead({ title: 'Welt', sub: 'Platin, DE' }), <Frame name="Spalte" flex="col" w="fill" grow={1}>{orte}{Teiler('Teiler „Höhe der Orte“')}{szene}</Frame>]);

  // ----- Schau-Gruppe: 3D vorn, die Werkzeugleiste senkrecht links unter den Ebenen, die Legende unten links -----
  const SW = 740, SH = 548 - DI.reiter;
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Raster', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt',
    children: Welt3D({ w: SW, h: SH, sel: 2, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 }), LegendBtn(8, SH - 40)] }) });

  // ----- Script und Probleme unter der Schau (240), nicht unter dem Inspector -----
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
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
    children: [InspectorHead({ word: 'Figur', icon: 'ph:person-simple-walk', title: 'Objekt 2 · ID 2', ident: 'Objekt 2 · Eventdatei 0390' }), felder] });

  const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;
  return Window({ name: 'App: Welt bauen, Figur bearbeiten (Werkbank)', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Zweiblattdorf'], ['Gebiet', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });
