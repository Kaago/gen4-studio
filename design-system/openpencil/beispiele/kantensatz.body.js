  // Smart Drawing (Phase 6.4b), zwei Frames und ein Brett aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'kantensatz' „Gelände malen: Kantensatz-Editor“: Werkbank der Welt, in der Schau Karte 0666 von oben (Draufsicht gedrückt) mit dem Plus aus
  //                dem Satz „Weg“ (als Platz-Raster gezeichnet: jede Zelle in vier Viertel, innen Weg, außen Boden) und einer Klippe von zwei Stufen
  //                mit dem Rand oben und dem Wandband im Süden; die aufrechte Werkzeugleiste mit „Gelände malen“ gedrückt und den neun Unterwerkzeugen
  //                als zweite Spalte daneben, „Terrain-Pinsel“ (T) gedrückt; im Inspector „Karte 0666“ mit „Palette“ (Platz „Kante N“ gewählt: die
  //                Notiz „Ein Klick hängt …“), „Ebenen“, „Kachel 0007“ eingeklappt, „Kantensätze“ offen mit dem Editor des Satzes „Weg“, „Quelle“
  //                eingeklappt; Statusleiste „Gelände malen · Terrain-Pinsel · Ziehen malt „Weg“ · ⇧Ziehen nimmt es weg“. Mit --dunkel gebaut heißt
  //                der Frame „Gelände malen: Kantensatz-Editor (Dunkel)“
  //   'muster'     „Gelände-Muster“: dieselbe Werkbank, in der Schau ein Ausschnitt von oben mit der Schicht „Gelände“ (gedrückt): Blockiert schraffiert,
  //                Wasser Wellen, Hohes Gras Punkte, Vorsprung Pfeile nach Süden, Eis Kreuzschraffur, Spezial und Unbekannt ohne Muster; unten links die
  //                Legende offen mit „Gelände“ (jede Farbe mit ihrem Muster) und „Ereignisse“
  //   'brett'      „Komponenten (App): Kantensatz“ (Seite „Komponenten“): Platz leer, gefüllt, gewählt, mit Varianten-Zähler, in Tüfteln mit Token;
  //                Varianten-Chip; Segment „Obere Zelle“/„Untere Zelle“; die Zeile eines Satzes mit Zähler; Musterproben; die Unterwerkzeuge T und K und
  //                ihre Statusleisten
  // Quellen: apps/studio/src/workbench/{paintTools.ts,paint/PaintContext.tsx}, world/paint/{ops,usePaintSession}.ts, workbench/inspector/{EdgeSetEditor.tsx,
  // edgeSets.ts,EdgeSetEditor.module.css,TilePalette.tsx,MatrixCellEditor.tsx}, world/scene/patterns.ts, world/legend.ts, workbench/{Legend.tsx,Legend.module.css},
  // ui/primitives/icons.ts, ui/gallery/pages/WorldPage.tsx (Blöcke „Gelände-Muster“, „Kantensatz-Editor“). Texte aus design-system/README.md, „Nachtrag 6.4b“.
  // Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, nicht aus dem ROM: die Kacheln sind die des synthetischen Tilesets `Tileset::synthetic_edges()`
  // (0000 bis 0005 wie in 6.4a mit Beispielnamen, 0006 bis 0015 die Kantenkacheln syn_kante_00 bis syn_kante_15), der Satz ist `weg13` („Weg“, außen
  // 0000, Mitte 0006, Kanten 0007 bis 000a, Außenecken 000b bis 000e, Innenecken 000f bis 0012); ihre „Bilder“ sind Platzhalter: eine Kantenkachel zeigt
  // ihre Maske als vier Viertel (innen Weg, außen Boden). Das Plus (x 4–12 z 8–10 und x 7–9 z 5–13) und die Klippe (x 20–25 z 20–24, Stufe 2) folgen
  // dem festen Musterfall und dem Browser-Teil (target/agent/gate-2026-11-08/6-4b/browser.md), Karte 0666 in Zelle 1, 21 über Fleetburg.
  // Nicht gerendert: OpenPencil antwortete beim Schreiben nicht (127.0.0.1:7600, HTTP 502). Die Vorlage ist gegen jsx-lib-app.js geschrieben und mit
  // einer Attrappe der Bausteine durchgelaufen, aber nie als Bild gesehen (Anleitung: openpencil/NACHTRAG-6-4b.md).
  // Dunkel: bash ../build-screen.sh kantensatz --dunkel. Szenenfarben heißen in OpenPencil ohne Präfix: v('gelaende/wasser'), v('hoehe/2').
  const MODE = 'kantensatz';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { kantensatz: DARK ? 'Gelände malen: Kantensatz-Editor (Dunkel)' : 'Gelände malen: Kantensatz-Editor', muster: 'Gelände-Muster', brett: 'Komponenten (App): Kantensatz' };
  const hex4 = (n) => n.toString(16).padStart(4, '0');

  // ----- Unterwerkzeuge (paintTools.ts): die sieben von 6.4a und T, K; gezeichnet wie die Werkzeuge (ToolKey), gedrückt mit gefülltem Symbol -----
  const UNTER = [['Pinsel', 'ph:paint-brush', 'B'], ['Rechteck', 'ph:rectangle', 'P'], ['Füllen', 'ph:paint-bucket', 'G'], ['Radierer', 'ph:eraser', 'E'], ['Pipette', 'ph:eyedropper', 'I'], ['Höhe', 'ph:mountains', 'H'], ['Rampe', 'ph:stairs', 'R'], ['Terrain-Pinsel', 'ph:mountains', 'T'], ['Klippenpinsel', 'ph:stack', 'K']];
  const subKeys = (sub) => UNTER.map(([n, i, k]) => ToolKey([n, n === sub ? i + '-fill' : i, k], n === sub));
  // Statusleiste von T und K (paintStatusHints): mit Satz, ohne Satz
  const HINTS = {
    'Terrain-Pinsel': [['Ziehen', 'malt „Weg“'], ['⇧Ziehen', 'nimmt es weg']],
    Klippenpinsel: [['Ziehen', 'hebt um 2 Stufen mit dem Rand „Weg“'], ['⇧Ziehen', 'senkt']],
    'Terrain-Pinsel ohne Satz': [['Ziehen', 'malt (erst einen Kantensatz wählen)'], ['⇧Ziehen', 'nimmt es weg']],
    'Klippenpinsel ohne Satz': [['Ziehen', 'hebt um 2 Stufen (erst einen Kantensatz wählen)'], ['⇧Ziehen', 'senkt']],
  };
  // Werkzeugleiste mit „Gelände malen“ gedrückt, aufrecht, die Unterwerkzeuge als zweite Spalte rechts daneben (data-paint-beside), wie in 6.4a
  const PaintToolbar = ({ sub, x = 8, y = 56 }) => {
    const sep = () => <Frame name="Trenner" w={48} h={1} bg={v('linie/leise')} />;
    const keys = WERKZEUGE.flatMap((g, gi) => [...(gi > 0 ? [sep()] : []), ...g.map((t) => ToolKey(t, t[0] === 'Gelände malen'))]);
    return <Frame name="Werkzeugleiste" position="absolute" x={x} y={y} flex="col" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame name="Griff und Lage" flex="row" h={24}><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:dots-six', DI.symbol, 's')}</Frame><Frame w={24} h="fill" flex="row" items="center" justify="center">{I('ph:caret-down', DI.symbol, 's')}</Frame></Frame>
      {keys}
      <Frame name="Unterwerkzeuge (daneben)" position="absolute" x={48 + 4} y={-1} flex="col" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>{subKeys(sub)}</Frame>
    </Frame>;
  };
  // Ebenen der Schau (SceneBar): „Gelände“, „Höhen“, „Ereignisse“, Trenner, „Draufsicht“
  const SceneBarX = (x, y, on = {}) => <Frame name="Ebenen der Schau" position="absolute" x={x} y={y} flex="row" items="center" gap={4} p={4} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Ebenen" flex="row" gap={4}>{IconBtn('ph:stack', { on: !!on.gelaende })}{IconBtn('ph:mountains', { on: !!on.hoehen })}{IconBtn('ph:map-pin', { on: on.ereignisse !== false })}</Frame>
    <Frame name="Trenner" w={1} h={24} bg={v('linie/leise')} />
    <Frame name="Ansichten" flex="row" gap={4}>{IconBtn('ph:drone', { on: !!on.draufsicht })}</Frame>
  </Frame>;

  // ----- Kacheln: synthetic_edges() mit Beispielnamen; „Bilder“ Platzhalter (keine Spielgrafik) -----
  // Zwei Farben des Satzes: innen (Weg) und außen (Boden); die Kantenkachel einer Maske zeigt ihre vier Ecken als Viertel (NW 1, NE 2, SW 4, SE 8)
  const INNEN = '#D6C08C', AUSSEN = '#A5BD5A', INNEN_DUNKEL = '#C6A56B', AUSSEN_DUNKEL = '#94AD42';
  const ROLLE = { ground: ['Boden', 'gelaende/unbekannt'], grass: ['Gras', 'gelaende/hohes-gras'], water: ['Wasser', 'gelaende/wasser'], wall: ['Wand', 'gelaende/blockiert'], ramp: ['Rampe', 'gelaende/unbekannt'], piece: ['Teilmodell', 'gelaende/spezial'] };
  // Platz, Maske und Name (edge.rs Place, edgeSets.ts PLACE_LABEL); weg13 belegt die 13 ersten mit 0006 bis 0012
  const PLAETZE = [['center', 15, 'Mitte'], ['n', 12, 'Kante N'], ['e', 5, 'Kante O'], ['s', 3, 'Kante S'], ['w', 10, 'Kante W'], ['outer_nw', 8, 'Ecke außen NW'], ['outer_ne', 4, 'Ecke außen NO'], ['outer_sw', 2, 'Ecke außen SW'], ['outer_se', 1, 'Ecke außen SO'], ['inner_nw', 14, 'Ecke innen NW'], ['inner_ne', 13, 'Ecke innen NO'], ['inner_sw', 11, 'Ecke innen SW'], ['inner_se', 7, 'Ecke innen SO'], ['diag_nwse', 9, 'Diagonale NW–SO'], ['diag_nesw', 6, 'Diagonale NO–SW']];
  const PLATZ = Object.fromEntries(PLAETZE.map(([p, m, l]) => [p, { mask: m, label: l }]));
  const KACHELN = [
    { n: '0000', tex: 'boden', role: 'ground', pic: [AUSSEN, AUSSEN_DUNKEL] },
    { n: '0001', tex: 'gras_a', role: 'grass', pic: ['#5A7B21', '#6B8C29'] },
    { n: '0002', tex: 'wasser_a', role: 'water', pic: ['#4A8CD6', '#3173C6'] },
    { n: '0003', tex: 'fels', role: 'wall', pic: ['#8C6B4A', '#73563A'] },
    { n: '0004', tex: 'treppe', role: 'ramp', pic: ['#C6CEB5', '#ADB59C'] },
    { n: '0005', tex: 'baum', role: 'piece', pic: ['#3A6B42', '#2D5A35'] },
    // syn_kante_00 bis _15: 0006 bis 0012 tragen die Masken der 13 Plätze von weg13, 0013 bis 0015 (weg15: Diagonalen, zweite Mitte) Beispielmasken
    ...Array.from({ length: 16 }, (_, i) => ({ n: hex4(6 + i), tex: 'syn_kante_' + String(i).padStart(2, '0'), role: 'ground', mask: i < 13 ? PLAETZE[i][1] : [9, 6, 15][i - 13] })),
  ];
  const kachel = (n) => KACHELN.find((k) => k.n === n);
  // weg13 („Weg“): je Platz die Token; „Fels“ ein leerer Satz mit block lower (wie die Galerie)
  const WEG = Object.fromEntries(PLAETZE.slice(0, 13).map(([p], i) => [p, [hex4(6 + i)]]));
  // Bild einer Kachel s × s: Kantenkachel als vier Viertel, sonst zwei Beispielfarben im Schachbrett
  const Bild = (k, s = 30) => {
    if (k.mask !== undefined) {
      const half = s / 2;
      const q = [[1, 0, 0], [2, half, 0], [4, 0, half], [8, half, half]].filter(([bit]) => (k.mask & bit) !== 0);
      return <Frame name={'Bild ' + k.n} w={s} h={s} bg={AUSSEN}>{q.map(([bit, x, y]) => <Frame name={'innen ' + bit} position="absolute" x={x} y={y} w={half} h={half} bg={INNEN} />)}</Frame>;
    }
    return <Frame name={'Bild ' + k.n} w={s} h={s} bg={k.pic[0]}><Frame position="absolute" x={0} y={s / 2} w={s / 2} h={s / 2} bg={k.pic[1]} /><Frame position="absolute" x={s / 2} y={0} w={s / 2} h={s / 2} bg={k.pic[1]} /></Frame>;
  };
  // Taste der Palette (TilePalette, wie 6.4a): 32 × 32, Rand linie/leise, gewählt auswahl/kante 3, Tüfteln mit Nummer
  const PalTile = (k, o = {}) => {
    const taste = <Frame name={'Kachel ' + k.n} w={32} h={32} rounded={4} bg={v('flaeche/taste')} stroke={v(o.sel ? 'auswahl/kante' : 'linie/leise')} strokeWidth={o.sel ? 3 : 1} overflow="hidden" flex="row" items="center" justify="center">{Bild(k)}</Frame>;
    return o.tinker ? <Frame flex="col" items="center" gap={2}>{taste}{Mono(k.n, { s: 10, lh: 12 })}</Frame> : taste;
  };
  // Abschnitt „Palette“: Gruppen nach Rolle, je Reihe höchstens neun Kacheln (die App bricht mit flex-wrap um); target: Notiz des gewählten Platzes
  const Palette = ({ sel, target, open = true }) => {
    const groups = ['ground', 'grass', 'water', 'wall', 'ramp', 'piece'].map((role) => [role, KACHELN.filter((k) => k.role === role)]).filter(([, ks]) => ks.length > 0);
    const reihen = (ks) => Array.from({ length: Math.ceil(ks.length / 9) }, (_, r) => ks.slice(r * 9, r * 9 + 9));
    return Section({ title: 'Palette', open, children: <Frame name="Palette" flex="col" gap={8} px={8} pt={4} pb={8} w="fill">
      {target && <Frame name="Ziel (data-palette-target)" flex="row" w="fill">{T('Ein Klick hängt die Kachel (mit Drehung) an den Platz „' + target + '“.', { s: 12, c: 'text/sekundaer', width: 316 })}</Frame>}
      {groups.map(([role, ks]) => <Frame name={'Gruppe ' + ROLLE[role][0]} flex="col" gap={4} w={324}>
        {T(ROLLE[role][0], { s: 12, c: 'text/sekundaer' })}
        {reihen(ks).map((reihe) => <Frame flex="row" gap={4} w={324}>{reihe.map((k) => PalTile(k, { sel: k.n === sel }))}</Frame>)}
      </Frame>)}
    </Frame> });
  };
  // Radio „Aktiv“ (natives Radio, accent-color akzent/grund), wie 6.4a
  const Radio = (on) => <Frame name={on ? 'Aktiv (an)' : 'Aktiv'} w={16} h={16} rounded={8} bg={v('flaeche/blatt')} stroke={v(on ? 'akzent/grund' : 'linie/stark')} strokeWidth={on ? 2 : 1} flex="row" items="center" justify="center">{on && <Frame w={8} h={8} rounded={4} bg={v('akzent/grund')} />}</Frame>;
  const Layers = ({ open = true } = {}) => Section({ title: 'Ebenen', open, children: <Frame name="Ebenen" flex="col" gap={6} px={8} pt={4} pb={8} w="fill">
    <Frame name="Ebene Boden" flex="row" items="center" gap={6} h={DI.steuer} w="fill">{Radio(true)}{T('Boden · Boden', { grow: 1, one: true })}{IconBtn('ph:eye')}</Frame>
    <Frame name="Hinzufügen" flex="row" items="center" gap={6} w="fill">{Field('Ebene 1', { grow: 1 })}{Btn({ label: 'Ebene hinzufügen', small: true })}</Frame>
  </Frame> });
  const Zu = (title) => Section({ title, open: false, children: null });

  // ----- Abschnitt „Kantensätze“ (EdgeSetEditor) -----
  // Zähler „13 von 13 Plätzen · Diagonalen 0 von 2“
  const zaehler = (places) => {
    const voll = PLAETZE.slice(0, 13).filter(([p]) => (places[p] || []).length > 0).length;
    const diag = PLAETZE.slice(13).filter(([p]) => (places[p] || []).length > 0).length;
    return voll + ' von 13 Plätzen · Diagonalen ' + diag + ' von 2';
  };
  // Zeile eines Satzes (data-edge-set): Radio, Name (text/s), Zähler (text/xs sekundär)
  const SetRow = (name, places, active) => <Frame name={'Satz ' + name} flex="row" items="center" gap={8} h={DI.steuer} w="fill">{Radio(active)}{T(name, { grow: 1, one: true })}{T(zaehler(places), { s: 12, c: 'text/sekundaer' })}</Frame>;
  // Platz (data-edge-place): 40 × 40 (raum/40), Rand linie/leise, radius/taste 6, Fläche flaeche/taste; leer gestrichelt ohne Fläche; gewählt auswahl/kante
  // mit Fokusstrich; mit mehreren Kacheln der Zähler unten rechts (auswahl/kante, akzent/auf, text/xs); Tüfteln die Token darunter (code/s)
  const Platz = (p, tokens = [], o = {}) => {
    const leer = tokens.length === 0;
    const taste = <Frame name={PLATZ[p].label + (leer ? ' (leer)' : '')} w={40} h={40} rounded={6} bg={leer ? undefined : v('flaeche/taste')} stroke={v(o.sel ? 'auswahl/kante' : 'linie/leise')} strokeWidth={o.sel ? 3 : 1} strokeDash={leer ? [4, 3] : undefined} dashPattern={leer ? [4, 3] : undefined} overflow="hidden" flex="row" items="center" justify="center">
      {!leer && Bild(kachel(tokens[0].slice(0, 4)), 38)}
      {tokens.length > 1 && <Frame name="Zähler" position="absolute" x={40 - 12} y={40 - 15} px={2} h={15} bg={v('auswahl/kante')} flex="row" items="center">{T(String(tokens.length), { s: 12, lh: 15, c: 'akzent/auf' })}</Frame>}
    </Frame>;
    return <Frame name={'Platz ' + p} flex="col" items="center" gap={2}>{taste}{o.tinker && tokens.length > 0 && Mono(tokens.join(' '), { s: 10, lh: 12 })}</Frame>;
  };
  // Raster der Plätze: Block 3 × 3, Block 2 × 2, Zeile der Diagonalen
  const BLOECKE = [[['outer_nw', 'n', 'outer_ne'], ['w', 'center', 'e'], ['outer_sw', 's', 'outer_se']], [['inner_nw', 'inner_ne'], ['inner_sw', 'inner_se']], [['diag_nwse', 'diag_nesw']]];
  const Raster = (places, sel, tinker) => <Frame name="Plätze" flex="row" items="start" gap={16} w="fill">
    {BLOECKE.map((rows, b) => <Frame name={'Block ' + (b + 1)} flex="col" gap={4}>{rows.map((row) => <Frame flex="row" gap={4}>{row.map((p) => Platz(p, places[p], { sel: p === sel, tinker }))}</Frame>)}</Frame>)}
  </Frame>;
  // Varianten-Chip (data-edge-variant): Höhe raum/24, Rand linie/standard, radius/plakette, Fläche flaeche/taste, Bild 16 × 16, „Kachel 0007“, „Entfernen“ leise
  const Chip = (token, tinker) => <Frame name={'Variante ' + token} flex="row" items="center" gap={4} pl={4} h={24} rounded={12} bg={v('flaeche/taste')} stroke={v('linie/standard')} strokeWidth={1}>
    <Frame w={16} h={16} rounded={4} overflow="hidden">{Bild(kachel(token.slice(0, 4)), 16)}</Frame>
    {T(tinker ? token : 'Kachel ' + token.slice(0, 4), { s: 12 })}
    {Btn({ label: 'Entfernen', art: 'Leise', small: true })}
  </Frame>;
  const BLOCK = (b) => SegS(['Obere Zelle', 'Untere Zelle'], b === 'lower' ? 'Untere Zelle' : 'Obere Zelle');
  const EdgeSets = ({ sets, active, sel, rise = 2, tinker, open = true }) => {
    const set = sets.find((s) => s.name === active);
    return Section({ title: 'Kantensätze', open, children: <Frame name="Kantensätze" flex="col" gap={8} px={8} pt={4} pb={8} w="fill">
      <Frame name="Sätze" flex="col" gap={2} w="fill">{sets.map((s) => SetRow(s.name, s.places, s.name === active))}</Frame>
      <Frame name="Neuer Kantensatz" flex="row" items="center" gap={6} w="fill">{Field('Satz ' + (sets.length + 1), { grow: 1 })}{Btn({ label: 'Neuer Kantensatz', small: true })}</Frame>
      <Frame name="Stufen der Klippe (data-cliff-rise)" flex="col" w="fill">{Row({ label: 'Stufen der Klippe', hint: '1 bis 8, je Stufe 8 E', children: NumField(String(rise)) })}</Frame>
      {set && <Frame name="Editor (data-edge-editor)" flex="col" gap={8} pt={8} w="fill" strokeWeights={{ top: 1, right: 0, bottom: 0, left: 0 }} stroke={v('linie/leise')}>
        {Raster(set.places, sel, tinker)}
        {sel ? <Frame name={'Varianten ' + sel} flex="col" gap={4} w="fill">
          {T(PLATZ[sel].label, { s: 12, c: 'text/sekundaer' })}
          {(set.places[sel] || []).length === 0 ? T('Noch keine Kachel: ein Klick in der Palette hängt eine an.', { s: 12, c: 'text/sekundaer', width: 316 }) : <Frame flex="row" gap={4} w="fill">{set.places[sel].map((t) => Chip(t, tinker))}</Frame>}
        </Frame> : T('Einen Platz wählen, dann eine Kachel in der Palette: sie kommt mit ihrer Drehung (Z, X) in den Platz.', { s: 12, c: 'text/sekundaer', width: 316 })}
        <Frame name="Felder" flex="col" w="fill" ml={-8}>
          {Row({ label: 'Außen', children: [<Frame w={16} h={16} rounded={4} overflow="hidden">{Bild(kachel(set.outside), 16)}</Frame>, T('Kachel ' + set.outside, { grow: 1, one: true }), Btn({ label: 'Aktive Kachel übernehmen', art: 'Leise', small: true })] })}
          {Row({ label: 'Blockiert an der Klippe', children: BLOCK(set.block) })}
          {Row({ label: 'Wand', hint: 'Für jede Kachel der Plätze; Kachelnummer, leer = keine', children: Field('0003') })}
        </Frame>
        <Frame flex="row">{Btn({ label: 'Satz entfernen', art: 'Leise', small: true })}</Frame>
      </Frame>}
    </Frame> });
  };
  const SAETZE = [{ name: 'Weg', outside: '0000', places: WEG }, { name: 'Fels', outside: '0000', block: 'lower', places: {} }];

  // ----- Draufsicht der Karte 0666 (keine Spielgrafik): Boden als eine Fläche, nur die Zellen des Satzes einzeln (Viertel wie die Bilder) -----
  const K = 14;
  // Masken aus Ecken: innen(cx, cz) sagt, ob Ecke (cx, cz) innen ist; Zelle (x, z) hat NW (x, z), NE (x+1, z), SW (x, z+1), SE (x+1, z+1)
  const maskOf = (innen, x, z) => (innen(x, z) ? 1 : 0) | (innen(x + 1, z) ? 2 : 0) | (innen(x, z + 1) ? 4 : 0) | (innen(x + 1, z + 1) ? 8 : 0);
  // Das Plus (Terrain-Pinsel): eine Ecke ist innen, wenn eine gemalte Zelle sie berührt
  const plus = (x, z) => (z >= 8 && z <= 10 && x >= 4 && x <= 12) || (x >= 7 && x <= 9 && z >= 5 && z <= 13);
  const plusEcke = (cx, cz) => plus(cx - 1, cz - 1) || plus(cx, cz - 1) || plus(cx - 1, cz) || plus(cx, cz);
  // Die Klippe x 20–25, z 20–24 auf Stufe 2: eine Ecke ist innen, wenn alle angrenzenden Zellen oben liegen
  const oben = (x, z) => x >= 20 && x <= 25 && z >= 20 && z <= 24;
  const klippeEcke = (cx, cz) => oben(cx - 1, cz - 1) && oben(cx, cz - 1) && oben(cx - 1, cz) && oben(cx, cz);
  const Zelle = (name, ox, oy, x, z, mask, innen = INNEN, aussen = AUSSEN) => {
    const half = K / 2;
    if (mask === 15) return <Frame name={name} position="absolute" x={ox + x * K} y={oy + z * K} w={K} h={K} bg={innen} />;
    return <Frame name={name} position="absolute" x={ox + x * K} y={oy + z * K} w={K} h={K} bg={aussen}>{[[1, 0, 0], [2, half, 0], [4, 0, half], [8, half, half]].filter(([b]) => (mask & b) !== 0).map(([b, qx, qy]) => <Frame position="absolute" x={qx} y={qy} w={half} h={half} bg={innen} />)}</Frame>;
  };
  const MalSzene = ({ w, h: H, children }) => {
    const ox = Math.round((w - 32 * K) / 2) + 60, oy = Math.round((H - 32 * K) / 2);
    const plusZellen = [];
    for (let z = 4; z <= 14; z++) for (let x = 3; x <= 13; x++) { const m = maskOf(plusEcke, x, z); if (m) plusZellen.push(Zelle('Plus ' + x + ', ' + z, ox, oy, x, z, m)); }
    // Klippe: das Plateau eine Stufe heller, der Rand oben aus den Plätzen (Viertel); im Süden das Wandband (Wand der oberen Kachel)
    const klippe = [];
    for (let z = 20; z <= 24; z++) for (let x = 20; x <= 25; x++) klippe.push(Zelle('Klippe ' + x + ', ' + z, ox, oy, x, z, maskOf(klippeEcke, x, z), INNEN_DUNKEL, AUSSEN_DUNKEL));
    return <Frame name="3D-Szene (Draufsicht)" w={w} h={H} bg={v('flaeche/welt')} overflow="hidden">
      <Frame name="Karte 0048 (Ausschnitt)" position="absolute" x={ox} y={oy + 32 * K + 2} w={32 * K} h={6 * K} bg="#7B9C31" />
      <Frame name="Boden 0000" position="absolute" x={ox} y={oy} w={32 * K} h={32 * K} bg={AUSSEN} />
      {plusZellen}
      {klippe}
      <Frame name="Wandband (Süd)" position="absolute" x={ox + 20 * K} y={oy + 25 * K} w={6 * K} h={6} bg={kachel('0003').pic[0]} />
      <Frame name="Zellrahmen 0666" position="absolute" x={ox} y={oy} w={32 * K} h={32 * K} stroke={v('auswahl/halo')} strokeWidth={4} />
      {children}
    </Frame>;
  };

  // ----- Werkbank der Welt (wie gelaende-malen.body.js), Fleetburg mit Karte 0666 in Zelle 1, 21 -----
  const SW = 740, SH = 548 - DI.reiter;
  const orte = [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 61 }),
    TreeRowX({ label: 'Jubelstadt', icon: 'ph:globe', depth: 1, open: false, count: 35 }),
    TreeRowX({ label: 'Fleetburg', icon: 'ph:globe', depth: 1, open: false, shown: true, sel: true, count: 13 }),
    TreeRowX({ label: 'Erzelingen', icon: 'ph:globe', depth: 1, open: false, count: 25 }),
  ];
  const szene = [
    TreeRowX({ label: 'Fleetburg', icon: 'ph:globe', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: true, count: 3 }),
    TreeRowX({ label: 'Karte 0666', tech: '1, 21', icon: 'ph:mountains', depth: 2, sel: true }),
    TreeRowX({ label: 'Karte 0048', tech: '1, 22', icon: 'ph:mountains', depth: 2 }),
    TreeRowX({ label: 'Karte 0049', tech: '1, 23', icon: 'ph:mountains', depth: 2 }),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 19 }),
  ];
  const Werkbank = ({ name, schauInhalt, inspector, status, undo }) => {
    const liste = WeltListe({ orte, szene });
    const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: true, h: 548, bg: 'flaeche/welt', children: schauInhalt });
    const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: 240, edge: 'top', children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
    const insp = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left', children: inspector });
    const dock = <Frame name="Dock Welt" flex="row" w="fill" grow={1}><Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>{insp}</Frame>;
    return Window({ name, active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 2 }, status,
      children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{ScharnierOben({ crumbs: [['Fleetburg'], ['Karte 0666', 'Zelle 1, 21']], depth: 'Bauen', light: { e: 0, w: 101, i: 162 }, undo })}{dock}</Frame>] });
  };
  const Kopf = () => InspectorHeadB({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0666', ident: 'Zelle 1, 21 · Fleetburg', badges: [Badge('Ungespeichert', 'ungespeichert')] });
  const malStatus = (sub, info) => Statusbar({ tool: 'Gelände malen · ' + sub, keys: HINTS[sub], info, problems: '101 Warnungen, 162 Hinweise', build: 'Noch nicht gebaut' });

  // ----- 'kantensatz': das Plus und die Klippe, der Editor des Satzes „Weg“ mit dem Platz „Kante N“ gewählt -----
  if (MODE === 'kantensatz') {
    return Werkbank({ name: NAMES.kantensatz, undo: 21,
      schauInhalt: MalSzene({ w: SW, h: SH, children: [SceneBarX(8, 8, { draufsicht: true }), PaintToolbar({ sub: 'Terrain-Pinsel' }), LegendBtn(8, SH - 40)] }),
      inspector: [Kopf(), <Frame name="Abschnitte" flex="col" w="fill">
        {Palette({ sel: '0007', target: 'Kante N' })}
        {Layers({ open: false })}
        {Zu('Kachel 0007')}
        {EdgeSets({ sets: SAETZE, active: 'Weg', sel: 'n' })}
        {Zu('Quelle')}
      </Frame>],
      status: malStatus('Terrain-Pinsel', 'Kachel x 8, y 9 · Karte 0666 · begehbar · Höhe 0 Stufen') });
  }

  // ----- Muster der Schicht „Gelände“ (patterns.ts): die Linien sind die Zellfarbe × 0,55; hier Schwarz mit 45 % darüber (dieselbe Farbe) -----
  // Zeichen je Muster für Zellen und Legende: Schraffur ╱, Wellen ∿, Punkte ⋮ (3 × 3), Pfeil ↓ (Vorsprung nach Süden), Kreuzschraffur ╳
  const KLASSEN = [
    ['Blockiert', 'gelaende/blockiert', 'Schraffur', '╱'],
    ['Wasser', 'gelaende/wasser', 'Wellen', '∿'],
    ['Hohes Gras', 'gelaende/hohes-gras', 'Punkte', '⁘'],
    ['Vorsprung', 'gelaende/vorsprung', 'Pfeil', '↓'],
    ['Eis', 'gelaende/eis', 'Kreuzschraffur', '╳'],
    ['Spezial', 'gelaende/spezial', null, null],
    ['Unbekannt', 'gelaende/unbekannt', null, null],
  ];
  const klasse = (name) => KLASSEN.find((k) => k[0] === name);
  const Muster = (zeichen, s) => <Frame name="Muster" position="absolute" x={0} y={0} w={s} h={s} opacity={0.45} flex="row" items="center" justify="center">{T(zeichen, { s: Math.round(s * 0.8), lh: s, hex: '#000000', align: 'center' })}</Frame>;
  const Probe = (name, s = 16) => { const [, tok, , z] = klasse(name); return <Frame name={'Probe ' + name} w={s} h={s} rounded={3} bg={v(tok)} overflow="hidden">{z && Muster(z, s)}</Frame>; };

  // ----- 'muster': ein Ausschnitt mit allen fünf Mustern von oben, Schicht „Gelände“ gedrückt, die Legende offen -----
  if (MODE === 'muster') {
    const G = 28, ox = 120, oy = 40;
    // Ausschnitt 20 × 14 Zellen: Bäume (blockiert) am Rand, ein Teich, Gras, eine Vorsprungkante nach Süden, Eis, Spezial (Tür), sonst Unbekannt (Boden)
    const art = (x, z) => {
      if (x === 0 || z === 0 || x === 19) return 'Blockiert';
      if (x >= 12 && x <= 17 && z >= 2 && z <= 6) return 'Wasser';
      if (x >= 2 && x <= 7 && z >= 2 && z <= 5) return 'Hohes Gras';
      if (z === 8 && x >= 1 && x <= 10) return 'Vorsprung';
      if (x >= 13 && x <= 17 && z >= 9 && z <= 12) return 'Eis';
      if (x === 4 && z === 11) return 'Spezial';
      return 'Unbekannt';
    };
    const zellen = [];
    for (let z = 0; z < 14; z++) for (let x = 0; x < 20; x++) {
      const [name, tok, , zeichen] = klasse(art(x, z));
      if (name === 'Unbekannt') continue;
      zellen.push(<Frame name={name + ' ' + x + ', ' + z} position="absolute" x={ox + x * G} y={oy + z * G} w={G} h={G} bg={v(tok)}>{zeichen && Muster(zeichen, G)}</Frame>);
    }
    const legende = <Frame name="Legende (offen)" position="absolute" x={8} y={SH - 8 - 300} w={200} flex="col" gap={8} p={12} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
      <Frame flex="row" items="center" gap={8} w="fill">{I('ph:list-bullets', DI.symbol, 'p')}{T('Legende', { w: 600, grow: 1 })}{I('ph:x', 16, 's')}</Frame>
      {T('Gelände', { s: 12, w: 600, c: 'text/sekundaer' })}
      {KLASSEN.map(([name]) => <Frame name={'Eintrag ' + name} flex="row" items="center" gap={8}>{Probe(name)}{T(name, { s: 12 })}</Frame>)}
      {T('Ereignisse', { s: 12, w: 600, c: 'text/sekundaer' })}
      {[['Figur', 'ph:person-simple-walk'], ['Tür', 'ph:door']].map(([n, i]) => <Frame name={'Eintrag ' + n} flex="row" items="center" gap={8}>{I(i, 16, 'p')}{T(n, { s: 12 })}</Frame>)}
    </Frame>;
    const schau = <Frame name="3D-Szene (Draufsicht, Gelände)" w={SW} h={SH} bg={v('flaeche/welt')} overflow="hidden">
      <Frame name="Boden (Unbekannt)" position="absolute" x={ox} y={oy} w={20 * G} h={14 * G} bg={v('gelaende/unbekannt')} />
      {zellen}
      {SceneBarX(8, 8, { gelaende: true, draufsicht: true })}
      {legende}
    </Frame>;
    return Werkbank({ name: NAMES.muster, undo: 0, schauInhalt: schau,
      inspector: [InspectorHeadB({ word: 'Karte', icon: 'ph:mountains', title: 'Karte 0005', ident: 'Zelle 3, 21 · Route 201' })],
      status: Statusbar({ tool: 'Auswählen', keys: [['Klick', 'wählt'], ['Ziehen', 'verschiebt']], info: 'Kachel x 6, y 8 · Karte 0005 · Vorsprung (JUMP_SOUTH) · Höhe 0 Stufen', problems: '101 Warnungen, 162 Hinweise' }) });
  }

  // ----- 'brett': „Komponenten (App): Kantensatz“ -----
  const BW = 880, TWB = BW - 32;
  const Zustand = (titel, inhalt) => <Frame name={titel} flex="col" gap={8}>{T(titel, { s: 12, w: 600, c: 'text/sekundaer', width: TWB })}{inhalt}</Frame>;
  const Karte = (inhalt) => <Frame flex="col" gap={12} p={16} w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{inhalt}</Frame>;
  const Mit = (bauteil, satz, bw) => <Frame flex="row" items="center" gap={16} w="fill"><Frame flex="col" w={bw}>{bauteil}</Frame>{T(satz, { s: 12, c: 'text/sekundaer', width: TWB - bw - 16 })}</Frame>;
  const Leiste = (sub, hints) => <Frame name="Statusleiste (Ausschnitt)" flex="row" items="center" gap={6} px={12} h={28} rounded={4} bg={v('flaeche/gehaeuse')}>{T('Gelände malen', { s: 12, w: 600 })}{T('·', { s: 12, c: 'text/sekundaer' })}{T(sub, { s: 12, w: 600 })}{hints.map(([k, a]) => <Frame flex="row" items="center" gap={4}>{T('·', { s: 12, c: 'text/sekundaer' })}{Kbd(k)}{T(a, { s: 12, c: 'text/sekundaer' })}</Frame>)}</Frame>;
  return <Frame name={NAMES.brett} flex="col" gap={24} p={32} w={BW + 64} rounded={16} bg={v('flaeche/deck')}>
    {T('Kantensatz', { f: Z, s: 24, lh: 30, w: 700 })}
    {T('Phase 6.4b: Terrain- und Klippenpinsel malen mit einem Kantensatz (13 + 2 Plätze nach Wang-Ecken); der Abschnitt „Kantensätze“ im Inspector legt ihn an, die Schicht „Gelände“ zeigt Muster. Keine neuen Tokens; neu sind die Symbol-Namen terrain und cliff (Phosphor Mountains, Stack).', { c: 'text/sekundaer', width: BW })}
    {Zustand('Platz: leer (gestrichelt, ohne Fläche), gefüllt (Bild der ersten Kachel), gewählt (auswahl/kante mit Fokusstrich), mit Varianten-Zähler, in Tüfteln mit den Token', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame flex="row" gap={8} items="start">{Platz('inner_nw', [])}{Platz('n', ['0007'])}{Platz('n', ['0007'], { sel: true })}{Platz('center', ['0006', '0015'])}{Platz('center', ['0006', '0015'], { tinker: true })}</Frame>, 'Taste 40 × 40 (raum/40), radius/taste; Tooltip der Name des Platzes, aria-label „Mitte, 2 Kacheln“. Ein Klick wählt (aria-pressed), ein zweiter hebt die Wahl auf.', 5 * 48)}
    </Frame>))}
    {Zustand('Raster des Satzes „Weg“ (weg13, vollständig) und eines leeren Satzes', Karte(<Frame flex="row" gap={24} items="start" w="fill">
      {Raster(WEG, 'outer_nw')}
      {Raster({}, null)}
    </Frame>))}
    {Zustand('Varianten-Chips des gewählten Platzes, Segment „Blockiert an der Klippe“, Zeile eines Satzes mit Zähler', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame flex="row" gap={4}>{Chip('0006')}{Chip('0015')}</Frame>, 'Chip: Bild 16 × 16, „Kachel 0006“ (Tüfteln das Token), „Entfernen“ leise; Höhe raum/24, radius/plakette.', 2 * 150)}
      {Mit(<Frame flex="col" gap={8}>{BLOCK('upper')}{BLOCK('lower')}</Frame>, '„Obere Zelle“ (Vorgabe, block fehlt in der Datei): das Backen blockiert die obere Zelle der Kante. „Untere Zelle“: der Strich schreibt Überschreibungen, der Ring unter dem Plateau blockiert.', 220)}
      {Mit(<Frame flex="col" gap={2} w={324}>{SetRow('Weg', WEG, true)}{SetRow('Fels', {}, false)}</Frame>, 'Radio „Aktiv“ (mit ihm malen T und K), Name, Zähler „13 von 13 Plätzen · Diagonalen 0 von 2“.', 324)}
    </Frame>))}
    {Zustand('Gelände-Muster (Schicht „Gelände“ und Legende): Linien in der Zellfarbe × 0,55 mit doppelter Deckkraft', Karte(<Frame flex="col" gap={12} w="fill">
      {Mit(<Frame flex="row" gap={12}>{KLASSEN.map(([name]) => <Frame flex="col" items="center" gap={4}>{Probe(name, 28)}{T(name, { s: 12 })}</Frame>)}</Frame>, 'Blockiert Schraffur, Wasser Wellen, Hohes Gras Punkte, Vorsprung Pfeil in Sprungrichtung (statt Schraffur), Eis Kreuzschraffur; Spezial und Unbekannt ohne Muster.', 7 * 72)}
    </Frame>))}
    {Zustand('Unterwerkzeuge T und K (in Ruhe und gedrückt) und ihre Statusleisten mit und ohne aktiven Satz', Karte(<Frame flex="col" gap={8} w="fill">
      {Mit(<Frame flex="row" gap={4}>{ToolKey(UNTER[7], false)}{ToolKey(UNTER[8], false)}{ToolKey([UNTER[7][0], UNTER[7][1] + '-fill', 'T'], true)}{ToolKey([UNTER[8][0], UNTER[8][1] + '-fill', 'K'], true)}</Frame>, 'Terrain-Pinsel T (terrain, Mountains), Klippenpinsel K (cliff, Stack); T ist im Malmodus „Terrain-Pinsel“, „Tür“ dann nur über seine Taste.', 4 * 44)}
      {Leiste('Terrain-Pinsel', HINTS['Terrain-Pinsel'])}
      {Leiste('Klippenpinsel', HINTS.Klippenpinsel)}
      {Leiste('Terrain-Pinsel', HINTS['Terrain-Pinsel ohne Satz'])}
      {Leiste('Klippenpinsel', HINTS['Klippenpinsel ohne Satz'])}
    </Frame>))}
  </Frame>;
