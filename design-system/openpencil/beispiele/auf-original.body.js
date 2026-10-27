  // Auf Original zurücksetzen (Phase 4.1b), drei Frames aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'karte'        „App: Welt, Auf Original zurücksetzen“: Zellen-Inspector der Karte 0003 mit offenem Menü „Aktionen“ (der Eintrag
  //                  „Auf Original zurücksetzen…“ unter dem Zeiger), darüber die Abdunklung und der Dialog „Karte 0003 auf Original
  //                  zurücksetzen?“ mit Vorher und Nachher. Das Frame zeigt zwei Augenblicke zugleich (Menü und Dialog), damit eines reicht.
  //   'header'       „App: Welt, Header zurücksetzen“: Gebiet Jubelstadt mit ungespeicherter Bearbeitung, darüber der Dialog „Header 0003 auf
  //                  Original zurücksetzen?“: Info zu den ungespeicherten Änderungen, Vorher und Nachher, drei Felder mit Häkchen (eines abgewählt)
  //   'aenderungen'  „App: Technik, Änderungen“: Liste „Technik“ mit sieben Einträgen, „Änderungen“ gewählt; Fläche mit drei Gruppen
  //                  „Karten“ (1), „Trainer“ (1, ungespeichert), „Matrizen“ (1, ohne Zurücksetzen)
  // Quellen: apps/studio/src/original/{ResetDialog,resetEntry,OriginalContext,HeadActions}.tsx und ResetDialog.module.css,
  // technik/ChangesSurface.tsx und .module.css (Fläche wie TechnikArea: flaeche/deck, 32 / 24 innen, Lesebreite 880; die Gruppen in einer Karte
  // flaeche/blatt mit Kante linie/standard und radius/fenster), technik/TechnikArea.tsx (sieben Einträge, Untertitel), ui/primitives/{BeforeAfter,
  // Dialog,Menu,Checkbox,Section,ListRow,Badge}.tsx, workbench/inspector/{MatrixCellEditor,HeaderEditor}.tsx, workbench/inspector/fields.ts
  // (Feldnamen). Texte aus design-system/README.md, „Nachtrag 4.1b“. Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“.
  // Beispielwerte: die Zeilen von Vorher und Nachher (Hashes, Größen, Header-Werte) und die Zelle der Karte sind erfunden, nicht aus dem ROM.
  // Vergleich mit den Screenshots unter target/agent/gate-2026-10-27/4-1b/ (blick-dialog-*, blick-aenderungen-*, blick-kopf-aktionen-hell.png).
  // Dunkel: bash ../build-screen.sh auf-original --dunkel.
  const MODE = 'karte';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { karte: 'App: Welt, Auf Original zurücksetzen', header: 'App: Welt, Header zurücksetzen', aenderungen: 'App: Technik, Änderungen' };
  const BASIS = 'Platin DE (Testprojekt)';
  const RESET = 'Auf Original zurücksetzen…';

  // ----- Dialog (Dialog-Primitive, 480 breit): Kreuz, Titel text/l, Beschreibung sekundär, Inhalt (Abstand 12), Tasten rechts -----
  const W = 480;
  const IW = W - 40;
  const Meldung = (text, art) => {
    const m = { info: ['status/info-flaeche', 'ph:info-fill', 'info'], warn: ['status/warnung-flaeche', 'ph:warning-fill', 'w'] }[art];
    return <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[1], 18, m[2])}{T(text, { width: IW - 24 - 26 })}</Frame>;
  };
  const Dialog = (title, inhalt) => <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T(title, { s: 18, w: 600, width: W - 72 })}
    {T('Stellt den Stand der Basis-ROM ' + BASIS + ' wieder her. ⌘Z nimmt es zurück.', { c: 'text/sekundaer', width: IW })}
    <Frame name="Inhalt" flex="col" gap={12} w="fill">{inhalt}</Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Zurücksetzen', art: 'Primär' })}</Frame>
  </Frame>;

  // ----- Vorher und Nachher (BeforeAfter): zwei Spalten (Abstand 8), je Kante linie/leise, radius/feld, Kopf flaeche/deck text/xs 600 sekundär;
  // Zeilen code/s (13/20); eine geänderte Zeile auf auswahl/flaeche mit − (Vorher) oder + (Nachher) in der Zeichenspalte, nie nur Farbe -----
  const SW = (IW - 8) / 2;
  const Zeile = ([text, anders], zeichen) => <Frame name={text} flex="row" items="center" w="fill" h={20} bg={anders ? v('auswahl/flaeche') : undefined}>
    <Frame flex="row" w={20} pl={8}>{anders && T(zeichen, { f: M, s: 13, lh: 20, c: 'text/sekundaer' })}</Frame>
    {T(text, { f: M, s: 13, lh: 20, one: true, grow: 1 })}
  </Frame>;
  const Seite = (titel, zeilen, zeichen) => <Frame name={titel} flex="col" w={SW} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1} overflow="hidden">
    <Frame flex="row" items="center" px={8} py={4} w="fill" bg={v('flaeche/deck')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>
    <Frame flex="col" py={4} w="fill">{zeilen.map((z) => Zeile(z, zeichen))}</Frame>
  </Frame>;
  const VorherNachher = (vorher, nachher) => <Frame name="Vorher und Nachher" flex="row" gap={8} w="fill">{Seite('Vorher', vorher, '−')}{Seite('Nachher', nachher, '+')}</Frame>;

  // ----- 'aenderungen': Technik › Änderungen -----
  if (MODE === 'aenderungen') {
    const eintraege = [
      ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
      ['Prüfen und Bauen', 'ph:hammer', 'gebaut vor 3 Minuten'],
      ['Veröffentlichen', 'ph:upload-simple', 'Bereit für einen Patch'],
      ['Änderungen', 'ph:git-diff', '3 Einträge weichen ab', true],
      ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
      ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
      ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
    ];
    const liste = ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
      {eintraege.map(([title, icon, sub, sel]) => ListRow({ title, icon, sub, sel }))}
    </Frame>]);

    // Zeile (ohne Kontextmenü): Name, rechts Plakette(n) und die kleine sekundäre Taste; gesperrt trägt sie den Tooltip „Für diese Art gibt es kein
    // Zurücksetzen.“ (Matrizen), den die Zeichnung nicht zeigt
    const Eintrag = (label, { unsaved, gesperrt } = {}) => ListRow({ title: label, mark: <Frame name="Rechts" flex="row" items="center" gap={6}>
      {Badge('Geändert', 'warn', 'ph:warning')}
      {unsaved && Badge('Ungespeichert', 'ungespeichert', 'ph:circle')}
      {Btn({ label: RESET, small: true, off: gesperrt })}
    </Frame> });
    const Gruppe = (title, rows) => <Frame name={'Gruppe ' + title} flex="col" w="fill">
      {Section({ title, count: rows.length, children: <Frame name={title} flex="col" gap={2} px={8} w="fill">{rows}</Frame> })}
    </Frame>;
    const gruppen = <Frame name="Gruppen" flex="col" w={880} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} overflow="hidden">
      {Gruppe('Karten', [Eintrag('Karte 0003')])}
      {Gruppe('Trainer', [Eintrag('Trainer 0001', { unsaved: true })])}
      {Gruppe('Matrizen', [Eintrag('Matrizen 0000', { gesperrt: true })])}
    </Frame>;

    const kopf = <Frame name="Kopf" flex="row" items="start" justify="between" gap={16} w={880}>
      <Frame flex="col" gap={4} grow={1}>
        {T('Änderungen', { f: Z, s: 24, lh: 30, w: 700 })}
        {T('Gegenüber der Basis-ROM ' + BASIS + '. Gezählt wird der gespeicherte Stand.', { width: 560 })}
      </Frame>
      {Btn({ label: 'Neu vergleichen' })}
    </Frame>;
    const flaeche = <Frame name="Fläche: Änderungen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>{kopf}{gruppen}</Frame>;
    return Window({ name: NAMES.aenderungen, active: 'technik', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 }, status: Statusbar({ build: 'Gebaut vor 3 Minuten', built: true }), children: [liste, flaeche] });
  }

  // ----- 'karte' und 'header': Welt bauen mit Liste, Schau (3D), Script unten und dem Inspector rechts -----
  const KARTE = MODE === 'karte';
  const orte = KARTE ? [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
    TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, count: 4 }),
    TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, open: false, count: 1 }),
    TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 6 }),
    TreeRowX({ label: 'Jubelstadt', icon: 'ph:globe', depth: 1, open: false, count: 11 }),
  ] : [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
    TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, count: 4 }),
    TreeRowX({ label: 'Jubelstadt', icon: 'ph:globe', depth: 1, open: true, count: 11, sel: true, shown: true }),
    TreeRowX({ label: 'Pokémon-Center', icon: 'ph:house', depth: 2 }),
    TreeRowX({ label: 'Pokémarkt', icon: 'ph:house', depth: 2 }),
  ];
  const szene = KARTE ? [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 }),
    TreeRowX({ label: 'Karte 0003 · Zelle 4, 2', icon: 'ph:mountains', depth: 2, sel: true }),
  ] : [
    TreeRowX({ label: 'Jubelstadt', icon: 'ph:globe', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: false, count: 4 }),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 12 }),
    TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 9 }),
  ];
  const liste = WeltListe({ orte, szene, orteH: 268 });
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: 548, bg: 'flaeche/welt',
    children: Welt3D({ w: 740, h: 548 - DI.reiter, sel: -1, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 })] }) });
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });

  // Zellen-Inspector (Bauen): Titel die Karte, Kennung die Zelle; Menü „Aktionen“: Teile, „Duplizieren“, Trenner, „Auf Original zurücksetzen…“ (undo)
  const inspectorKarte = [
    InspectorHead({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0003', ident: 'Zelle 4, 2' }),
    <Frame name="Felder" flex="col" w="fill" pt={2}>
      {Row({ label: 'Karte', hint: '0 bis 665, leer = keine Karte', children: Field('3') })}
      {Row({ label: 'Höhe', hint: '0 bis 255', children: NumField('0') })}
    </Frame>,
    MenuX({ x: 340 - 8 - 260, y: 44, w: 260, items: [
      { label: 'Teile importieren…' }, { label: 'Teile exportieren…' }, { label: 'Duplizieren' }, '-',
      { label: RESET, icon: 'ph:arrow-counter-clockwise', hi: true },
    ] }),
  ];
  // Gebiets-Inspector mit ungespeicherter Bearbeitung (Musik, Wetter, Kamera geändert)
  const inspectorHeader = [
    InspectorHead({ word: 'Gebiet', icon: 'ph:globe', title: 'Jubelstadt', ident: 'Jubelstadt · C01', badges: [['Ungespeichert', 'ungespeichert']] }),
    Section({ title: 'Karten', open: false }), Section({ title: 'Aussehen', open: false }), Section({ title: 'Texte und Scripts', open: false }),
  ];
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: true, w: 340, edge: 'left', children: KARTE ? inspectorKarte : inspectorHeader });
  const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;
  const crumbs = KARTE ? [['Sinnoh'], ['Karte 0003', 'Zelle 4, 2']] : [['Jubelstadt', '0003']];
  const win = Window({ name: NAMES[MODE], active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: KARTE ? 0 : 1 },
    status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: KARTE ? 'Sinnoh, Karte 0003 (4, 2)' : 'Jubelstadt' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs, depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: KARTE ? 0 : 3 })}{dock}</Frame>] });

  if (KARTE) {
    // Karte: eine Änderung, ihr Name steht im Titel; Zeilen des Kernels (Bewegungsrechte, Gebäude, Modell, BDHC mit den ersten acht Stellen der xxh3)
    const dialog = Dialog('Karte 0003 auf Original zurücksetzen?', VorherNachher(
      [['Bewegungsrechte 5c0e71a2', true], ['Gebäude 6', true], ['Modell 38412 Byte 9d24be07', false], ['BDHC 1240 Byte 0f3a8c51', false]],
      [['Bewegungsrechte 1b7f42d9', true], ['Gebäude 5', true], ['Modell 38412 Byte 9d24be07', false], ['BDHC 1240 Byte 0f3a8c51', false]],
    ));
    return MitDialog(NAMES.karte, win, dialog);
  }

  // Header: Info (ungespeichert), Vorher und Nachher mit den serde-Namen (Reihenfolge des Kernels: alphabetisch), je Feld ein Häkchen mit dem
  // deutschen Namen aus fields.ts und rechts „vorher → nachher“ in code/s sekundär; „Kamera“ ist abgewählt, „Zurücksetzen“ bleibt frei
  const Feld = (on, label, werte) => <Frame name={label} flex="row" items="center" justify="between" gap={12} w="fill" minH={DI.zeile}>
    {Check(on, label)}{Mono(werte, { s: 13, lh: 20 })}
  </Frame>;
  const dialog = Dialog('Header 0003 auf Original zurücksetzen?', [
    Meldung('Auch ungespeicherte Änderungen werden ersetzt; ⌘Z stellt sie wieder her.', 'info'),
    VorherNachher(
      [['camera_angle_id: 4', true], ['music_day_id: 1102', true], ['weather_id: 2', true]],
      [['camera_angle_id: 0', true], ['music_day_id: 1015', true], ['weather_id: 0', true]],
    ),
    <Frame name="Felder" flex="col" gap={4} w="fill">
      {Feld(false, 'Kamera', '4 → 0')}
      {Feld(true, 'Musik Tag', '1102 → 1015')}
      {Feld(true, 'Wetter', '2 → 0')}
    </Frame>,
  ]);
  return MitDialog(NAMES.header, win, dialog);
