  // App: Welt bauen, Raum mit Plakette „Geteilt“ (Phase 3f.4): gewählt ist das Pokémon-Center von Jubelstadt (Header 0006), dessen Karte 16 andere
  // Räume nutzen. Der Inspector-Kopf trägt „Geteilt mit 16 Räumen“, das Menü „Aktionen“ ist offen mit „Für diesen Raum abspalten…“ unter dem Zeiger;
  // im Baum „Orte“ trägt die Zeile des Raums das Symbol „geteilt“. Quellen: apps/studio/src/workbench/{WorldTree,sharedRooms}.ts(x),
  // workbench/inspector/{HeaderEditor,InspectorHead,SplitRoomDialog}.tsx, workbench/places/PlacePicker.tsx; Plan
  // docs/superpowers/plans/2026-10-26-phase-3f4-nahtlos-abspalten.md (Entscheidungen „Plakette „Geteilt““, „Abspalten in der App“);
  // Gate docs/phase-3f-gate.md (4.3), (4.4). Die Szene ist der Platzhalter der Bibliothek (keine Innenraum-Grafik).
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);

  const geteilt = <Frame name="geteilt" flex="row" items="center">{I('ph:share-network', DI.symbol, 's')}</Frame>;
  const orte = [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
    TreeRowX({ label: 'Jubelstadt', icon: 'ph:globe', depth: 1, open: true, count: 11 }),
    TreeRowX({ label: 'Pokémon-Center', icon: 'ph:house', depth: 2, sel: true, shown: true, end: geteilt }),
    TreeRowX({ label: 'Pokémon-Center, 1. Stock', icon: 'ph:house', depth: 2 }),
    TreeRowX({ label: 'Pokémarkt', icon: 'ph:house', depth: 2 }),
    TreeRowX({ label: 'Haus 1', icon: 'ph:house', depth: 2 }),
  ];
  const szene = [
    TreeRowX({ label: 'Pokémon-Center', icon: 'ph:house', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 }),
    TreeRowX({ label: 'Karte 0191 · Zelle 0, 0', icon: 'ph:mountains', depth: 2, open: false }),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 3 }),
    TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 2 }),
  ];
  const liste = WeltListe({ orte, szene, orteH: 268 });

  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: 548, bg: 'flaeche/welt',
    children: Welt3D({ w: 740, h: 548 - DI.reiter, sel: -1, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 })] }) });
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });

  // ----- Inspector: Kopf mit Plakette und offenem Menü „Aktionen“, darunter die Abschnitte des Raums mit dem Picker „Ort“ -----
  const kopf = InspectorHeadB({ word: 'Raum', icon: 'ph:house', title: 'Pokémon-Center', ident: 'Jubelstadt · C01PC0101', badges: [SharedBadge(16)] });
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: true, w: 340, edge: 'left',
    children: [kopf,
      Section({ title: 'Karten', children: Row({ label: 'Ort', children: PlaceBtn('Jubelstadt', '#E7A5C6') }) }),
      Section({ title: 'Aussehen', open: false }), Section({ title: 'Texte und Scripts', open: false }),
      ContextMenu({ x: 92, y: 60, w: 240, items: [{ label: 'Raum hier anbinden…' }, { label: 'Ort umbenennen…' }, { label: 'Für diesen Raum abspalten…', hi: true }] })] });

  const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;
  return Window({ name: 'App: Welt bauen, Raum geteilt (Plakette)', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Jubelstadt' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Jubelstadt'], ['Pokémon-Center', '0006']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });
