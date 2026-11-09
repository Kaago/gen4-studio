  // Teile importieren: Modell aus OBJ (Phase 6.5a), ein Frame aus einer Vorlage: zwei Zustände des Dialogs „Teile in Karte 0005 importieren“ (480 breit)
  // nebeneinander auf der Abdunklung:
  //   links   der Würfel g4_imp_cube.obj: Quelle „Datei“, die Zeile „Umwandeln“ („Farben je Textur“ 16 | 4, Feld „Einheit“ 16 mit Hilfe), die Prüfliste mit
  //           „Modell“ (ok, angekreuzt) und darunter die Vorschau: Hinweis „OBJ-Modell umgewandelt: …“, die Zeilen „OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen“,
  //           „g4_imp_cube · 16 × 16 · 16 Farben · neu“, „VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B“ und das Budget „Modell“ Ruhig; Taste „1 Teil übernehmen“
  //   rechts  ein zu großes Modell: Zeile „Modell“ mit Fehler (Kästchen gesperrt) und dem Satz von model.too-big, Budget „Modell“ Voll, Taste „0 Teile übernehmen“ gesperrt
  // Quellen: apps/studio/src/workbench/inspector/{ImportPartsDialog.tsx,ImportPartsDialog.module.css,importParts.ts}, ui/primitives/{Dialog,Segmented,TextField,
  // CheckList,Checkbox,Budget,Button}.tsx. Texte aus design-system/README.md, „Nachtrag 6.5a“. Beispielwerte, keine Spieldaten: das Testmodell g4_imp_cube ist
  // selbst erzeugt (g4k import-samples), die Größen 21.740 B und 139.264 B sind Beispiele.
  // Gerendert am 09.11.2026 auf main nach dem Merge (App: Werkbank, Reihe 5: hell x 0, dunkel x 1520, y 5000), im zweiten Lauf (siehe unten); Anleitung: openpencil/NACHTRAG-6-5a.md.
  // Dunkel: bash ../build-screen.sh import-modell --dunkel.
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAME = 'Teile importieren: Modell aus OBJ' + (DARK ? ' (Dunkel)' : '');
  const W = 480;
  const IW = W - 40;
  const de = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');

  // ----- Dialog (Dialog-Primitive, 480 breit): Kreuz, Titel text/l, Inhalt (Abstand 12, ImportPartsDialog .body), Tasten rechts -----
  const Dialog = (title, inhalt, tasten) => <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T(title, { s: 18, w: 600, width: W - 72 })}
    <Frame name="Inhalt" flex="col" gap={12} w="fill">{inhalt}</Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{tasten.map((t) => Btn(t))}</Frame>
  </Frame>;
  const Leise = (text, w = IW) => T(text, { s: 12, c: 'text/sekundaer', width: w });

  // ----- Quelle „Datei“: Segmente „Quelle“, Feld „Pfade, eine je Zeile“ (mehrzeilig, Bezeichnung darüber), „Dateien wählen…“ (nur in der gebündelten App) -----
  const Quelle = (pfad) => <Frame name="Quelle" flex="col" gap={8} w="fill">
    {SegS(['Datei', 'Karte im Projekt'], 'Datei')}
    <Frame name="Pfade" flex="col" gap={4} w="fill">
      {T('Pfade, eine je Zeile', { s: 12, w: 600 })}
      <Frame flex="row" items="start" px={8} py={6} h={72} w="fill" rounded={4} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{T(pfad, { one: true, grow: 1 })}</Frame>
    </Frame>
    {Btn({ label: 'Dateien wählen…', small: true })}
  </Frame>;

  // ----- Zeile „Umwandeln“ (data-import-convert): Titel text/s betont; „Farben je Textur“ (text/s sekundär über Segmented s „16“ | „4“) und, nur bei einem
  // 3D-Modell, das Feld „Einheit“ (Bezeichnung darüber, Hilfe text/xs sekundär darunter) -----
  const Umwandeln = (modell) => <Frame name="Umwandeln" flex="col" gap={4} w="fill">
    {T('Umwandeln', { s: 12, w: 600 })}
    <Frame name="Optionen" flex="row" items="start" gap={12} w="fill">
      <Frame name="Farben je Textur" flex="col" gap={4}>{T('Farben je Textur', { s: 12, c: 'text/sekundaer' })}{SegS(['16', '4'], '16')}</Frame>
      {modell && <Frame name="Einheit" flex="col" gap={4} w={220}>
        {T('Einheit', { s: 12, w: 600 })}
        {Field('16', { w: 220 })}
        {Leise('Welteinheiten je Modelleinheit; 16 = eine Kachel', 220)}
      </Frame>}
    </Frame>
  </Frame>;

  // ----- Prüfliste (CheckList, eine Zeile je Teil): Zustandssymbol, Kästchen mit „Name · Größe Bytes · Datei“, darunter Satz, Hinweise und die Vorschau -----
  const SYMBOL = { ok: ['ph:check-circle-fill', 'ok'], error: ['ph:x-circle-fill', 'f'], warning: ['ph:warning-fill', 'w'], info: ['ph:info-fill', 'info'] };
  const Teil = ({ state, label, on, off, lines }) => <Frame name={'Teil ' + state} flex="row" items="start" gap={8} py={4} w="fill">
    <Frame pt={2}>{I(SYMBOL[state][0], DI.symbol, SYMBOL[state][1])}</Frame>
    <Frame name="Teil" flex="col" gap={2} w={IW - 26}>{Check(on, label, off)}{lines}</Frame>
  </Frame>;

  // ----- Budget „Modell“ (Budget.tsx) wie im Abschnitt „Budget“ des Inspectors (Nachtrag 6.2), hier unter dem Teil; Tüfteln die Grenze hex dahinter -----
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

  const wuerfel = Dialog('Teile in Karte 0005 importieren', [
    Quelle('~/Modelle/g4_imp_cube.obj'),
    Umwandeln(true),
    <Frame name="Teile" flex="col" w="fill">{Teil({ state: 'ok', label: 'Modell · 21.740 Bytes · g4_imp_cube.obj', on: true, lines: [
      Leise('OBJ-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 2 Streifen.', IW - 26),
      Leise('Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 7,6.', IW - 26),
      Leise('BDHC bleibt unverändert, Laufhöhen prüfen.', IW - 26),
      <Frame name="Vorschau" flex="col" gap={2} pt={4} w="fill">
        {Leise('OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen', IW - 26)}
        {Leise('g4_imp_cube · 16 × 16 · 16 Farben · neu', IW - 26)}
        {Leise('VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B', IW - 26)}
        {BudgetModell(21740, 'calm')}
      </Frame>,
    ] })}</Frame>,
  ], [{ label: 'Abbrechen' }, { label: '1 Teil übernehmen', art: 'Primär' }]);

  const zuGross = Dialog('Teile in Karte 0005 importieren', [
    Quelle('~/Modelle/stadt.obj'),
    Umwandeln(true),
    <Frame name="Teile" flex="col" w="fill">{Teil({ state: 'error', label: 'Modell · 139.264 Bytes · stadt.obj', on: false, off: true, lines: [
      T('Das Modell ist 139.264 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.', { s: 12, c: 'text/sekundaer', width: IW - 26 }),
      Leise('OBJ-Modell umgewandelt: 5.120 Flächen, 4.800 Vierecke, 320 Dreiecke, 410 Streifen.', IW - 26),
      <Frame name="Vorschau" flex="col" gap={2} pt={4} w="fill">
        {Leise('OBJ · 4.800 Vierecke · 320 Dreiecke · 410 Streifen', IW - 26)}
        {Leise('stadt · 64 × 64 · im Paket', IW - 26)}
        {Leise('VRAM des Gebiets 21.760 B · Vanilla höchstens 110.640 B', IW - 26)}
        {BudgetModell(139264, 'full')}
      </Frame>,
    ] })}</Frame>,
  ], [{ label: 'Abbrechen' }, { label: '0 Teile übernehmen', art: 'Primär', off: true }]);

  // Die Zeile der Dialoge steckt in einer Spalte. Der erste Lauf (09.11.2026, die Reihe als Wurzel) kam mit zerfallenem Layout zurück (Dialoge 40 breit,
  // 7.578 hoch); vermutlich lag es am ersten Laden neuer Symbole, nicht an der Reihe (bei 6.4a lief ein ebenso zerfallener Frame unverändert sauber nach)
  return <Frame name={NAME} flex="col" p={40} w={W * 2 + 120} rounded={10} bg={v('flaeche/ueberlagerung')}>
    <Frame name="Dialoge" flex="row" items="start" gap={40}>
      {wuerfel}
      {zuGross}
    </Frame>
  </Frame>;
