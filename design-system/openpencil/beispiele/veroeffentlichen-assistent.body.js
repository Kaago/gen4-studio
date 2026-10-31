  // Veröffentlichen (Phase 4.5), vier Frames aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'version'  „App: Assistent, Veröffentlichen, Version“: die Welt in der Schau, die Schrittleiste im Scharnier („Version, Patch, Prüfen,
  //              Fertig“, Schritt 1), in der Hand der Schritt Version: links Version 0.1.1 und zwei Zeilen Änderungsnotizen, rechts der
  //              Speicherort mit „Ordner wählen…“ und der Dateiname; darunter der Satz „Schreibt … und die Begleitdatei nach …“
  //   'pruefen'  „App: Assistent, Veröffentlichen, Prüfen“: nach dem Lauf, Schritt 3: Erfolgsmeldung der Selbstprüfung, „Was du weitergibst“
  //              mit drei Dateien, der Hinweis ohne ROM-Inhalt; „Schließen“ statt „Abbrechen“, kein „Zurück“ (die Veröffentlichung ist
  //              geschrieben), die Taste heißt „Fertig“
  //   'liste'    „App: Technik, Veröffentlichen mit Liste“: Karte Basis-ROM (gefunden) und Karte „Veröffentlichen“ mit der Taste und den zwei
  //              bisherigen Veröffentlichungen, die neueste zuerst; Statusleiste mit „main · keine Änderung“
  //   'sichern'  „App: Dialog, Stand sichern“: der Dialog „Stand sichern?“ über derselben Fläche mit der Voreinstellung aus dem Assistenten
  //              (Betreff „Veröffentlicht v0.1.1“, Notiz), Prüfliste mit drei Zeilen, „Abbrechen“ und „Sichern“ (primär)
  // Die Titelleiste hat in jedem Frame die Taste „Veröffentlichen“ (Symbol ph:upload-simple) zwischen „Bauen“ und „Thema wechseln“; die
  // Titelleiste der Bibliothek kennt nur vier Tasten, darum steht sie hier als eigener Baustein (Titelleiste, Fenster), die Bibliothek bleibt
  // unberührt. Quellen: apps/studio/src/publish/{PublishWizard.tsx,PublishWizard.module.css,publishWords.ts}, git/CommitDialog.tsx und
  // .module.css, git/gitWords.ts, technik/PublishSurface.tsx und .module.css, ui/frame/TitleBar.tsx, ui/primitives/{Table,TextField,
  // CheckList,Dialog}, workbench/wizards/{WizardFrame,WizardBar,WizardStep.module.css}. Texte aus design-system/README.md, „Nachtrag 4.5“.
  // Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“. Beispielwerte, nicht aus dem ROM: die Version, die Notizen,
  // die Größen, die Daten, die Git-Version, der Pfad. Vergleich mit den Screenshots unter target/agent/gate-<datum>/4-5/ (Namen
  // blick-<fläche>-<zustand>-<thema>.png, siehe das README); sie zeigen echte Werte des Testprojekts, verglichen werden Aufbau, Maße und
  // Texte.
  // Dunkel: bash ../build-screen.sh veroeffentlichen-assistent --dunkel.
  const MODE = 'version';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { version: 'App: Assistent, Veröffentlichen, Version', pruefen: 'App: Assistent, Veröffentlichen, Prüfen', liste: 'App: Technik, Veröffentlichen mit Liste', sichern: 'App: Dialog, Stand sichern' };
  const ORDNER = '/Users/du/Spiele/Sinnoh-Remix_DSPRE_contents/build';
  const DATEI = 'Sinnoh-Remix-v0.1.1.xdelta';
  const SCHRITTE = ['Version', 'Patch', 'Prüfen', 'Fertig'];

  // ----- Titelleiste wie Titlebar aus jsx-lib-app.js, rechts fünf Symboltasten: Speichern, Bauen, Veröffentlichen, Thema, Hilfe -----
  const Titelleiste = ({ project = 'Sinnoh-Remix', game = 'Platin, DE' } = {}) => (
    <Frame name="Titelleiste" flex="row" items="center" gap={16} pl={12} pr={12} h={40} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
      <Frame flex="row" gap={8} items="center" w={54}>{['#FF5F57', '#FEBC2E', '#28C840'].map((c) => <Ellipse w={12} h={12} fill={c} />)}</Frame>
      <Frame flex="row" items="center" gap={12} w={560}>{IconBtn('ph:folder-open', { c: DARK ? '#E7EFF7' : 'p' })}{T(project, { s: 15, w: 600 })}<Frame flex="row" items="center" px={8} h={18} rounded={9} bg={v('flaeche/deck')}>{T(game, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame><Frame flex="row" items="center" gap={4}>{I('ph:check', 12, 'ok')}{T('Alles gespeichert', { s: 12, c: 'text/sekundaer' })}</Frame></Frame>
      <Frame flex="row" items="center" gap={8} px={8} h={28} w={360} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{I('ph:magnifying-glass', 18)}{T('Suchen oder Befehl eingeben', { c: 'text/sekundaer', grow: 1 })}{Kbd('⌘K')}</Frame>
      <Frame flex="row" items="center" justify="end" gap={8} grow={1}>{['ph:floppy-disk', 'ph:hammer', 'ph:upload-simple', DARK ? 'ph:sun' : 'ph:moon', 'ph:question'].map((i) => IconBtn(i, { c: DARK ? '#E7EFF7' : 'p' }))}</Frame>
    </Frame>
  );
  // Fenster 1440 × 900 wie Window aus jsx-lib-app.js, mit der Titelleiste oben
  const Fenster = ({ name, active, children, status }) => (
    <Frame name={name} w={1440} h={900} flex="col" bg={v('flaeche/gehaeuse')} overflow="hidden" rounded={10} stroke={v('linie/standard')} strokeWidth={1}>
      {Titelleiste()}
      <Frame flex="row" w="fill" grow={1}>{Atlas(active, DARK)}<Frame flex="row" grow={1} h="fill">{children}</Frame></Frame>
      {status}
    </Frame>
  );

  // ----- Schrittleiste im Scharnier wie WizardBar aus jsx-lib-app.js, aber mit den vier Pillen (der Ergebnis-Schritt „Fertig“ ist eine
  // eigene Pille) und den Zuständen des Assistenten: `submitAt` = „Prüfen“ (dort heißt die Taste „Fertig“), `closed` = die Veröffentlichung ist
  // geschrieben: „Schließen“ statt „Abbrechen“, kein „Zurück“ -----
  const Schrittleiste = ({ step, closed }) => {
    const L = '#E7EFF7', LL = '#C6CEDE';
    const last = step === 2;
    return (
      <Frame name="Scharnier (Assistent)" flex="row" items="center" gap={16} pl={4} pr={12} h={44} w="fill" bg={v('scharnier/grund')} strokeWeights={{ top: 2, right: 0, bottom: 2, left: 0 }} stroke={v('bereich/technik-punkt')}>
        <Frame w={24} h={32} flex="row" items="center" justify="center"><Icon name="ph:dots-six-vertical" size={18} color={LL} /></Frame>
        <Frame flex="row" items="center" gap={8}><Frame w={26} h={26} rounded={6} bg={v('bereich/technik-flaeche')} flex="row" items="center" justify="center">{I('ph:upload-simple-fill', 16, 'technik')}</Frame>{T('Veröffentlichen', { f: Z, w: 700, s: 15, c: 'scharnier/text' })}</Frame>
        <Frame flex="row" items="center" gap={4} grow={1} justify="center">{SCHRITTE.map((n, i) => { const s = i < step ? 'fertig' : i === step ? 'aktiv' : 'offen'; return <Frame flex="row" items="center" gap={4}>{i > 0 && <Frame w={16} h={2} rounded={1} bg={v(s === 'offen' ? 'scharnier/kante' : 'scharnier/leise')} />}<Frame flex="row" items="center" gap={6} pl={4} pr={8} h={28} rounded={14} bg={s === 'aktiv' ? v('scharnier/text') : undefined}><Frame w={20} h={20} rounded={10} flex="row" items="center" justify="center" bg={s === 'aktiv' ? v('scharnier/grund') : undefined} stroke={s === 'aktiv' ? undefined : v('scharnier/leise')} strokeWidth={1}>{s === 'fertig' ? <Icon name="ph:check-bold" size={12} color={L} /> : T(String(i + 1), { s: 12, w: 700, c: s === 'aktiv' ? 'scharnier/text' : 'scharnier/leise' })}</Frame>{T(n, { w: s === 'aktiv' ? 600 : 400, c: s === 'aktiv' ? 'scharnier/grund' : 'scharnier/leise' })}</Frame></Frame>; })}</Frame>
        <Frame flex="row" items="center" gap={8}>
          <Frame flex="row" items="center" px={12} h={32}>{T(closed ? 'Schließen' : 'Abbrechen', { w: 600, c: 'scharnier/text' })}</Frame>
          {step > 0 && !closed && <Frame flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('scharnier/taste')} stroke={v('scharnier/kante')} strokeWeights={{ top: 1, right: 1, bottom: 2, left: 1 }}>{T('Zurück', { w: 600, c: 'scharnier/text' })}</Frame>}
          {step > 0 && !closed && Kbd('⌘[', true)}
          <Frame flex="row" items="center" gap={6} px={12} h={32} rounded={6} bg={v('akzent/grund')}>{T(last ? 'Fertig' : 'Weiter', { w: 600, c: 'akzent/auf' })}</Frame>{Kbd('↵', true)}
        </Frame>
      </Frame>
    );
  };

  // ----- Die Welt in der Schau, wie in welt-leeren.body.js (der Assistent öffnet sich über der Ansicht, die bleibt benutzbar) -----
  const place = (n, c, cnt, o = {}) => ListRow({ title: n, caret: o.open ? 'open' : 'closed', icon: o.icon || 'ph:map-trifold', depth: o.d || 0, sel: o.sel, tech: o.tech, right: undefined, mark: cnt !== undefined ? <Frame flex="row" items="center" gap={6}><Frame w={8} h={8} rounded={1} bg={c} />{T(String(cnt), { s: 12, f: M, c: 'text/sekundaer' })}</Frame> : undefined });
  const weltListe = ListPane([
    PanelHead({ title: 'Orte', sub: 'Platin, DE', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:plus')}</Frame> }),
    <Frame flex="col" gap={8} px={8} pt={8} pb={4} w="fill">{T('125 Orte, 593 Header', { s: 12, c: 'text/sekundaer' })}{Search('Ort, Raum oder Kürzel suchen')}</Frame>,
    <Frame flex="col" px={4} w="fill">
      {place('Route 216', '#E7A5C6', 2)}{place('Route 217', '#E7A5C6', 3)}{place('Route 218', '#E7A5C6', 3)}{place('Route 219', '#E7A5C6', 1)}{place('Route 221', '#E7A5C6', 2)}{place('Route 222', '#E7A5C6', 4)}
      {place('Zweiblattdorf', '#DEE79C', 7, { open: true })}
      {['T01R0101', 'T01R0102', 'T01R0201', 'T01R0202', 'T01R0301', 'T01R0401'].map((r) => ListRow({ title: r, tech: '1 Karte', caret: 'closed', icon: 'ph:house', depth: 1 }))}
      {place('Sandgemme', '#E7A5C6', 9)}{place('Flori', '#A5DEA5', 7)}{place('Blumenladen', '#E7A5C6', 1)}{place('Trostu', '#A5DEC6', 9)}
    </Frame>,
  ]);
  const tiles = []; const pal = ['#7B9C31', '#94AD42', '#C68C52', '#6B8C29', '#A5BD5A', '#B57B4A', '#5A7B21', '#8CAD39'];
  for (let r = 0; r < 7; r++) for (let c = 0; c < 9; c++) { if ((r === 0 && c > 6) || (r === 6 && c < 2) || (r === 2 && c === 4) || (r === 3 && c === 4)) continue; tiles.push(<Frame position="absolute" x={250 + c * 58 - r * 14} y={95 + r * 46} w={56} h={44} bg={pal[(r * 3 + c * 5) % 8]} />); }
  const schau = [
    DockTabs(['3D', 'Raster', 'Weltkarte'], '3D'),
    <Frame name="3D-Szene" w="fill" grow={1} bg={v('flaeche/welt')}>
      {tiles}
      <Frame position="absolute" x={612} y={150} w={60} h={40} bg="#E7A57B" /><Frame position="absolute" x={690} y={140} w={46} h={60} bg="#E7A57B" /><Frame position="absolute" x={420} y={240} w={30} h={24} bg="#3173C6" />
    </Frame>,
  ];

  // ----- Bausteine der Hand (WizardStep.module.css und PublishWizard.module.css): Spalten mit 16 / 12 innen, Abstand 8, die Linie zwischen den
  // Spalten linie/leise; Beschriftung text/s 600; Felder wie TextField (Beschriftung, Feld 32 hoch, Radius 4, darunter Hinweis text/xs) -----
  const Spalte = (children, o = {}) => <Frame flex="col" gap={8} px={16} py={12} grow={o.wide ? 2 : 1} h="fill" strokeWeights={o.first ? undefined : { top: 0, right: 0, bottom: 0, left: 1 }} stroke={o.first ? undefined : v('linie/leise')}>{children}</Frame>;
  const Feld = ({ label, w, h = 32, lines, hint }) => <Frame flex="col" gap={4} w={w}>
    {T(label, { w: 600 })}
    <Frame flex="col" justify={lines.length > 1 ? 'start' : 'center'} px={8} py={lines.length > 1 ? 6 : 0} w="fill" h={h} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{lines.map((l) => T(l))}</Frame>
    {hint && T(hint, { s: 12, c: 'text/sekundaer', width: w })}
  </Frame>;
  // Fuß: Zusammenfassung (WizardFrame `summary`): Listensymbol, der Satz; ohne Plakette, die Veröffentlichung ist rückgängig zu machen (eine neue Fassung)
  const Fuss = (text) => <Frame name="Fuß: Zusammenfassung" flex="row" items="center" gap={8} px={16} py={8} w="fill" bg={v('flaeche/blatt')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {I('ph:list', 18, 's')}{T(text, { grow: 1 })}
  </Frame>;
  // Meldung wie Notice aus jsx-lib-app.js, aber mit fester Textbreite (Notice gibt dem Text keine Breite, er liefe über die Spalte hinaus)
  const Meldung = ({ art = 'ok', title, text, width }) => {
    const m = { ok: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok'], info: ['status/info-flaeche', 'status/info', 'ph:info-fill', 'info'] }[art];
    return <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[2], 18, m[3])}<Frame flex="col" gap={4}>{title && T(title, { w: 600, c: m[1], width: width - 24 - 18 - 8 })}{text && T(text, { width: width - 24 - 18 - 8 })}</Frame></Frame>;
  };

  // ----- 'version': Schritt 1 -----
  if (MODE === 'version') {
    const links = Spalte([
      Feld({ label: 'Version', w: 320, lines: ['0.1.1'] }),
      Feld({ label: 'Änderungsnotizen', w: 480, h: 72, lines: ['Der Rivale kämpft ab Level 12.', 'Neue Brücke auf Route 216.'], hint: 'Was sich seit der letzten Fassung geändert hat. Steht in der Begleitdatei.' }),
    ], { first: true });
    const rechts = Spalte([
      <Frame name="Speicherort" flex="col" gap={4} w="fill">{T('Speicherort', { w: 600 })}{T(ORDNER, { f: M, s: 13, lh: 20, width: 400 })}</Frame>,
      Btn({ label: 'Ordner wählen…', icon: 'ph:folder-open' }),
      Feld({ label: 'Dateiname', w: 320, lines: [DATEI] }),
    ]);
    const hand = <Frame name="Assistent: Version" flex="col" w="fill" h="fill" bg={v('flaeche/deck')}>
      <Frame flex="row" w="fill" grow={1}>{links}{rechts}</Frame>
      {Fuss('Schreibt ' + DATEI + ' und die Begleitdatei nach ' + ORDNER + '.')}
    </Frame>;
    return Fenster({ name: NAMES.version, active: 'welt',
      status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 1 von 4: Version' }),
      children: [weltListe, Deck({ schau, scharnier: Schrittleiste({ step: 0, closed: false }), hand })] });
  }

  // ----- 'pruefen': Schritt 3 nach dem Lauf (Veröffentlichung geschrieben: geschlossen) -----
  if (MODE === 'pruefen') {
    const COLS = 1440 - 88;
    const c1 = Math.round(COLS / 3) - 32;
    // Tabelle „Was du weitergibst“ (Table): Datei und Größe; Kopf 32 hoch text/xs 600 sekundär, Linie linie/standard; Zeilen dichte/zeile, Linie linie/leise
    const TW = 480;
    const Zeile = (name, groesse) => <Frame name="Zeile" flex="row" items="center" w="fill" h={DI.zeile} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
      <Frame flex="row" px={8} grow={1}>{T(name)}</Frame><Frame flex="row" justify="end" px={8} w={96}>{T(groesse)}</Frame>
    </Frame>;
    const tabelle = <Frame name="Tabelle: Was du weitergibst" flex="col" w={TW}>
      <Frame name="Kopfzeile" flex="row" items="center" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>
        <Frame flex="row" px={8} grow={1}>{T('Datei', { s: 12, w: 600, c: 'text/sekundaer' })}</Frame><Frame flex="row" justify="end" px={8} w={96}>{T('Größe', { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>
      </Frame>
      {Zeile(DATEI, '5,1 KiB')}{Zeile(DATEI + '.txt', '612 Byte')}{Zeile('README.txt', '248 Byte')}
    </Frame>;
    const links = Spalte([Meldung({ art: 'ok', title: 'Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.', text: 'Basis d40f19ac → Ziel 7be2c014, geprüft in 148 ms', width: c1 })], { first: true });
    const rechts = Spalte([
      T('Was du weitergibst', { s: 13, w: 600, c: 'text/sekundaer' }),
      tabelle,
      T('Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.', { c: 'text/sekundaer', width: TW }),
    ], { wide: true });
    const hand = <Frame name="Assistent: Prüfen" flex="col" w="fill" h="fill" bg={v('flaeche/deck')}>
      <Frame flex="row" w="fill" grow={1}>{links}{rechts}</Frame>
    </Frame>;
    return Fenster({ name: NAMES.pruefen, active: 'welt',
      status: Statusbar({ keys: [['↵', 'Weiter'], ['Esc', 'Abbrechen']], info: 'Schritt 3 von 4: Prüfen', build: 'Gebaut gerade eben', built: true }),
      children: [weltListe, Deck({ schau, scharnier: Schrittleiste({ step: 2, closed: true }), hand })] });
  }

  // ----- Liste „Technik“ mit acht Einträgen (seit 4.3b); der Untertitel von „Veröffentlichen“ nennt die letzte Fassung -----
  const technikListe = (aenderungen) => ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
    {[
      ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
      ['Textprojekt', 'ph:file-code', 'DSPRE-Layout'],
      ['Prüfen und Bauen', 'ph:hammer', 'gebaut gerade eben'],
      ['Veröffentlichen', 'ph:upload-simple', 'Veröffentlicht v0.1.1', true],
      ['Änderungen', 'ph:git-diff', aenderungen],
      ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
      ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
      ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
    ].map(([title, icon, s, sel]) => ListRow({ title, icon, sub: s, sel }))}
  </Frame>]);

  // ----- Statusleiste wie in aenderungen-git.body.js: Befunde, Git-Element (Taste „leise“ S, Symbol ph:git-branch, text/xs), Build-Stand, Dichte -----
  const Statusleiste = (git) => <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame grow={1} />
    {T('101 Warnungen, 10 Hinweise', { s: 12, w: 600, c: 'status/warnung' })}
    <Frame name="Git" flex="row" items="center" gap={6} px={8} h={24} rounded={6}>{I('ph:git-branch', 12, 'p')}{T(git, { s: 12 })}</Frame>
    <Frame flex="row" items="center" gap={4}>{I('ph:check-circle-fill', 12, 'ok')}{T('Gebaut gerade eben', { s: 12, c: 'text/sekundaer' })}</Frame>
    {T('Komfort', { s: 12, w: 600 })}
  </Frame>;

  // ----- Fläche Technik › Veröffentlichen (PublishSurface): flaeche/deck, innen 32 / 24, Abstand 16; Kopf und Karten in der Lesebreite 880;
  // Karten flaeche/blatt, Kante linie/standard, radius/fenster, innen 16, Abstand 8 -----
  const BW = 880;
  const Karte = (name, title, badge, children) => <Frame name={name} flex="col" gap={8} p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
    <Frame flex="row" items="center" gap={8}>{T(title, { s: 15, w: 600 })}{badge}</Frame>
    {children}
  </Frame>;
  const Angabe = (label, value, o = {}) => <Frame flex="col" gap={2} grow={o.grow}>{T(label, { s: 12, c: 'text/sekundaer' })}{o.mono ? Mono(value, { c: 'text/primaer', s: 13, one: o.one }) : T(value)}</Frame>;
  const basis = Karte('Karte: Basis-ROM', 'Basis-ROM', Badge('Gefunden', 'ok', 'ph:check'), [
    T('Ein Patch beschreibt den Unterschied zu deiner Basis-ROM. Die ROM bleibt außerhalb des Projekts; Gen4 Studio merkt sich nur, wo sie liegt.', { width: 600 }),
    <Frame flex="row" gap={24} w="fill">
      {Angabe('ROM', 'Platin DE (Testprojekt)')}
      {Angabe('SHA-1', 'd40f19ac', { mono: true })}
      {Angabe('Datei', '/Users/du/Spiele/Sinnoh-Remix.nds', { mono: true, one: true, grow: 1 })}
    </Frame>,
  ]);
  // Tabelle „Bisherige Veröffentlichungen“ (Table, Spalten Version 88, Datum 96, Datei, Größe 88, Notizen, Taste 44; Datei und Notizen teilen den Rest)
  const TB = BW - 32;
  const REST = Math.floor((TB - 88 - 96 - 88 - 44) / 2);
  const Zelle = (w, inhalt, o = {}) => <Frame flex="row" items="center" justify={o.end ? 'end' : 'start'} px={8} w={w} h="fill">{inhalt}</Frame>;
  const Kopf = (s, w, end) => <Frame flex="row" items="center" justify={end ? 'end' : 'start'} px={8} w={w} h="fill">{T(s, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>;
  const Veroeff = ([version, datum, datei, groesse, notiz]) => <Frame name={'Zeile ' + version} flex="row" items="center" w="fill" h={DI.zeile} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    {Zelle(88, T(version))}{Zelle(96, T(datum))}{Zelle(REST, Mono(datei, { c: 'text/primaer', s: 13, one: true }))}{Zelle(88, T(groesse), { end: true })}{Zelle(REST, T(notiz, { one: true }))}
    <Frame flex="row" items="center" justify="center" w={44} h="fill">{I('ph:folder-open', 18, 's')}</Frame>
  </Frame>;
  const tabelle = <Frame name="Tabelle: Bisherige Veröffentlichungen" flex="col" w={TB}>
    <Frame name="Kopfzeile" flex="row" items="center" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>
      {Kopf('Version', 88)}{Kopf('Datum', 96)}{Kopf('Datei', REST)}{Kopf('Größe', 88, true)}{Kopf('Notizen', REST)}<Frame w={44} />
    </Frame>
    {Veroeff(['0.1.1', '31.10.2026', 'Sinnoh-Remix-v0.1.1.xdelta', '5,1 KiB', 'Der Rivale kämpft ab Level 12.'])}
    {Veroeff(['0.1.0', '30.10.2026', 'Sinnoh-Remix-v0.1.0.xdelta', '4,7 KiB', 'Erste Fassung'])}
  </Frame>;
  const veroeffentlichen = Karte('Karte: Veröffentlichen', 'Veröffentlichen', undefined, [
    T('Der Assistent fragt Version und Änderungsnotizen, baut die ROM und schreibt den Patch mit Begleitdatei an den Ort, den du wählst.', { width: 600 }),
    <Frame flex="row" items="center" gap={8} w="fill">{Btn({ label: 'Veröffentlichen…', art: 'Primär' })}</Frame>,
    <Frame flex="col" pt={8}>{T('Bisherige Veröffentlichungen', { w: 600 })}</Frame>,
    tabelle,
  ]);
  const flaeche = <Frame name="Fläche: Veröffentlichen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
    <Frame flex="col" gap={4}>
      {T('Veröffentlichen', { f: Z, s: 24, lh: 30, w: 700 })}
      {T('Ein Patch gibt deine Änderungen weiter, ohne die ROM: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.', { c: 'text/sekundaer', width: 520 })}
    </Frame>
    {basis}{veroeffentlichen}
  </Frame>;

  // ----- 'liste': die Fläche nach zwei Veröffentlichungen, der Stand ist gesichert -----
  if (MODE === 'liste') {
    return Fenster({ name: NAMES.liste, active: 'technik', status: Statusleiste('main · keine Änderung'), children: [technikListe('Keine Änderung seit dem Commit'), flaeche] });
  }

  // ----- 'sichern': „Stand sichern?“ (Dialog, 480 breit, 20 innen, Abstand 12): Titel text/l, Beschreibung sekundär, Prüfliste, Felder, Tasten
  // rechts; die Statusleiste zeigt noch drei Änderungen -----
  const W = 480;
  const IW = W - 40;
  const Pruefzeile = (symbol, farbe, text) => <Frame flex="row" items="start" gap={8} w="fill">
    <Frame pt={2}>{I(symbol, DI.symbol, farbe)}</Frame>{T(text, { grow: 1, width: IW - 26 })}
  </Frame>;
  const Eingabe = ({ label, lines, h = 32, hint }) => <Frame flex="col" gap={4} w="fill">
    {T(label, { w: 600 })}
    <Frame flex="col" justify={lines.length > 1 || h > 32 ? 'start' : 'center'} px={8} py={h > 32 ? 6 : 0} w="fill" h={h} rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{lines.map((l) => T(l))}</Frame>
    {hint && T(hint, { s: 12, c: 'text/sekundaer', width: IW })}
  </Frame>;
  const inhalt = <Frame name="Inhalt" flex="col" gap={12} w="fill">
    <Frame name="Vor dem Sichern" flex="col" gap={4} w="fill">
      {Pruefzeile('ph:check-circle-fill', 'ok', 'Alles gespeichert')}
      {Pruefzeile('ph:check-circle-fill', 'ok', 'Git gefunden: 2.54.0 (Apple Git-157)')}
      {Pruefzeile('ph:check-circle-fill', 'ok', '3 geänderte Dateien seit dem letzten Commit')}
    </Frame>
    {Eingabe({ label: 'Betreff', lines: ['Veröffentlicht v0.1.1'] })}
    {Eingabe({ label: 'Notiz', lines: ['Der Rivale kämpft ab Level 12.', 'Neue Brücke auf Route 216.'], h: 72, hint: 'Optional; steht im Commit unter dem Betreff.' })}
  </Frame>;
  const dialog = <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T('Stand sichern?', { s: 18, w: 600, width: W - 72 })}
    {T('Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.', { c: 'text/sekundaer', width: IW })}
    {inhalt}
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Sichern', art: 'Primär' })}</Frame>
  </Frame>;
  return MitDialog(NAMES.sichern, Fenster({ name: NAMES.sichern, active: 'technik', status: Statusleiste('main · 3 Änderungen'), children: [technikListe('3 Änderungen seit dem Commit'), flaeche] }), dialog);
