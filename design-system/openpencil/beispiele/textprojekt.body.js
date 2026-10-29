  // Textprojekt als Quelle (Phase 4.3b), drei Frames aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'technik'     „App: Technik, Textprojekt“: Liste „Technik“ mit acht Einträgen, „Textprojekt“ gewählt (Untertitel „2 Konflikte“); Fläche mit dem
  //                 abgeglichenen Kopf („Textprojekt, Schema-Version 1. Zuletzt abgeglichen vor 2 Minuten.“, „Jetzt abgleichen“, „Zurücknehmen…“),
  //                 darin der Abschnitt „Auf zwei Seiten geändert“ (2): die Zeile „Attacke 0001 · MOVE_POUND“ aufgeklappt mit Feldtabelle
  //                 Feld / Studio / DSPRE (zwei Zeilen) und den zwei gleichrangigen Tasten, darunter die Zeile „Pokémon 0025 · SPECIES_PIKACHU“
  //                 zu, mit der Plakette „Ungespeichert“; der Abschnitt „Zuletzt übernommen“ (3) mit „aus DSPRE“ und „aus Datei“. Unten rechts
  //                 die Meldung „2 Konflikte zwischen Studio und DSPRE“ (Warnung, bleibt stehen) mit „Lösen…“
  //   'einrichten'  „App: Dialog, Als Textprojekt einrichten“: Technik › Textprojekt im DSPRE-Layout (Kopftext, „Als Textprojekt einrichten…“),
  //                 darüber der Dialog im Lauf: Prüfliste (zwei Häkchen, eine Info) und die Fortschrittszeile „Einträge schreiben … 9 von 16 Arten“.
  //                 Im Lauf hat der Dialog weder Tasten noch Kreuz (`dismissible` aus); vor dem Start stehen „Abbrechen“ und „Einrichten“ (primär) da
  //   'umbenennen'  „App: Dialog, ID umbenennen“: Pokémon › Trainer, der Kopf „Teenager Gunther“ mit Nummer, ID-Zeile (Mono, sekundär, Taste
  //                 „ID kopieren“) und dem Menü „Aktionen“; darüber der Dialog „ID umbenennen“ mit Feld „Neue ID“ und dem Plan „7 Verweise in
  //                 4 Dateien werden nachgezogen.“ samt den vier Dateien
  // Quellen: apps/studio/src/technik/{TextProjectSurface,syncWords,TechnikArea}.tsx und TextProjectSurface.module.css, textproject/{AdoptDialog,
  // RenameIdDialog,EntityIdLine}.tsx und .module.css, app/{useExternalSync,useTextProjectDialogs}.ts, App.tsx (die Meldungen), ui/frame/ToastHost.tsx,
  // ui/primitives/{Dialog,CheckList,Section,Table,Badge,Notice}, data/trainer/TrainerView.tsx. Texte aus design-system/README.md, „Nachtrag 4.3b“.
  // Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“. Beispielwerte, nicht aus dem ROM: die Feldwerte der Konflikte, die
  // Namen der Einträge und die vier Dateien des Plans (wie sie der Kern nennt, ist nur der Aufbau „data/ids/…“, „data/trainers/…“).
  // Vergleich mit den Screenshots unter target/agent/gate-2026-10-28/4-3b/ (blick-textprojekt-*, blick-einrichten-*, blick-trainer-kopf-id-*,
  // blick-umbenennen-plan-*); sie entstanden vor den Korrekturen der UI-Test-Runde, darum steht dort in der Spalte „Studio“ noch ohne „(ungespeichert)“.
  // Dunkel: bash ../build-screen.sh textprojekt --dunkel.
  const MODE = 'technik';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { technik: 'App: Technik, Textprojekt', einrichten: 'App: Dialog, Als Textprojekt einrichten', umbenennen: 'App: Dialog, ID umbenennen' };
  const BASIS = 'Platin DE (Testprojekt)';

  // ----- Dialog (Dialog-Primitive, 480 breit): Kreuz (außer im Lauf), Titel text/l, Beschreibung sekundär, Inhalt (Abstand 12), Tasten rechts -----
  const W = 480;
  const IW = W - 40;
  const Dialog = ({ title, text, inhalt, tasten = [], kreuz = true }) => <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    {kreuz && <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>}
    {T(title, { s: 18, w: 600, width: W - 72 })}
    {text && T(text, { c: 'text/sekundaer', width: IW })}
    {inhalt}
    {tasten.length > 0 && <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{tasten.map((t) => Btn(t))}</Frame>}
  </Frame>;

  // ----- Liste „Technik“: acht Einträge, „Textprojekt“ (Symbol Dateicode) an zweiter Stelle, nach „Projekt“ -----
  const technikListe = (sub) => ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
    {[
      ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
      ['Textprojekt', 'ph:file-code', sub, true],
      ['Prüfen und Bauen', 'ph:hammer', 'gebaut vor 3 Minuten'],
      ['Veröffentlichen', 'ph:upload-simple', 'Bereit für einen Patch'],
      ['Änderungen', 'ph:git-diff', '3 Einträge weichen ab'],
      ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
      ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
      ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
    ].map(([title, icon, s, sel]) => ListRow({ title, icon, sub: s, sel }))}
  </Frame>]);

  // ----- Fläche Technik › Textprojekt: Fläche flaeche/deck, innen 32 / 24, Lesebreite 880, Abstand 16; Titel font/titel-s, Satz text/s -----
  const BW = 880;
  const Kopf = (lead, tasten) => <Frame name="Kopf" flex="row" items="start" justify="between" gap={16} w={BW}>
    <Frame flex="col" gap={4} grow={1}>
      {T('Textprojekt', { f: Z, s: 24, lh: 30, w: 700 })}
      {T(lead, { width: 560 })}
    </Frame>
    {tasten && <Frame name="Tasten" flex="row" items="center" gap={8}>{tasten}</Frame>}
  </Frame>;

  // Eine Zeile eines Abschnitts: Name (Taste „leise“ klein mit Pfeil bei Konflikten, sonst Text) und Plakette
  const ZeilenKopf = (label, plakette, pfeil) => <Frame name={label} flex="row" items="center" gap={8} p={4} w="fill">
    {pfeil ? Btn({ label, art: 'Leise', small: true, icon: pfeil }) : T(label)}
    {plakette}
  </Frame>;

  // Feldtabelle des Konflikts (Table, Größe S): drei gleiche Spalten, Kopf text/xs 600 sekundär, Zeilen dichte/zeile, Werte code/s
  const TW = BW - 16 - 8 - 24;
  const CW = Math.floor(TW / 3);
  const Zelle = (s, mono, kopf) => <Frame flex="row" items="center" px={8} w={CW} h="fill">{kopf ? T(s, { s: 12, w: 600, c: 'text/sekundaer' }) : mono ? T(s, { f: M, s: 13 }) : T(s)}</Frame>;
  const Tabelle = (titel, kopf, zeilen) => <Frame name={titel} flex="col" w={TW}>
    <Frame name="Kopfzeile" flex="row" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>{kopf.map((k) => Zelle(k, false, true))}</Frame>
    {zeilen.map((z) => <Frame name={z[0]} flex="row" w="fill" h={DI.zeile} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>{z.map((c) => Zelle(c, true))}</Frame>)}
  </Frame>;

  const konflikte = <Frame name="Zeilen" flex="col" gap={4} px={8} pt={4} pb={8} w="fill">
    <Frame name="Konflikt aufgeklappt" flex="col" gap={4} p={0} w="fill">
      {ZeilenKopf('Attacke 0001 · MOVE_POUND', Badge('Konflikt', 'warn', 'ph:warning'), 'ph:caret-down')}
      <Frame name="Abweichungen" flex="col" gap={8} pt={4} pb={8} pl={24} w="fill">
        {Tabelle('Abweichungen: Attacke 0001 · MOVE_POUND', ['Feld', 'Studio', 'DSPRE'], [['data.power', '40', '35'], ['data.accuracy', '100', '95']])}
        <Frame name="Wahl" flex="row" items="center" gap={8}>{Btn({ label: 'Studio-Fassung behalten', small: true })}{Btn({ label: 'DSPRE-Fassung übernehmen', small: true })}</Frame>
      </Frame>
    </Frame>
    <Frame name="Konflikt zu" flex="col" w="fill">{ZeilenKopf('Pokémon 0025 · SPECIES_PIKACHU', Badge('Ungespeichert', 'ungespeichert', 'ph:circle'), 'ph:caret-right')}</Frame>
  </Frame>;

  const uebernommen = <Frame name="Zeilen" flex="col" gap={4} px={8} pt={4} pb={8} w="fill">
    {[['Attacke 0033 · MOVE_TACKLE', 'aus DSPRE'], ['Pokémon 0026 · SPECIES_RAICHU', 'aus DSPRE'], ['Trainer 0001 · TRAINER_TEENAGER_GUNTHER', 'aus Datei']].map(([l, von]) => ZeilenKopf(l, Badge(von, 'neutral')))}
  </Frame>;

  // Die Karte der Abschnitte: flaeche/blatt, Kante linie/standard, radius/fenster; je Abschnitt Kopf mit Zähler
  const Abschnitt = (title, count, children) => <Frame name={'Abschnitt ' + title} flex="col" w="fill">{Section({ title, count, children })}</Frame>;
  const gruppen = <Frame name="Abschnitte" flex="col" w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} overflow="hidden">
    {Abschnitt('Auf zwei Seiten geändert', 2, konflikte)}
    {Abschnitt('Zuletzt übernommen', 3, uebernommen)}
  </Frame>;

  // Eine Meldung unten rechts (Toast): 360 breit, links 4 px in der Farbe der Art, Symbol, Satz, kleine sekundäre Taste, Kreuz; 16 vom Rand, über der Statusleiste
  const Meldung = (text, taste) => <Frame name="Meldung" position="absolute" x={1440 - 16 - 360} y={900 - 28 - 12 - 76} w={360} flex="row" rounded={6} bg={v('status/warnung')} pl={4} shadow={SCHATTEN}>
    <Frame flex="row" items="start" gap={8} py={8} pl={12} pr={8} grow={1} rounded={5} bg={v('flaeche/blatt')}>
      {I('ph:warning-fill', 18, 'w')}
      <Frame flex="col" gap={8} grow={1}>{T(text, { width: 240 })}<Frame flex="row">{Btn({ label: taste, small: true })}</Frame></Frame>
      <Frame w={DI.steuer} h={DI.steuer} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    </Frame>
  </Frame>;

  const fenster = (name, sub, flaeche, status) => Window({ name, active: 'technik', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 },
    status: status || Statusbar({ build: 'Gebaut vor 3 Minuten', built: true }), children: [technikListe(sub), flaeche] });

  // ----- 'technik' -----
  if (MODE === 'technik') {
    const flaeche = <Frame name="Fläche: Textprojekt" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
      {Kopf('Textprojekt, Schema-Version 1. Zuletzt abgeglichen vor 2 Minuten.', [Btn({ label: 'Jetzt abgleichen' }), Btn({ label: 'Zurücknehmen…' })])}
      {gruppen}
    </Frame>;
    return <Frame name={NAMES.technik} w={1440} h={900} rounded={10} overflow="hidden">
      {fenster(NAMES.technik, '2 Konflikte', flaeche)}
      {Meldung('2 Konflikte zwischen Studio und DSPRE', 'Lösen…')}
    </Frame>;
  }

  // ----- 'einrichten': der Dialog im Lauf über Technik › Textprojekt im DSPRE-Layout -----
  if (MODE === 'einrichten') {
    const flaeche = <Frame name="Fläche: Textprojekt" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
      {Kopf('Das Projekt liegt im DSPRE-Layout. Als Textprojekt bekommt jeder Eintrag eine lesbare Datei unter data/, bereit für Git; DSPRE öffnet den Ordner weiter.')}
      <Frame name="Tasten" flex="row" items="center" gap={8}>{Btn({ label: 'Als Textprojekt einrichten…', art: 'Primär' })}</Frame>
    </Frame>;
    // Prüfliste (CheckList): Symbol und Zeile, nie nur Farbe; Fortschritt text/s sekundär (role status)
    const Pruefzeile = (symbol, farbe, text) => <Frame flex="row" items="start" gap={8} w="fill">
      <Frame pt={2}>{I(symbol, DI.symbol, farbe)}</Frame>{T(text, { grow: 1, width: 'fill' })}
    </Frame>;
    const inhalt = <Frame name="Inhalt" flex="col" gap={12} w="fill">
      <Frame name="Vor dem Einrichten" flex="col" gap={4} w="fill">
        {Pruefzeile('ph:check-circle-fill', 'ok', 'Alles gespeichert')}
        {Pruefzeile('ph:check-circle-fill', 'ok', 'Basis-ROM gefunden: ' + BASIS)}
        {Pruefzeile('ph:info-fill', 'info', 'Scripts werden dabei übernommen')}
      </Frame>
      {T('Einträge schreiben … 9 von 16 Arten', { c: 'text/sekundaer', width: IW })}
    </Frame>;
    const dialog = Dialog({ title: 'Als Textprojekt einrichten?', text: 'Schreibt g4-project.json und rund 6 500 Dateien unter data/. Die Spieldateien bleiben, wie sie sind; zurücknehmen geht in Technik › Textprojekt.', inhalt, kreuz: false });
    return MitDialog(NAMES.einrichten, fenster(NAMES.einrichten, 'DSPRE-Layout', flaeche), dialog);
  }

  // ----- 'umbenennen': Pokémon › Trainer, Kopf mit ID-Zeile, darüber „ID umbenennen“ -----
  const liste = ListPane([PanelHead({ title: 'Trainer', sub: '928 Trainer' }), <Frame name="Spalte" flex="col" gap={8} p={8} w="fill">
    {Search('Name, Klasse oder Nummer')}
    <Frame name="Gruppen" flex="col" gap={2} w="fill">
      {ListRow({ title: 'Route 201', caret: 'closed', right: '3' })}
      {ListRow({ title: 'Route 202', caret: 'open', right: '3' })}
      {ListRow({ title: 'Teenager Gunther', tech: '0001', right: 'Lv. 5', depth: 1, sel: true })}
      {ListRow({ title: 'Teenager Logan', tech: '0002', right: 'Lv. 5', depth: 1 })}
      {ListRow({ title: 'Göre Nadine', tech: '0003', right: 'Lv. 5', depth: 1 })}
      {ListRow({ title: 'Route 203', caret: 'closed', right: '5' })}
    </Frame>
  </Frame>]);
  // Kopf der Trainer-Ansicht (TrainerView): Name font/titel-s, Nummer und ID in code/s sekundär (Grundlinie), „ID kopieren“, rechts das Menü „Aktionen“
  const kopf = <Frame name="Kopf" flex="row" items="center" gap={8} w="fill">
    <Frame flex="row" items="end" gap={8} grow={1}>
      {T('Teenager Gunther', { f: Z, s: 24, lh: 30, w: 700 })}
      {Mono('0001', { s: 13 })}
      {Mono('TRAINER_TEENAGER_GUNTHER', { s: 13 })}
      <Frame name="ID kopieren" w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:copy', DI.symbol, 's')}</Frame>
    </Frame>
    <Frame name="Aktionen" flex="row" items="center" justify="center" px={8} h={DI.steuer} rounded={6}>{I('ph:dots-three', DI.symbol, 'p')}</Frame>
  </Frame>;
  const ansicht = <Frame name="Ansicht" flex="col" gap={16} p={16} grow={1} h="fill" bg={v('flaeche/schau')}>
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
  const win = Window({ name: NAMES.umbenennen, active: 'pokemon', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 0 },
    status: Statusbar({ info: 'Teenager Gunther, 0001', build: 'Gebaut vor 3 Minuten', built: true }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Pokémon'], ['Trainer'], ['Teenager Gunther', '0001']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{ansicht}</Frame>] });

  // Dialog „ID umbenennen“: Feld „Neue ID“ (Beschriftung sichtbar), darunter die Info mit dem Plan (Satz mit geschütztem Leerzeichen zwischen Zahl und Wort)
  // und den Dateien in code/s, höchstens acht; ein Satz der Ablehnung stünde an dieser Stelle als Fehlermeldung, „Umbenennen“ wäre dann gesperrt
  const DATEIEN = ['data/ids/trainers.json', 'data/trainers/0001.json', 'data/events/0329.json', 'data/scripts/0329.json'];
  const plan = <Frame name="Plan" flex="col" gap={4} px={12} py={8} w="fill" rounded={6} bg={v('status/info-flaeche')}>
    <Frame flex="row" items="start" gap={8} w="fill">
      {I('ph:info-fill', 18, 'info')}
      <Frame flex="col" gap={4} grow={1}>
        {T('7 Verweise in 4 Dateien werden nachgezogen.', { width: IW - 24 - 26 })}
        {DATEIEN.map((f) => T(f, { f: M, s: 13, lh: 20, width: IW - 24 - 26 }))}
      </Frame>
    </Frame>
  </Frame>;
  const feld = <Frame name="Neue ID" flex="col" gap={4} w="fill">{T('Neue ID', { s: 12, w: 600 })}{Field('TRAINER_TEENAGER_GUNNAR', { open: true, mono: false })}</Frame>;
  const dialog = Dialog({ title: 'ID umbenennen', inhalt: <Frame name="Inhalt" flex="col" gap={12} w="fill">{feld}{plan}</Frame>, tasten: [{ label: 'Abbrechen' }, { label: 'Umbenennen', art: 'Primär' }] });
  return MitDialog(NAMES.umbenennen, win, dialog);
