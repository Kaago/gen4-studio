  // Flags und Variablen (Phase 3d.1), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'liste'   „App: Technik, Flags und Variablen“: Liste „Technik“ mit neun Einträgen, „Flags und Variablen“ gewählt (Untertitel „4.097 Flags · 135 frei“);
  //             Fläche mit Titel, Satz, Filterzeile (Art, Bereich, drei Kästchen, Suche), links die Liste in sechs Zeilen (unbenannt und kartenlokal
  //             „an 54 Orten“, eigener Name `FLAG_HAT_MEDAILLE` fett mit dem pret-Namen grau und gewählt, pret-Name, frei, mit unklarem Zugriff, nur gesetzt)
  //             und der Spalte „Live“ (Kopf in `status/warnung`, Zellen „–“), rechts das Detail von `FLAG_HAT_MEDAILLE` mit „Wo gesetzt“ (Scriptzeile)
  //             und „Wo geprüft“ (Figur, Script)
  //   'picker'  „App: Dialog, Flag wählen“: Welt bauen, Figur bearbeiten (Werkbank) im Hintergrund, im Inspector das Feld „Sichtbarkeits-Flag“
  //             auf „Keins“; darüber der Picker „Flag wählen“ mit „Keins (0)“ als erster Zeile (hervorgehoben), Treffern, Bereichsfilter und dem Fuß
  //             „Freies Flag nehmen“, „Im Verzeichnis zeigen“
  //   'brett'   „Komponenten (App): Flag-Picker und Flag-Feld“ (Seite „Komponenten“): das Flag-Feld in Bauen und Tüfteln (Name, unbenannt, Keins, Variable
  //             eines Auslösers, Zelle im Level-Script), die Zeile des Pickers in vier Zuständen, sein Fuß in vier Zuständen, die Zeile der Liste in
  //             sieben Zuständen
  // Quellen: apps/studio/src/flags/{FlagsSurface,FlagList,FlagDetail,FlagPicker,FlagField,flagWords}.tsx/.ts und .module.css, workbench/inspector/
  // {eventFields.ts,EventEditor.tsx,LevelScriptSection.tsx}, technik/TechnikArea.tsx, ui/primitives/{Picker,ListRow,FilterChips,Reference,Section,Badge,Notice}.
  // Texte aus design-system/README.md, „Nachtrag 3d.1“. Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM oder dem Spielstand: der eigene Name
  // `FLAG_HAT_MEDAILLE` und seine Beschreibung, die Zahlen der Verwendung und die Verweise.
  // Nicht gerendert: OpenPencil war beim Schreiben nicht erreichbar. Die Vorlage ist gegen jsx-lib-app.js geschrieben und mit einer Attrappe der
  // Bausteine (Frame, Text, Icon, designVar) durchgelaufen, aber nie als Bild gesehen. Beim ersten Rendern Maße und Umbrüche mit den Screenshots
  // abgleichen (Anleitung: openpencil/NACHTRAG-3D-1.md).
  // Dunkel: bash ../build-screen.sh flags-variablen --dunkel.
  const MODE = 'liste';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { liste: 'App: Technik, Flags und Variablen', picker: 'App: Dialog, Flag wählen', brett: 'Komponenten (App): Flag-Picker und Flag-Feld' };

  // ----- Wörter (flagWords.ts): Nummer als 0x plus vier große Hexziffern; Name = eigener, sonst pret, sonst Art und Nummer -----
  const hex4 = (n) => '0x' + n.toString(16).toUpperCase().padStart(4, '0');
  const numText = (n) => hex4(n) + ' · ' + n;
  const nameOf = (e) => e.own || e.pret || (e.kind === 'var' ? 'Variable ' : 'Flag ') + hex4(e.n);

  // Beispielzeilen der Liste (Flags, nach Nummer): own = eigener Name, pret = Name des Spiels, usage = usageText, places = „an N Orten“ (kartenlokal)
  const ZEILEN = [
    { n: 0x0003, own: null, pret: null, usage: null, places: 54 },
    { n: 0x0094, own: 'FLAG_HAT_MEDAILLE', pret: 'FLAG_UNUSED_0x0094', usage: 'gesetzt 1 · geprüft 2' },
    { n: 0x00C3, own: null, pret: 'FLAG_HIDE_TWINLEAF_MOM', usage: 'gesetzt 1 · geprüft 3' },
    { n: 0x00F0, own: null, pret: 'FLAG_UNUSED_0x00F0', usage: 'unbenutzt', free: true },
    { n: 0x0997, own: null, pret: 'FLAG_VILLA_FURNITURE_TABLE', usage: 'gesetzt 2 · geprüft 1 · unklar 1' },
    { n: 0x0AA0, own: null, pret: null, usage: 'gesetzt 1 · geprüft 0' },
  ];

  // ----- Zeile der Liste (FlagRow, ListRow mit `strong` und `techSubtitle`): Name vorne (eigener fett), darunter in Mono der pret-Name unter einem
  // eigenen und die Nummer in Hex und Dezimal; rechts die Verwendung (xs, sekundär), „reserviert“ (Info, die Vorlage als Tooltip), „frei“ (Erfolg) und die
  // leere Zelle „Live“ (32 breit, „–“, Tooltip „Kommt mit dem Spielmodus (3d.9)“). Gewählt: `auswahl/flaeche`, Kante links 2 px `auswahl/kante` -----
  const Live = () => <Frame name="Live" flex="row" items="center" justify="center" w={32}>{T('–', { s: 12, c: 'text/sekundaer' })}</Frame>;
  const Zeile = (e, o = {}) => {
    const sel = !!o.sel;
    const own = !!e.own;
    const sub = (own && e.pret ? e.pret + ' · ' : '') + numText(e.n);
    const usage = e.places ? 'an ' + e.places + (e.places === 1 ? ' Ort' : ' Orten') : e.usage;
    return <Frame name={nameOf(e)} flex="row" items="center" gap={8} px={12} minH={40} w={o.w || 'fill'} rounded={4} bg={sel ? v('auswahl/flaeche') : undefined} strokeWeights={sel ? { top: 0, right: 0, bottom: 0, left: 2 } : undefined} stroke={sel ? v('auswahl/kante') : undefined}>
      <Frame flex="col" grow={1}>
        {T(nameOf(e), { w: own ? 600 : 400, c: sel ? 'akzent/text' : 'text/primaer', one: true })}
        {Mono(sub, { one: true, c: sel ? 'akzent/text' : 'text/sekundaer' })}
      </Frame>
      <Frame name="Rechts" flex="row" items="center" gap={6}>
        {T(usage, { s: 12, c: 'text/sekundaer' })}
        {e.reserved && Badge('reserviert', 'info')}
        {e.free && Badge('frei', 'ok')}
        {Live()}
      </Frame>
    </Frame>;
  };
  // Kopf der Liste: rechts „Live“ in der Farbe der Warnung (`status/warnung`, 600), 32 breit, die Spalte wartet auf den Spielmodus (3d.9)
  const ListenKopf = () => <Frame name="Kopf" flex="row" justify="end" px={12} w="fill">
    <Frame flex="row" justify="center" w={32}>{T('Live', { s: 12, w: 600, c: 'status/warnung' })}</Frame>
  </Frame>;

  // ----- Plakette-Chip der Filterzeile (FilterChips): 24 hoch, Rand `linie/standard`, `flaeche/taste`, text/xs 600; gewählt `auswahl/kante` auf `auswahl/flaeche`
  // in `akzent/text`. Mit `single` ist genau einer gewählt und es gibt kein „Filter lösen“ -----
  const Chip = (label, on) => <Frame name={'Chip ' + label} flex="row" items="center" px={8} h={24} rounded={12} bg={v(on ? 'auswahl/flaeche' : 'flaeche/taste')} stroke={v(on ? 'auswahl/kante' : 'linie/standard')} strokeWidth={1}>
    {T(label, { s: 12, w: 600, c: on ? 'akzent/text' : 'text/primaer' })}
  </Frame>;
  const Chips = (name, labels, active) => <Frame name={name} flex="row" items="center" gap={4} w="fill">{labels.map((l) => Chip(l, l === active))}</Frame>;

  // ----- Verweis des Details: Chip der Fläche (Welt: Figuren und Scripts, Pokémon: Trainer), Name vom Kern, daneben in Mono die Quelle -----
  const Verweis = (b, label, quelle) => Ref(b, label, quelle);
  const Gruppe = (kopf, chips) => <Frame name={'Gruppe ' + kopf} flex="col" gap={4} w="fill">
    {T(kopf, { s: 12, w: 600, c: 'text/sekundaer' })}
    <Frame flex="row" gap={4} w="fill">{chips}</Frame>
  </Frame>;
  const Verwendung = (title, count, gruppen) => <Frame name={title} flex="col" w="fill">
    {Section({ title, count, children: <Frame name="Gruppen" flex="col" gap={8} px={8} pb={8} w="fill">{gruppen}</Frame> })}
  </Frame>;

  // ----- Detail (FlagDetail): Karte `flaeche/blatt`, Kante `linie/standard`, `radius/fenster`, innen 12 / 16; Kopf: Name (text/m, eigener 600), darunter der pret-Name
  // und „0x0094 · 148 · allgemein“ in Mono xs sekundär, die Beschreibung (oder „Keine Beschreibung“ sekundär xs), die Tasten (sekundär S) -----
  const Detail = (w) => <Frame name="Detail" flex="col" gap={8} px={16} py={12} w={w} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
    <Frame name="Kopf" flex="col" gap={4} w="fill">
      {T('FLAG_HAT_MEDAILLE', { s: 15, w: 600, width: w - 32 })}
      {Mono('FLAG_UNUSED_0x0094', { width: w - 32 })}
      {Mono('0x0094 · 148 · allgemein', { width: w - 32 })}
      {T('Der Spieler hat die Medaille bekommen.', { width: w - 32 })}
      <Frame name="Tasten" flex="row" items="center" gap={8} pt={4}>{Btn({ label: 'Umbenennen…', small: true })}{Btn({ label: 'Beschreibung…', small: true })}</Frame>
    </Frame>
    {Verwendung('Wo gesetzt', 1, [
      Gruppe('Scriptdatei 0412', [Verweis('welt', 'Script 1 · SetFlag', 'Scriptdatei 0412')]),
    ])}
    {Verwendung('Wo geprüft', 2, [
      Gruppe('Eventdatei 0390', [Verweis('welt', 'Objekt 3 · Sichtbarkeits-Flag', 'Eventdatei 0390')]),
      Gruppe('Scriptdatei 0040', [Verweis('welt', 'Script 2 · CheckFlag', 'Scriptdatei 0040')]),
    ])}
  </Frame>;

  // ----- 'liste': Liste „Technik“ mit neun Einträgen und die Fläche „Flags und Variablen“ -----
  if (MODE === 'liste') {
    const technikListe = ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
      {[
        ['Projekt', 'ph:folder-open', 'Kartenbild der Startstadt'],
        ['Textprojekt', 'ph:file-code', 'Abgeglichen'],
        ['Prüfen und Bauen', 'ph:hammer', 'gebaut vor 3 Minuten'],
        ['Veröffentlichen', 'ph:upload-simple', 'Bereit für einen Patch'],
        ['Änderungen', 'ph:git-diff', 'Keine Abweichung'],
        ['Patches', 'ph:bandaids', 'Dynamische Header verfügbar'],
        ['Emulator', 'ph:game-controller', 'Kein Emulator gewählt'],
        ['Werkzeuge', 'ph:toolbox', '1 von 2 bereit'],
        ['Flags und Variablen', 'ph:flag', '4.097 Flags · 135 frei', true],
      ].map(([title, icon, s, sel]) => ListRow({ title, icon, sub: s, sel }))}
    </Frame>]);

    // Fläche (FlagsSurface): `flaeche/deck`, innen 32 / 24, Abstand 16; alles in der Lesebreite 880 (der Satz unter dem Titel höchstens 64ch, rund 520),
    // Liste und Detail darunter bis 1280 breit (Fenster 1440: 1440 − 88 Atlas − 272 Liste − 48 = 1032), ab 760 nebeneinander, je (1032 − 16) / 2 = 508
    const BW = 880;
    const SPLIT = 1440 - 88 - 272 - 48;
    const CW = Math.floor((SPLIT - 16) / 2);
    const kopf = <Frame name="Kopf" flex="col" gap={4} w={BW}>
      {T('Flags und Variablen', { f: Z, s: 24, lh: 30, w: 700 })}
      {T('Welche Flags und Variablen das Spiel kennt und wo Scripts, Figuren, Auslöser, Level-Scripts und Trainer sie setzen oder prüfen.', { width: 520 })}
    </Frame>;
    const filter = <Frame name="Filter" flex="col" gap={8} w={BW}>
      {Chips('Art', ['Flags', 'Variablen'], 'Flags')}
      {Chips('Bereich', ['Alle', 'kartenlokal', 'allgemein', 'versteckte Items', 'Trainer', 'System', 'Tagesflags'], 'Alle')}
      <Frame name="Kästchen" flex="row" items="center" gap={16} w="fill">{Check(false, 'nur unbenutzt')}{Check(false, 'nur frei')}{Check(false, 'nur mit eigenem Namen')}</Frame>
      <Frame name="Suche" flex="row" w={360}>{Search('Name oder Nummer suchen')}</Frame>
    </Frame>;
    // Karte der Liste: Kopf mit „Live“, die Zeilen (2 Abstand), der Fuß (xs, sekundär) „6 von 4.097 · Script-lokale Variablen … (nicht im Index)“
    const liste = <Frame name="Liste Flags" flex="col" gap={8} px={8} py={12} w={CW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
      {ListenKopf()}
      <Frame name="Zeilen" flex="col" gap={2} w="fill">{ZEILEN.map((e) => Zeile(e, { sel: e.n === 0x0094 }))}</Frame>
      <Frame flex="row" px={12} w="fill">{T('6 von 4.097 · Script-lokale Variablen 1.284 Vorkommen, Flags aus Variablen 37 (nicht im Index)', { s: 12, c: 'text/sekundaer', width: CW - 40 })}</Frame>
    </Frame>;
    const flaeche = <Frame name="Fläche: Flags und Variablen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
      {kopf}{filter}
      <Frame name="Liste und Detail" flex="row" items="start" gap={16} w={SPLIT}>{liste}{Detail(CW)}</Frame>
    </Frame>;
    return Window({ name: NAMES.liste, active: 'technik', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 0 },
      status: Statusbar({ build: 'Gebaut vor 3 Minuten', built: true }), children: [technikListe, flaeche] });
  }

  // ----- Flag-Feld (FlagField): Zeile des Inspectors (Bezeichnung 116), der Name als Chip der Fläche Geschichten (Name vom Kern, die Nummer in Mono; Keins ohne
  // Nummer) und „Wählen…“ (leise, S) auf einer Zeile, die umbricht, wenn es eng ist (`wrap`: der Chip allein, „Wählen…“ darunter); in Tüfteln darunter das Zahlfeld
  // (Dez | Hex, 0 bis 65535). 0 ist „Keins“ und ein gültiger Wert; Bauen trägt den Satz der Zeile (hint) -----
  const Wahl = () => Btn({ label: 'Wählen…', art: 'Leise', small: true });
  const Feld = ({ label = 'Sichtbarkeits-Flag', name, tech, hint, tuefteln, wrap, zahl }) => {
    const chip = Ref('geschichten', name, tech);
    const line = wrap ? <Frame name="Zeile" flex="col" gap={6} w="fill">{chip}{Wahl()}</Frame> : <Frame name="Zeile" flex="row" items="center" gap={6} w="fill">{chip}{Wahl()}</Frame>;
    return Row({ label, hint, children: <Frame name="Flag-Feld" flex="col" gap={4} grow={1}>{line}{tuefteln && NumField(String(zahl), { base: true })}</Frame> });
  };

  // ----- 'picker': der Picker „Flag wählen“ über dem Inspector einer Figur -----
  if (MODE === 'picker') {
    // Hintergrund: Welt bauen, Figur bearbeiten (Werkbank) wie werkbank-figur.body.js, verkürzt (die Liste „Welt“ mit wenigen Zeilen), im Inspector das Feld
    const orte = <Frame name="Orte" flex="col" w="fill" h={248} overflow="hidden">
      {PanelHead({ title: 'Orte', actions: <Frame flex="row" items="center" gap={8}>{SegS([['', 'ph:tree-structure'], ['', 'ph:list']], 'ph:tree-structure')}{IconBtn('ph:sort-ascending')}{IconBtn('ph:plus')}{IconBtn('ph:caret-down')}</Frame> })}
      <Frame flex="col" gap={6} p={8} w="fill">{T('1 Weltkarte, 72 Gebiete, 492 Räume', { s: 12, c: 'text/sekundaer', one: true })}{Search('Gebiet, Raum oder Kürzel suchen')}</Frame>
      <Frame name="Baum „Orte“" flex="col" px={4} w="fill">
        {TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 })}
        {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, shown: true, count: 6 })}
        {TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, count: 0 })}
        {TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 })}
      </Frame>
    </Frame>;
    const fig = (n, x, z) => TreeRowX({ label: 'Objekt ' + n + ' · ID ' + n, tech: 'x ' + x + ', z ' + z, techW: 44, icon: 'ph:person-simple-walk', depth: 2, sel: n === 2 });
    const szene = <Frame name="In dieser Szene" flex="col" w="fill" grow={1} overflow="hidden">
      {PanelHead({ title: 'In dieser Szene', actions: <Frame flex="row" items="center" gap={4}>{IconBtn('ph:arrow-left')}{IconBtn('ph:arrow-right', { off: true })}{IconBtn('ph:plus')}</Frame> })}
      <Frame flex="col" gap={6} p={8} w="fill">{Search('Name oder Nummer suchen')}</Frame>
      <Frame name="Baum „In dieser Szene“" flex="col" px={4} w="fill">
        {TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true })}
        {TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: true, count: 8 })}
        {fig(0, 119, 878)}{fig(1, 108, 889)}{fig(2, 105, 875)}{fig(3, 108, 869)}
      </Frame>
    </Frame>;
    const liste = ListPane([PanelHead({ title: 'Welt', sub: 'Platin, DE' }), <Frame name="Spalte" flex="col" w="fill" grow={1}>{orte}{Teiler('Teiler „Höhe der Orte“')}{szene}</Frame>]);
    const SW = 740, SH = 548 - DI.reiter;
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Raster', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt',
      children: Welt3D({ w: SW, h: SH, sel: 2, children: [LayerBar(8, 8), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 }), LegendBtn(8, SH - 40)] }) });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const quiet = (label) => <Frame flex="row" items="center" px={8} h={24} rounded={6}>{T(label, { w: 600 })}</Frame>;
    const felder = <Frame name="Felder" flex="col" w="fill" pt={2}>
      {Row({ label: 'x (Kachel)', children: NumField('105') })}
      {Row({ label: 'z (Kachel)', children: NumField('875') })}
      {Row({ label: 'Lokale ID', children: NumField('2') })}
      {Row({ label: 'Grafik', children: NumField('148') })}
      {Row({ label: 'Bewegung', children: NumField('15') })}
      {Row({ label: 'Blickrichtung', children: Field('Unten', { select: true }) })}
      {Row({ label: 'Script', children: <Frame flex="col" gap={4} grow={1}><Frame flex="row" w="fill">{NumField('3')}</Frame><Frame flex="row" gap={6}>{quiet('Öffnen')}{quiet('Neues Script')}</Frame></Frame> })}
      {Row({ label: 'Trainer', children: [Mono('kein Trainer', { c: 'text/primaer', grow: 1 }), quiet('Ändern…')] })}
      {Row({ label: 'Sichtlinie', children: Field('Aus', { select: true }) })}
      {Row({ label: 'Sichtweite', children: NumField('0') })}
      {Feld({ name: 'Keins', hint: 'Die Figur ist da, solange das Flag 0 ist.' })}
    </Frame>;
    const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
      children: [InspectorHead({ word: 'Figur', icon: 'ph:person-simple-walk', title: 'Objekt 2 · ID 2', ident: 'Objekt 2 · Eventdatei 0390' }), felder] });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}>
      <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
      {inspector}
    </Frame>;
    const hintergrund = Window({ name: NAMES.picker, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Zweiblattdorf' }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Zweiblattdorf'], ['Gebiet', '0411'], ['Objekt 2 · ID 2', 'Eventdatei 0390']], depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });

    // Zeile des Pickers (Picker, PickerRow): Symbol der Fläche Geschichten (`bereich/geschichten`), Name (eigener 600) mit zweiter Zeile xs sekundär (der pret-Name
    // unter einem eigenen, sonst der Bereich), Nummer in Mono xs, rechts die Verwendung xs. Die erste Zeile ist hervorgehoben (`auswahl/flaeche`, Text
    // `akzent/text`, zweite Zeile, Nummer und Verwendung erben ihn); Mindesthöhe `dichte/zeile`, innen 12
    const W = 480;
    const IW = W - 40;
    const Treffer = ({ label, strong, desc, tech, count, hi }) => <Frame name={label} flex="row" items="center" gap={8} px={12} py={2} minH={desc ? 36 : DI.zeile} w="fill" rounded={4} bg={hi ? v('auswahl/flaeche') : undefined}>
      {I('ph:chats-circle', DI.symbol, 'geschichten')}
      <Frame flex="col" grow={1}>
        {T(label, { w: strong ? 600 : 400, c: hi ? 'akzent/text' : 'text/primaer', one: true })}
        {desc && T(desc, { s: 12, c: hi ? 'akzent/text' : 'text/sekundaer', one: true })}
      </Frame>
      {tech && Mono(tech, { c: hi ? 'akzent/text' : 'text/sekundaer' })}
      {count && T(count, { s: 12, c: hi ? 'akzent/text' : 'text/sekundaer' })}
    </Frame>;
    // Filter des Pickers (Segmented S mit `wrap`): Alle und die Bereiche der Liste; was nicht in die Zeile passt, steht in einer zweiten, die Optionen
    // behalten den Abstand 2
    const seg = (l, on) => <Frame flex="row" items="center" px={8} h={24} rounded={4} bg={on ? v('flaeche/blatt') : undefined} stroke={on ? v('auswahl/kante') : undefined} strokeWidth={1}>{T(l, { w: 600, c: on ? 'akzent/text' : 'text/primaer' })}</Frame>;
    const filter = <Frame name="Filter" flex="col" gap={2} p={2} w="fill" rounded={6} bg={v('flaeche/deck')} stroke={v('linie/standard')} strokeWidth={1}>
      <Frame flex="row" gap={2}>{['Alle', 'kartenlokal', 'allgemein', 'versteckte Items'].map((l) => seg(l, l === 'Alle'))}</Frame>
      <Frame flex="row" gap={2}>{['Trainer', 'System', 'Tagesflags'].map((l) => seg(l, false))}</Frame>
    </Frame>;
    const fuss = <Frame name="Fuß" flex="col" gap={4} pt={4} w="fill" strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
      <Frame flex="row" items="center" gap={6}>{Btn({ label: 'Freies Flag nehmen', art: 'Leise', small: true })}{Btn({ label: 'Im Verzeichnis zeigen', art: 'Leise', small: true })}</Frame>
    </Frame>;
    const dialog = <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
      {T('Flag wählen', { s: 18, w: 600, width: W - 72 })}
      <Frame name="Picker" flex="col" gap={8} w={IW}>
        {Field('Name oder Nummer', { icon: 'ph:magnifying-glass', ph: true })}
        {filter}
        <Frame name="Treffer" flex="col" w="fill">
          {Treffer({ label: 'Keins (0)', hi: true })}
          {Treffer({ label: 'Flag 0x0003', desc: 'kartenlokal', tech: '0x0003', count: 'gesetzt 2 · geprüft 5' })}
          {Treffer({ label: 'FLAG_HAT_MEDAILLE', strong: true, desc: 'FLAG_UNUSED_0x0094', tech: '0x0094', count: 'gesetzt 1 · geprüft 2' })}
          {Treffer({ label: 'FLAG_HIDE_TWINLEAF_MOM', desc: 'allgemein', tech: '0x00C3', count: 'gesetzt 1 · geprüft 3' })}
          {Treffer({ label: 'FLAG_UNUSED_0x00F0', desc: 'allgemein', tech: '0x00F0', count: 'unbenutzt' })}
          {Treffer({ label: 'FLAG_VILLA_FURNITURE_TABLE', desc: 'System', tech: '0x0997', count: 'gesetzt 2 · geprüft 1 · unklar 1' })}
        </Frame>
        {fuss}
      </Frame>
    </Frame>;
    return MitDialog(NAMES.picker, hintergrund, dialog);
  }

  // ----- 'brett': „Komponenten (App): Flag-Picker und Flag-Feld“ auf der Seite „Komponenten“ -----
  const BW = 880;
  const TW = BW - 32;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer', width: TW })}{inhalt}</Frame>;
  const Karte = (inhalt) => <Frame flex="col" gap={12} p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  // Eine Zeile mit ihrem Zustand daneben: das Bauteil links, ein Satz rechts
  const Mit = (bauteil, satz, bw) => <Frame flex="row" items="center" gap={16} w="fill">
    <Frame flex="col" w={bw}>{bauteil}</Frame>
    {T(satz, { s: 12, c: 'text/sekundaer', width: TW - bw - 16 })}
  </Frame>;
  // Der Inspector als Ausschnitt (340 breit, wie die Gruppe Inspector)
  const Ausschnitt = (zeilen) => <Frame name="Inspector (Ausschnitt)" flex="col" w={340} pt={2} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>{zeilen}</Frame>;
  const Zelle = (w, kind) => <Frame flex="row" items="center" px={8} py={4} w={w}>{kind}</Frame>;

  // Die Zeile des Pickers (wie in 'picker'): in vier Zuständen, 440 breit
  const PW = 440;
  const Treffer = ({ label, strong, desc, tech, count, hi }) => <Frame name={label} flex="row" items="center" gap={8} px={12} py={2} minH={desc ? 36 : DI.zeile} w={PW} rounded={4} bg={hi ? v('auswahl/flaeche') : undefined}>
    {I('ph:chats-circle', DI.symbol, 'geschichten')}
    <Frame flex="col" grow={1}>
      {T(label, { w: strong ? 600 : 400, c: hi ? 'akzent/text' : 'text/primaer', one: true })}
      {desc && T(desc, { s: 12, c: hi ? 'akzent/text' : 'text/sekundaer', one: true })}
    </Frame>
    {tech && Mono(tech, { c: hi ? 'akzent/text' : 'text/sekundaer' })}
    {count && T(count, { s: 12, c: hi ? 'akzent/text' : 'text/sekundaer' })}
  </Frame>;
  // Der Fuß des Pickers: Status (xs) über zwei leisen Tasten; gesperrt bleibt die Taste fokussierbar, der Grund steht als Tooltip und für Vorleser
  const Fuss = ({ status, gesperrt }) => <Frame name="Fuß" flex="col" gap={4} pt={4} w={PW} strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
    {status && T(status, { s: 12, c: 'text/sekundaer' })}
    <Frame flex="row" items="center" gap={6}>
      {Btn({ label: 'Freies Flag nehmen', art: 'Leise', small: true, off: !!gesperrt })}{Btn({ label: 'Im Verzeichnis zeigen', art: 'Leise', small: true })}
    </Frame>
  </Frame>;

  // Zeilen der Liste in sieben Zuständen: ein Beispiel je Zustand, daneben was er sagt
  const LW = 480;
  const ZL = (e, o) => Zeile(e, { ...o, w: LW });
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Flag-Picker und Flag-Feld', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Technik › Flags und Variablen und die Felder im Inspector: ein Flag oder eine Variable wird mit ihrem Namen gezeigt, über den Picker gewählt und, in Tüfteln, als Zahl geschrieben. 0 ist „Keins“ und ein gültiger Wert. Eigene Namen stehen fett, der pret-Name des Spiels grau darunter.', { c: 'text/sekundaer', width: BW })}
    {Zustand('Flag-Feld: Name als Chip der Fläche Geschichten, Nummer in Mono, „Wählen…“; Bauen trägt den Satz der Zeile', Karte(<Frame flex="row" gap={16} items="start" w="fill">
      {Ausschnitt([
        Feld({ name: 'FLAG_HAT_MEDAILLE', tech: '0x0094', wrap: true, hint: 'Die Figur ist da, solange das Flag 0 ist.' }),
        Feld({ name: 'Keins', hint: 'Die Figur ist da, solange das Flag 0 ist.' }),
        Feld({ name: 'Flag 0x0094', tech: '0x0094', hint: undefined }),
      ])}
      {Ausschnitt([
        Feld({ label: 'Variable', name: 'VAR_UNUSED_0x4050', tech: '0x4050', wrap: true }),
        Row({ label: 'Wert', hint: 'Der Auslöser feuert, wenn die Variable diesen Wert hat.', children: NumField('1') }),
      ])}
    </Frame>))}
    {Zustand('Flag-Feld in Tüfteln: dazu das Zahlfeld mit Dez | Hex (0 bis 65535); Name und Nummer folgen der Zahl', Karte(<Frame flex="row" gap={16} items="start" w="fill">
      {Ausschnitt([Feld({ name: 'FLAG_HIDE_TWINLEAF_MOM', tech: '0x00C3', wrap: true, tuefteln: true, zahl: 195 })])}
      {Ausschnitt([Feld({ name: 'Keins', tuefteln: true, zahl: 0 })])}
    </Frame>))}
    {Zustand('Flag-Feld in der Tabelle des Level-Scripts (ohne Bezeichnung, `bare`): die Variable einer Bedingung, daneben Wert und Script als Zahlfelder', Karte(
      <Frame name="Variablen-Bedingungen" flex="col" w="fill">
        <Frame name="Kopfzeile" flex="row" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>
          {[['Variable', 360], ['Wert', 120], ['Script', 120]].map(([s, w]) => <Frame flex="row" items="center" px={8} w={w} h="fill">{T(s, { s: 12, w: 600, c: 'text/sekundaer' })}</Frame>)}
        </Frame>
        <Frame name="Zeile" flex="row" items="center" w="fill" strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
          {Zelle(360, <Frame flex="row" items="center" gap={6}>{Ref('geschichten', 'VAR_UNUSED_0x4050', '0x4050')}{Wahl()}</Frame>)}
          {Zelle(120, NumField('1'))}
          {Zelle(120, NumField('5'))}
        </Frame>
      </Frame>))}
    {Zustand('Zeile des Pickers: „Keins (0)“ als erste Zeile (hervorgehoben, wird beim Tippen nur gefunden, wenn die Suche passt), eigener Name fett mit dem pret-Namen darunter, pret-Name mit dem Bereich, ohne Namen mit Art und Nummer', Karte(<Frame flex="col" gap={2} w="fill">
      {Treffer({ label: 'Keins (0)', hi: true })}
      {Treffer({ label: 'FLAG_HAT_MEDAILLE', strong: true, desc: 'FLAG_UNUSED_0x0094', tech: '0x0094', count: 'gesetzt 1 · geprüft 2' })}
      {Treffer({ label: 'FLAG_HIDE_TWINLEAF_MOM', desc: 'allgemein', tech: '0x00C3', count: 'gesetzt 1 · geprüft 3' })}
      {Treffer({ label: 'Flag 0x0003', desc: 'kartenlokal', tech: '0x0003', count: 'gesetzt 2 · geprüft 5' })}
    </Frame>))}
    {Zustand('Fuß des Pickers: offen, solange die Scripts nicht übernommen sind (gesperrt, Grund als Tooltip), bei leerem Pool, beim Lesen', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(Fuss({}), 'Offen: „Freies Flag nehmen“ nimmt die kleinste freie Nummer des allgemeinen Bereichs und schließt den Picker; „Im Verzeichnis zeigen“ öffnet Technik › Flags und Variablen auf der Nummer des Feldes (nur mit Feld).', PW)}
      {Mit(Fuss({ gesperrt: true }), 'Gesperrt, Scripts nicht übernommen: „Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche Flags Scripts benutzen.“ Bei Variablen heißt die Taste „Freie Variable nehmen“.', PW)}
      {Mit(Fuss({ gesperrt: true }), 'Gesperrt, Pool leer: „Kein freies Flag im allgemeinen Bereich.“ (bei Variablen „Keine freie Variable im allgemeinen Bereich.“). Ein Fehler des Kerns beim Nehmen steht ebenso als Grund.', PW)}
      {Mit(Fuss({ status: 'Lädt …', gesperrt: true }), 'Beim Lesen: die Zeile „Lädt …“ über den Tasten; „Freies Flag nehmen“ wartet, ohne Grund. Ein Lesefehler steht dort in der Farbe des Fehlers.', PW)}
    </Frame>))}
    {Zustand('Zeile der Liste in sieben Zuständen: eigener Name, pret-Name, unbenannt, frei, reserviert (nur Variablen), kartenlokal, gewählt', Karte(<Frame flex="col" gap={4} w="fill">
      <Frame flex="row" justify="end" w={LW}><Frame flex="row" px={12} w="fill" justify="end"><Frame flex="row" justify="center" w={32}>{T('Live', { s: 12, w: 600, c: 'status/warnung' })}</Frame></Frame></Frame>
      {Mit(ZL({ n: 0x0094, own: 'FLAG_HAT_MEDAILLE', pret: 'FLAG_UNUSED_0x0094', usage: 'gesetzt 1 · geprüft 2' }), 'Eigener Name: Name fett, darunter in Mono der pret-Name und die Nummer.', LW)}
      {Mit(ZL({ n: 0x00C3, pret: 'FLAG_HIDE_TWINLEAF_MOM', usage: 'gesetzt 1 · geprüft 3' }), 'pret-Name: Name des Spiels, darunter nur die Nummer.', LW)}
      {Mit(ZL({ n: 0x0AA0, usage: 'gesetzt 1 · geprüft 0' }), 'Unbenannt: Art und Nummer als Name („Flag 0x0AA0“, „Variable 0x4050“).', LW)}
      {Mit(ZL({ n: 0x00F0, pret: 'FLAG_UNUSED_0x00F0', usage: 'unbenutzt', free: true }), 'Frei: Plakette „frei“ (Erfolg); allgemeiner Bereich, ohne Verweis und ohne eigenen Namen.', LW)}
      {Mit(ZL({ kind: 'var', n: 0x406C, pret: 'VAR_UNUSED_0x406C', usage: 'unbenutzt', reserved: true }), 'Reserviert: Plakette „reserviert“ (Info) mit der Vorlage „Leere Region“ als Tooltip; sie nimmt die Nummer aus „frei“.', LW)}
      {Mit(ZL({ n: 0x0003, usage: null, places: 54 }), 'Kartenlokal: „an 54 Orten“ statt Gesetzt und Geprüft („an 1 Ort“).', LW)}
      {Mit(ZL({ n: 0x0094, own: 'FLAG_HAT_MEDAILLE', pret: 'FLAG_UNUSED_0x0094', usage: 'gesetzt 1 · geprüft 2' }, { sel: true }), 'Gewählt: Fläche `auswahl/flaeche`, Kante links.', LW)}
    </Frame>))}
  </Frame>;
