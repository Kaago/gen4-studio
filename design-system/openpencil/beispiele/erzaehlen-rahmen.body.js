  // Erzählen-Rahmen (Phase 3d.2), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'inspector'  „App: Welt, Erzählen, Inspector Figur“: Atlas auf Welt, Scharnier mit den drei Segmenten „Tiefe“ (Erzählen gedrückt, Symbol
  //                `narrate`), über der Schau die Rezept-Leiste (senkrecht links, fünf Tasten, drei gesperrt „Kommt mit 3d.5“), rechts der Inspector einer
  //                Figur in Erzählen: Aussehen (Sprite und „Ändern…“), Bewegung (Wort), Blickrichtung, Trainer; Statusleiste ohne Werkzeugkürzel
  //   'entwurf'    „App: Welt, Erzählen, Entwurf Raum anlegen“: der Assistent „Raum anlegen“ im Scharnier (Schrittleiste Vorlage, Ort, Eingang,
  //                Zusammenfassung, Schritt 4), in der Schau der Entwurf (gestrichelter Zellrahmen und halbtransparenter Marker der Tür), unten die
  //                Zusammenfassung mit den Sätzen des Assistenten und der Plakette „Nicht rückgängig“ (die Sätze des Kerns stehen nur in Tüfteln)
  //   'brett'      „Komponenten (App): Rezept-Taste und Segmente“ (Seite „Komponenten“): die Rezept-Taste in vier Zuständen, in vier Bereichsfarben
  //                und in Größe S, die Segmente „Tiefe“ in drei Zuständen, das Ergebnis „Als Nächstes“ mit den Tasten
  // Quellen: apps/studio/src/ui/frame/{shellState,Hinge,RecipeBar}.tsx und .module.css, ui/primitives/RecipeButton.tsx und .module.css,
  // workbench/recipes.ts, workbench/inspector/{EventEditor,FigureLookField,fields,eventFields,headerSections}, world/scene/draft.ts,
  // workbench/wizards/{NewRoomWizard,WizardResult,DraftNotes,wizardSpecs}. Texte aus design-system/README.md, „Nachtrag 3d.2“.
  // Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: Raum „Haus 5“ (Kopie der Karte 0411), Aussehen 148, Zellen und Kacheln.
  // Der Entwurf nutzt kein neues Token: szene/auswahl/halo gestrichelt bei 50 % (hier `auswahl/halo`, wie Welt3D).
  // Vergleich mit den Screenshots unter target/agent/gate-2026-10-31/3d-2/ (entstehen im Lauf des Gates; Namen siehe Gate-Bericht).
  // Dunkel: bash ../build-screen.sh erzaehlen-rahmen --dunkel.
  const MODE = 'inspector';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { inspector: 'App: Welt, Erzählen, Inspector Figur', entwurf: 'App: Welt, Erzählen, Entwurf Raum anlegen', brett: 'Komponenten (App): Rezept-Taste und Segmente' };

  // ----- Rezept-Taste (RecipeButton): Kachel in der Bereichsfarbe, Name text/s 600; Größe M dichte/werkzeug (40), S dichte/steuer (32);
  // Ruhe: blatt, Kante linie/leise; Hover: deck, Kante linie/standard; Aktiv: auswahl/flaeche, auswahl/kante, Text akzent/text, Symbol gefüllt;
  // Gesperrt: Name und Kachel in text/deaktiviert (Kachel ohne Bereichsfarbe) und ein Schloss dahinter (nie nur Farbe) -----
  const Rezept = ({ label, icon, b = 'welt', zustand = 'ruhe', s = false }) => {
    const gesperrt = zustand === 'gesperrt', aktiv = zustand === 'aktiv', hover = zustand === 'hover';
    const hoehe = s ? DI.steuer : DI.werkzeug;
    const k = s ? hoehe - 4 - 2 : hoehe - 8 - 2;
    return <Frame name={'Rezept ' + label} flex="row" items="center" gap={s ? 6 : 8} pl={s ? 2 : 4} pr={s ? 8 : 12} h={hoehe} rounded={6} bg={v(aktiv ? 'auswahl/flaeche' : hover ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(aktiv ? 'auswahl/kante' : hover ? 'linie/standard' : 'linie/leise')} strokeWidth={1}>
      <Frame name="Kachel" w={k} h={k} rounded={4} flex="row" items="center" justify="center" bg={v(gesperrt ? 'flaeche/deck' : 'bereich/' + b + '-flaeche')}>{I(icon + (aktiv ? '-fill' : ''), DI.symbol, gesperrt ? 'd' : b)}</Frame>
      {T(label, { w: 600, c: gesperrt ? 'text/deaktiviert' : aktiv ? 'akzent/text' : 'text/primaer' })}
      {gesperrt && I('ph:lock', DI.symbol, 'd')}
    </Frame>;
  };
  // Die fünf Tasten der Leiste (workbench/recipes.ts); die letzten drei sind gesperrt, der Grund steht im Tooltip
  const REZEPTE = [
    { label: 'Gebiet anlegen', icon: 'ph:grid-four', b: 'welt' },
    { label: 'Raum anlegen', icon: 'ph:house', b: 'welt' },
    { label: 'Neue Figur', icon: 'ph:person-simple-walk', b: 'welt', zustand: 'gesperrt' },
    { label: 'Tür verbinden', icon: 'ph:door', b: 'welt', zustand: 'gesperrt' },
    { label: 'Neuer Trainer hier', icon: 'ph:paw-print', b: 'pokemon', zustand: 'gesperrt' },
  ];
  // Rezept-Leiste (RecipeBar, role="toolbar", „Rezepte“): dasselbe Blatt, dieselbe Kante und derselbe Schatten wie die Werkzeugleiste, senkrecht
  // links, die Tasten so breit wie die breiteste (hier 168)
  const Leiste = (x, y) => <Frame name="Rezept-Leiste" position="absolute" x={x} y={y} flex="col" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    {REZEPTE.map((r) => <Frame w={168} flex="row">{Rezept(r)}</Frame>)}
  </Frame>;

  // ----- Scharnier oben mit den Segmenten „Tiefe“: Erzählen | Bauen | Tüfteln (Wort und Symbol; gewählt: Fläche scharnier/kante, Innenkante, Symbol gefüllt) -----
  const TIEFEN = [['Erzählen', 'ph:book-open-text'], ['Bauen', 'ph:hammer'], ['Tüfteln', 'ph:wrench']];
  const Tiefe = (aktiv) => <Frame name="Tiefe" flex="row" items="center" gap={2} p={2} h={32} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWidth={1}>
    {TIEFEN.map(([o, i]) => { const on = o === aktiv; return <Frame flex="row" items="center" gap={6} px={12} h="fill" rounded={4} bg={on ? v('scharnier/kante') : undefined} stroke={on ? v('scharnier/leise') : undefined} strokeWidth={1}>{I(on ? i + '-fill' : i, DI.symbol, 'st')}{T(o, { w: 600, c: 'scharnier/text' })}</Frame>; })}
  </Frame>;
  const Hinge = (crumbs, aktiv) => <Frame name="Scharnier" flex="row" items="center" gap={12} pl={12} pr={8} h={DI.scharnier} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('scharnier/kante')}>
    <Frame name="Brotkrume" flex="row" items="center" gap={8} grow={1} overflow="hidden">{crumbs.map(([n, tech], idx) => { const last = idx === crumbs.length - 1; return <Frame flex="row" items="center" gap={8}>{idx > 0 && T('›', { c: 'scharnier/leise' })}{T(n, { w: last ? 600 : 400, c: last ? 'scharnier/text' : 'scharnier/leise' })}{tech && T(tech, { f: M, s: 12, c: 'scharnier/leise', one: last })}</Frame>; })}</Frame>
    {Tiefe(aktiv)}
    {Light({ e: 0, w: 101, i: 10 })}
    <Frame name="Verlauf" flex="row" items="center" gap={6} px={8} h={32}>{I('ph:clock-counter-clockwise', DI.symbol, 'st')}{T('0', { w: 600, c: 'scharnier/text' })}</Frame>
    <Frame name="Wiederholen" w={32} h={32} flex="row" items="center" justify="center">{I('ph:arrow-clockwise', DI.symbol, 'sl')}</Frame>
    <Frame name="Hier spielen" flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('live/grund')}>{I('ph:play-fill', DI.symbol, 'live')}{T('Hier spielen', { w: 700, c: 'live/auf' })}{Kbd('⌘R', true)}</Frame>
    {MaxButton(false)}
  </Frame>;

  // ----- Liste „Welt“ (Orte und „In dieser Szene“), wie werkbank-figur.body.js, gekürzt -----
  const fig = (n, x, z) => TreeRowX({ label: 'Objekt ' + n + ' · ID ' + n, tech: 'x ' + x + ', z ' + z, techW: 44, icon: 'ph:person-simple-walk', depth: 2, sel: n === 2 });
  const liste = ListPane([PanelHead({ title: 'Welt', sub: 'Platin, DE' }), <Frame name="Spalte" flex="col" w="fill" grow={1}>
    <Frame name="Orte" flex="col" w="fill" h={248} overflow="hidden">
      {PanelHead({ title: 'Orte', actions: <Frame flex="row" items="center" gap={8}>{IconBtn('ph:plus')}</Frame> })}
      <Frame flex="col" gap={6} p={8} w="fill">{T('1 Weltkarte, 72 Gebiete, 492 Räume', { s: 12, c: 'text/sekundaer', one: true })}{Search('Gebiet, Raum oder Kürzel suchen')}</Frame>
      <Frame name="Baum „Orte“" flex="col" px={4} w="fill">
        {TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 })}
        {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, shown: true, count: 6 })}
        {TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, count: 0 })}
        {TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 })}
      </Frame>
    </Frame>
    {Teiler('Teiler „Höhe der Orte“')}
    <Frame name="In dieser Szene" flex="col" w="fill" grow={1} overflow="hidden">
      {PanelHead({ title: 'In dieser Szene', actions: <Frame flex="row" items="center" gap={4}>{IconBtn('ph:plus')}</Frame> })}
      <Frame name="Baum „In dieser Szene“" flex="col" px={4} w="fill">
        {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true })}
        {TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: true, count: 8 })}
        {fig(0, 119, 878)}{fig(1, 108, 889)}{fig(2, 105, 875)}{fig(3, 108, 869)}
        {TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: true, count: 4 })}
      </Frame>
    </Frame>
  </Frame>]);

  const SW = 740;

  // ----- 'inspector': Figur in Erzählen -----
  if (MODE === 'inspector') {
    const SH = 548 - DI.reiter;
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Raster', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt',
      children: Welt3D({ w: SW, h: SH, sel: 2, children: [LayerBar(8, 8), Leiste(8, 56), LegendBtn(8, SH - 40)] }) });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    // Felder einer Figur in Erzählen (eventFieldSpecs „object“, from: „erzaehlen“): nicht gezeigt sind x, z, Lokale ID, Sichtlinie, Sichtweite
    const leise = (label) => <Frame flex="row" items="center" px={8} h={24} rounded={6}>{T(label, { w: 600 })}</Frame>;
    const felder = <Frame name="Felder" flex="col" w="fill" pt={2}>
      {Row({ label: 'Aussehen', children: <Frame flex="row" items="center" gap={8} grow={1}>{Sprite('F', 'welt', 32)}{T('Aussehen 148', { grow: 1 })}{leise('Ändern…')}</Frame> })}
      {Row({ label: 'Bewegung', children: Field('Blickt nach unten', { select: true }) })}
      {Row({ label: 'Blickrichtung', children: Field('Unten', { select: true }) })}
      {Row({ label: 'Trainer', children: [Mono('kein Trainer', { c: 'text/primaer', grow: 1 }), leise('Ändern…')] })}
    </Frame>;
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
      children: [InspectorHead({ word: 'Figur', icon: 'ph:person-simple-walk', title: 'Objekt 2 · ID 2', ident: 'Figur 2 · Eventdatei 0390' }), felder] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" h={832 - DI.scharnier}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
      {inspector}
    </Frame>;
    // Statusleiste: in Erzählen lehrt sie kein Werkzeugkürzel (die Buchstaben sind stumm)
    return Window({ name: NAMES.inspector, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ info: 'Zweiblattdorf, Raster 0' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{Hinge([['Zweiblattdorf'], ['Außen', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], 'Erzählen')}{dock}</Frame>] });
  }

  // ----- 'entwurf': „Raum anlegen“ vor dem Anlegen, der Entwurf in der Schau -----
  if (MODE === 'entwurf') {
    const SH = 548 - DI.reiter;
    // Entwurf (world/scene/draft.ts): gestrichelter Rahmen (Strich 1, Lücke 0,5 Kachel) in auswahl/halo bei 50 %, hier um die Zelle des neuen
    // Raums neben dem Gebäude; 1 × 1 Kachelrahmen um die geplante Tür; der Marker der Tür als Platzhalter halbtransparent (kein Sprite vor 3d.5)
    const Zelle = (x, y, w, h) => <Frame name="Entwurf: Zelle" position="absolute" x={x} y={y} w={w} h={h} opacity={0.5} stroke={v('auswahl/halo')} strokeWidth={2} strokeDash={[8, 4]} dashPattern={[8, 4]} />;
    const Kachel = (x, y) => <Frame name="Entwurf: Kachel" position="absolute" x={x} y={y} w={18} h={18} opacity={0.5} stroke={v('auswahl/halo')} strokeWidth={2} strokeDash={[6, 3]} dashPattern={[6, 3]} />;
    const Marker = (x, y) => <Frame name="Entwurf: Marker Tür" position="absolute" x={x} y={y} w={14} h={14} rounded={2} opacity={0.5} bg={v('ereignis/warp')} stroke="#101821" strokeWidth={1} />;
    const szeneBild = Welt3D({ w: SW, h: SH, sel: -1, children: [LayerBar(8, 8), Zelle(430, 120, 140, 110), Kachel(468, 238), Marker(470, 240), LegendBtn(8, SH - 40)] });
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: 548 - 286 + 44, bg: 'flaeche/welt', children: szeneBild });
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
      children: [InspectorHead({ word: 'Gebiet', icon: 'ph:globe', title: 'Zweiblattdorf', ident: 'Zweiblattdorf · T01' }), Section({ title: 'Ort', children: Row({ label: 'Ort', children: Field('Zweiblattdorf') }) })] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" h={832 - DI.scharnier - 286}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}</Frame>
      {inspector}
    </Frame>;
    // Zusammenfassung (Schritt 4): die Zeilen des Assistenten in Erzählen (roomSummary), kein Satz des Kerns; Fuß mit dem ersten Satz und der Plakette
    const spalte = WizardCol([
      Notice('Legt den Raum „Haus 5“ mit einer Kopie der Karte 0411 an, mit Gebäuden.', 'info'),
      Notice('Legt eine Tür zwischen „Haus 5“ und Zweiblattdorf an.', 'info'),
    ], true);
    const hand = WizardHand({ name: 'Assistent: Zusammenfassung', cols: [spalte], foot: 'Legt den Raum „Haus 5“ mit einer Kopie der Karte 0411 an, mit Gebäuden.' });
    return Window({ name: NAMES.entwurf, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ keys: [['↵', 'Anlegen'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 4 von 4: Zusammenfassung' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{WizardBar({ title: 'Raum anlegen', steps: ['Vorlage', 'Ort', 'Eingang', 'Zusammenfassung'], step: 3, last: 'Anlegen', b: 'welt' })}{dock}{hand}</Frame>] });
  }

  // ----- 'brett': „Komponenten (App): Rezept-Taste und Segmente“ auf der Seite „Komponenten“ -----
  const BW = 880;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer' })}{inhalt}</Frame>;
  const Reihe = (children) => <Frame flex="row" items="center" gap={12} w={BW}>{children}</Frame>;
  const Beschr = (text, w = 200) => T(text, { s: 12, c: 'text/sekundaer', width: w });
  const Leist = (aktiv) => <Frame flex="row" items="center" px={12} h={DI.scharnier} w={BW} bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('scharnier/kante')}>
    {Tiefe(aktiv)}
  </Frame>;
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Rezept-Taste und Segmente', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Ein Rezept ist eine Taste mit Symbol und Name, nie nur ein Buchstabe. In Erzählen steht die Rezept-Leiste über der Schau, im Ergebnis eines Assistenten stehen die Tasten unter „Als Nächstes“. Eine gesperrte Taste trägt ein Schloss und sagt im Tooltip, warum.', { c: 'text/sekundaer', width: BW })}
    {Zustand('Rezept-Taste in vier Zuständen: Ruhe, Hover, Aktiv (aria-pressed), Gesperrt (Schloss, Grund „Kommt mit 3d.5“ im Tooltip)', Reihe([
      Rezept({ label: 'Raum anlegen', icon: 'ph:house' }),
      Rezept({ label: 'Raum anlegen', icon: 'ph:house', zustand: 'hover' }),
      Rezept({ label: 'Raum anlegen', icon: 'ph:house', zustand: 'aktiv' }),
      Rezept({ label: 'Neue Figur', icon: 'ph:person-simple-walk', zustand: 'gesperrt' }),
    ]))}
    {Zustand('Die Kachel trägt die Farbe des Bereichs, wie eine Referenz', Reihe([
      Rezept({ label: 'Gebiet anlegen', icon: 'ph:grid-four', b: 'welt' }),
      Rezept({ label: 'Neuer Trainer hier', icon: 'ph:paw-print', b: 'pokemon' }),
      Rezept({ label: 'Neue Geschichte', icon: 'ph:chats-circle', b: 'geschichten' }),
      Rezept({ label: 'Neuer Gegenstand', icon: 'ph:backpack', b: 'gegenstaende' }),
    ]))}
    {Zustand('Segmente „Tiefe“ im Scharnier: Erzählen, Bauen, Tüfteln (gewählt: Fläche und Innenkante, Symbol gefüllt)', <Frame flex="col" gap={8}>
      {Leist('Erzählen')}{Leist('Bauen')}{Leist('Tüfteln')}
      {Beschr('Erzählen gibt es nur im Bereich Welt. Jeder andere Bereich zeigt dort Bauen, das Segment bleibt auf Erzählen stehen.', BW)}
    </Frame>)}
    {Zustand('Ergebnis „Als Nächstes“: die Rezepte als Tasten der Größe S, danach die Zeilen als Text', <Frame flex="col" gap={16} w={BW}>
      <Frame flex="col" gap={8} p={16} w={560} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
        {Notice('Der Raum „Haus 5“ ist angelegt.', 'ok', 'Raum angelegt')}
        {T('Als Nächstes', { w: 600, c: 'text/sekundaer', s: 12 })}
        <Frame name="Nächste Schritte" flex="row" gap={8}>{Rezept({ label: 'Neue Figur hier', icon: 'ph:person-simple-walk', zustand: 'gesperrt', s: true })}</Frame>
      </Frame>
      <Frame flex="col" gap={8} p={16} w={560} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
        {Notice('Die Welt ist leer.', 'ok', 'Welt geleert')}
        {T('Als Nächstes', { w: 600, c: 'text/sekundaer', s: 12 })}
        <Frame name="Nächste Schritte" flex="row" gap={8}>{Rezept({ label: 'Gebiet anlegen', icon: 'ph:grid-four', s: true })}</Frame>
      </Frame>
      {Beschr('Nach „Gebiet anlegen“: „Tür verbinden“ und „Neue Figur hier“ (beide gesperrt, „Kommt mit 3d.5“). Nach „Gebiet erweitern“: „Gelände anpassen“ (gesperrt, „Kommt mit Phase 6“).', BW)}
    </Frame>)}
  </Frame>;
