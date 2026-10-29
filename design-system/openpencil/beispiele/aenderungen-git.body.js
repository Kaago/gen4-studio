  // Git minimal (Phase 4.4), drei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'commit'      „App: Technik, Änderungen gegen letzten Commit“: Liste „Technik“ mit acht Einträgen, „Änderungen“ gewählt (Untertitel
  //                 „2 Änderungen seit dem Commit“); Fläche mit dem Satz zum letzten Commit, „Neu vergleichen“, dem Umschalter auf „Gegen letzten
  //                 Commit“, links die Gruppen „Textarchive“ (1) und „Trainer“ (1), „Trainer 0001“ gewählt, rechts sein Detail mit „Öffnen“,
  //                 „Auf Original zurücksetzen…“ und der Änderungstabelle Feld / Commit / Projekt (drei Zeilen, eine Seite fehlt); in der
  //                 Statusleiste „main · 2 Änderungen“ mit dem Symbol `git`
  //   'text'        „App: Technik, Änderung einer Nachricht“: dieselbe Fläche, „Textarchiv 0554“ gewählt; das Detail zeigt zwei Nachrichten:
  //                 eine geänderte mit Wort-Spannen (entfernt durchgestrichen, dazugekommen unterstrichen) und eine neue mit Plakette „Neu“
  //   'einrichten'  „App: Dialog, Git einrichten“: Technik › Änderungen auf „Gegen letzten Commit“ ohne Repository (Info, Satz, „Git einrichten…“),
  //                 darüber der Dialog „Git einrichten?“ vor dem Lauf: rechtlicher Hinweis (Warnung, immer), Prüfliste mit vier Zeilen,
  //                 „Abbrechen“ und „Einrichten“ (primär). Im Lauf fehlen Tasten und Kreuz, darunter steht die Phase („Dateien aufnehmen …“)
  //   'brett'       „Komponenten (App): Änderungstabelle und Git“ (Seite „Komponenten“): die Änderungstabelle mit Feldern (eine Seite fehlt,
  //                 Fußzeile), mit deutschem Feldnamen beim Header, mit Nachrichten und Wort-Spannen; das Element der Statusleiste in vier Zuständen
  // Quellen: apps/studio/src/technik/{ChangesSurface,EntityDiff}.tsx und .module.css, technik/{diffWords,useEntityDiff,useUnsaved}.ts,
  // ui/primitives/ChangeTable.tsx und .module.css, ui/frame/{GitStatusItem,StatusLine}.tsx, git/{GitInitDialog.tsx,gitWords.ts},
  // ui/primitives/{Segmented,Section,ListRow,Badge,Notice,CheckList,Dialog}. Texte aus design-system/README.md, „Nachtrag 4.4“.
  // Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“. Beispielwerte, nicht aus dem ROM: Commit 4c1d2e3 „Ausgangsstand“,
  // die Feldwerte des Trainers, die Nachrichten des Textarchivs und die Git-Version.
  // Vergleich mit den Screenshots unter target/agent/gate-2026-10-29/4-4/ (blick-aenderungen-commit-*, blick-klon-commit-text-*,
  // blick-git-init-dialog-*); sie entstanden vor der Korrekturrunde nach dem UI-Test: dort ist „Git einrichten…“ noch so breit wie die Fläche,
  // die drei Spalten der Tabelle sind gleich breit (der Pfad bricht mitten im Wort) und die Prüfliste sagt „Git gefunden: git version …“.
  // Die Vorlage folgt dem Code, nicht diesen drei Stellen.
  // Dunkel: bash ../build-screen.sh aenderungen-git --dunkel.
  const MODE = 'commit';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { commit: 'App: Technik, Änderungen gegen letzten Commit', text: 'App: Technik, Änderung einer Nachricht', einrichten: 'App: Dialog, Git einrichten', brett: 'Komponenten (App): Änderungstabelle und Git' };
  const RESET = 'Auf Original zurücksetzen…';
  const COMMIT = 'Commit 4c1d2e3 „Ausgangsstand“';

  // ----- Element der Statusleiste (GitStatusItem): Taste „leise“ S mit Symbol git, Schrift text/xs; „Git: nicht lesbar“ in status/warnung -----
  const GitItem = (text, unlesbar) => <Frame name="Git" flex="row" items="center" gap={6} px={8} h={24} rounded={6}>
    {I('ph:git-branch', 12, unlesbar ? 'w' : 'p')}{T(text, { s: 12, c: unlesbar ? 'status/warnung' : 'text/primaer' })}
  </Frame>;
  // Statusleiste wie Statusbar aus jsx-lib-app.js, rechts: Befunde, Git (ohne Repository fehlt es), Build-Stand, Dichte (keine Agenten verbunden)
  const Statusleiste = (git) => <Frame name="Statusleiste" flex="row" items="center" gap={16} px={12} h={28} w="fill" bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    <Frame grow={1} />
    {T('101 Warnungen, 10 Hinweise', { s: 12, w: 600, c: 'status/warnung' })}
    {git && GitItem(git)}
    <Frame flex="row" items="center" gap={4}>{I('ph:check-circle-fill', 12, 'ok')}{T('Gebaut vor 3 Minuten', { s: 12, c: 'text/sekundaer' })}</Frame>
    {T('Komfort', { s: 12, w: 600 })}
  </Frame>;

  // ----- Änderungstabelle (ChangeTable): drei Spalten; Kopf 32 hoch, text/xs 600 sekundär, Linie linie/standard; Zeilen 4 / 8 innen, Linie
  // linie/leise, oben ausgerichtet; Feld und Werte in code/s (13/20). Die Feldspalte ist so breit wie ihr Inhalt (höchstens 28 Zeichen und 40 %
  // der Tabelle, ein Pfad bricht nur nach „.“ und vor „[“), die beiden Seiten teilen den Rest. Fehlt eine Seite: „—“ (vorgelesen „fehlt“) -----
  const ZB = 7.8; // Breite eines Zeichens in code/s (Atkinson Hyperlegible Mono 13)
  const Code = (s, o = {}) => T(s, { f: M, s: 13, lh: 20, ...o });
  // Eine Spanne des Wort-Diffs: entfernt auf status/fehler-flaeche und durchgestrichen, dazugekommen auf status/erfolg-flaeche und unterstrichen
  // (nicht nur Farbe), Radius radius/plakette; die Linie ist ein 1-px-Rahmen in text/primaer, weil die Zeichnung keine Textauszeichnung kennt
  const Spanne = ([op, text]) => {
    if (op === 'same') return Code(text);
    const w = Math.round(text.length * ZB);
    return <Frame name={op === 'added' ? 'ins' : 'del'} w={w} h={20} rounded={10} bg={v(op === 'added' ? 'status/erfolg-flaeche' : 'status/fehler-flaeche')}>
      {Code(text)}
      <Frame position="absolute" x={0} y={op === 'added' ? 17 : 10} w={w} h={1} bg={v('text/primaer')} />
    </Frame>;
  };
  // Ein Wert: null = fehlt, ein Text, oder Zeilen aus Spannen (die Zeichnung bricht die Zeilen von Hand, die App im Fluss)
  const Wert = (wert, breite) => wert === null
    ? Code('—', { c: 'text/sekundaer' })
    : typeof wert === 'string'
      ? Code(wert, { width: breite })
      : <Frame flex="col" w={breite}>{wert.map((zeile) => <Frame flex="row" items="center">{zeile.map(Spanne)}</Frame>)}</Frame>;
  const Tabelle = ({ name, spalten, feldW, w, zeilen, mehr }) => {
    const sw = Math.floor((w - feldW) / 2);
    const Kopf = (s, cw) => <Frame flex="row" items="center" px={8} w={cw} h="fill">{T(s, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>;
    return <Frame name={'Änderungstabelle ' + name} flex="col" w={w}>
      <Frame name="Kopfzeile" flex="row" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>
        {Kopf(spalten[0], feldW)}{Kopf(spalten[1], sw)}{Kopf(spalten[2], sw)}
      </Frame>
      {zeilen.map(([feld, alt, neu]) => <Frame name="Zeile" flex="row" items="start" w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
        <Frame flex="row" items="center" gap={6} px={8} py={4} w={feldW}>{typeof feld === 'string' ? Code(feld) : feld}</Frame>
        <Frame flex="row" px={8} py={4} w={sw}>{Wert(alt, sw - 16)}</Frame>
        <Frame flex="row" px={8} py={4} w={sw}>{Wert(neu, sw - 16)}</Frame>
      </Frame>)}
      {mehr && <Frame flex="row" px={8} py={6} w="fill">{T(mehr, { s: 12, c: 'text/sekundaer' })}</Frame>}
    </Frame>;
  };

  // Beispielzeilen: Trainer 0001 gegen den letzten Commit (Feldpfade wie in data/trainers/, das Item fehlte im Commit)
  const TRAINER = [
    ['data.party[0].level', '5', '7'],
    ['data.party[0].species', 'SPECIES_STARLY', 'SPECIES_SHINX'],
    ['data.party[0].item', null, 'ITEM_ORAN_BERRY'],
  ];
  // Textarchiv 0554: Nachricht 3 geändert (Wort-Spannen), Nachricht 7 neu (nur dazugekommen, die alte Seite fehlt)
  const NACHRICHTEN = [
    [<Frame flex="row">{Code('3')}</Frame>,
      [[['same', 'Der Laden hat heute ']], [['removed', 'zu'], ['same', '.']]],
      [[['same', 'Der Laden hat heute ']], [['added', 'bis acht offen'], ['same', '.']]]],
    [<Frame flex="row" items="center" gap={6}>{Code('7')}{Badge('Neu', 'info')}</Frame>,
      null,
      [[['added', 'Komm bald wieder!']]]],
  ];

  // ----- Liste „Technik“: acht Einträge wie seit 4.3b; der Untertitel von „Änderungen“ folgt dem Vergleich -----
  const technikListe = (sub) => ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
    {[
      ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
      ['Textprojekt', 'ph:file-code', 'DSPRE-Layout'],
      ['Prüfen und Bauen', 'ph:hammer', 'gebaut vor 3 Minuten'],
      ['Veröffentlichen', 'ph:upload-simple', 'Bereit für einen Patch'],
      ['Änderungen', 'ph:git-diff', sub, true],
      ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
      ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
      ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
    ].map(([title, icon, s, sel]) => ListRow({ title, icon, sub: s, sel }))}
  </Frame>]);

  // ----- Fläche Technik › Änderungen: flaeche/deck, innen 32 / 24, Abstand 16; Kopf und Umschalter in der Lesebreite 880, Liste und Detail
  // darunter bis 1280 breit (Fenster 1440: 1440 − 88 Atlas − 272 Liste − 48 = 1032), ab 760 nebeneinander, je (1032 − 16) / 2 -----
  const BW = 880;
  const SPLIT = 1440 - 88 - 272 - 48;
  const CW = Math.floor((SPLIT - 16) / 2);
  const Kopf = (lead, taste) => <Frame name="Kopf" flex="row" items="start" justify="between" gap={16} w={BW}>
    <Frame flex="col" gap={4} grow={1}>
      {T('Änderungen', { f: Z, s: 24, lh: 30, w: 700 })}
      {lead && T(lead, { width: 600 })}
    </Frame>
    {taste && Btn({ label: taste })}
  </Frame>;
  // Umschalter (Segmented S, Name „Vergleichen mit“), unter dem Kopf, solange ein Projekt offen ist
  const Umschalter = (aktiv) => <Frame name="Vergleichen mit" flex="row" w={BW}>{SegS(['Gegen Basis', 'Gegen letzten Commit', 'Gegen gespeichert'], aktiv)}</Frame>;

  const fenster = (name, sub, flaeche, git) => Window({ name, active: 'technik', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 0 },
    status: Statusleiste(git), children: [technikListe(sub), flaeche] });

  // ----- 'commit' und 'text': Liste der Einträge seit dem Commit und das Detail des gewählten -----
  if (MODE === 'commit' || MODE === 'text') {
    const TEXT = MODE === 'text';
    // Zeile: Name, rechts die Plakette (gegen den Commit „Geändert“, „Neu“, „Gelöscht“) und die kleine sekundäre Taste zum Zurücksetzen
    const Eintrag = (label, sel) => ListRow({ title: label, sel, mark: <Frame name="Rechts" flex="row" items="center" gap={6}>
      {Badge('Geändert', 'warn', 'ph:warning')}{Btn({ label: RESET, small: true })}
    </Frame> });
    const Gruppe = (title, rows) => <Frame name={'Gruppe ' + title} flex="col" w="fill">
      {Section({ title, count: rows.length, children: <Frame name={title} flex="col" gap={2} px={8} w="fill">{rows}</Frame> })}
    </Frame>;
    const liste = <Frame name="Gruppen" flex="col" w={CW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} overflow="hidden">
      {Gruppe('Textarchive', [Eintrag('Textarchiv 0554', TEXT)])}
      {Gruppe('Trainer', [Eintrag('Trainer 0001', !TEXT)])}
    </Frame>;

    // Detail (EntityDiff): Karte flaeche/blatt, Kante linie/standard, radius/fenster, innen 16, Abstand 12; Kopf mit Titel text/l, darunter
    // „{alte Seite} → {neue Seite}“ text/s sekundär, rechts „Öffnen“ und „Auf Original zurücksetzen…“ (beide sekundär, klein)
    const IW = CW - 32;
    const TASTEN = 62 + 6 + 196;
    const detailKopf = (titel) => <Frame name="Kopf" flex="row" items="start" justify="between" gap={12} w="fill">
      <Frame flex="col" gap={2} w={IW - TASTEN - 12}>
        {T(titel, { s: 18, w: 600 })}
        {T(COMMIT + ' → Projekt (gespeichert)', { c: 'text/sekundaer', width: IW - TASTEN - 12 })}
      </Frame>
      <Frame name="Tasten" flex="row" items="center" gap={6}>{Btn({ label: 'Öffnen', small: true })}{Btn({ label: RESET, small: true })}</Frame>
    </Frame>;
    const tabelle = TEXT
      ? Tabelle({ name: 'Textarchiv 0554', spalten: ['Nachricht', 'Commit', 'Projekt'], feldW: 80, w: IW, zeilen: NACHRICHTEN })
      : Tabelle({ name: 'Trainer 0001', spalten: ['Feld', 'Commit', 'Projekt'], feldW: Math.round('data.party[0].species'.length * ZB) + 16, w: IW, zeilen: TRAINER });
    const detail = <Frame name="Detail" flex="col" gap={12} p={16} w={CW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
      {detailKopf(TEXT ? 'Textarchiv 0554' : 'Trainer 0001')}{tabelle}
    </Frame>;

    const flaeche = <Frame name="Fläche: Änderungen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
      {Kopf('Gegenüber dem letzten Commit 4c1d2e3 „Ausgangsstand“ auf main. Gezählt wird der gespeicherte Stand.', 'Neu vergleichen')}
      {Umschalter('Gegen letzten Commit')}
      <Frame name="Liste und Detail" flex="row" items="start" gap={16} w={SPLIT}>{liste}{detail}</Frame>
    </Frame>;
    return fenster(NAMES[MODE], '2 Änderungen seit dem Commit', flaeche, 'main · 2 Änderungen');
  }

  // ----- 'einrichten': „Git einrichten?“ vor dem Lauf, über „Gegen letzten Commit“ ohne Repository -----
  if (MODE === 'einrichten') {
    const flaeche = <Frame name="Fläche: Änderungen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
      {Kopf()}
      {Umschalter('Gegen letzten Commit')}
      <Frame name="Ohne Repository" flex="col" gap={16} w={BW}>
        {Notice('Dieses Projekt liegt in keinem Git-Repository.', 'info')}
        {T('Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.', { width: 600 })}
        <Frame name="Angebot" flex="row">{Btn({ label: 'Git einrichten…' })}</Frame>
      </Frame>
    </Frame>;

    // Dialog (480 breit): Kreuz, Titel text/l, Beschreibung sekundär, Inhalt (Abstand 12), Tasten rechts; im Lauf ohne Tasten und Kreuz
    const W = 480;
    const IW = W - 40;
    const Pruefzeile = (symbol, farbe, text) => <Frame flex="row" items="start" gap={8} w="fill">
      <Frame pt={2}>{I(symbol, DI.symbol, farbe)}</Frame>{T(text, { grow: 1, width: IW - 26 })}
    </Frame>;
    const inhalt = <Frame name="Inhalt" flex="col" gap={12} w="fill">
      {Notice('Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).', 'warn')}
      <Frame name="Vor dem Einrichten" flex="col" gap={4} w="fill">
        {Pruefzeile('ph:check-circle-fill', 'ok', 'Alles gespeichert')}
        {Pruefzeile('ph:check-circle-fill', 'ok', 'Git gefunden: 2.54.0 (Apple Git-157)')}
        {Pruefzeile('ph:info-fill', 'info', 'Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt')}
        {Pruefzeile('ph:info-fill', 'info', '.gitignore vorhanden: der Block von Gen4 Studio kommt dazu')}
      </Frame>
    </Frame>;
    const dialog = <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
      {T('Git einrichten?', { s: 18, w: 600, width: W - 72 })}
      {T('Legt in diesem Projektordner ein lokales Git-Repository an: .gitignore und .gitattributes, dann einen ersten Commit „Ausgangsstand“. Gen4 Studio richtet kein Remote ein und lädt nichts hoch.', { c: 'text/sekundaer', width: IW })}
      {inhalt}
      <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Einrichten', art: 'Primär' })}</Frame>
    </Frame>;
    return MitDialog(NAMES.einrichten, fenster(NAMES.einrichten, 'Kein Git', flaeche, null), dialog);
  }

  // ----- 'brett': „Komponenten (App): Änderungstabelle und Git“ auf der Seite „Komponenten“ -----
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer' })}{inhalt}</Frame>;
  const Karte = (inhalt) => <Frame flex="col" p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  const TW = BW - 32;
  // Das Element auf einem Stück Statusleiste (flaeche/gehaeuse, 28 hoch), daneben sein Tooltip als Satz
  const Leiste = (text, tip, unlesbar) => <Frame flex="row" items="center" gap={16} w={BW}>
    <Frame flex="row" items="center" px={12} h={28} w={300} bg={v('flaeche/gehaeuse')} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>{GitItem(text, unlesbar)}</Frame>
    {T(tip, { s: 12, c: 'text/sekundaer', width: BW - 300 - 16 })}
  </Frame>;
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Änderungstabelle und Git', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Technik › Änderungen, Detail eines Eintrags: Feld, alte Seite, neue Seite. Die Köpfe der Seiten folgen dem Vergleich (Basis / Projekt, Commit / Projekt, Gespeichert / Ungespeichert). Das Git-Element steht in der Statusleiste nach den Befunden; ein Klick zeigt Technik › Änderungen gegen den letzten Commit.', { c: 'text/sekundaer', width: BW })}
    {Zustand('Felder: JSON-Pfad in Mono, eine Seite fehlt („—“, vorgelesen „fehlt“), Fußzeile für gekappte Zeilen', Karte(
      Tabelle({ name: 'Felder', spalten: ['Feld', 'Commit', 'Projekt'], feldW: Math.round('data.party[0].species'.length * ZB) + 16, w: TW, zeilen: TRAINER, mehr: 'und 3 weitere Felder' })))}
    {Zustand('Header: deutscher Feldname, der Pfad steht im Tooltip', Karte(
      Tabelle({ name: 'Header', spalten: ['Feld', 'Basis', 'Projekt'], feldW: 112, w: TW, zeilen: [['Musik Tag', '1102', '1015'], ['Wetter', '0', '3']] })))}
    {Zustand('Nachrichten: entfernt durchgestrichen auf status/fehler-flaeche, dazugekommen unterstrichen auf status/erfolg-flaeche', Karte(
      Tabelle({ name: 'Nachrichten', spalten: ['Nachricht', 'Commit', 'Projekt'], feldW: 80, w: TW, zeilen: NACHRICHTEN })))}
    {Zustand('Git in der Statusleiste', <Frame flex="col" gap={8}>
      {Leiste('main · 3 Änderungen', 'Git: 3 geänderte Dateien seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.')}
      {Leiste('main · noch kein Commit', 'Git: noch kein Commit.')}
      {Leiste('4c1d2e3 (losgelöst) · keine Änderung', 'Git: 0 geänderte Dateien seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.')}
      {Leiste('Git: nicht lesbar', 'Der Grund, warum der Status nicht lesbar war (Satz des Kerns).', true)}
      {T('Ohne Repository steht kein Element in der Statusleiste.', { c: 'text/sekundaer', width: BW })}
    </Frame>)}
  </Frame>;
