  // Freiform und Austausch (Phase 6.5b), zwei Frames aus einer Vorlage; MODE wählt den Frame (in der erzeugten .jsx umstellen):
  //   'import'  „Teile importieren: Freiform und Tiled“: zwei Zustände des Dialogs „Teile in Karte 0666 importieren“ (480 breit) einer Welt-Karte
  //             nebeneinander auf der Abdunklung:
  //             links   der Würfel g4_imp_cube.obj als Freiform: Quelle „Datei“, „Umwandeln“ („Farben je Textur“ 16 | 4, „Einheit“ 32), „Lage“ mit
  //                     den Feldern x, y, z (−40, 0, 56) und der Hilfe, die Zeile „Freiform (glTF) · g4_imp_cube.obj“ (ok, angekreuzt) mit den
  //                     Hinweisen, der Vorschau (Textur „neu“) und dem Budget „Modell“ Ruhig; Taste „1 Teil übernehmen“
  //             rechts  eine Tiled-Karte 0666.tmj und eine .per daneben: die Zeile „Tiled (TMJ) · 0666.tmj“ (ok) mit „1 Ebene, 37 Zellen geändert“
  //                     und dem Satz von tmj.buildings-ignored, darunter die gesperrte Zeile „Kollision“ und die Notiz „Bei Welt-Karten entstehen
  //                     Modell, Terrain und Permissions beim Backen.“; Taste „1 Teil übernehmen“
  //   'quelle'  „Quelle: Als glTF übernehmen“: drei Ausschnitte des Inspectors (340 breit) nebeneinander: der Abschnitt „Quelle“ einer binären
  //             Karte mit beiden Tasten; nach Form 2 die Welt-Karte mit der Zeile des Backens und „Übernommen als glTF: …“; eine Ablehnung von
  //             Form 2 (world.adopt-texture) als Fehlermeldung unter den Tasten
  // Quellen: apps/studio/src/workbench/inspector/{ImportPartsDialog.tsx,ImportPartsDialog.module.css,ExportPartDialog.tsx,WorldSourceSection.tsx,
  // WorldSourceSection.module.css,importParts.ts}, ui/primitives/{Dialog,Segmented,TextField,CheckList,Checkbox,Budget,Button,Notice,Section}.tsx.
  // Texte aus design-system/README.md, „Nachtrag 6.5b“. Beispielwerte, keine Spieldaten: das Testmodell g4_imp_cube ist selbst erzeugt
  // (g4k import-samples), die Lage folgt dem Szenario pt-freiform (Zelle 13, 19); Größen und Polygonzahlen sind Beispiele (die Zahlen von Form 2
  // nach der Messung auf Platin DE, Karte 0, aus docs/formats/world_map.md), der Material- und Paketname der Ablehnung ein Muster.
  // Gerendert am 09.11.2026 auf main nach dem Merge (App: Werkbank, 'import' Reihe 7, 'quelle' Reihe 8; hell x 0, dunkel x 1520), 'quelle' nach
  // einem Korrekturlauf (Meldungen mit Satzbreite, siehe NoticeQ); beim Schreiben antwortete OpenPencil nicht (Anleitung: openpencil/NACHTRAG-6-5b.md).
  // Dunkel: bash ../build-screen.sh welt-austausch --dunkel (der Name trägt dann „(Dunkel)“).
  const MODE = 'import';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { import: 'Teile importieren: Freiform und Tiled' + (DARK ? ' (Dunkel)' : ''), quelle: 'Quelle: Als glTF übernehmen' + (DARK ? ' (Dunkel)' : '') };
  const W = 480;
  const IW = W - 40;
  const de = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const Leise = (text, w = IW) => T(text, { s: 12, c: 'text/sekundaer', width: w });

  // ----- 'import': der Dialog wie in import-modell.body.js (Dialog-Primitive, 480 breit), dazu „Lage“ und die Zeilen der Welt -----
  const Dialog = (title, inhalt, tasten) => <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T(title, { s: 18, w: 600, width: W - 72 })}
    <Frame name="Inhalt" flex="col" gap={12} w="fill">{inhalt}</Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{tasten.map((t) => Btn(t))}</Frame>
  </Frame>;

  // Quelle „Datei“: Segmente „Quelle“, Feld „Pfade, eine je Zeile“ (mehrzeilig, Bezeichnung darüber), „Dateien wählen…“ (nur in der gebündelten App)
  const Quelle = (pfade) => <Frame name="Quelle" flex="col" gap={8} w="fill">
    {SegS(['Datei', 'Karte im Projekt'], 'Datei')}
    <Frame name="Pfade" flex="col" gap={4} w="fill">
      {T('Pfade, eine je Zeile', { s: 12, w: 600 })}
      <Frame flex="col" items="start" gap={2} px={8} py={6} h={72} w="fill" rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{pfade.map((p) => T(p, { one: true, width: IW - 16 }))}</Frame>
    </Frame>
    {Btn({ label: 'Dateien wählen…', small: true })}
  </Frame>;

  // „Umwandeln“ (data-import-convert): „Farben je Textur“ über Segmented s, das Feld „Einheit“ mit Hilfe
  const Umwandeln = (einheit) => <Frame name="Umwandeln" flex="col" gap={4} w="fill">
    {T('Umwandeln', { s: 12, w: 600 })}
    <Frame name="Optionen" flex="row" items="start" gap={12} w="fill">
      <Frame name="Farben je Textur" flex="col" gap={4}>{T('Farben je Textur', { s: 12, c: 'text/sekundaer' })}{SegS(['16', '4'], '16')}</Frame>
      <Frame name="Einheit" flex="col" gap={4} w={220}>
        {T('Einheit', { s: 12, w: 600 })}
        {Field(einheit, { w: 220 })}
        {Leise('Welteinheiten je Modelleinheit; 16 = eine Kachel', 220)}
      </Frame>
    </Frame>
  </Frame>;

  // „Lage“ (data-import-at): Titel text/s betont, drei TextField mit sichtbarer Bezeichnung (je raum/24 × 4 = 96 breit, Abstand raum/12), Hilfe darunter
  const Lage = (x, y, z) => <Frame name="Lage" flex="col" gap={4} w="fill">
    {T('Lage', { s: 12, w: 600 })}
    <Frame name="Felder" flex="row" items="start" gap={12} w="fill">
      {[['x', x], ['y', y], ['z', z]].map(([achse, wert]) => <Frame name={'Feld ' + achse} flex="col" gap={4} w={96}>{T(achse, { s: 12, w: 600 })}{Field(wert, { w: 96 })}</Frame>)}
    </Frame>
    {Leise('Welteinheiten; 16 = eine Kachel; 0, 0, 0 = Kartenmitte')}
  </Frame>;

  // Prüfliste (CheckList): Zustandssymbol, Kästchen mit „Name · Datei“, darunter Sätze und die Vorschau
  const SYMBOL = { ok: ['ph:check-circle-fill', 'ok'], error: ['ph:x-circle-fill', 'f'], warning: ['ph:warning-fill', 'w'], info: ['ph:info-fill', 'info'] };
  const Teil = ({ state, label, on, off, lines }) => <Frame name={'Teil ' + label} flex="row" items="start" gap={8} py={4} w="fill">
    <Frame pt={2}>{I(SYMBOL[state][0], DI.symbol, SYMBOL[state][1])}</Frame>
    <Frame name="Teil" flex="col" gap={2} w={IW - 26}>{Check(on, label, off)}{lines}</Frame>
  </Frame>;

  // Budget „Modell“ (Budget.tsx, Nachtrag 6.2): hier das Modell der Karte nach dem Backen mit der Freiform
  const FILL = { calm: 'akzent/grund', tight: 'status/warnung', full: 'status/fehler' };
  const BudgetModell = (used, state, w = IW - 26) => {
    const max = 61440;
    const share = Math.min(Math.max(used, 0), max) / max;
    return <Frame name="Budget Modell" flex="col" gap={4} pt={4} w={w}>
      <Frame name="Kopf" flex="row" items="center" gap={8} minH={DI.zeile} w="fill">
        {T('Modell', { c: 'text/sekundaer' })}
        {state === 'tight' && Badge('Knapp', 'warn', 'ph:warning')}
        {state === 'full' && Badge('Voll', 'fehler', 'ph:x-circle')}
      </Frame>
      <Frame name="Balken" w={w} h={8} rounded={4} bg={v('flaeche/deck')} overflow="hidden">
        <Frame position="absolute" x={0} y={0} w={Math.max(Math.round(share * w), 4)} h={8} rounded={4} bg={v(FILL[state])} />
      </Frame>
      <Frame name="Wert" flex="row" items="center" gap={8} w="fill">{T(de(used) + ' von ' + de(max) + ' B')}</Frame>
    </Frame>;
  };

  if (MODE === 'import') {
    const freiform = Dialog('Teile in Karte 0666 importieren', [
      Quelle(['~/Modelle/g4_imp_cube.obj']),
      Umwandeln('32'),
      Lage('-40', '0', '56'),
      <Frame name="Teile" flex="col" w="fill">{Teil({ state: 'ok', label: 'Freiform (glTF) · g4_imp_cube.obj', on: true, lines: [
        Leise('OBJ-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 2 Streifen.', IW - 26),
        Leise('Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 7,6.', IW - 26),
        <Frame name="Vorschau" flex="col" gap={2} pt={4} w="fill">
          {Leise('g4_imp_cube · 16 × 16 · 16 Farben · neu', IW - 26)}
          {BudgetModell(2144, 'calm')}
        </Frame>,
      ] })}</Frame>,
    ], [{ label: 'Abbrechen' }, { label: '1 Teil übernehmen', art: 'Primär' }]);

    const tiled = Dialog('Teile in Karte 0666 importieren', [
      Quelle(['~/Tiled/0666.tmj', '~/Tiled/karte_0666.per']),
      Leise('Bei Welt-Karten entstehen Modell, Terrain und Permissions beim Backen.'),
      <Frame name="Teile" flex="col" w="fill">
        {Teil({ state: 'ok', label: 'Tiled (TMJ) · 0666.tmj', on: true, lines: [
          Leise('1 Ebene, 37 Zellen geändert', IW - 26),
          Leise('Die Gebäude der Tiled-Datei weichen von denen der Karte ab und werden nicht übernommen; Gebäude bearbeitest du im Studio.', IW - 26),
        ] })}
        {Teil({ state: 'ok', label: 'Kollision · 2.048 Bytes · karte_0666.per', on: false, off: true, lines: [] })}
      </Frame>,
    ], [{ label: 'Abbrechen' }, { label: '1 Teil übernehmen', art: 'Primär' }]);

    return <Frame name={NAMES.import} flex="row" items="start" gap={40} p={40} w={W * 2 + 120} rounded={10} bg={v('flaeche/ueberlagerung')}>
      {freiform}
      {tiled}
    </Frame>;
  }

  // ----- 'quelle': der Abschnitt „Quelle“ (WorldSourceSection) in drei Zuständen; Raster mit Abstand raum/6, Rand raum/4 oben/unten und raum/8
  // seitlich; Tasten sekundär klein in einer Zeile, die umbricht (Abstand raum/8); Meldungen über Notice -----
  const PW = 340;
  const QW = PW - 16;
  // Die Zeile bricht in der App um (flex-wrap): bei 324 px Breite passen die beiden Tasten nicht nebeneinander (zusammen gut 400), also hier untereinander.
  const Tasten = () => <Frame name="Tasten" flex="col" items="start" gap={8} w={QW}>
    {Btn({ label: 'Karte als Quelle übernehmen', small: true })}
    {Btn({ label: 'Als glTF übernehmen (Form 2)', small: true })}
  </Frame>;
  // Meldung wie Notice() aus jsx-lib-app, aber mit fester Breite des Satzes (Abschnitt 324, Rand 2 × 12, Symbol 18 + Abstand 8): Notice() gibt dem
  // Text keine Breite, die langen Sätze liefen im ersten Bild (09.11.2026) über den Rand des Inspectors hinaus
  const NoticeQ = (text, art) => { const m = { ok: ['status/erfolg-flaeche', 'ph:check-circle-fill', 'ok'], fehler: ['status/fehler-flaeche', 'ph:x-circle-fill', 'f'] }[art];
    return <Frame name={'Meldung ' + art} flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v(m[0])}>{I(m[1], 18, m[2])}{T(text, { width: QW - 24 - 26 })}</Frame>; };
  const Binaer = (meldung) => <Frame name="Quelle (binär)" flex="col" gap={6} px={8} py={4} w="fill">
    {T('Binäre Karte aus dem ROM oder aus DSPRE.', { width: QW })}
    {Tasten()}
    {meldung}
  </Frame>;
  const Welt = (bake, meldung) => <Frame name="Quelle (Welt-Karte)" flex="col" gap={6} px={8} py={4} w="fill">
    {T('Welt-Karte')}
    {T('Gebacken · Modell ' + bake[0] + ' B · Terrain ' + bake[1] + ' B', { width: QW })}
    <Frame name="Tasten" flex="row" items="center" gap={8}>{Btn({ label: 'Als binäre Karte übernehmen', small: true })}</Frame>
    {meldung}
  </Frame>;
  const Panel = (titel, kennung, inhalt) => <Frame name={'Inspector ' + titel} flex="col" w={PW} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1} overflow="hidden">
    {InspectorHead({ word: 'Karte', icon: 'ph:mountains', title: titel, ident: kennung })}
    {Section({ title: 'Quelle', children: inhalt })}
    {Section({ title: 'Budget', open: false, children: null })}
  </Frame>;

  return <Frame name={NAMES.quelle} flex="row" items="start" gap={24} p={24} w={PW * 3 + 96} rounded={10} bg={v('flaeche/deck')}>
    {Panel('Karte 0000', 'Zelle 3, 27 · Zweiblattdorf', Binaer(null))}
    {Panel('Karte 0000', 'Zelle 3, 27 · Zweiblattdorf', Welt(['38.224', '146'], NoticeQ('Übernommen als glTF: 815 Vanilla-Polygone, 816 gebackene Flächen.', 'ok')))}
    {Panel('Karte 0123', 'Zelle 5, 2 · Route 201', Binaer(NoticeQ('Material „lake_lm2“ der Karte 0123 bindet keine Textur des Pakets 0006; Form 2 geht für diese Karte nicht, Form 1 schon. (world.adopt-texture)', 'fehler')))}
  </Frame>;
