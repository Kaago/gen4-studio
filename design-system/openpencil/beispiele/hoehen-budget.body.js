  // Höhen und Budget (Phase 6.2), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'hoehen'  „App: Welt bauen, Schicht Höhen“: Werkbank der Welt, in der Schau die Ebenen der Schau „Gelände | Höhen | Ereignisse | Draufsicht“ mit
  //             „Höhen“ und „Ereignisse“ gedrückt, darunter eine Zweiblattdorf-ähnliche Karte in drei Stufen (2, 3, 4) mit Isolinien in `hoehe/0` auf der
  //             höheren Seite; im Inspector „Karte 0100“ mit eingeklapptem „Budget“; Statusleiste „Kachel x 20, y 10 · Karte 0100 · begehbar · Höhe 4 Stufen“
  //   'budget'  „App: Inspector, Karte mit Budget“: Werkbank mit dem Raum Haus 2 (Sandgemme) in der Schau, im Inspector „Karte 0188“ (Zelle 0, 0) mit „Karte“,
  //             „Höhe“ und dem offenen Abschnitt „Budget“: „Modell“ Knapp, „Terrain (BDHC)“ Ruhig, „Gebäude“ Ruhig, „Texturen des Gebiets“ Knapp
  //   'brett'   „Komponenten (App): Budget und Höhenstufen“ (Seite „Komponenten“): das Budget in drei Zuständen, in Tüfteln, ohne Gebiet und beim Lesen;
  //             die sieben Höhenstufen mit Zahlen; die Isolinien an einem Beispiel; die Szenenleiste mit den vier Tasten
  // Quellen: apps/studio/src/world/{ground,hover}.ts, world/scene/heightLayer.ts, app/useSceneLayers.ts, workbench/SceneBar.tsx, ui/primitives/{Budget.tsx,
  // Budget.module.css,icons.ts}, workbench/inspector/{MapBudgetSection,MatrixCellEditor}.tsx, ui/gallery/pages/WorldPage.tsx. Texte aus design-system/README.md,
  // „Nachtrag 6.2“. Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: die Höhen der Karte, die Zahlen des Budgets (Modell 57.344 B,
  // VRAM 112.896 B; die Bilder der Test-Umgebung zeigen auf Karte 188 alles Ruhig).
  // Gerendert am 09.11.2026, alle drei Modi im ersten Lauf ohne Korrektur (App: Werkbank x 0 / x 1520, y 3000; Komponenten x 3440, y 16440); beim Schreiben
  // war OpenPencil nicht erreichbar (Anleitung: openpencil/NACHTRAG-6-2.md).
  // Dunkel: bash ../build-screen.sh hoehen-budget --dunkel (die Höhenfarben sind Szenenfarben und in beiden Themen gleich).
  const MODE = 'hoehen';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { hoehen: 'App: Welt bauen, Schicht Höhen', budget: 'App: Inspector, Karte mit Budget', brett: 'Komponenten (App): Budget und Höhenstufen' };

  // ----- Zahlen wie toLocaleString("de-DE"): Punkt als Tausendertrenner; hex wie MapBudgetSection („0xF000“) -----
  const de = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const hex = (n) => '0x' + n.toString(16).toUpperCase();

  // ----- Budget (Budget.tsx, [data-budget-state]): Name (text/s, sekundär, Zeile dichte/zeile) mit der Plakette des Zustands, der Balken (role="meter", 8 hoch,
  // radius/plakette, Spur flaeche/deck; Füllung ruhig akzent/grund, knapp status/warnung, voll status/fehler; über der Grenze voll), die Wörter „<used> von
  // <max> <Einheit>“ und in Tüfteln die Grenze hex in Mono, zuletzt der Hinweis (text/xs, sekundär). Ruhig hat keine Plakette -----
  const FILL = { calm: 'akzent/grund', tight: 'status/warnung', full: 'status/fehler' };
  const BudgetX = ({ label, used, max, state, unit, note, text, tech, w = 300 }) => {
    const share = max > 0 ? Math.min(Math.max(used, 0), max) / max : 0;
    const words = text || de(used) + ' von ' + de(max) + (unit ? ' ' + unit : '');
    return <Frame name={'Budget ' + label} flex="col" gap={4} py={4} w={w}>
      <Frame name="Kopf" flex="row" items="center" gap={8} minH={DI.zeile} w="fill">
        {T(label, { c: 'text/sekundaer' })}
        {state === 'tight' && Badge('Knapp', 'warn', 'ph:warning')}
        {state === 'full' && Badge('Voll', 'fehler', 'ph:x-circle')}
      </Frame>
      <Frame name="Balken" w={w} h={8} rounded={4} bg={v('flaeche/deck')} overflow="hidden">
        {share > 0 && <Frame position="absolute" x={0} y={0} w={Math.max(Math.round(share * w), 4)} h={8} rounded={4} bg={v(FILL[state])} />}
      </Frame>
      <Frame name="Wert" flex="row" items="center" gap={8} w="fill">{T(words)}{tech && Mono(tech)}</Frame>
      {note && T(note, { s: 12, c: 'text/sekundaer' })}
    </Frame>;
  };
  // Die Zeile „Texturen des Gebiets“ ohne Gebiet oder Paket: der Satz des Kerns statt des Balkens
  const BudgetMissing = (satz, w = 300) => <Frame name="Budget Texturen des Gebiets (fehlt)" flex="col" gap={4} py={4} w={w}>
    {T('Texturen des Gebiets', { c: 'text/sekundaer' })}
    {T(satz, { width: w })}
  </Frame>;
  // Die vier Zeilen des Abschnitts in der Reihenfolge von MapBudgetSection; b: { model, bdhc, buildings, vram: [used, state], textures }; tinker: hex dazu
  const BudgetRows = (b, tinker, w = 300) => <Frame name="Zeilen" flex="col" gap={4} px={8} w="fill">
    {BudgetX({ label: 'Modell', used: b.model[0], max: 61440, state: b.model[1], unit: 'B', tech: tinker ? hex(61440) : undefined, w })}
    {BudgetX({ label: 'Terrain (BDHC)', used: b.bdhc[0], max: 36864, state: b.bdhc[1], unit: 'B', tech: tinker ? hex(36864) : undefined, w })}
    {BudgetX({ label: 'Gebäude', used: b.buildings[0], max: 32, state: b.buildings[1], w })}
    {BudgetX({ label: 'Texturen des Gebiets', used: b.vram[0], max: 262144, state: b.vram[1], text: 'VRAM ' + de(b.vram[0]) + ' von ' + de(262144) + ' B · ' + b.textures + (b.textures === 1 ? ' Textur' : ' Texturen'), note: 'Vanilla höchstens ' + de(110640) + ' B', tech: tinker ? hex(262144) : undefined, w })}
  </Frame>;
  // Der Abschnitt „Budget“ (Section, offen oder eingeklappt) mit dem Satz und den Zeilen
  const BudgetSection = (b, o = {}) => Section({ title: 'Budget', open: o.open !== false, children: <Frame name="Budget" flex="col" gap={4} w="fill">
    <Frame flex="row" px={8} py={4} w="fill">{T('Grenzen des Spiels; Speichern geht immer.', { s: 12, c: 'text/sekundaer' })}</Frame>
    {b ? BudgetRows(b, o.tinker, o.w) : <Frame flex="row" px={8} py={4} w="fill">{T('Wird gelesen …', { s: 12, c: 'text/sekundaer' })}</Frame>}
  </Frame> });

  // ----- Höhenstufen (heightLayer.ts): Stufe = halbe Kachel (8 E), gerundet; Farbe hoehe/<Stufe mod 7>; Isolinien hoehe/0 auf der höheren Seite -----
  const stepColor = (step) => v('hoehe/' + (((step % 7) + 7) % 7));
  // Ebenen der Schau (SceneBar): „Gelände“ (ph:stack), „Höhen“ (ph:mountains), „Ereignisse“ (ph:map-pin), Trenner, „Draufsicht“ (ph:drone, Symbol topDown);
  // on: welche gedrückt sind. „Höhen“ und „Gelände“ schließen sich aus
  const SceneBarX = (x, y, on = {}) => <Frame name="Ebenen der Schau" position="absolute" x={x} y={y} flex="row" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Ebenen" flex="row" gap={4}>{IconBtn('ph:stack', { on: !!on.gelaende })}{IconBtn('ph:mountains', { on: !!on.hoehen })}{IconBtn('ph:map-pin', { on: on.ereignisse !== false })}</Frame>
    <Frame name="Trenner" w={1} h={24} bg={v('linie/leise')} />
    <Frame name="Ansichten" flex="row" gap={4}>{IconBtn('ph:drone', { on: !!on.draufsicht })}</Frame>
  </Frame>;

  // Eine Zweiblattdorf-ähnliche Karte (keine Spielgrafik) in COLS × ROWS Kacheln, schräg gestaucht (Kachel 36 × 28, die Karte 648 breit in der Schau 740): Grund Stufe 2, ein Hang Stufe 3 im Osten und
  // Süden, darauf ein Plateau Stufe 4; Fläche der Stufe mit 42 % über dem Platzhalter-Grund, Isolinien 2 px in hoehe/0 an jeder Kante mit Stufenwechsel, an der
  // Kante der höheren Kachel. Die Kachel unter dem Zeiger ist x 20, y 10 der Karte (hier Spalte 12, Zeile 4: Plateau, Stufe 4)
  const COLS = 18, ROWS = 13, TW = 36, TH = 28;
  const stepAt = (c, r) => {
    if (c < 0 || r < 0 || c >= COLS || r >= ROWS) return null;
    if (c >= 10 && c <= 15 && r >= 1 && r <= 5) return 4;
    if ((c >= 8 && r <= 7) || (c >= 1 && c <= 4 && r >= 8 && r <= 11)) return 3;
    return 2;
  };
  const HeightMap = ({ w, h: H, children }) => {
    const ox = Math.round((w - COLS * TW) / 2) + 40, oy = Math.round((H - ROWS * TH) / 2);
    const pal = ['#7B9C31', '#94AD42', '#C68C52', '#6B8C29', '#A5BD5A', '#B57B4A'];
    const grund = [], flaeche = [], linien = [];
    for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
      const x = ox + c * TW, y = oy + r * TH, s = stepAt(c, r);
      grund.push(<Frame position="absolute" x={x} y={y} w={TW} h={TH} bg={pal[(r * 3 + c * 5) % 6]} />);
      flaeche.push(<Frame position="absolute" x={x} y={y} w={TW} h={TH} bg={stepColor(s)} opacity={0.42} />);
      // Ostkante und Südkante je Kachel einmal; die Linie liegt an der Seite der höheren Kachel
      const east = stepAt(c + 1, r), south = stepAt(c, r + 1);
      if (east !== null && east !== s) linien.push(<Frame position="absolute" x={east > s ? x + TW : x + TW - 2} y={y} w={2} h={TH} bg={v('hoehe/0')} />);
      if (south !== null && south !== s) linien.push(<Frame position="absolute" x={x} y={south > s ? y + TH : y + TH - 2} w={TW} h={2} bg={v('hoehe/0')} />);
    }
    // Marker der Ereignisse (Ebene „Ereignisse“ an): Figuren als Kreis, Türen als Quadrat, in den Szenen-Farben
    const at = (c, r) => [ox + c * TW + TW / 2, oy + r * TH + TH / 2];
    const figs = [[3, 2], [6, 6], [12, 3], [14, 9], [2, 10]].map(([c, r]) => { const [x, y] = at(c, r); return <Frame position="absolute" x={x - 9} y={y - 9} w={18} h={18} rounded={9} bg={v('ereignis/figur')} stroke="#101821" strokeWidth={1} />; });
    const doors = [[5, 1], [11, 1], [7, 9]].map(([c, r]) => { const [x, y] = at(c, r); return <Frame position="absolute" x={x - 7} y={y - 7} w={14} h={14} rounded={2} bg={v('ereignis/warp')} stroke="#101821" strokeWidth={1} />; });
    return <Frame name="3D-Szene (Höhen)" w={w} h={H} bg={v('flaeche/welt')} overflow="hidden">
      <Frame name="Grund" position="absolute" x={0} y={0} w={w} h={H}>{grund}</Frame>
      <Frame name="Höhen: Flächen" position="absolute" x={0} y={0} w={w} h={H}>{flaeche}</Frame>
      <Frame name="Höhen: Isolinien" position="absolute" x={0} y={0} w={w} h={H}>{linien}</Frame>
      {doors}{figs}{children}
    </Frame>;
  };

  // ----- Werkbank der Welt (wie werkbank-figur.body.js, verkürzt): Liste „Welt“, Scharnier oben, Schau mit 3D und Weltkarte (Bauen), Script und Probleme,
  // rechts der Inspector (340) -----
  const Werkbank = ({ name, orte, szene, crumbs, schauInhalt, inspector, info }) => {
    const liste = WeltListe({ orte, szene });
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt', children: schauInhalt });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top',
      children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const insp = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left', children: inspector });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}><Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>{insp}</Frame>;
    return Window({ name, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info }),
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs, depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 0 })}{dock}</Frame>] });
  };
  // Felder der Kartenzelle (MatrixCellEditor): Bezeichnung über dem Feld, darunter der Hinweis
  const Stapel = (label, feld, hint) => <Frame name={label} flex="col" gap={4} px={8} py={4} w="fill">{T(label, { c: 'text/sekundaer' })}{feld}{hint && T(hint, { s: 12, c: 'text/sekundaer' })}</Frame>;
  const Zelle = (karte, zelle, budget, o) => [
    InspectorHead({ word: 'Karte', icon: 'ph:mountains', title: 'Karte ' + karte, ident: 'Zelle ' + zelle }),
    <Frame name="Felder" flex="col" w="fill" pt={4}>
      {Stapel('Karte', Field(String(Number(karte))), '0 bis 665, leer = keine Karte')}
      {Stapel('Höhe', NumField('0'), '0 bis 255')}
      {BudgetSection(budget, o)}
      <Frame flex="row" px={8} py={8} w="fill">{T('„Duplizieren“ (Menü „Aktionen“) trägt eine Kopie der Karte in diese Zelle ein und lässt sich nicht rückgängig machen: Die Kopie ist sofort als neue Karte am Ende des Archivs vorhanden.', { s: 12, c: 'text/sekundaer', width: 316 })}</Frame>
    </Frame>,
  ];

  // ----- 'hoehen': die Schicht „Höhen“ in der Schau -----
  if (MODE === 'hoehen') {
    const SW = 740, SH = 548 - DI.reiter;
    const orte = [
      TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
      TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, shown: true, sel: true, count: 6 }),
      TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, count: 0 }),
      TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 }),
    ];
    const szene = [
      TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true }),
      TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 }),
      TreeRowX({ label: 'Karte 0100', tech: '0, 0', icon: 'ph:mountains', depth: 2, open: false, sel: true }),
      TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 5 }),
      TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 3 }),
    ];
    return Werkbank({ name: NAMES.hoehen, orte, szene, crumbs: [['Zweiblattdorf'], ['Karte 0100', 'Zelle 0, 0']],
      info: 'Kachel x 20, y 10 · Karte 0100 · begehbar · Höhe 4 Stufen',
      schauInhalt: HeightMap({ w: SW, h: SH, children: [SceneBarX(8, 8, { hoehen: true }), FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 }), LegendBtn(8, SH - 40)] }),
      inspector: Zelle('0100', '0, 0', { model: [21452, 'calm'], bdhc: [300, 'calm'], buildings: [3, 'calm'], vram: [37744, 'calm'], textures: 38 }, { open: false }) });
  }

  // ----- 'budget': der Inspector einer Karte mit dem Abschnitt „Budget“ -----
  if (MODE === 'budget') {
    const SW = 740, SH = 548 - DI.reiter;
    const orte = [
      TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 1 }),
      TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: true, count: 1 }),
      TreeRowX({ label: 'Haus 2', icon: 'ph:house', depth: 2, shown: true, sel: true }),
    ];
    const szene = [
      TreeRowX({ label: 'Haus 2', icon: 'ph:house', open: true }),
      TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 1 }),
      TreeRowX({ label: 'Karte 0188', tech: '0, 0', icon: 'ph:mountains', depth: 2, open: false, sel: true }),
      TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 2 }),
      TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 2 }),
    ];
    // Ein Raum als Platzhalter (keine Spielgrafik): Boden, Wände, zwei Figuren, der Zellrahmen der gewählten Karte (auswahl/halo)
    const raum = <Frame name="3D-Szene (Raum)" w={SW} h={SH} bg={v('flaeche/welt')} overflow="hidden">
      <Frame position="absolute" x={160} y={110} w={420} h={300} stroke={v('auswahl/halo')} strokeWidth={8} />
      <Frame position="absolute" x={200} y={130} w={180} h={110} bg="#C68C52" />
      <Frame position="absolute" x={200} y={110} w={180} h={20} bg="#6B8CEF" />
      <Frame position="absolute" x={270} y={180} w={18} h={18} rounded={9} bg={v('ereignis/figur')} stroke="#101821" strokeWidth={1} />
      <Frame position="absolute" x={320} y={160} w={18} h={18} rounded={9} bg={v('ereignis/figur')} stroke="#101821" strokeWidth={1} />
      <Frame position="absolute" x={250} y={226} w={14} h={14} rounded={2} bg={v('ereignis/warp')} stroke="#101821" strokeWidth={1} />
      {SceneBarX(8, 8, { gelaende: true })}{FloatingToolbar({ tool: 'Auswählen', anchor: 'links', x: 8, y: 56 })}{LegendBtn(8, SH - 40)}
    </Frame>;
    return Werkbank({ name: NAMES.budget, orte, szene, crumbs: [['Sandgemme'], ['Haus 2', '0422'], ['Karte 0188', 'Zelle 0, 0']], info: 'Sandgemme, Karte 0188 (0, 0)',
      schauInhalt: raum,
      inspector: Zelle('0188', '0, 0', { model: [57344, 'tight'], bdhc: [21452, 'calm'], buildings: [16, 'calm'], vram: [112896, 'tight'], textures: 45 }) });
  }

  // ----- 'brett': „Komponenten (App): Budget und Höhenstufen“ auf der Seite „Komponenten“ -----
  const BW = 880;
  const TWB = BW - 32;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer', width: TWB })}{inhalt}</Frame>;
  const Karte = (inhalt) => <Frame flex="col" gap={12} p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  const Mit = (bauteil, satz, bw) => <Frame flex="row" items="center" gap={16} w="fill">
    <Frame flex="col" w={bw}>{bauteil}</Frame>
    {T(satz, { s: 12, c: 'text/sekundaer', width: TWB - bw - 16 })}
  </Frame>;
  const Ausschnitt = (zeilen) => <Frame name="Inspector (Ausschnitt)" flex="col" w={340} pt={2} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>{zeilen}</Frame>;
  const Stufe = (step) => <Frame name={'Stufe ' + step} flex="row" items="center" gap={8} h={DI.zeile}>
    <Frame w={24} h={16} rounded={4} bg={stepColor(step)} stroke={v('linie/leise')} strokeWidth={1} />
    {T('Stufe ' + step + ' · ' + step * 8 + ' E')}
  </Frame>;
  // Isolinien an einem Beispiel: ein 3 × 3-Block Stufe 2 in einer Fläche Stufe 0 (12 Kantenstücke), Kachel 32
  const Iso = () => {
    const K = 32, N = 5, kacheln = [], linien = [];
    const s = (c, r) => (c >= 1 && c <= 3 && r >= 1 && r <= 3 ? 2 : 0);
    for (let r = 0; r < N; r++) for (let c = 0; c < N; c++) {
      kacheln.push(<Frame position="absolute" x={c * K} y={r * K} w={K} h={K} bg={stepColor(s(c, r))} opacity={0.42} stroke={v('linie/leise')} strokeWidth={1} />);
      if (c + 1 < N && s(c + 1, r) !== s(c, r)) linien.push(<Frame position="absolute" x={s(c + 1, r) > s(c, r) ? (c + 1) * K : (c + 1) * K - 2} y={r * K} w={2} h={K} bg={v('hoehe/0')} />);
      if (r + 1 < N && s(c, r + 1) !== s(c, r)) linien.push(<Frame position="absolute" x={c * K} y={s(c, r + 1) > s(c, r) ? (r + 1) * K : (r + 1) * K - 2} w={K} h={2} bg={v('hoehe/0')} />);
    }
    return <Frame name="Isolinien (Beispiel)" w={N * K} h={N * K} bg={v('flaeche/welt')}>{kacheln}{linien}</Frame>;
  };
  const W = 300;
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Budget und Höhenstufen', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Phase 6.2: Grenzen des Spiels, ruhig gezeigt, und die Schicht „Höhen“ der Schau. Ein Budget blockiert das Speichern nie. Die Höhen sind Szenenfarben (cividis) und in beiden Themen gleich.', { c: 'text/sekundaer', width: BW })}
    {Zustand('Budget in drei Zuständen: Ruhig ohne Plakette, ab 90 % „Knapp“ (Warnung), über der Grenze „Voll“ (Fehler); der Balken ist dann voll, die Wörter nennen die wahre Zahl', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(BudgetX({ label: 'Terrain (BDHC)', used: 300, max: 36864, state: 'calm', unit: 'B', w: W }), 'Ruhig: Füllung akzent/grund, keine Plakette. [data-budget-state="calm"]', W)}
      {Mit(BudgetX({ label: 'Modell', used: 58000, max: 61440, state: 'tight', unit: 'B', w: W }), 'Knapp: Plakette „Knapp“ (Warnung), Füllung status/warnung; ab 90 % der Grenze, 32 von 32 Gebäuden sind Knapp. [data-budget-state="tight"]', W)}
      {Mit(BudgetX({ label: 'Gebäude', used: 33, max: 32, state: 'full', w: W }), 'Voll: Plakette „Voll“ (Fehler), Füllung status/fehler; das Spiel lädt nur 32 Gebäude, den Rest lässt es still weg. [data-budget-state="full"]', W)}
    </Frame>))}
    {Zustand('Abschnitt „Budget“ im Inspector einer Karte: Bauen (links) und Tüfteln mit den Grenzen hex (rechts)', Karte(<Frame flex="row" gap={16} items="start" w="fill">
      {Ausschnitt([BudgetSection({ model: [21452, 'calm'], bdhc: [300, 'calm'], buildings: [3, 'calm'], vram: [37744, 'calm'], textures: 38 }, { w: 300 })])}
      {Ausschnitt([BudgetSection({ model: [21452, 'calm'], bdhc: [300, 'calm'], buildings: [3, 'calm'], vram: [37744, 'calm'], textures: 38 }, { w: 300, tinker: true })])}
    </Frame>))}
    {Zustand('Ohne Gebiet oder Paket und beim Lesen: der Satz des Kerns statt des Balkens; „Wird gelesen …“ bis die Antwort für diese Karte da ist', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(BudgetMissing('Kein Header zeigt diese Karte.', W), 'Weitere Sätze: „Gebietsdaten 12 fehlen.“, „Texturpaket 6 nicht lesbar: <Grund>.“, „Gebäude-Texturpaket 6 nicht lesbar: <Grund>.“', W)}
      {Mit(Ausschnitt([BudgetSection(null)]), 'Beim Lesen und nach dem Wechsel der Karte; eine Antwort für eine ältere Karte fällt weg.', 340)}
    </Frame>))}
    {Zustand('Höhenstufen: eine Stufe ist eine halbe Kachel (8 E), Farbe szene/hoehe/<Stufe mod 7>, ab Stufe 7 von vorn; Fläche je Kachel mit 42 %', Karte(<Frame flex="row" gap={32} items="start" w="fill">
      <Frame name="Sieben Stufen" flex="col" gap={2}>{[0, 1, 2, 3, 4, 5, 6].map(Stufe)}</Frame>
      <Frame flex="col" gap={8}>{Iso()}{T('Isolinien in hoehe/0 an jeder Kante mit Stufenwechsel, auf der höheren Seite (12 Kantenstücke um den Block).', { s: 12, c: 'text/sekundaer', width: 260 })}</Frame>
    </Frame>))}
    {Zustand('Ebenen der Schau: „Gelände | Höhen | Ereignisse“, dahinter „Draufsicht“ (Symbol topDown, Phosphor Drone); „Höhen“ und „Gelände“ schließen sich aus', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame name="Leiste Gelände" w={200} h={48}>{SceneBarX(0, 0, { gelaende: true })}</Frame>, 'Die Szene öffnet mit „Gelände“ und „Ereignisse“.', 200)}
      {Mit(<Frame name="Leiste Höhen" w={200} h={48}>{SceneBarX(0, 0, { hoehen: true })}</Frame>, '„Höhen“ an schaltet „Gelände“ aus und umgekehrt; beide aus geht. Statusleiste: „… · Höhe 4 Stufen“, Tüfteln „… · Höhe 32 E (4 Stufen)“.', 200)}
    </Frame>))}
  </Frame>;
