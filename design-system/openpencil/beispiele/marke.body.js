  // Brett „Marke“ (Phase 4.0) für die Seite „Grundlagen“: Konstruktion mit Maßlinien, Formen und Farben, die Marke in 1024 (verkleinert), 128,
  // 32 und 16 px auf Hell und auf Dunkel, Schutzraum. Quelle der Geometrie: design-system/scripts/marke.py und marke/gen4-studio-marke.svg
  // (README, Abschnitt „Marke“). Die fünf Formen tragen die Hex-Werte des dunklen Themas wie das SVG (v() bände die hellen Werte);
  // Maßlinien in live/grund, Maßzahlen in live/text. Bauen: bash ../build-screen.sh marke; render auf „Grundlagen“ (x 1720, y 2900).
  const MK = { koerper: '#42527B', ds: '#182131', akzent: '#6384F7' };
  const DK = { grund: '#212939', blatt: '#293142', text: '#E7EFF7', leise: '#ADB5C6' };
  // Die Marke im Raster 1024; B = Kantenlänge des gezeichneten Rasters. ohneRand: nur der Körper (824), ohne die 100 px Rand.
  const Marke = (B, o = {}) => {
    const s = o.ohneRand ? B / 824 : B / 1024, d = o.ohneRand ? 100 : 0, r = (x, y, w, h, rad, bg) => <Frame position="absolute" x={(x - d) * s} y={(y - d) * s} w={w * s} h={h * s} rounded={rad * s} bg={bg} />;
    return <Frame name={o.name || 'Marke ' + B} w={B} h={B}>
      {r(100, 100, 824, 824, 184, MK.koerper)}
      {r(220, 220, 584, 252, 64, MK.ds)}
      {r(220, 552, 584, 252, 64, MK.ds)}
      {r(220, 496, 584, 32, 16, MK.akzent)}
      {r(298, 630, 96, 96, 24, MK.akzent)}
    </Frame>;
  };
  const Karte = (title, text, children, o = {}) => <Frame name={title} flex="col" gap={14} p={24} w={o.w} grow={o.w ? undefined : 1} h={o.h} rounded={12} bg={v('flaeche/taste')} stroke={v('linie/leise')} strokeWidth={1}>
    {T(title, { s: 18, w: 600 })}
    {text && T(text, { c: 'text/sekundaer', width: o.textW || 'fill' })}
    {children}
  </Frame>;
  // Konstruktion: Marke in 1/2 (512), Ursprung O; Maße in Pixeln des Rasters 1024
  const K = 0.5, OX = 96, OY = 64, px = (n) => OX + n * K, py = (n) => OY + n * K;
  const ML = v('live/grund'), MT = 'live/text';
  const Lab = (x, y, w, s, align = 'center', c = MT) => <Frame position="absolute" x={x} y={y} w={w} h={16}>{T(s, { f: M, s: 12, c, width: w, align })}</Frame>;
  const Lin = (x, y, w, h) => <Frame position="absolute" x={x} y={y} w={Math.max(w, 1)} h={Math.max(h, 1)} bg={ML} />;
  // waagrechte Maßlinie von a bis b (Raster) auf Höhe y (Leinwand), Zahl darüber
  const HD = (a, b, y, s, c) => [Lin(px(a), y, (b - a) * K, 1), Lin(px(a), y - 4, 1, 9), Lin(px(b) - 1, y - 4, 1, 9), Lab((px(a) + px(b)) / 2 - 40, y - 20, 80, s, 'center', c)];
  // senkrechte Maßlinie von a bis b (Raster) bei x (Leinwand), Zahl links davon
  const VD = (a, b, x, s, c) => [Lin(x, py(a), 1, (b - a) * K), Lin(x - 4, py(a), 9, 1), Lin(x - 4, py(b) - 1, 9, 1), Lab(x - 52, (py(a) + py(b)) / 2 - 8, 46, s, 'right', c)];
  // Hinweislinie vom Punkt (Raster) nach rechts zur Beschriftung; mit unten: erst senkrecht bis unten (Raster), dann nach rechts
  const CX = px(1024) + 24;
  const CO = (x, y, title, sub, unten) => [unten ? Lin(px(x), py(y), 1, (unten - y) * K) : null, Lin(px(x), py(unten || y), CX - px(x) - 8, 1), <Frame position="absolute" x={px(x) - 3} y={py(y) - 3} w={7} h={7} rounded={4} bg={ML} />,
    <Frame position="absolute" x={CX} y={py(unten || y) - 9} w={268} flex="col" gap={2}>{T(title, { s: 13, w: 600 })}{T(sub, { s: 12, c: 'text/sekundaer', width: 268 })}</Frame>];
  const konstruktion = <Frame name="Konstruktion mit Maßen" w={912} h={652} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>
    <Frame position="absolute" x={px(0)} y={py(0)} w={512} h={512} stroke={v('linie/standard')} strokeWidth={1} strokeDash={[6, 4]} dashPattern={[6, 4]} />
    <Frame position="absolute" x={px(0)} y={py(0)}>{Marke(512, { name: 'Marke 1024 (1:2)' })}</Frame>
    {HD(0, 100, OY - 16, '100')}{HD(100, 924, OY - 16, '824')}{HD(924, 1024, OY - 16, '100')}
    {HD(100, 220, py(1024) + 20, '120')}{HD(220, 804, py(1024) + 20, '584')}{HD(804, 924, py(1024) + 20, '120')}
    {VD(100, 220, OX - 16, '120')}{VD(220, 472, OX - 16, '252')}{VD(472, 552, OX - 16, '80')}{VD(552, 804, OX - 16, '252')}{VD(804, 924, OX - 16, '120')}
    {HD(220, 298, py(678), '78', 'live/grund')}{VD(552, 630, px(346), '78', 'live/grund')}{VD(726, 804, px(346), '78', 'live/grund')}
    {CO(850, 150, 'Körper', '824 × 824 bei 100/100, Radius 184 · scharnier/grund')}
    {CO(784, 300, 'Bildschirm oben', '584 × 252 bei 220/220, Radius 64 = 184 − 120 · szene/ds/grund')}
    {CO(804, 512, 'Scharnier', '584 × 32 bei y 496, Mitte auf 512, Radius 16 · akzent/grund')}
    {CO(784, 640, 'Bildschirm unten', '584 × 252 bei 220/552; Block beider Bildschirme 584 × 584')}
    {CO(380, 712, 'Auswahl', '96 × 96 bei 298/630, Radius 24, links, oben, unten je 78 · akzent/grund', 870)}
    <Frame position="absolute" x={CX} y={py(1024) + 8} w={268} flex="col" gap={2}>{T('Raster 1024 × 1024', { s: 13, w: 600 })}{T('gestrichelt; Maße in Pixeln, gezeichnet 1:2', { s: 12, c: 'text/sekundaer', width: 268 })}</Frame>
  </Frame>;
  // Formen und Farben
  const Swatch = (hex, tok, wert, rolle) => <Frame flex="row" items="center" gap={12} w="fill" minH={40}>
    <Frame w={32} h={32} rounded={6} bg={hex} stroke={v('linie/leise')} strokeWidth={1} />
    <Frame flex="col" grow={1}>{T(tok, { f: M, s: 12 })}{T(rolle, { s: 12, c: 'text/sekundaer' })}</Frame>
    {Mono(wert)}
  </Frame>;
  const Form = (name, mass) => <Frame flex="row" items="start" gap={12} w="fill">{T(name, { w: 600, width: 96 })}{T(mass, { c: 'text/sekundaer', grow: 1 })}</Frame>;
  const farben = Karte('Formen und Farben', 'Fünf Formen, drei Farben, keine Kontur, kein Schatten, keine Textur. Die Farben sind die Werte des Themas Dunkel und der Szene, alle auf dem RGB555-Raster.', [
    <Frame flex="col" gap={8} w="fill">
      {Swatch(MK.koerper, 'scharnier/grund (Dunkel)', '#42527B', 'Körper: das Gehäuse in der Farbe des Scharniers')}
      {Swatch(MK.ds, 'szene/ds/grund', '#182131', 'beide Bildschirme')}
      {Swatch(MK.akzent, 'akzent/grund (Dunkel)', '#6384F7', 'Scharnier und Auswahl, der einzige Akzent')}
    </Frame>,
    <Frame w="fill" h={1} bg={v('linie/leise')} />,
    <Frame flex="col" gap={8} w="fill">
      {Form('Körper', '1 abgerundetes Quadrat')}
      {Form('Bildschirme', '2, konzentrisch zum Körper')}
      {Form('Scharnier', '1 Linie mit runden Enden')}
      {Form('Auswahl', '1 Quadrat: „das bist du gerade“')}
      {Form('„4“', 'keine Ziffer; Name und Generation stehen daneben')}
    </Frame>,
    <Frame w="fill" h={1} bg={v('linie/leise')} />,
    T('Quelle: marke/gen4-studio-marke.svg (fünf <rect>, je Farbe eine Gruppe mit dem Token im id), PNG 1024 aus scripts/marke.py.', { s: 12, c: 'text/sekundaer', width: 470 }),
  ]);
  // Größen auf Hell und auf Dunkel
  const Groesse = (B, label, dunkel) => <Frame flex="col" items="center" gap={8}>
    <Frame w={Math.max(B, 16)} h={256} flex="col" justify="end" items="center">{Marke(B)}</Frame>
    {T(label, { f: M, s: 12, c: dunkel ? undefined : 'text/sekundaer', hex: dunkel ? DK.leise : undefined })}
  </Frame>;
  const Reihe = (dunkel) => <Frame name={dunkel ? 'Auf Dunkel' : 'Auf Hell'} flex="row" items="end" gap={40} px={32} py={28} w="fill" rounded={8} bg={dunkel ? DK.grund : v('flaeche/blatt')} stroke={dunkel ? DK.blatt : v('linie/leise')} strokeWidth={1}>
    {Groesse(256, '1024 (1:4)', dunkel)}{Groesse(128, '128', dunkel)}{Groesse(32, '32', dunkel)}{Groesse(16, '16', dunkel)}
  </Frame>;
  const groessen = (dunkel) => Karte(dunkel ? 'Auf Dunkel' : 'Auf Hell', dunkel
    ? 'Auf dunklem Dock steht der Körper in Platin 700 als hellere Form. Bei 16 px: Körper, zwei dunkle Flächen, die Linie als heller Streifen, die Auswahl als Kobaltpunkt.'
    : 'Auf hellem Dock und Finder steht der Körper als dunkle Form; eine Kontur braucht er nicht. Ein Symbol für beide, die Ecken außerhalb des Körpers sind durchsichtig.', [Reihe(dunkel)]);
  // Schutzraum: Körper in 206 (1:4), Zone getönt und gestrichelt
  const Zone = (rand, titel, satz) => <Frame flex="col" gap={10} grow={1} w={1}>
    <Frame w="fill" h={300} flex="row" items="center" justify="center" rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>
      <Frame flex="col" p={rand} bg={v('status/warnung-flaeche')} stroke={ML} strokeWidth={1} strokeDash={[6, 4]} dashPattern={[6, 4]}>{Marke(206, { ohneRand: true, name: 'Körper 824 (1:4)' })}</Frame>
    </Frame>
    {T(titel, { w: 600, width: 'fill' })}{T(satz, { c: 'text/sekundaer', width: 'fill' })}
  </Frame>;
  const schutzraum = Karte('Schutzraum', 'Um den Körper bleibt Raum frei, gemessen an der Körperbreite. Gezeichnet 1:4 (Körper 206 statt 824), die Zone getönt.', [
    <Frame flex="row" gap={24} w="fill">
      {Zone(5, 'Im Symbol: 20 px bei 824', 'Ein Viertel des Scharnier-Abstands (80 / 4). Der Rand von 100 px im Raster 1024 enthält ihn.')}
      {Zone(26, 'In Dokumenten und in der App: 103 px bei 824', 'Mindestens 1/8 der Körperbreite um den Körper frei (Galerie, späterer Über-Dialog).')}
    </Frame>,
  ]);
  const regeln = Karte('Einsatz', null, [
    <Frame flex="col" gap={10} w="fill">
      {Form('App-Symbol', 'Dock, Finder, „Über Gen4 Studio“; tauri icon skaliert aus dem PNG 1024, kein eigenes Kleinsymbol.')}
      {Form('Kleine Größen', 'Linie 32 px dick (eine Pixelzeile bei 32), Auswahl 96 px (drei Pixel bei 32, anderthalb bei 16).')}
      {Form('Nicht', 'keine Kontur, kein Schatten, keine Ziffer, nicht drehen, keine anderen Farben, keine Landschaft auf den Bildschirmen.')}
      {Form('Neu erzeugen', 'nach einer Token-Änderung python3 design-system/scripts/marke.py, dann die Symbole (docs/release.md, „App-Symbol“).')}
    </Frame>,
  ], { w: 560 });
  return <Frame name="Marke" flex="col" w={1640} gap={32} p={56} bg={v('flaeche/gehaeuse')}>
    <Frame flex="col" w="fill" gap={12}>
      {T('Marke', { f: Z, s: 32, w: 700, lh: 38 })}
      {T('Die Marke ist das Doppeldeck selbst, auf das Nötigste gebracht: zwei Bildschirme übereinander, dazwischen das Scharnier als Kobaltlinie, unten auf der Hand die Auswahl. Sie ist das App-Symbol von Gen4 Studio (Phase 4.0).', { c: 'text/sekundaer', width: 760 })}
    </Frame>
    <Frame flex="row" w="fill" gap={24}>
      {Karte('Konstruktion', 'Raster 1024 × 1024 wie die macOS-App-Symbole. Der Block aus beiden Bildschirmen ist quadratisch, ihre Radien sind konzentrisch zum Körper.', [konstruktion], { w: 960 })}
      {farben}
    </Frame>
    <Frame flex="row" w="fill" gap={24}>{groessen(false)}{groessen(true)}</Frame>
    <Frame flex="row" w="fill" gap={24}>{schutzraum}{regeln}</Frame>
  </Frame>;
