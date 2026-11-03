  // Löschen (Phase 3d.4), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'dialog'  „App: Trainer, Löschen mit Verweisen“: Pokémon › Trainer, Liste mit dem Kästchen „Gelöschte zeigen“ (aus), „Teenager Gunther“
  //             gewählt, darüber der Dialog „Trainer 0001 löschen“ im Zustand Verweise: Fehlermeldung „2 Verweise: zuerst umhängen.“, die
  //             Verweise als Zeilen mit Name und Quelle in Mono, „Abbrechen“ und „Löschen“ (primär, gesperrt)
  //   'kopf'    „App: Welt, Inspector mit Plakette gelöscht“: Gebiet Zweiblattdorf mit dem Raum „Haus 1“ als Grabstein: der Baum „Orte“ mit der
  //             Zeile mit der Plakette „gelöscht“ (das Kästchen „Gelöschte zeigen“ im Kopf der Orte zeichnet `WeltListe` nicht, siehe das Brett),
  //             der Inspector-Kopf mit „gelöscht“ und „Ungespeichert“ und dem offenen Menü „Aktionen“ (der Eintrag „Löschen…“ unter dem Zeiger);
  //             das Frame zeigt zwei Augenblicke zugleich
  //   'brett'   „Komponenten (App): Entitätszustand und Dialog Löschen“ (Seite „Komponenten“): die drei Plaketten und ein Kopf mit allen,
  //             „Gelöschte zeigen“ aus und an mit der gelöschten Zeile und ihrem Kontextmenü, die acht Zustände des Dialogs, „Gelöscht“ in
  //             Technik › Änderungen (Zeile mit „Wiederherstellen“, die Plaketten „Gelöscht“ und „Wiederhergestellt“ im Diff)
  // Quellen: apps/studio/src/delete/{DeleteDialog,DeleteContext,ShowDeleted}.tsx und DeleteDialog.module.css, ui/primitives/EntityStateBadge.tsx,
  // workbench/inspector/{InspectorHead,HeaderEditor,MatrixCellEditor}.tsx, original/HeadActions.tsx, data/trainer/{TrainerList,TrainerView}.tsx,
  // data/encounters/{EncounterList,EncounterView}.tsx, workbench/{WorldTree,HeaderList}.tsx, technik/{ChangesSurface,EntityDiff,diffWords}.ts(x),
  // ui/primitives/{Dialog,Checkbox,Badge,Notice,ListRow,TreeRow,Menu}.tsx. Texte aus design-system/README.md, „Nachtrag 3d.4“.
  // Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: die Verweise des Trainers (Figur und Script), der Raum „Haus 1“,
  // die Dateien der freigegebenen Nummer und die Zeilen von Technik › Änderungen.
  // Vergleich mit den Screenshots unter target/agent/3d-4/ (blick-dialog-*, blick-verweise-*, blick-liste-*, blick-aenderungen-*, je
  // {hell,dunkel,kompakt}); sie entstehen im Gate dieser Phase. Die Vorlage folgt dem Code, nicht den Bildern.
  // Dunkel: bash ../build-screen.sh loeschen --dunkel.
  const MODE = 'dialog';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { dialog: 'App: Trainer, Löschen mit Verweisen', kopf: 'App: Welt, Inspector mit Plakette gelöscht', brett: 'Komponenten (App): Entitätszustand und Dialog Löschen' };
  const RESET = 'Auf Original zurücksetzen…';

  // ----- Plakette Entitätszustand (EntityStateBadge): Badge, 16 hoch, mit dem Symbol der Art (nie nur Farbe); geändert Warnung (`warning`), neu Info
  // (`info`), gelöscht Fehler (`error`, Phosphor XCircle); das Wort klein geschrieben; in der App trägt der Umschlag `data-entity-state` -----
  const Zustand = (art) => ({ geaendert: Badge('geändert', 'warn', 'ph:warning'), neu: Badge('neu', 'info', 'ph:info'), geloescht: Badge('gelöscht', 'fehler', 'ph:x-circle') })[art];
  const Ungespeichert = Badge('Ungespeichert', 'ungespeichert', 'ph:circle');

  // ----- Dialog „… löschen“ (Dialog-Primitive, 480 breit): Kreuz, Titel text/l (ohne Fragezeichen, ohne Beschreibung), Inhalt (Abstand 12),
  // Tasten rechts, eine primäre -----
  const W = 480;
  const IW = W - 40;
  const Meldung = (text, art) => {
    const m = { info: ['status/info-flaeche', 'ph:info-fill', 'info'], warn: ['status/warnung-flaeche', 'ph:warning-fill', 'w'], fehler: ['status/fehler-flaeche', 'ph:x-circle-fill', 'f'] }[art];
    return <Frame name="Meldung" flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[1], 18, m[2])}{T(text, { width: IW - 24 - 26 })}</Frame>;
  };
  const Dialog = (title, inhalt, tasten, w = W) => <Frame name="Dialog" flex="col" gap={12} p={20} w={w} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={w - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T(title, { s: 18, w: 600, width: w - 72 })}
    <Frame name="Inhalt" flex="col" gap={12} w="fill">{inhalt}</Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{tasten.map((t) => Btn(t))}</Frame>
  </Frame>;
  // Verweise (ListRow in einer `listbox`, Name „Verweise“): Name, Quelle in Mono (`tech`); ein Klick führt hin und schließt den Dialog
  const Verweise = (zeilen) => <Frame name="Verweise" flex="col" w="fill">{zeilen.map(([title, tech]) => ListRow({ title, tech }))}</Frame>;
  // „Nummer freigeben (nicht rückgängig)“ mit Plakette, darunter was entfernt wird (code/s, sekundär)
  const Freigabe = (an, dateien) => <Frame name="Nummer freigeben" flex="col" gap={4} w="fill">
    <Frame flex="row" items="center" gap={8}>{Check(an, 'Nummer freigeben (nicht rückgängig)')}{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}</Frame>
    {T(dateien, { f: M, s: 13, lh: 20, c: 'text/sekundaer', width: IW })}
  </Frame>;
  const Leise = (text) => T(text, { c: 'text/sekundaer', width: IW });
  const TASTEN_LOESCHEN = (off, label = 'Löschen') => [{ label: 'Abbrechen' }, { label, art: 'Primär', off }];
  const REFS = [['Figur · Route 202, Kachel 166, 813', 'Eventdatei 0329'], ['Script · Teenager Gunther', 'Scriptdatei 0329']];

  // ----- 'dialog': Pokémon › Trainer, darüber „Trainer 0001 löschen“ mit Verweisen -----
  if (MODE === 'dialog') {
    // Liste (TrainerList): Kopf „Trainer“ mit Zahl, das Kästchen „Gelöschte zeigen“ (aus) im Kopf, Gruppen nach Ort, Zeilen mit Name, Nummer in Mono
    // und „Lv. 5“; ein Grabstein trüge die Plakette „gelöscht“ vor dem Level (siehe Brett)
    const liste = ListPane([PanelHead({ title: 'Trainer', sub: '928 Trainer' }), <Frame name="Spalte" flex="col" gap={8} p={8} w="fill">
      {Search('Name oder Nummer suchen')}
      {Check(false, 'Gelöschte zeigen')}
      {ListRow({ title: 'Route 202', caret: 'open', right: '3' })}
      {ListRow({ title: 'Teenager Gunther', tech: '0001', right: 'Lv. 5', depth: 1, sel: true })}
      {ListRow({ title: 'Teenager Logan', tech: '0002', right: 'Lv. 5', depth: 1 })}
      {ListRow({ title: 'Göre Nadine', tech: '0003', right: 'Lv. 5', depth: 1 })}
      {ListRow({ title: 'Route 203', caret: 'closed', right: '5' })}
    </Frame>]);
    // Kopf (TrainerView, HeadActions): Name font/titel-s, Nummer in code/s sekundär, rechts das Menü „Aktionen“ (Löschen…, Auf Original zurücksetzen…)
    const kopf = <Frame name="Kopf" flex="row" items="center" gap={8} w="fill">
      <Frame flex="row" items="end" gap={8} grow={1}>{T('Teenager Gunther', { f: Z, s: 24, lh: 30, w: 700 })}{Mono('0001', { s: 13 })}</Frame>
      <Frame name="Aktionen" flex="row" items="center" justify="center" px={8} h={DI.steuer} rounded={6}>{I('ph:dots-three', DI.symbol, 'p')}</Frame>
    </Frame>;
    const ansicht = <Frame name="Ansicht" flex="col" gap={16} p={16} w="fill" grow={1} bg={v('flaeche/schau')}>
      {kopf}
      <Frame name="Team" flex="col" gap={8} w="fill">
        <Frame flex="row" items="end" gap={8}>{T('Team', { s: 15, w: 600 })}{T('1 von 6', { c: 'text/sekundaer' })}</Frame>
        {MonCard({ name: 'STARALILI', lv: 5, w: 234 })}
      </Frame>
      <Frame name="Steht in der Welt" flex="col" gap={8} w="fill">
        {T('Steht in der Welt', { s: 15, w: 600 })}
        <Frame flex="row">{Ref('welt', 'Route 202, Kachel 166, 813', 'Eventdatei 0329', 'ph:map-trifold')}</Frame>
      </Frame>
    </Frame>;
    const win = Window({ name: NAMES.dialog, active: 'pokemon', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 0 },
      status: Statusbar({ info: 'Teenager Gunther, 0001', build: 'Gebaut vor 3 Minuten', built: true }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Pokémon'], ['Trainer'], ['Teenager Gunther', '0001']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{ansicht}</Frame>] });
    // Zustand Verweise: Fehlermeldung mit der Zahl, darunter die Zeilen; „Löschen“ gesperrt, bis nichts mehr auf den Trainer zeigt
    const dialog = Dialog('Trainer 0001 löschen', [Meldung('2 Verweise: zuerst umhängen.', 'fehler'), Verweise(REFS)], TASTEN_LOESCHEN(true));
    return MitDialog(NAMES.dialog, win, dialog);
  }

  // Inspector-Kopf wie `InspectorHead` aus jsx-lib-app.js, aber mit fertigen Plaketten (die Bibliothek zeichnet `badges` ohne Symbol, die Plakette
  // „gelöscht“ braucht aber ihr Symbol: nie nur Farbe)
  const KopfMitPlakette = ({ word, icon, title, ident, plaketten }) => <Frame name="Inspector-Kopf" flex="row" items="start" gap={8} pt={8} pb={8} pl={DI.innen} pr={8} w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    <Frame flex="col" gap={2} grow={1}>
      <Frame name="Augenbraue" flex="row" items="center" gap={4}>{I(icon, 12, 's')}{T(word, { s: 12, c: 'text/sekundaer' })}</Frame>
      {T(title, { f: Z, s: 24, lh: 30, w: 700, one: true })}
      {ident && Mono(ident, { one: true })}
    </Frame>
    <Frame name="Zustände" flex="row" gap={4}>{plaketten}</Frame>
    <Frame name="Aktionen" flex="row" items="center" justify="center" px={8} h={DI.steuer} rounded={6}>{I('ph:dots-three', DI.symbol, 'p')}</Frame>
  </Frame>;

  // ----- 'kopf': Welt bauen, der Raum „Haus 1“ als Grabstein, Zeile im Baum „Orte“ mit Plakette, Inspector-Kopf mit Plakette -----
  if (MODE === 'kopf') {
    // Baum „Orte“: die Zeile des Raums mit der Plakette „gelöscht“ am Ende (sie steht nur bei eingeschaltetem „Gelöschte zeigen“ da)
    const orte = [
      TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
      TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: true, count: 4 }),
      TreeRowX({ label: 'Haus 1', icon: 'ph:house', depth: 2, sel: true, shown: true, end: Zustand('geloescht') }),
      TreeRowX({ label: 'Haus 2', icon: 'ph:house', depth: 2 }),
      TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, open: false, count: 1 }),
    ];
    const szene = [
      TreeRowX({ label: 'Haus 1', icon: 'ph:house', open: true }),
      TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: false, count: 1 }),
      TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 2 }),
      TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 1 }),
    ];
    const liste = WeltListe({ orte, szene, orteH: 268 });
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: 548, bg: 'flaeche/welt',
      children: Welt3D({ w: 740, h: 548 - DI.reiter, sel: -1, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 })] }) });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    // Kopf des Raums: Plaketten der Zustände vor Ungespeichert (gelöscht zuerst), Menü „Aktionen“ offen: Löschen… unter dem Zeiger
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: true, w: 340, edge: 'left', children: [
      KopfMitPlakette({ word: 'Raum', icon: 'ph:house', title: 'Haus 1', ident: 'Zweiblattdorf · T01R0101', plaketten: [Zustand('geloescht'), Ungespeichert] }),
      Section({ title: 'Karten', open: false }), Section({ title: 'Aussehen', open: false }), Section({ title: 'Texte und Scripts', open: false }),
      MenuX({ x: 340 - 8 - 260, y: 44, w: 260, items: [
        { label: 'Raum hier anbinden…' }, { label: 'Ort umbenennen…' }, { label: 'Löschen…', hi: true }, { label: 'Für diesen Raum abspalten…' }, '-',
        { label: RESET, icon: 'ph:arrow-counter-clockwise' },
      ] }),
    ] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
      {inspector}
    </Frame>;
    return Window({ name: NAMES.kopf, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 },
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf, Haus 1' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Zweiblattdorf'], ['Haus 1', 'T01R0101']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 2 })}{dock}</Frame>] });
  }

  // ----- 'brett': „Komponenten (App): Entitätszustand und Dialog Löschen“ auf der Seite „Komponenten“ -----
  const BW = 1000;
  const Titel = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer' })}{inhalt}</Frame>;
  const Karte = (inhalt, w = BW - 64) => <Frame flex="col" gap={8} p={16} w={w} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  // Die acht Zustände des Dialogs (je 480 breit, zwei Spalten); der Satz der Ablehnung kommt vom Kern, die übrigen aus DeleteDialog.tsx
  const ORIGINAL = 'Original aus der Basis-ROM: der Eintrag bleibt im ROM; „Auf Original zurücksetzen“ holt ihn zurück.';
  const BLEIBT = 'Inhalt und Member bleiben; ⌘Z holt den Eintrag zurück.';
  const Zustaende = [
    ['Laden', Dialog('Trainer 0001 löschen', [T('Prüfe Verweise …', { c: 'text/sekundaer', width: IW })], TASTEN_LOESCHEN(true))],
    ['Abgelehnt: nur „Schließen“, der Grund steht als Fehlermeldung', Dialog('Header 0003 löschen', [Meldung('Original-Header lassen sich nicht löschen: der Spielcode verweist unsichtbar darauf (ARM9, Overlays).', 'fehler')], [{ label: 'Schließen' }])],
    ['Verweise: „Löschen“ gesperrt, jede Zeile führt zur Quelle', Dialog('Trainer 0001 löschen', [Meldung('2 Verweise: zuerst umhängen.', 'fehler'), Verweise(REFS)], TASTEN_LOESCHEN(true))],
    ['Neu, nicht der letzte: Grabstein, die Nummer bleibt belegt (leiser Satz des Kerns)', Dialog('Trainer 0930 löschen', [Meldung(BLEIBT, 'info'), Leise('Nummer 930 bleibt belegt, weil 3 spätere Einträge leben.')], TASTEN_LOESCHEN(false))],
    ['Neu, am Ende: „Nummer freigeben“ angehakt, die Taste nennt es', Dialog('Trainer 0930 löschen', [Meldung(BLEIBT, 'info'), Freigabe(true, 'Entfernt 1 Datei: data/trainers/0930.json')], TASTEN_LOESCHEN(false, 'Löschen und Nummer freigeben'))],
    ['Original: Warnung, die Nummer lässt sich nie freigeben', Dialog('Trainer 0012 löschen', [Meldung(ORIGINAL, 'warn'), Meldung(BLEIBT, 'info'), Leise('Original-Einträge lassen sich nicht freigeben.')], TASTEN_LOESCHEN(false))],
    ['Läuft: Kästchen und beide Tasten gesperrt', Dialog('Trainer 0930 löschen', [Meldung(BLEIBT, 'info'), Freigabe(true, 'Entfernt 1 Datei: data/trainers/0930.json'), T('Lösche …', { c: 'text/sekundaer', width: IW })], [{ label: 'Abbrechen', off: true }, { label: 'Löschen und Nummer freigeben', art: 'Primär', off: true }])],
    ['Gescheitert: der Grund des Kerns unter dem Inhalt, der Dialog bleibt, „Löschen“ geht wieder', Dialog('Trainer 0930 löschen', [Meldung(BLEIBT, 'info'), Meldung('Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).', 'fehler')], TASTEN_LOESCHEN(false))],
  ];
  const paare = [[0, 1], [2, 3], [4, 5], [6, 7]];
  // „Gelöschte zeigen“ (ShowDeleted: Kästchen im Listenkopf, `data-show-deleted`) und die gelöschte Zeile mit Plakette; Kontextmenü mit „Wiederherstellen“
  const Zeile = (title, tech, tomb, sel) => ListRow({ title, tech, sel, mark: tomb ? Zustand('geloescht') : undefined, right: 'Lv. 5' });
  const Liste = (an) => <Frame flex="col" gap={4} w={320}>
    {Check(an, 'Gelöschte zeigen')}
    <Frame flex="col" w="fill">{Zeile('Teenager Gunther', '0001')}{an && Zeile('Teenager Anton', '0930', true, true)}{Zeile('Teenager Logan', '0002')}</Frame>
  </Frame>;
  // Technik › Änderungen: Zeile mit Plakette „Gelöscht“ und „Wiederherstellen“ statt „Auf Original zurücksetzen…“ (neue Entität ohne Original)
  const Aenderung = (label, taste) => ListRow({ title: label, mark: <Frame name="Rechts" flex="row" items="center" gap={6}>{Badge('Gelöscht', 'fehler', 'ph:x-circle')}{Btn({ label: taste, small: true })}</Frame> });
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW} rounded={16} bg={v('flaeche/deck')}>
    {T('Entitätszustand und Dialog Löschen', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Löschen ist ein Grabstein: der Eintrag bleibt in den Dateien, ⌘Z und „Wiederherstellen“ holen ihn zurück. Die Plakette steht im Kopf der Entität, in den Listen und in Technik › Änderungen; keine neuen Tokens.', { c: 'text/sekundaer', width: BW - 64 })}
    {Titel('Plakette: geändert (Warnung), neu (Info), gelöscht (Fehler); im Kopf vor Ungespeichert, Gesperrt und Nicht rückgängig', Karte(
      <Frame flex="col" gap={12}>
        <Frame flex="row" items="center" gap={8}>{Zustand('geaendert')}{Zustand('neu')}{Zustand('geloescht')}</Frame>
        <Frame name="Kopf mit Plaketten" flex="row" items="center" gap={4}>{Zustand('geloescht')}{Ungespeichert}{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}</Frame>
      </Frame>))}
    {Titel('Gelöschte zeigen: aus (Standard, Grabsteine fehlen) und an (Zeile mit Plakette, Kontextmenü „Wiederherstellen“)', Karte(
      <Frame flex="row" items="start" gap={32}>
        {Liste(false)}
        <Frame flex="col" w={320}>{Liste(true)}{ContextMenu({ x: 200, y: 96, w: 200, items: [{ label: 'Wiederherstellen', hi: true }, { label: RESET }] })}</Frame>
      </Frame>))}
    {Titel('Dialog Löschen: acht Zustände (Titel „{Name} löschen“, höchstens eine primäre Taste)', <Frame name="Zustände" flex="col" gap={16}>
      {paare.map(([a, b]) => <Frame flex="row" items="start" gap={16}>
        {[a, b].map((i) => <Frame flex="col" gap={8}>{T(Zustaende[i][0], { s: 12, c: 'text/sekundaer', width: W })}{Zustaende[i][1]}</Frame>)}
      </Frame>)}
    </Frame>)}
    {Titel('Gelöscht in Technik › Änderungen: Plakette „Gelöscht“; neue Entität „Wiederherstellen“, Original „Auf Original zurücksetzen…“; im Diff „Gelöscht“ oder „Wiederhergestellt“ über den Feldern', Karte(
      <Frame flex="col" gap={12}>
        <Frame flex="col" w="fill">{Aenderung('Trainer 0930', 'Wiederherstellen')}{Aenderung('Trainer 0012', RESET)}</Frame>
        <Frame name="Diff-Plaketten" flex="row" items="center" gap={8}>{Badge('Gelöscht', 'fehler', 'ph:x-circle')}{Badge('Wiederhergestellt', 'info', 'ph:info')}</Frame>
      </Frame>))}
  </Frame>;
