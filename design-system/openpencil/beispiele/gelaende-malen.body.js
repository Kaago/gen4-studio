  // Gelände malen (Phase 6.4a), drei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'malen'    „Gelände malen: Route 201“: Werkbank der Welt, in der Schau die Karte 0667 von oben (Draufsicht gedrückt) neben Karte 0666, ein
  //              Pinselstrich mit seiner Vorschau (Platten zu 85 %), die aufrechte Werkzeugleiste mit „Gelände malen“ gedrückt und den sieben
  //              Unterwerkzeugen als zweite Spalte daneben (die Szene ist niedriger als 773 px); im Inspector „Karte 0667“ mit „Palette“, „Ebenen“,
  //              „Kachel 0001“, „Quelle“ eingeklappt und „Budget“; Statusleiste „Gelände malen · Pinsel · Ziehen malt · Rechtsklick nimmt auf ·
  //              Z dreht · X spiegelt“. Mit --dunkel gebaut heißt der Frame „Gelände malen: Route 201 (Dunkel)“
  //   'entwurf'  „Neue Karte malen: Entwurf“: Werkbank mit dem Reiter „Weltkarte“ vorn, die Zelle des Entwurfs gestrichelt (auswahl/kante), darüber
  //              der Dialog „Neue Karte malen“ mit „Name“, „Boden“ (Palette), „Höhe“, den Notizen des Entwurfs und der Plakette „Nicht rückgängig“
  //   'tileset'  „Tileset-Editor“: wie 'malen', die Wasserkachel gewählt und „Kachel 0002“ offen (Rolle, Gelände, Blockiert, Wand, Herkunft);
  //              unten rechts in der Schau als Ausschnitt der Abschnitt ohne Datei in HeartGold und Tüfteln (gesperrt, „Tileset im Projekt anlegen“)
  //   'brett'    „Komponenten (App): Malen“ (Seite „Komponenten“): Unterwerkzeuge in Ruhe und gedrückt, die Statusleiste je Unterwerkzeug,
  //              Palettenkacheln mit und ohne Bild, gewählt, mit Marke, in Tüfteln; Ebenenzeilen; Budget mit abgelehntem Backen; die Notiz der binären Karte
  // Quellen: apps/studio/src/workbench/{paintTools.ts,paint/PaintContext.tsx,NewPaintedMapDialog.tsx,ExtendAreaDialog.tsx}, ui/frame/FloatingToolbar.tsx
  // und .module.css, app/ToolHints.tsx, world/scene/{strokes,strokePreview}.ts, workbench/inspector/{TilePalette,LayerList,TileEditor,MapBudgetSection,
  // MatrixCellEditor}.tsx und ihre .module.css, ui/primitives/icons.ts, ui/gallery/pages/WorldPage.tsx (Blöcke „Malwerkzeuge“, „Palette“, „Ebenen“).
  // Texte aus design-system/README.md, „Nachtrag 6.4a“. Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: die Kacheln sind die des
  // synthetischen Tilesets mit Beispielnamen (boden, weg, gras_a, wasser_a, fels, treppe), ihre „Bilder“ Platzhalter aus Szenenfarben; Karte, Zelle,
  // Header und Zahlen folgen dem Phasen-Gate über die Oberfläche (target/agent/gate-2026-11-07/6-4a/ui-gate.md), das Budget ist ein Beispiel.
  // Nicht gerendert: OpenPencil antwortete beim Schreiben nicht (127.0.0.1:7600, HTTP 502). Die Vorlage ist gegen jsx-lib-app.js geschrieben und mit
  // einer Attrappe der Bausteine durchgelaufen, aber nie als Bild gesehen (Anleitung: openpencil/NACHTRAG-6-4a.md).
  // Dunkel: bash ../build-screen.sh gelaende-malen --dunkel. Szenenfarben heißen in OpenPencil ohne Präfix: v('gelaende/wasser'), v('hoehe/2').
  const MODE = 'malen';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { malen: DARK ? 'Gelände malen: Route 201 (Dunkel)' : 'Gelände malen: Route 201', entwurf: 'Neue Karte malen: Entwurf', tileset: 'Tileset-Editor', brett: 'Komponenten (App): Malen' };
  const de = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const hex2 = (n) => '0x' + n.toString(16).toUpperCase().padStart(2, '0');

  // ----- Unterwerkzeuge (paintTools.ts): Name, Symbol (Phosphor), Taste; gezeichnet wie die Werkzeuge (ToolKey), gedrückt mit gefülltem Symbol -----
  const UNTER = [['Pinsel', 'ph:paint-brush', 'B'], ['Rechteck', 'ph:rectangle', 'P'], ['Füllen', 'ph:paint-bucket', 'G'], ['Radierer', 'ph:eraser', 'E'], ['Pipette', 'ph:eyedropper', 'I'], ['Höhe', 'ph:mountains', 'H'], ['Rampe', 'ph:stairs', 'R']];
  const subKeys = (sub) => UNTER.map(([n, i, k]) => ToolKey([n, n === sub ? i + '-fill' : i, k], n === sub));
  // Statusleiste je Unterwerkzeug (paintStatusHints): Schlüssel und Wirkung; die Rampe nennt Richtung und Stufen
  const HINTS = {
    Pinsel: [['Ziehen', 'malt'], ['Rechtsklick', 'nimmt auf'], ['Z', 'dreht'], ['X', 'spiegelt']],
    Rechteck: [['Ziehen', 'füllt ein Rechteck'], ['Rechtsklick', 'nimmt auf'], ['Z', 'dreht'], ['X', 'spiegelt']],
    'Füllen': [['Klick', 'füllt die Fläche'], ['Rechtsklick', 'nimmt auf'], ['Z', 'dreht'], ['X', 'spiegelt']],
    Radierer: [['Ziehen', 'radiert'], ['Rechtsklick', 'nimmt auf']],
    Pipette: [['Klick', 'nimmt die Kachel auf']],
    'Höhe': [['Klick', 'hebt'], ['⇧Klick', 'senkt'], ['Ziehen', 'über mehrere']],
    Rampe: [['Klick', 'setzt Rampe nach Osten, 1 Stufe'], ['Z', 'dreht'], ['1/2', 'Stufen'], ['⇧Klick', 'entfernt']],
  };
  // Werkzeugleiste mit „Gelände malen“ gedrückt (FloatingToolbar mit paint): aufrecht; beside: die Unterwerkzeuge als zweite Spalte rechts daneben
  // (data-paint-beside, oben bündig, Abstand raum/4, wie die Leiste gezeichnet, ohne Trennstrich), sonst unter einem Trennstrich in der Leiste
  const PaintToolbar = ({ sub = 'Pinsel', x = 8, y = 56, beside = false } = {}) => {
    const sep = (name = 'Trenner') => <Frame name={name} w={48} h={1} bg={v('linie/leise')} />;
    const keys = WERKZEUGE.flatMap((g, gi) => [...(gi > 0 ? [sep()] : []), ...g.map((t) => ToolKey(t, t[0] === 'Gelände malen'))]);
    const group = <Frame name="Unterwerkzeuge" flex="col" items="center" gap={4}>{subKeys(sub)}</Frame>;
    return <Frame name="Werkzeugleiste" position="absolute" x={x} y={y} flex="col" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Griff und Lage" flex="row" h={24}><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:dots-six', DI.symbol, 's')}</Frame><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:caret-down', DI.symbol, 's')}</Frame></Frame>
      {keys}
      {!beside && sep('Trenner (Unterwerkzeuge)')}
      {!beside && group}
      {beside && <Frame name="Unterwerkzeuge (daneben)" position="absolute" x={48 + 4} y={-1} flex="col" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{subKeys(sub)}</Frame>}
    </Frame>;
  };
  // Flache Leiste (Galerie „Malwerkzeuge“): Werkzeuge, Trennstrich, Unterwerkzeuge in einer Zeile
  const PaintToolbarFlat = (sub) => {
    const sep = () => <Frame name="Trenner" w={1} h={DI.werkzeug} bg={v('linie/leise')} />;
    const keys = WERKZEUGE.flatMap((g, gi) => [...(gi > 0 ? [sep()] : []), ...g.map((t) => ToolKey(t, t[0] === 'Gelände malen'))]);
    return <Frame name="Werkzeugleiste (flach)" flex="row" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Griff und Lage" flex="row" h={DI.werkzeug}><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:dots-six-vertical', DI.symbol, 's')}</Frame><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:caret-down', DI.symbol, 's')}</Frame></Frame>
      {keys}{sep()}<Frame name="Unterwerkzeuge" flex="row" items="center" gap={4}>{subKeys(sub)}</Frame>
    </Frame>;
  };
  // Ebenen der Schau (SceneBar, wie hoehen-budget.body.js): „Gelände“, „Höhen“, „Ereignisse“, Trenner, „Draufsicht“
  const SceneBarX = (x, y, on = {}) => <Frame name="Ebenen der Schau" position="absolute" x={x} y={y} flex="row" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Ebenen" flex="row" gap={4}>{IconBtn('ph:stack', { on: !!on.gelaende })}{IconBtn('ph:mountains', { on: !!on.hoehen })}{IconBtn('ph:map-pin', { on: on.ereignisse !== false })}</Frame>
    <Frame name="Trenner" w={1} h={24} bg={v('linie/leise')} />
    <Frame name="Ansichten" flex="row" gap={4}>{IconBtn('ph:drone', { on: !!on.draufsicht })}</Frame>
  </Frame>;

  // ----- Kacheln (Beispiel-Tileset: die Rollen des synthetischen Tilesets mit Beispielnamen) -----
  // „Bild“ einer Kachel: Platzhalter aus zwei Beispiel-Hex (keine Spielgrafik); ohne Bild die Platzhalterfarbe der Rolle (szene/gelaende/…)
  const ROLLE = { ground: ['Boden', 'gelaende/unbekannt'], grass: ['Gras', 'gelaende/hohes-gras'], water: ['Wasser', 'gelaende/wasser'], wall: ['Wand', 'gelaende/blockiert'], ramp: ['Rampe', 'gelaende/unbekannt'], piece: ['Teilmodell', 'gelaende/spezial'] };
  const KACHELN = [
    { n: '0000', tex: 'boden', role: 'ground', pic: ['#A5BD5A', '#94AD42'] },
    { n: '0001', tex: 'weg', role: 'ground', pic: ['#D6C08C', '#C6A56B'] },
    { n: '0002', tex: 'wasser_a', role: 'water', pic: ['#4A8CD6', '#3173C6'] },
    { n: '0003', tex: 'gras_a', role: 'grass', pic: ['#5A7B21', '#6B8C29'] },
    { n: '0004', tex: 'fels', role: 'wall', pic: ['#8C6B4A', '#73563A'] },
    { n: '0005', tex: 'treppe', role: 'ramp', pic: ['#C6CEB5', '#ADB59C'] },
    { n: '0006', tex: 'baum', role: 'piece', pic: ['#3A6B42', '#2D5A35'] },
  ];
  const kachel = (n) => KACHELN.find((k) => k.n === n);
  // Taste der Palette (TilePalette): 32 × 32, Rand linie/leise, radius/feld (4), Fläche flaeche/taste; gewählt Rand und Fokusstrich auswahl/kante;
  // Marke der Drehung/Spiegelung unten rechts (auswahl/kante, akzent/auf, text/xs); Tüfteln die Nummer darunter (code/s, sekundär)
  const PalTile = (k, o = {}) => {
    const fill = o.pic === false || !k.pic ? <Frame w={30} h={30} bg={v(ROLLE[k.role][1])} /> : <Frame name="Bild" w={30} h={30} bg={k.pic[0]}><Frame position="absolute" x={0} y={15} w={15} h={15} bg={k.pic[1]} /><Frame position="absolute" x={15} y={0} w={15} h={15} bg={k.pic[1]} /></Frame>;
    const taste = <Frame name={'Kachel ' + k.n} w={32} h={32} rounded={4} bg={v('flaeche/taste')} stroke={v(o.sel ? 'auswahl/kante' : 'linie/leise')} strokeWidth={o.sel ? 3 : 1} overflow="hidden" flex="row" items="center" justify="center">
      {fill}
      {o.mark && <Frame name="Marke" position="absolute" x={32 - 6 - o.mark.length * 6} y={20} px={2} h={12} bg={v('auswahl/kante')} flex="row" items="center">{T(o.mark, { s: 10, lh: 12, c: 'akzent/auf' })}</Frame>}
    </Frame>;
    return o.tinker ? <Frame flex="col" items="center" gap={2}>{taste}{Mono(k.n, { s: 10, lh: 12 })}</Frame> : taste;
  };
  // Abschnitt „Palette“: Gruppen nach Rolle (Überschrift text/xs sekundär), Kacheln umbrechend mit Abstand 4
  const Palette = ({ sel, mark, pic = true, tinker, open = true, w = 324 }) => {
    const groups = ['ground', 'grass', 'water', 'wall', 'ramp', 'piece'].map((role) => [role, KACHELN.filter((k) => k.role === role)]).filter(([, ks]) => ks.length > 0);
    return Section({ title: 'Palette', open, children: <Frame name="Palette" flex="col" gap={8} px={8} pt={4} pb={8} w="fill">
      {groups.map(([role, ks]) => <Frame name={'Gruppe ' + ROLLE[role][0]} flex="col" gap={4} w={w}>
        {T(ROLLE[role][0], { s: 12, c: 'text/sekundaer' })}
        <Frame flex="row" gap={4} w={w}>{ks.map((k) => PalTile(k, { sel: k.n === sel, mark: k.n === sel ? mark : undefined, pic, tinker }))}</Frame>
      </Frame>)}
    </Frame> });
  };
  // Radio „Aktiv“ (natives Radio, accent-color akzent/grund)
  const Radio = (on) => <Frame name={on ? 'Aktiv (an)' : 'Aktiv'} w={16} h={16} rounded={8} bg={v('flaeche/blatt')} stroke={v(on ? 'akzent/grund' : 'linie/stark')} strokeWidth={on ? 2 : 1} flex="row" items="center" justify="center">{on && <Frame w={8} h={8} rounded={4} bg={v('akzent/grund')} />}</Frame>;
  // Zeile der Ebenen (LayerList, data-layer-row): Radio, „Name · Rolle“, Auge (verborgen: „Zeigen“, eye-slash gedrückt), „Entfernen“ bei Überlagerungen
  const LayerRow = ({ name, overlay, active, hidden }) => <Frame name={'Ebene ' + name} flex="row" items="center" gap={6} h={DI.steuer} w="fill">
    {Radio(active)}
    {T(name + ' · ' + (overlay ? 'Überlagerung' : 'Boden'), { grow: 1, one: true })}
    {IconBtn(hidden ? 'ph:eye-slash' : 'ph:eye', { on: hidden })}
    {overlay && Btn({ label: 'Entfernen', art: 'Leise', small: true })}
  </Frame>;
  const Layers = ({ rows, open = true, next = 'Ebene 1' }) => Section({ title: 'Ebenen', open, children: <Frame name="Ebenen" flex="col" gap={6} px={8} pt={4} pb={8} w="fill">
    <Frame name="Zeilen" flex="col" gap={2} w="fill">{rows.map(LayerRow)}</Frame>
    <Frame name="Hinzufügen" flex="row" items="center" gap={6} w="fill">{Field(next, { grow: 1 })}{Btn({ label: 'Ebene hinzufügen', small: true })}</Frame>
  </Frame> });
  // Abschnitt „Kachel 0002“ (TileEditor): Inspector-Zeilen; locked: ohne Datei (gesperrt, „Tileset im Projekt anlegen“ darüber); hgss: Schrittgeräusch;
  // tinker: „Gelände“ als Zahlfeld hex mit Hinweis
  const Select = (text, off) => <Frame flex="row" items="center" gap={8} px={8} h={32} w="fill" rounded={4} bg={v(off ? 'flaeche/deck' : 'flaeche/blatt')} stroke={v(off ? 'linie/leise' : 'linie/standard')} strokeWidth={1}>{T(text, { grow: 1, one: true, c: off ? 'text/deaktiviert' : 'text/primaer' })}{I('ph:caret-down', 16, off ? 'd' : 's')}</Frame>;
  const TileSection = ({ k, kind, kindName, blocked, wall, origin, locked, hgss, sound = 0, tinker, open = true, pkg = '0006' }) => Section({ title: 'Kachel ' + k.n, open, children: <Frame name={'Kachel ' + k.n} flex="col" gap={6} px={0} pt={4} pb={8} w="fill">
    {locked && <Frame name="Tileset anlegen" flex="col" gap={6} px={8} w="fill">
      <Frame flex="row">{Btn({ label: 'Tileset im Projekt anlegen', small: true })}</Frame>
      {T('Die Vorgaben stehen dann in data/tilesets/' + pkg + '.json und gelten für alle Welt-Karten dieses Pakets.', { s: 12, c: 'text/sekundaer', width: 316 })}
    </Frame>}
    {Row({ label: 'Rolle', children: Select(ROLLE[k.role][0], locked) })}
    {tinker ? Row({ label: 'Gelände', hint: hex2(kind) + ' ' + kindName, children: Field(hex2(kind), { mono: true, off: locked }) }) : Row({ label: 'Gelände', children: Select(kindName, locked) })}
    {Row({ label: 'Blockiert', children: Check(blocked, 'Blockiert', locked) })}
    {hgss && Row({ label: 'Schrittgeräusch', hint: '0 bis 127', children: Field(String(sound), { off: locked }) })}
    {Row({ label: 'Wand', hint: 'Kachelnummer, leer = keine', children: Field(wall || 'keine', { ph: !wall, off: locked }) })}
    {origin && <Frame flex="row" px={8} w="fill">{T(origin, { s: 12, c: 'text/sekundaer' })}</Frame>}
  </Frame> });

  // ----- Budget (wie hoehen-budget.body.js), dazu der Satz „Nicht gebacken: …“ unter „Modell“ (data-budget-refused) -----
  const FILL = { calm: 'akzent/grund', tight: 'status/warnung', full: 'status/fehler' };
  const BudgetX = ({ label, used, max, state, unit, note, text, w = 300 }) => {
    const share = max > 0 ? Math.min(Math.max(used, 0), max) / max : 0;
    const words = text || de(used) + ' von ' + de(max) + (unit ? ' ' + unit : '');
    return <Frame name={'Budget ' + label} flex="col" gap={4} py={4} w={w}>
      <Frame name="Kopf" flex="row" items="center" gap={8} minH={DI.zeile} w="fill">{T(label, { c: 'text/sekundaer' })}{state === 'tight' && Badge('Knapp', 'warn', 'ph:warning')}{state === 'full' && Badge('Voll', 'fehler', 'ph:x-circle')}</Frame>
      <Frame name="Balken" w={w} h={8} rounded={4} bg={v('flaeche/deck')} overflow="hidden">{share > 0 && <Frame position="absolute" x={0} y={0} w={Math.max(Math.round(share * w), 4)} h={8} rounded={4} bg={v(FILL[state])} />}</Frame>
      {T(words)}
      {note && T(note, { s: 12, c: 'text/sekundaer', width: w })}
    </Frame>;
  };
  const BudgetSection = (b, o = {}) => Section({ title: 'Budget', open: o.open !== false, children: <Frame name="Budget" flex="col" gap={4} w="fill">
    <Frame flex="row" px={8} py={4} w="fill">{T('Grenzen des Spiels; Speichern geht immer.', { s: 12, c: 'text/sekundaer' })}</Frame>
    <Frame name="Zeilen" flex="col" gap={4} px={8} w="fill">
      {BudgetX({ label: 'Modell', used: b.model[0], max: 61440, state: b.model[1], unit: 'B' })}
      {b.refused && <Frame name="Nicht gebacken" flex="row" w="fill">{T('Nicht gebacken: ' + b.refused + ' Das Spiel bekommt das letzte Modell.', { s: 12, c: 'text/sekundaer', width: 300 })}</Frame>}
      {BudgetX({ label: 'Terrain (BDHC)', used: b.bdhc, max: 36864, state: 'calm', unit: 'B' })}
      {BudgetX({ label: 'Gebäude', used: 0, max: 32, state: 'calm' })}
      {BudgetX({ label: 'Texturen des Gebiets', used: b.vram, max: 262144, state: 'calm', text: 'VRAM ' + de(b.vram) + ' von ' + de(262144) + ' B · ' + b.textures + ' Texturen', note: 'Vanilla höchstens ' + de(110640) + ' B' })}
    </Frame>
  </Frame> });
  const Quelle = () => Section({ title: 'Quelle', open: false, children: null });

  // ----- Draufsicht der Karte 0667 (keine Spielgrafik): Flächen statt Kacheln, damit die Datei unter ihrer Knotengrenze bleibt. Kachel K px;
  // links ein Streifen von Karte 0666 (das gebackene Modell als Platzhalter), die Zelle von 0667 mit Zellrahmen auswahl/halo; Gras, Wasser, Weg,
  // Plateau (Stufe 2) mit Wandband im Süden und Treppe im Westen wie in der Gate-Karte; preview: Zellen eines Pinselstrichs als Platten (85 %) -----
  const K = 14;
  const MalSzene = ({ w, h: H, preview = [], children }) => {
    const ox = Math.round((w - 32 * K) / 2) + 60, oy = Math.round((H - 32 * K) / 2);
    const R = (name, x0, z0, x1, z1, bg, extra = {}) => <Frame name={name} position="absolute" x={ox + x0 * K} y={oy + z0 * K} w={(x1 - x0 + 1) * K} h={(z1 - z0 + 1) * K} bg={bg} {...extra} />;
    const platten = preview.map(([x, z]) => <Frame name={'Vorschau ' + x + ', ' + z} position="absolute" x={ox + x * K} y={oy + z * K} w={K} h={K} bg={kachel('0001').pic[0]} opacity={0.85} />);
    return <Frame name="3D-Szene (Draufsicht)" w={w} h={H} bg={v('flaeche/welt')} overflow="hidden">
      <Frame name="Karte 0666 (Ausschnitt)" position="absolute" x={ox - 6 * K} y={oy} w={6 * K - 2} h={32 * K} bg="#7B9C31"><Frame position="absolute" x={20} y={180} w={40} h={30} rounded={4} bg="#4A7B5A" /><Frame position="absolute" x={0} y={290} w={84} h={16} bg="#D6C08C" /></Frame>
      {R('Boden', 0, 0, 31, 31, kachel('0000').pic[0])}
      {R('Gras', 16, 3, 28, 10, kachel('0003').pic[0])}
      {R('Wasser', 2, 24, 8, 29, kachel('0002').pic[1])}
      {R('Weg', 0, 16, 31, 16, kachel('0001').pic[1])}
      {R('Plateau (Stufe 2)', 18, 20, 22, 22, '#B5CE6B')}
      {R('Wandband', 18, 23, 22, 23, kachel('0004').pic[0], { h: 6 })}
      {R('Treppe', 16, 20, 17, 22, kachel('0005').pic[0])}
      {platten}
      <Frame name="Zellrahmen 0667" position="absolute" x={ox} y={oy} w={32 * K} h={32 * K} stroke={v('auswahl/halo')} strokeWidth={4} />
      {children}
    </Frame>;
  };

  // ----- Werkbank der Welt (wie hoehen-budget.body.js), mit eigener Statusleiste und wählbarem Reiter der Schau -----
  const SW = 740, SH = 548 - DI.reiter;
  const orte = [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 73 }),
    TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, count: 6 }),
    TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, open: false, shown: true, sel: true, count: 0 }),
    TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 }),
  ];
  const szene = (karten) => [
    TreeRowX({ label: 'Route 201', icon: 'ph:globe', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: karten.length }),
    ...karten.map(([n, at, sel]) => TreeRowX({ label: 'Karte ' + n, tech: at, icon: 'ph:mountains', depth: 2, sel })),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 1 }),
    TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 1 }),
  ];
  const Werkbank = ({ name, karten, crumbs, tab = '3D', schauInhalt, inspector, status }) => {
    const liste = WeltListe({ orte, szene: szene(karten) });
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: tab, focused: true, h: 548, bg: 'flaeche/welt', children: schauInhalt });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top', children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const insp = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left', children: inspector });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}><Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>{insp}</Frame>;
    return Window({ name, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 }, status,
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs, depth: 'Bauen', light: { e: 0, w: 101, i: 10 }, undo: 1 })}{dock}</Frame>] });
  };
  const Kopf = (n, zelle) => InspectorHeadB({ word: 'Karte', icon: 'ph:mountains', title: 'Karte ' + n, ident: 'Zelle ' + zelle + ' · Route 201', badges: [Badge('Ungespeichert', 'ungespeichert')] });
  const BUDGET = { model: [2064, 'calm'], bdhc: 216, vram: 64480, textures: 42 };
  const malStatus = (sub, info) => Statusbar({ tool: 'Gelände malen · ' + sub, keys: HINTS[sub], info, problems: '101 Warnungen, 10 Hinweise', build: 'Noch nicht gebaut' });
  const KARTEN = [['0666', '0, 0'], ['0667', '1, 0', true]];
  const CRUMBS = [['Route 201'], ['Karte 0667', 'Zelle 1, 0']];
  const STRICH = [[4, 12], [5, 12], [6, 13], [7, 13], [8, 13], [9, 14]];

  // ----- 'malen': ein Pinselstrich auf Karte 0667, Draufsicht -----
  if (MODE === 'malen') {
    return Werkbank({ name: NAMES.malen, karten: KARTEN, crumbs: CRUMBS,
      schauInhalt: MalSzene({ w: SW, h: SH, preview: STRICH, children: [SceneBarX(8, 8, { draufsicht: true }), PaintToolbar({ sub: 'Pinsel', beside: true }), LegendBtn(8, SH - 40)] }),
      inspector: [Kopf('0667', '1, 0'), <Frame name="Abschnitte" flex="col" w="fill">
        {Palette({ sel: '0001' })}
        {Layers({ rows: [{ name: 'Boden', active: true }, { name: 'Büsche', overlay: true }], next: 'Ebene 2' })}
        {TileSection({ k: kachel('0001'), kind: 0, kindName: 'NONE', blocked: false, origin: 'Von dir gesetzt', open: false })}
        {Quelle()}
        {BudgetSection(BUDGET)}
      </Frame>],
      status: malStatus('Pinsel', 'Kachel x 9, y 14 · Karte 0667 · begehbar · Höhe 0 Stufen') });
  }

  // ----- 'tileset': „Kachel 0002“ offen, Ausschnitt ohne Datei (HeartGold, Tüfteln) -----
  if (MODE === 'tileset') {
    const ausschnitt = <Frame name="Ausschnitt: ohne Datei (HeartGold, Tüfteln)" position="absolute" x={SW - 340 - 16} y={SH - 400} w={340} flex="col" rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      {TileSection({ k: kachel('0000'), kind: 0, kindName: 'NONE', blocked: false, origin: 'Nicht eindeutig, Vorgabe Boden', locked: true, hgss: true, tinker: true, pkg: '0019' })}
    </Frame>;
    return Werkbank({ name: NAMES.tileset, karten: KARTEN, crumbs: CRUMBS,
      schauInhalt: MalSzene({ w: SW, h: SH, children: [SceneBarX(8, 8, { draufsicht: true }), PaintToolbar({ sub: 'Pinsel', beside: true }), ausschnitt] }),
      inspector: [Kopf('0667', '1, 0'), <Frame name="Abschnitte" flex="col" w="fill">
        {Palette({ sel: '0002' })}
        {Layers({ rows: [{ name: 'Boden', active: true }], open: false })}
        {TileSection({ k: kachel('0002'), kind: 0x15, kindName: 'WATER_SEA', blocked: false, wall: '', origin: 'Von dir gesetzt' })}
        {Quelle()}
        {BudgetSection(BUDGET, { open: false })}
      </Frame>],
      status: malStatus('Pinsel', 'Route 201, Karte 0667 (1, 0)') });
  }

  // ----- 'entwurf': Weltkarte mit der gestrichelten Zelle, darüber der Dialog „Neue Karte malen“ -----
  if (MODE === 'entwurf') {
    // Weltkarte (32er-Zellen, Beispiel-Hex), um Route 201: die Zelle des Entwurfs (Spalte 4, Zeile 2) gestrichelt in auswahl/kante (strich/fokus, 2)
    const pic = ['#7B9C31', '#94AD42', '#A5BD5A', '#6B8C29'];
    const rows = [0, 1, 2, 3, 4].map((r) => [0, 1, 2, 3, 4, 5, 6, 7].map((c) => {
      if (r === 2 && c === 4) return <Frame name="Entwurf: Zelle 1, 0" w={32} h={32} stroke={v('auswahl/kante')} strokeWidth={2} strokeDash={[4, 3]} dashPattern={[4, 3]} flex="row" items="center" justify="center">{T('+', { s: 15, c: 'text/sekundaer' })}</Frame>;
      if ((r + c) % 5 === 0 || r === 4) return GridCell({ size: 32, plus: r === 4 && c === 4 });
      return GridCell({ size: 32, map: String(600 + r * 8 + c).padStart(4, '0'), pic: pic[(r * 3 + c) % 4], sel: r === 2 && c === 3 });
    }));
    const karte = <Frame name="Weltkarte" flex="col" w={SW} h={SH} bg={v('flaeche/schau')}>
      {WorldMapHead('Weltkarte')}
      <Frame flex="row" justify="center" p={24} w="fill">{AreaGridX(rows, 'Weltkarte')}</Frame>
    </Frame>;
    const win = Werkbank({ name: 'Fenster', karten: [['0666', '0, 0', true]], crumbs: [['Route 201'], ['Karte 0666', 'Zelle 0, 0']], tab: 'Weltkarte', schauInhalt: karte,
      inspector: [InspectorHeadB({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0666', ident: 'Zelle 0, 0 · Route 201' })],
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Route 201, Karte 0666 (0, 0)' }) });
    const DW = 480;
    const Stapel = (label, feld, hint) => <Frame name={label} flex="col" gap={4} w="fill">{T(label, { w: 600 })}{feld}{hint && T(hint, { s: 12, c: 'text/sekundaer' })}</Frame>;
    const dialog = <Frame name="Dialog „Neue Karte malen“" flex="col" gap={12} p={20} w={DW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Schließen" position="absolute" x={DW - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
      {T('Neue Karte malen', { s: 18, w: 600 })}
      {Stapel('Name', Field('Malweg'), 'Pflicht')}
      <Frame name="Boden" flex="col" gap={4} w="fill">{T('Boden', { w: 600 })}{Palette({ sel: '0000', w: DW - 56 })}</Frame>
      {Stapel('Höhe', NumField('0'), 'Stufen, −16 bis 62')}
      <Frame name="Notizen" flex="col" gap={4} w="fill">{['Neue Karte „Malweg“ in Zelle x 1, y 0 der Weltkarte 0289.', 'Gebietsdaten von Header 593.', 'Boden aus Tileset 0006, Kachel 0000, Höhe 0.'].map((s) => T(s, { s: 12, c: 'text/sekundaer', width: DW - 40 }))}</Frame>
      <Frame name="Nicht rückgängig" flex="row" items="center" gap={8} w="fill">{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}{T('Karte 0667 wird angelegt', { s: 12 })}</Frame>
      <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Karte malen', art: 'Primär' })}</Frame>
    </Frame>;
    return MitDialog(NAMES.entwurf, win, dialog);
  }

  // ----- 'brett': „Komponenten (App): Malen“ -----
  const BW = 880, TWB = BW - 32;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer', width: TWB })}{inhalt}</Frame>;
  const Karte = (inhalt) => <Frame flex="col" gap={12} p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  const Mit = (bauteil, satz, bw) => <Frame flex="row" items="center" gap={16} w="fill"><Frame flex="col" w={bw}>{bauteil}</Frame>{T(satz, { s: 12, c: 'text/sekundaer', width: TWB - bw - 16 })}</Frame>;
  const Ausschnitt = (zeilen) => <Frame name="Inspector (Ausschnitt)" flex="col" w={340} pt={2} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>{zeilen}</Frame>;
  const Leiste = (keys, hints) => <Frame name="Statusleiste (Ausschnitt)" flex="row" items="center" gap={6} px={12} h={28} rounded={4} bg={v('flaeche/gehaeuse')}>{T('Gelände malen', { s: 12, w: 600 })}{T('·', { s: 12, c: 'text/sekundaer' })}{T(keys, { s: 12, w: 600 })}{hints.map(([k, a]) => <Frame flex="row" items="center" gap={4}>{T('·', { s: 12, c: 'text/sekundaer' })}{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>;
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Malen', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Phase 6.4a: „Gelände malen“ auf einer Welt-Karte. Sieben Unterwerkzeuge mit Tiled-Kürzeln, die Palette, die Ebenen und der Tileset-Editor im Inspector der Zelle; jeder Strich ist ein Undo-Schritt. Keine neuen Tokens; neu sind die Symbole rectangle, bucket, eraser, eyedropper, ramp, eye und eyeSlash (Phosphor Rectangle, PaintBucket, Eraser, Eyedropper, Stairs, Eye, EyeSlash).', { c: 'text/sekundaer', width: BW })}
    {Zustand('Unterwerkzeuge: in Ruhe (oben) und mit „Pinsel“ gedrückt in der flachen Leiste (unten); genau eins gedrückt, Symbol gefüllt, auswahl/flaeche mit auswahl/kante', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame name="In Ruhe" flex="row" gap={4}>{UNTER.map((t) => ToolKey(t, false))}</Frame>, 'Pinsel B, Rechteck P, Füllen G, Radierer E, Pipette I, Höhe H, Rampe R. Tooltip: Name, Taste, darunter die Wirkung.', 7 * 44)}
      {PaintToolbarFlat('Pinsel')}
      {T('Aufrecht stehen sie unter einem Trennstrich unter den Werkzeugen; ist die Szene niedriger als 773 px (Kompakt 657 px), als zweite Spalte rechts daneben, oben bündig.', { s: 12, c: 'text/sekundaer', width: TWB })}
    </Frame>))}
    {Zustand('Statusleiste je Unterwerkzeug (ToolHints); Tüfteln hängt „Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“ an', Karte(<Frame flex="col" gap={6} w="fill">{UNTER.map(([n]) => Leiste(n, HINTS[n]))}</Frame>))}
    {Zustand('Palettenkacheln: mit Bild (die ganze Textur, ohne Glättung), ohne Bild in der Platzhalterfarbe der Rolle, gewählt, gewählt mit Marke der Drehung und Spiegelung, Tüfteln mit Nummer', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame flex="row" gap={8}>{KACHELN.map((k) => PalTile(k))}</Frame>, 'Mit Bild. Tooltip „Kachel 0003 · gras_a · 32 × 32 Texel · Gras“.', 7 * 40)}
      {Mit(<Frame flex="row" gap={8}>{KACHELN.map((k) => PalTile(k, { pic: false }))}</Frame>, 'Ohne Bild (synthetisches Tileset, Format 5): szene/gelaende/unbekannt, hohes-gras, wasser, blockiert, spezial.', 7 * 40)}
      {Mit(<Frame flex="row" gap={8}>{PalTile(kachel('0001'), { sel: true })}{PalTile(kachel('0001'), { sel: true, mark: '90° ⇆' })}{PalTile(kachel('0003'), { tinker: true })}</Frame>, 'Gewählt (aria-pressed): Rand und Fokusstrich auswahl/kante; die Marke „90°“, „⇆“ oder beides; Tüfteln die Nummer darunter und im Tooltip „kind 0x02, Kollision 0x00“.', 3 * 40)}
    </Frame>))}
    {Zustand('Ebenen: Radio „Aktiv“ (der Pinsel malt dorthin), Name und Rolle, Auge nur für die Szene, „Entfernen“ nur bei Überlagerungen', Karte(<Frame flex="row" gap={16} items="start" w="fill">
      {Ausschnitt([Layers({ rows: [{ name: 'Boden', active: true }, { name: 'Büsche', overlay: true }], next: 'Ebene 2' })])}
      {Ausschnitt([Layers({ rows: [{ name: 'Boden' }, { name: 'Büsche', overlay: true, active: true, hidden: true }], next: 'Ebene 2' })])}
    </Frame>))}
    {Zustand('Budget einer Welt-Karte nach einem abgelehnten Backen: „Modell“ zeigt das letzte Modell, darunter der Satz (data-budget-refused)', Karte(<Frame flex="row" gap={16} items="start" w="fill">
      {Ausschnitt([BudgetSection({ model: [61440, 'full'], bdhc: 4120, vram: 64480, textures: 42, refused: 'Das gebackene Modell hat 63 120 Byte, das Spiel fasst höchstens 61 440 Byte; die Karte behält ihr letztes Modell.' })])}
      {T('Der Satz nimmt die erste Meldung der Stufe „error“ (sonst die erste) und ergänzt einen fehlenden Schlusspunkt; ohne Satz „Nicht gebacken. Das Spiel bekommt das letzte Modell.“ Die Zahlen sind Beispielwerte.', { s: 12, c: 'text/sekundaer', width: TWB - 356 })}
    </Frame>))}
    {Zustand('Binäre Karte beim Malen: oben im Inspector die Notiz mit dem Weg zur Welt-Karte (data-paint-binary, data-paint-adopt)', Karte(<Frame flex="col" gap={8} w={340}>
      {Notice('Boden malen: Karte als Quelle übernehmen', 'info')}
      <Frame flex="row">{Btn({ label: 'Karte als Quelle übernehmen', small: true })}</Frame>
    </Frame>))}
  </Frame>;
