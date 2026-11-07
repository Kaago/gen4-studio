  // Quelle einer Karte und Welt-Konflikt (Phase 6.3b), zwei Frames aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'quelle'    „App: Inspector, Karte als Quelle“: Werkbank der Welt (Zweiblattdorf, Karte 0000 gewählt), in der Schau ein Platzhalter des Dorfs
  //               (das gebackene Modell, keine Spielgrafik), rechts der Inspector „Karte 0000“ mit „Karte“, „Höhe“, dem offenen Abschnitt „Quelle“ einer
  //               Welt-Karte („Welt-Karte“, „Gebacken · Modell 38.876 B · Terrain 146 B“, „Als binäre Karte übernehmen“, die Erfolgsmeldung „Übernommen.“)
  //               und darunter „Budget“ eingeklappt. Unten rechts in der Schau als Ausschnitt der zweite Zustand: der Abschnitt einer binären
  //               Karte außerhalb eines Textprojekts („Karte als Quelle übernehmen“ gesperrt, darunter der Grund)
  //   'konflikt'  „App: Textprojekt, Welt-Konflikt“: Technik › Textprojekt, Abschnitt „Auf zwei Seiten geändert“ (1) mit der Zeile „Karte 0000“
  //               aufgeklappt: der Satz „Diese Welt-Karte wurde außerhalb des Studios geändert.“, die Tabelle Feld / Studio / DSPRE (Backen gegen
  //               Member), „und 566 weitere Abweichungen“ und die zwei Tasten „Neu backen“ und „Als binäre Karte übernehmen“; „Zuletzt übernommen“ (0);
  //               unten rechts die Meldung „1 Konflikt zwischen Studio und DSPRE“ mit „Lösen…“
  // Quellen: apps/studio/src/workbench/inspector/{WorldSourceSection.tsx,WorldSourceSection.module.css,MatrixCellEditor.tsx},
  // technik/{TextProjectSurface.tsx,stepWords.ts}, ui/primitives/{Section,Button,Notice,Table}. Texte aus design-system/README.md, „Nachtrag 6.3b“.
  // Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte: die Zahlen der Zeile des Backens und die Abweichungen stammen aus dem Lauf der Test-Umgebung
  // (Karte 0000 nach „Karte als Quelle übernehmen“, Member durch 0001 ersetzt), die Bilder unter target/agent/gate-2026-11-07/6-3b/ (blick-quelle-*,
  // blick-konflikt-*).
  // Nicht gerendert: OpenPencil antwortete beim Schreiben nicht. Beim ersten Rendern Maße und Umbrüche mit den Screenshots abgleichen
  // (Anleitung: openpencil/NACHTRAG-6-3b.md).
  // Dunkel: bash ../build-screen.sh welt-quelle --dunkel.
  const MODE = 'quelle';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { quelle: 'App: Inspector, Karte als Quelle', konflikt: 'App: Textprojekt, Welt-Konflikt' };

  // ----- Abschnitt „Quelle“ (WorldSourceSection): Raster mit Abstand raum/6, Rand raum/4 oben/unten und raum/8 seitlich; Sätze text/s primär,
  // Hinweise text/xs sekundär; Tasten sekundär klein in einer Zeile (Abstand raum/8); Meldungen über Notice (Erfolg „ok“, Fehler „fehler“) -----
  const QW = 340 - 16;
  const Quelle = ({ world, textProject = true, bake, refused, notice, open = true }) => Section({ title: 'Quelle', open, children: <Frame name={world ? 'Quelle (Welt-Karte)' : 'Quelle (binär)'} flex="col" gap={6} px={8} py={4} w="fill">
    {world ? T('Welt-Karte') : T('Binäre Karte aus dem ROM oder aus DSPRE.', { width: QW })}
    {world && bake && T('Gebacken · Modell ' + bake[0] + ' B · Terrain ' + bake[1] + ' B', { width: QW })}
    {world && refused && Notice(refused, 'fehler')}
    <Frame name="Tasten" flex="row" items="center" gap={8}>
      {world ? Btn({ label: 'Als binäre Karte übernehmen', small: true }) : Btn({ label: 'Karte als Quelle übernehmen', small: true, off: !textProject })}
    </Frame>
    {!world && !textProject && T('Boden malen gibt es im Textprojekt (Technik › Textprojekt).', { s: 12, c: 'text/sekundaer', width: QW })}
    {notice && Notice(notice, 'ok')}
  </Frame> });

  // ----- 'quelle': Werkbank der Welt mit dem Inspector einer Kartenzelle -----
  if (MODE === 'quelle') {
    const SW = 740, SH = 548 - DI.reiter;
    const orte = [
      TreeRowX({ label: 'Scripts', icon: 'ph:file-code' }),
      TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 1 }),
      TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: true, shown: true, sel: true, count: 6 }),
      TreeRowX({ label: 'Haus 1', icon: 'ph:house', depth: 2 }),
      TreeRowX({ label: 'Haus 2', icon: 'ph:house', depth: 2 }),
    ];
    const szene = [
      TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true }),
      TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 }),
      TreeRowX({ label: 'Karte 0000', tech: '3, 27', icon: 'ph:mountains', depth: 2, open: false, sel: true }),
      TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 8 }),
    ];
    // Platzhalter des Dorfs (keine Spielgrafik): Wiese, Wege, vier Häuser, ein Teich; der Zellrahmen der gewählten Karte (auswahl/halo)
    const dorf = <Frame name="3D-Szene (gebackenes Modell)" w={SW} h={SH} bg={v('flaeche/welt')} overflow="hidden">
      <Frame position="absolute" x={120} y={40} w={500} h={400} bg="#7B9C31" />
      <Frame position="absolute" x={350} y={40} w={40} h={400} bg="#D6C08C" />
      <Frame position="absolute" x={160} y={200} w={420} h={30} bg="#D6C08C" />
      {[[180, 90], [440, 90], [180, 260], [440, 260]].map(([x, y]) => <Frame position="absolute" x={x} y={y} w={110} h={80} rounded={4} bg="#4A7B5A" stroke="#101821" strokeWidth={1} />)}
      <Frame position="absolute" x={320} y={360} w={100} h={60} rounded={4} bg="#4A8CD6" />
      <Frame position="absolute" x={120} y={40} w={500} h={400} stroke={v('auswahl/halo')} strokeWidth={6} />
      {FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 8 })}
      {LegendBtn(8, SH - 40)}
      <Frame name="Zustand binär (Ausschnitt)" position="absolute" x={SW - 340 - 16} y={SH - 200} w={340} flex="col" pt={2} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>
        {Quelle({ world: false, textProject: false })}
      </Frame>
    </Frame>;
    const Stapel = (label, feld, hint) => <Frame name={label} flex="col" gap={4} px={8} py={4} w="fill">{T(label, { c: 'text/sekundaer' })}{feld}{hint && T(hint, { s: 12, c: 'text/sekundaer' })}</Frame>;
    const inspector = [
      InspectorHead({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0000', ident: 'Zelle 3, 27 · Zweiblattdorf' }),
      <Frame name="Felder" flex="col" w="fill" pt={4}>
        {Stapel('Karte', Field('0'), '0 bis 665, leer = keine Karte')}
        {Stapel('Höhe', NumField('0'), '0 bis 255')}
        {Quelle({ world: true, bake: ['38.876', '146'], notice: 'Übernommen.' })}
        {Section({ title: 'Budget', open: false, children: null })}
      </Frame>,
    ];
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt', children: dorf });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const insp = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left', children: inspector });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}><Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>{insp}</Frame>;
    return Window({ name: NAMES.quelle, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 },
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf, Karte 0000 (3, 27)' }),
      children: [WeltListe({ orte, szene }), <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Zweiblattdorf'], ['Karte 0000', 'Zelle 3, 27']], depth: 'Bauen', light: { e: 0, w: 101, i: 162 }, undo: 1 })}{dock}</Frame>] });
  }

  // ----- 'konflikt': Technik › Textprojekt mit dem Konflikt einer Welt-Karte -----
  const technikListe = ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
    {[
      ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
      ['Textprojekt', 'ph:file-code', '1 Konflikt', true],
      ['Prüfen und Bauen', 'ph:hammer', 'Noch nicht gebaut'],
      ['Veröffentlichen', 'ph:upload-simple', 'Bereit zum Veröffentlichen'],
      ['Änderungen', 'ph:git-diff', 'Keine Abweichung'],
      ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
      ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
      ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
    ].map(([title, icon, s, sel]) => ListRow({ title, icon, sub: s, sel }))}
  </Frame>]);
  const BW = 880;
  const TW = BW - 16 - 8 - 24;
  const CW = Math.floor(TW / 3);
  const Zelle = (s, mono, kopf) => <Frame flex="row" items="center" px={8} w={CW} h="fill">{kopf ? T(s, { s: 12, w: 600, c: 'text/sekundaer' }) : mono ? T(s, { f: M, s: 13 }) : T(s)}</Frame>;
  const Tabelle = (titel, kopf, zeilen) => <Frame name={titel} flex="col" w={TW}>
    <Frame name="Kopfzeile" flex="row" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>{kopf.map((k) => Zelle(k, false, true))}</Frame>
    {zeilen.map((z) => <Frame name={z[0]} flex="row" w="fill" h={DI.zeile} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{z.map((c) => Zelle(c, true))}</Frame>)}
  </Frame>;
  const ZeilenKopf = (label, plakette, pfeil) => <Frame name={label} flex="row" items="center" gap={8} p={4} w="fill">
    {pfeil ? Btn({ label, art: 'Leise', small: true, icon: pfeil }) : T(label)}
    {plakette}
  </Frame>;
  const zeilen = [0, 1, 2, 3, 4].map((i) => ['permissions[' + (71 + i) + '].collision', '0', '128']);
  const konflikt = <Frame name="Zeilen" flex="col" gap={4} px={8} pt={4} pb={8} w="fill">
    <Frame name="Konflikt aufgeklappt" flex="col" gap={4} w="fill">
      {ZeilenKopf('Karte 0000', Badge('Konflikt', 'warn', 'ph:warning'), 'ph:caret-down')}
      <Frame name="Abweichungen" flex="col" gap={8} pt={4} pb={8} pl={24} w="fill">
        {T('Diese Welt-Karte wurde außerhalb des Studios geändert.', { s: 12, c: 'text/sekundaer', width: TW })}
        {Tabelle('Abweichungen: Karte 0000', ['Feld', 'Studio', 'DSPRE'], zeilen)}
        {T('und 566 weitere Abweichungen', { s: 12, c: 'text/sekundaer', width: TW })}
        <Frame name="Wahl" flex="row" items="center" gap={8}>{Btn({ label: 'Neu backen', small: true })}{Btn({ label: 'Als binäre Karte übernehmen', small: true })}</Frame>
      </Frame>
    </Frame>
  </Frame>;
  const Abschnitt = (title, count, children) => <Frame name={'Abschnitt ' + title} flex="col" w="fill">{Section({ title, count, children })}</Frame>;
  const gruppen = <Frame name="Abschnitte" flex="col" w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} overflow="hidden">
    {Abschnitt('Auf zwei Seiten geändert', 1, konflikt)}
    {Abschnitt('Zuletzt übernommen', 0, <Frame flex="row" px={12} py={8} w="fill">{T('Nichts von außen geändert.', { c: 'text/sekundaer' })}</Frame>)}
  </Frame>;
  const kopf = <Frame name="Kopf" flex="row" items="start" justify="between" gap={16} w={BW}>
    <Frame flex="col" gap={4} grow={1}>
      {T('Textprojekt', { f: Z, s: 24, lh: 30, w: 700 })}
      {T('Textprojekt, Schema-Version 4. Zuletzt abgeglichen gerade eben.', { width: 560 })}
    </Frame>
    <Frame name="Tasten" flex="row" items="center" gap={8}>{Btn({ label: 'Jetzt abgleichen' })}{Btn({ label: 'Zurücknehmen…' })}</Frame>
  </Frame>;
  const flaeche = <Frame name="Fläche: Textprojekt" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>{kopf}{gruppen}</Frame>;
  const Meldung = (text, taste) => <Frame name="Meldung" position="absolute" x={1440 - 16 - 360} y={900 - 28 - 12 - 76} w={360} flex="row" rounded={6} bg={v('status/warnung')} pl={4} shadow={SCHATTEN}>
    <Frame flex="row" items="start" gap={8} py={8} pl={12} pr={8} grow={1} rounded={5} bg={v('flaeche/blatt')}>
      {I('ph:warning-fill', 18, 'w')}
      <Frame flex="col" gap={8} grow={1}>{T(text, { width: 240 })}<Frame flex="row">{Btn({ label: taste, small: true })}</Frame></Frame>
      <Frame w={DI.steuer} h={DI.steuer} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    </Frame>
  </Frame>;
  return <Frame name={NAMES.konflikt} w={1440} h={900} rounded={10} overflow="hidden">
    {Window({ name: NAMES.konflikt, active: 'technik', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 0 },
      status: Statusbar({ build: 'Noch nicht gebaut' }), children: [technikListe, flaeche] })}
    {Meldung('1 Konflikt zwischen Studio und DSPRE', 'Lösen…')}
  </Frame>;
