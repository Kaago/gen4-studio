  // Figur, Dialog, Tür (Phase 3d.5), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'dialog'  „App: Welt, Erzählen, Dialog-Editor“: im Hintergrund die Welt in Erzählen mit dem Inspector einer Figur (Felder in Erzählen, am Ende
  //             das Feld „Dialog“ mit DS-Vorschau, Zeile und „Dialog bearbeiten…“), darüber, abgedunkelt, das Modal „Dialog von Objekt 2“ (960 px,
  //             `Dialog size="wide"`): links zwei Nachrichten als Karten, die zweite mit der Plakette „Geteilt mit 2 Figuren“ und „Für diese Figur
  //             abspalten“, rechts der DS-Bildschirm der gewählten Nachricht mit Blätterer, unten „Schließen“
  //   'figur'   „App: Welt, Erzählen, Neue Figur“: der Assistent im Scharnier (Schritte Wo und wie, Dialog, Zusammenfassung; Schritt 2), in der Hand
  //             „Was sie sagt“ mit dem Nachrichten-Feld, der Notiz und dem Sichtbarkeits-Flag, rechts die Vorschau (DS-Bildschirm); in der Schau der
  //             Entwurf: der halbtransparente Sprite der Figur auf einem gestrichelten Kachelrahmen
  //   'brett'   „Komponenten (App): Plakette Geteilt, Feld Dialog, Tür-Gegenstück“ (Seite „Komponenten“): die Plakette „Geteilt mit N Figuren“, das
  //             Feld „Dialog“ in fünf Zuständen (lokal, gemeinsam, Trainer, nichts, nicht lesbar), die Zeile „Gegenstück“ in drei Zuständen und das
  //             Ergebnis „Als Nächstes“ nach „Neue Figur“
  // Quellen: apps/studio/src/workbench/inspector/{DialogSection,WarpField,EventEditor}.tsx, workbench/dialog/{DialogEditor,dialogModel} (und .module.css),
  // workbench/wizards/{NewFigureWizard,PlaceField,TextPreview,WizardResult}.tsx, workbench/recipes.ts, world/scene/draft.ts, ui/primitives/Dialog.tsx.
  // Texte aus design-system/README.md, „Nachtrag 3d.5“. Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: die Texte der Figuren
  // („Willkommen in Zweiblattdorf!“ …), Archiv 554 mit den Nachrichten 4 und 5, Aussehen 6, Zellen und Kacheln.
  // Kein neues Token: die Plakette „Geteilt“ ist `Badge(…, 'warn', 'ph:share-network')` (status/warnung), das Symbol `shared` gibt es seit 3f.4.
  // Der Entwurf nutzt wie in 3d.2 szene/auswahl/halo gestrichelt bei 50 % (hier `auswahl/halo`, wie Welt3D).
  // Maßstab der DS-Bildschirme: die App zeigt im Inspector und im Dialog-Editor Maßstab 1 (256 × 192); in der Hand des Assistenten reicht der Platz im
  // Frame nur für 0,75 (die App rollt die Hand). Das Modal des Dialog-Editors ist 960 breit, höchstens Fensterbreite minus 2 × raum/16.
  // Vergleich mit den Screenshots unter target/agent/gate-2026-11-02/3d-5/ (entstehen im UI-Test des Gates; Namen blick-{figur,dialog,dialog-geteilt,
  // tuer,trainer}-{hell,dunkel,kompakt}.png). Dunkel: bash ../build-screen.sh figur-dialog-tuer --dunkel.
  const MODE = 'dialog';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { dialog: 'App: Welt, Erzählen, Dialog-Editor', figur: 'App: Welt, Erzählen, Neue Figur', brett: 'Komponenten (App): Plakette Geteilt, Feld Dialog, Tür-Gegenstück' };

  // ----- Rezept-Taste (RecipeButton) wie in erzaehlen-rahmen.body.js: Kachel in der Bereichsfarbe, Name text/s 600; Größe M dichte/werkzeug, S dichte/steuer;
  // Gesperrt: Name und Kachel in text/deaktiviert und ein Schloss dahinter (nie nur Farbe) -----
  const Rezept = ({ label, icon, b = 'welt', zustand = 'ruhe', s = false }) => {
    const gesperrt = zustand === 'gesperrt', aktiv = zustand === 'aktiv';
    const hoehe = s ? DI.steuer : DI.werkzeug;
    const k = s ? hoehe - 4 - 2 : hoehe - 8 - 2;
    return <Frame name={'Rezept ' + label} flex="row" items="center" gap={s ? 6 : 8} pl={s ? 2 : 4} pr={s ? 8 : 12} h={hoehe} rounded={6} bg={v(aktiv ? 'auswahl/flaeche' : 'flaeche/blatt')} stroke={v(aktiv ? 'auswahl/kante' : 'linie/leise')} strokeWidth={1}>
      <Frame name="Kachel" w={k} h={k} rounded={4} flex="row" items="center" justify="center" bg={v(gesperrt ? 'flaeche/deck' : 'bereich/' + b + '-flaeche')}>{I(icon + (aktiv ? '-fill' : ''), DI.symbol, gesperrt ? 'd' : b)}</Frame>
      {T(label, { w: 600, c: gesperrt ? 'text/deaktiviert' : aktiv ? 'akzent/text' : 'text/primaer' })}
      {gesperrt && I('ph:lock', DI.symbol, 'd')}
    </Frame>;
  };
  // Die fünf Tasten der Leiste (workbench/recipes.ts): seit 3d.5 laufen alle, keine trägt ein Schloss
  const REZEPTE = [
    { label: 'Gebiet anlegen', icon: 'ph:grid-four', b: 'welt' },
    { label: 'Raum anlegen', icon: 'ph:house', b: 'welt' },
    { label: 'Neue Figur', icon: 'ph:person-simple-walk', b: 'welt' },
    { label: 'Tür verbinden', icon: 'ph:door', b: 'welt' },
    { label: 'Neuer Trainer hier', icon: 'ph:paw-print', b: 'pokemon' },
  ];
  const Leiste = (x, y) => <Frame name="Rezept-Leiste" position="absolute" x={x} y={y} flex="col" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    {REZEPTE.map((r) => <Frame w={168} flex="row">{Rezept(r)}</Frame>)}
  </Frame>;

  // ----- Scharnier mit den Segmenten „Tiefe“ (wie erzaehlen-rahmen.body.js) -----
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

  // ----- Liste „Welt“ wie in erzaehlen-rahmen.body.js, gekürzt -----
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
  const leise = (label) => <Frame flex="row" items="center" px={8} h={24} rounded={6}>{T(label, { w: 600 })}</Frame>;
  const KARTE = (children, w, extra = {}) => <Frame flex="col" gap={8} p={16} w={w} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} {...extra}>{children}</Frame>;

  // ----- Feld „Dialog“ (DialogSection): Linie linie/leise oben, raum/8 innen; Überschrift text/xs sekundär, DS-Bildschirm, Zeile text/s sekundär,
  // Taste leise S. zustand: 'lokal' | 'nichts' | 'gemeinsam' | 'trainer' | 'unlesbar' -----
  const DIALOG_ZEILEN = ['Willkommen in Zweiblattdorf!', 'Hier beginnt deine Reise.'];
  const DialogFeld = ({ zustand = 'lokal', w = 340, scale = 1, titel = 'Dialog' }) => {
    const zeile = { lokal: '2 Nachrichten · 3 Seiten', nichts: 'Sagt nichts', gemeinsam: 'Gemeinsames Script 2005: in Bauen bearbeiten.', trainer: 'Kampftexte des Trainers: im Bereich Pokémon.', unlesbar: 'Script nicht lesbar: Zeile 3: Befehl unbekannt' }[zustand];
    return <Frame name={'Feld Dialog: ' + zustand} flex="col" gap={6} p={8} w={w} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
      {T(titel, { s: 12, c: 'text/sekundaer' })}
      {DSScreen({ scale, lines: zustand === 'lokal' ? DIALOG_ZEILEN : [] })}
      {T(zeile, { c: 'text/sekundaer', width: w - 16 })}
      {zustand === 'lokal' && <Frame flex="row" items="center" gap={8}>{leise('Dialog bearbeiten…')}</Frame>}
      {zustand === 'nichts' && <Frame flex="row" items="center" gap={8}><Frame name="Dialog anlegen… (gesperrt)" flex="row" items="center" px={8} h={24} rounded={6}>{T('Dialog anlegen…', { w: 600, c: 'text/deaktiviert' })}</Frame>{T('Kommt mit 3d.7', { s: 12, c: 'text/sekundaer' })}</Frame>}
    </Frame>;
  };

  // ----- Plakette „Geteilt mit N Figuren“: Art Warnung (status/warnung), Symbol shared -----
  const Geteilt = (n) => Badge(n === 1 ? 'Geteilt mit 1 Figur' : 'Geteilt mit ' + n + ' Figuren', 'warn', 'ph:share-network');

  // ----- Zeile „Gegenstück“ der Tür (WarpField): Text und die leise Taste „Tür verbinden…“ -----
  const Gegenstueck = (text) => Row({ label: 'Gegenstück', children: <Frame flex="row" items="center" gap={8} grow={1}>{T(text, { grow: 1 })}{leise('Tür verbinden…')}</Frame> });

  // ----- Nachrichten-Feld der Texte (MessageEditor) als Kasten: Feld linie/standard, mehrzeilig -----
  const Nachricht = (zeilen, w) => <Frame name="Text der Nachricht" flex="col" px={8} py={6} w={w} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{zeilen.map((z) => T(z, { f: M, s: 12, c: 'text/primaer', width: w - 18 }))}</Frame>;
  // Blätterer der DS-Seite: „Vorige Seite“, „Seite 1 von 2“, „Nächste Seite“
  const Blaettern = (seite, von) => <Frame name="Blätterer" flex="row" items="center" gap={8}>
    <Frame w={32} h={24} flex="row" items="center" justify="center" rounded={6}>{T('‹', { w: 600, c: seite === 1 ? 'text/deaktiviert' : 'text/primaer' })}</Frame>
    {T('Seite ' + seite + ' von ' + von, { s: 13, c: 'text/sekundaer', align: 'center', width: 96 })}
    <Frame w={32} h={24} flex="row" items="center" justify="center" rounded={6}>{T('›', { w: 600, c: seite === von ? 'text/deaktiviert' : 'text/primaer' })}</Frame>
  </Frame>;

  // ----- 'dialog': Inspector einer Figur mit dem Feld, darüber das Modal -----
  if (MODE === 'dialog') {
    const SH = 548 - DI.reiter;
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Raster', 'Weltkarte'], active: '3D', focused: false, h: 548, bg: 'flaeche/welt',
      children: Welt3D({ w: SW, h: SH, sel: 2, children: [LayerBar(8, 8), Leiste(8, 56), LegendBtn(8, SH - 40)] }) });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    // Felder einer Figur in Erzählen (wie erzaehlen-rahmen.body.js, 'inspector'), am Ende das Feld „Dialog“
    const felder = <Frame name="Felder" flex="col" w="fill" pt={2}>
      {Row({ label: 'Aussehen', children: <Frame flex="row" items="center" gap={8} grow={1}>{Sprite('F', 'welt', 32)}{T('Aussehen 148', { grow: 1 })}{leise('Ändern…')}</Frame> })}
      {Row({ label: 'Bewegung', children: Field('Steht still', { select: true }) })}
      {Row({ label: 'Blickrichtung', children: Field('Unten', { select: true }) })}
      {Row({ label: 'Trainer', children: [Mono('kein Trainer', { c: 'text/primaer', grow: 1 }), leise('Ändern…')] })}
      {DialogFeld({ zustand: 'lokal' })}
    </Frame>;
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
      children: [InspectorHead({ word: 'Figur', icon: 'ph:person-simple-walk', title: 'Objekt 2 · ID 2', ident: 'Figur 2 · Eventdatei 0390' }), felder] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" h={832 - DI.scharnier}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
      {inspector}
    </Frame>;
    const fenster = Window({ name: NAMES.dialog, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ info: 'Zweiblattdorf, Raster 0' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{Hinge([['Zweiblattdorf'], ['Außen', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], 'Erzählen')}{dock}</Frame>] });

    // Das Modal (DialogEditor, Dialog size="wide"): 960 breit, Titel „Dialog von Objekt 2“, Aktion „Schließen“ (primär); Spalten: Nachrichten flexibel,
    // DS-Seite rechts (256 + Rand). Karte: Kante linie/leise, radius/feld, raum/8; die gewählte mit linie/stark.
    const MW = 960, SP = 256 + 16;
    const LW = MW - 40 - 16 - SP - 18;
    const Karte = ({ kopf, text, geteilt, gewaehlt }) => <Frame name={'Nachricht ' + kopf} flex="col" gap={6} p={8} w={LW} rounded={4} stroke={v(gewaehlt ? 'linie/stark' : 'linie/leise')} strokeWidth={1}>
      <Frame flex="row" items="center" gap={8} w="fill">{leise(kopf)}{geteilt && Geteilt(geteilt)}</Frame>
      {Nachricht(text, LW - 18)}
      {geteilt && <Frame flex="row" items="center" px={12} h={DI.steuer} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} strokeWeights={{ top: 1, right: 1, bottom: 2, left: 1 }}>{T('Für diese Figur abspalten', { w: 600 })}</Frame>}
    </Frame>;
    const inhalt = <Frame name="Spalten" flex="row" gap={16} w="fill">
      <Frame name="Nachrichten" flex="col" gap={8}>
        {Karte({ kopf: 'Nachricht 4 · Archiv 554', text: DIALOG_ZEILEN, gewaehlt: true })}
        {Karte({ kopf: 'Nachricht 5 · Archiv 554', text: ['Pass auf dich auf!'], geteilt: 2 })}
      </Frame>
      <Frame name="DS-Seite" flex="col" items="center" gap={6}>
        {T('Nachricht 4', { s: 12, c: 'text/sekundaer', width: 256 })}
        {DSScreen({ scale: 1, lines: DIALOG_ZEILEN })}
        {Blaettern(1, 2)}
      </Frame>
    </Frame>;
    const modal = <Frame name="Dialog-Editor" flex="col" gap={12} p={20} w={MW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Schließen" position="absolute" x={MW - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
      {T('Dialog von Objekt 2', { s: 18, w: 600, width: MW - 72 })}
      {inhalt}
      <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Schließen', art: 'Primär' })}</Frame>
    </Frame>;
    return MitDialog(NAMES.dialog, fenster, modal);
  }

  // ----- 'figur': „Neue Figur“, Schritt „Dialog“, der Entwurf mit Sprite in der Schau -----
  if (MODE === 'figur') {
    // Dock wie assistent-ort.body.js: unter dem Streifen des Assistenten (286) bleiben 502; Schau und Script/Probleme im Verhältnis 548 : 788
    const DH = 832 - DI.scharnier - 286, SH = Math.round(DH * 548 / 788), SZ = SH - DI.reiter;
    // Entwurf (world/scene/draft.ts): 1 × 1 Kachelrahmen gestrichelt in auswahl/halo bei 50 %, darauf der Sprite des Aussehens halbtransparent (50 %),
    // auf der Kachel stehend. Die Figur blickt im Entwurf immer nach unten (bekannt, zurückgestellt).
    const Kachel = (x, y) => <Frame name="Entwurf: Kachel" position="absolute" x={x} y={y} w={22} h={22} opacity={0.5} stroke={v('auswahl/halo')} strokeWidth={2} strokeDash={[6, 3]} dashPattern={[6, 3]} />;
    const EntwurfFigur = (x, y) => <Frame name="Entwurf: Figur (Sprite)" position="absolute" x={x} y={y} opacity={0.5}>{Sprite('F', 'welt', 28)}</Frame>;
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: SH, bg: 'flaeche/welt',
      children: Welt3D({ w: SW, h: SZ, sel: -1, children: [LayerBar(8, 8), Kachel(420, 150), EntwurfFigur(423, 124), LegendBtn(8, SZ - 40)] }) });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: DH - SH, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
      children: [InspectorHead({ word: 'Gebiet', icon: 'ph:globe', title: 'Zweiblattdorf', ident: 'Zweiblattdorf · T01' }), Section({ title: 'Ort', children: Row({ label: 'Ort', children: Field('Zweiblattdorf') }) })] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" h={DH}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
      {inspector}
    </Frame>;
    // Schritt „Dialog“: links „Was sie sagt“ (Nachrichten-Feld, Notiz, Sichtbarkeits-Flag als Chip der Fläche Geschichten mit „Wählen…“), rechts die Vorschau
    const links = WizardCol([
      T('Was sie sagt', { w: 600 }),
      Nachricht(DIALOG_ZEILEN, 640),
      T('Enter: neue Zeile; Umschalt+Enter: neue Seite (▼).', { s: 12, c: 'text/sekundaer' }),
      Row({ label: 'Sichtbarkeits-Flag', children: <Frame flex="row" items="center" gap={8} grow={1}>{T('Keins', { grow: 1 })}{leise('Wählen…')}</Frame> }),
      T('Solange das Flag gesetzt ist, ist die Figur nicht zu sehen; „Keins“: immer zu sehen.', { s: 12, c: 'text/sekundaer', width: 640 }),
    ], true);
    const rechts = WizardCol([
      T('Vorschau', { w: 600 }),
      <Frame name="Vorschau" flex="col" items="center" gap={6}>{DSScreen({ scale: 0.75, lines: DIALOG_ZEILEN })}{Blaettern(1, 1)}</Frame>,
    ], false);
    // Fuß: die Zeile der Zusammenfassung (ohne die Plakette „Nicht rückgängig“: die Figur lässt sich zurücknehmen)
    const fuss = <Frame name="Fuß: Zusammenfassung" flex="row" items="center" gap={8} px={16} py={8} w="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>{I('ph:list', 18, 's')}{T('Neue Figur in „Zweiblattdorf“, Kachel x 106, z 874', { grow: 1 })}</Frame>;
    const hand = <Frame name="Assistent: Dialog" flex="col" w="fill" h={286} bg={v('flaeche/deck')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
      <Frame flex="row" w="fill" grow={1}>{[links, rechts]}</Frame>
      {fuss}
    </Frame>;
    return Window({ name: NAMES.figur, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 2 von 3: Dialog' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{WizardBar({ title: 'Neue Figur', steps: ['Wo und wie', 'Dialog', 'Zusammenfassung'], step: 1, last: 'Aufstellen', b: 'welt' })}{dock}{hand}</Frame>] });
  }

  // ----- 'brett': „Komponenten (App): Plakette Geteilt, Feld Dialog, Tür-Gegenstück“ auf der Seite „Komponenten“ -----
  const BW = 1100;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer' })}{inhalt}</Frame>;
  const Reihe = (children) => <Frame flex="row" items="start" gap={16} w={BW}>{children}</Frame>;
  const Beschr = (text, w = BW) => T(text, { s: 12, c: 'text/sekundaer', width: w });
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Plakette „Geteilt“, Feld „Dialog“, Tür-Gegenstück', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Eine Nachricht, die auch andere Figuren zeigen, trägt die Plakette „Geteilt mit N Figuren“ und lässt sich für eine Figur abspalten. Das Feld „Dialog“ zeigt, was eine Figur sagt, und sagt bei allem, was der Dialog-Editor nicht bearbeitet, wo es sich bearbeiten lässt. Eine Tür nennt ihr Gegenstück.', { c: 'text/sekundaer', width: BW })}
    {Zustand('Plakette „Geteilt mit N Figuren“ (Art Warnung, Symbol shared): eine Figur, mehrere Figuren', <Frame flex="row" items="center" gap={12}>{Geteilt(1)}{Geteilt(2)}{Geteilt(5)}</Frame>)}
    {Zustand('Feld „Dialog“ in fünf Zuständen: lokal, gemeinsam, Trainer, nichts, nicht lesbar (Inspector, 340 breit; Maßstab 0,5 statt 1 des Inspectors)', Reihe([
      <Frame flex="col" gap={4}>{DialogFeld({ zustand: 'lokal', w: 200, scale: 0.5 })}</Frame>,
      <Frame flex="col" gap={4}>{DialogFeld({ zustand: 'gemeinsam', w: 200, scale: 0.5 })}</Frame>,
      <Frame flex="col" gap={4}>{DialogFeld({ zustand: 'trainer', w: 200, scale: 0.5 })}</Frame>,
      <Frame flex="col" gap={4}>{DialogFeld({ zustand: 'nichts', w: 200, scale: 0.5 })}</Frame>,
      <Frame flex="col" gap={4}>{DialogFeld({ zustand: 'unlesbar', w: 200, scale: 0.5 })}</Frame>,
    ]))}
    {Beschr('Ein Schild nennt das Feld „Text“ und zeigt den DS-Bildschirm in den Farben des Schilds. Nur ein lokales Script öffnet den Dialog-Editor; ohne Script ist „Dialog anlegen…“ gesperrt („Kommt mit 3d.7“).')}
    {Zustand('Zeile „Gegenstück“ im Inspector einer Tür: führt zurück, führt nicht zurück, Ziel hat die Tür nicht', <Frame flex="col" gap={4} w={500}>
      {Gegenstueck('Tür 2 in Zweiblattdorf · führt zurück')}
      {Gegenstueck('Tür 2 in Zweiblattdorf · führt nicht zurück')}
      {Gegenstueck('Ziel hat keine Tür 7')}
    </Frame>)}
    {Zustand('Ergebnis „Als Nächstes“ nach „Neue Figur“: „Dialog bearbeiten“ läuft, „Zu einer Geschichte“ ist gesperrt', KARTE([
      Notice('Objekt 3 in Eventdatei 0390', 'ok', 'Figur aufgestellt'),
      T('Als Nächstes', { w: 600, c: 'text/sekundaer', s: 12 }),
      <Frame name="Nächste Schritte" flex="row" gap={8}>{Rezept({ label: 'Dialog bearbeiten', icon: 'ph:text-aa', b: 'texte', s: true })}{Rezept({ label: 'Zu einer Geschichte', icon: 'ph:chats-circle', b: 'geschichten', zustand: 'gesperrt', s: true })}</Frame>,
      T('Ausprobieren: speichern, bauen und die Figur ansprechen', { c: 'text/sekundaer' }),
    ], 560))}
    {Beschr('Nach „Tür verbinden“ und „Neuer Trainer hier“ steht „Hier spielen“, gesperrt („Kommt mit 3d.9“). Nach „Gebiet anlegen“ laufen „Tür verbinden“ und „Neue Figur hier“.')}
  </Frame>;
