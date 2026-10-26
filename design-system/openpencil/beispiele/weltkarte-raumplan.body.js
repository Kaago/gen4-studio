  // App: Welt bauen, Raumplan (Weltkarte), Phase 3f.1: in Bauen hat die Schau nur „3D“ und „Weltkarte“ (den Reiter „Raster“ gibt es nur in Tüfteln).
  // Gewählt ist im Raum „Ebene 1“ von Ewigwald (3 × 3 Karten, eigene Karte ohne Gebietsabschnitt) die Karte in der Mitte; die Weltkarte zeigt die
  // Ansicht „Raumplan“: Zellen 64 px, „+“ rechts und unten, der Inspector die Karte mit „Höhe“. Quellen: apps/studio/src/workbench/{WorldMapPanel,
  // AreaGrid,breadcrumb}.ts(x), workbench/inspector/MatrixCellEditor.tsx; Plan docs/superpowers/plans/2026-10-26-phase-3f1-raster-nach-tuefteln.md
  // (Entscheidungen „Ansichten der Weltkarte“, „„+““, „Raumplan“, „Wörter“); Gate docs/phase-3f-gate.md (1.3), (1.6), (1.9).
  // Beispielwerte: Kartennummern 0338 bis 0346 (die Mitte 0342 wie im Gate), Farben der Kartenbilder sind Platzhalter (Wald), keine Spielgrafik.
  // Dunkel: bash ../build-screen.sh weltkarte-raumplan --dunkel.
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);

  // ----- Liste „Welt“: Ewigwald offen, der Raum „Ebene 1“ gewählt und gezeigt; In dieser Szene der Raum mit seinen neun Karten -----
  const orte = [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
    TreeRowX({ label: 'Flori', icon: 'ph:globe', depth: 1, open: false, count: 7 }),
    TreeRowX({ label: 'Ewigwald', icon: 'ph:globe', depth: 1, open: true, count: 2 }),
    TreeRowX({ label: 'Ebene 1', icon: 'ph:house', depth: 2, sel: true, shown: true }),
    TreeRowX({ label: 'Ebene 2', icon: 'ph:house', depth: 2 }),
    TreeRowX({ label: 'Route 205', icon: 'ph:globe', depth: 1, count: 1 }),
  ];
  const karte = (n, x, y) => TreeRowX({ label: 'Karte ' + n + ' · Zelle ' + x + ', ' + y, icon: 'ph:mountains', depth: 2, open: false, sel: n === '0342' });
  const szene = [
    TreeRowX({ label: 'Ebene 1', icon: 'ph:house', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 9 }),
    karte('0338', 0, 0), karte('0339', 1, 0), karte('0340', 2, 0), karte('0341', 0, 1), karte('0342', 1, 1), karte('0343', 2, 1), karte('0344', 0, 2), karte('0345', 1, 2), karte('0346', 2, 2),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 6 }),
    TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 2 }),
  ];
  const liste = WeltListe({ orte, szene, orteH: 268 });

  // ----- Schau-Gruppe: Reiter „3D“ und „Weltkarte“, vorn die Weltkarte in der Ansicht „Raumplan“ -----
  const wald = ['#3F6B29', '#4A7B31', '#527B39', '#3F6321', '#4A7329', '#5A8442', '#3F6B31', '#4A6B29', '#527331'];
  const zelle = (i) => GridCell({ map: String(338 + i).padStart(4, '0'), pic: wald[i], sel: i === 4 });
  const plus = () => GridCell({ plus: true });
  const plan = AreaGridX([
    [zelle(0), zelle(1), zelle(2), plus()],
    [zelle(3), zelle(4), zelle(5), plus()],
    [zelle(6), zelle(7), zelle(8), plus()],
    [plus(), plus(), plus()],
  ], 'Raumplan, 3 Zeilen, 3 Spalten');
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: 'Weltkarte', focused: true, h: 548, bg: 'flaeche/blatt',
    children: [WorldMapHead('Raumplan', 'Ewigwald, Ebene 1'), <Frame name="Weltkarte" flex="col" w="fill" grow={1} p={16} bg={v('flaeche/welt')}>{plan}</Frame>] });
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });

  // ----- Inspector „Karte“: Titel die Karte, Kennung die Zelle (ein Raum hat keinen Gebietsabschnitt, das Feld dafür fehlt in Bauen) -----
  const felder = <Frame name="Felder" flex="col" w="fill" pt={2}>
    {Row({ label: 'Karte', hint: '0 bis 665, leer = keine Karte', children: Field('342') })}
    {Row({ label: 'Höhe', hint: '0 bis 255', children: NumField('0') })}
    <Frame px={DI.innen} py={8} w="fill">{T('„Duplizieren“ (Menü „Aktionen“) trägt eine Kopie der Karte in diese Zelle ein und lässt sich nicht rückgängig machen: Die Kopie ist sofort als neue Karte am Ende des Archivs vorhanden.', { s: 12, c: 'text/sekundaer', width: 312 })}</Frame>
  </Frame>;
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
    children: [InspectorHead({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0342', ident: 'Zelle 1, 1' }), felder] });

  const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;
  return Window({ name: 'App: Welt bauen, Raumplan (Weltkarte)', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Ewigwald, Karte 0342 (1, 1)' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Ewigwald'], ['Ebene 1', '0203'], ['Karte 0342', 'Zelle 1, 1']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });
