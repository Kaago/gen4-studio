  // Brett „Komponenten (App): Projekt“ (Phase 4.0) für die Seite „Komponenten“: die Projektkarte in ihren Zuständen, ihr Menü in drei Formen,
  // die Maße, „Als Vorschaubild“ im Reitermenü der Schau und in der Palette, die Toasts. Vorlagen: Test-Umgebung (g2-kartenmenue-mit-eigenem-bild,
  // g3-start-standardbild-menue-ohne-entfernen, g4-nicht-gefunden-menue, g1-3d-reitermenue, g1-weltkarte-menue-gesperrt, g1-palette-gesperrt-ohne-karte).
  // Quellen: ui/frame/StartPane.tsx, ProjectPreview.tsx, dockMenu.ts, app/paletteEntries.ts, project/previewMessages.ts.
  const Karte = (title, text, children, o = {}) => <Frame name={title} flex="col" gap={14} p={24} w={o.w || 'fill'} grow={o.w ? undefined : 1} rounded={12} bg={v('flaeche/taste')} stroke={v('linie/leise')} strokeWidth={1}>
    {T(title, { s: 18, w: 600 })}{text && T(text, { c: 'text/sekundaer', width: 'fill' })}{children}
  </Frame>;
  const Zustand = (label, karte) => <Frame flex="col" gap={8}>{T(label, { s: 13, w: 600, c: 'text/sekundaer' })}{karte}</Frame>;
  const KW = 284;
  const zustaende = Karte('Projektkarte', 'Die Karte eines Projekts in der Startliste. Reihenfolge des Bildes: eigenes Bild (.g4/preview.png), sonst Standardbild (.g4/cache/preview-standard.png), sonst der Name auf dem leeren Bildschirm. Die ganze Karte öffnet das Projekt; das Menü „…“ liegt oben rechts auf dem Rahmen.', [
    <Frame flex="row" gap={12} w="fill" items="start">
      {Zustand('Bild eigen', ProjKarte({ name: 'Sinnoh-Remix', zeit: 'vor 20 Minuten', bild: 'eigen', w: KW }))}
      {Zustand('Standardbild', ProjKarte({ name: 'Johto-Abenteuer', game: 'HeartGold/SoulSilver, DE', zeit: 'vor 2 Stunden', bild: 'standard', w: KW }))}
      {Zustand('Platzhalter mit Name', ProjKarte({ name: 'Neues-Sinnoh', zeit: 'vor 3 Tagen', w: KW }))}
      {Zustand('Zuletzt (und Hover)', ProjKarte({ name: 'Sinnoh-Remix', zeit: 'gerade eben', bild: 'eigen', zuletzt: true, hover: true, w: KW }))}
      {Zustand('nicht gefunden', ProjKarte({ name: 'Alte-Kopie', fehlt: true, w: KW }))}
    </Frame>,
  ]);
  const Menue = (label, items) => <Frame flex="col" gap={8}>{T(label, { s: 13, w: 600, c: 'text/sekundaer' })}<Frame w={220} h={8 + items.reduce((n, it) => n + (it === '-' ? 9 : 32), 0)}>{MenuX({ x: 0, y: 0, w: 220, items })}</Frame></Frame>;
  const menue = Karte('Menü der Karte', 'Name „Weitere Aktionen für <Name>“. „Vorschaubild wählen…“ öffnet den Dateidialog „Vorschaubild wählen“ (Filter „Bilder (PNG, JPEG)“).', [
    <Frame flex="row" gap={24} w="fill" items="start">
      {Menue('Mit eigenem Bild', MENU_EIGEN)}
      {Menue('Mit Standardbild oder ohne Bild', MENU_STANDARD)}
      {Menue('nicht gefunden', MENU_FEHLT)}
    </Frame>,
  ]);
  const Mass = (a, b) => <Frame flex="row" gap={12} w="fill">{T(a, { w: 600, width: 112 })}{T(b, { c: 'text/sekundaer', grow: 1, width: 'fill' })}</Frame>;
  const masse = Karte('Maße', null, [
    <Frame flex="col" gap={8} w="fill">
      {Mass('Raster', 'Spalten ab 288 px (auto-fill), Abstand 16; Start bis 960 px breit, also drei Karten je Reihe')}
      {Mass('Karte', 'blatt, Rand linie/standard (Hover linie/stark), radius/fenster')}
      {Mass('DS-Rahmen', '8 px eingerückt, 4 : 3, bildschirm-rahmen, radius/bildschirm')}
      {Mass('Bild', '256 × 192, ganzzahlig skaliert, pixelig; alt=""')}
      {Mass('Text', 'Name text-m-betont, Spiel als Plakette, Zeit text-xs; innen 8, unten 12')}
    </Frame>,
  ], { w: 560 });
  // Reitermenü der Schau: Reiterleiste mit „3D“ und „Weltkarte“, das Menü darunter
  const Reiter = (aktiv) => <Frame flex="row" items="end" h={28} w={270} bg={v('flaeche/gehaeuse')}>{['3D', 'Weltkarte'].map((l) => <Frame flex="row" items="center" px={8} h={28} bg={l === aktiv ? v('flaeche/schau') : undefined} strokeWeights={l === aktiv ? { top: 2, right: 0, bottom: 0, left: 0 } : undefined} stroke={l === aktiv ? v('akzent/grund') : undefined}>{T(l, { c: l === aktiv ? 'text/primaer' : 'text/sekundaer' })}</Frame>)}</Frame>;
  const Tip = (text) => <Frame flex="row" px={8} py={4} rounded={4} bg={v('scharnier/grund')}>{T(text, { s: 12, c: 'scharnier/text' })}</Frame>;
  const Reitermenue = (label, aktiv, aus) => <Frame flex="col" gap={8}>{T(label, { s: 13, w: 600, c: 'text/sekundaer' })}
    <Frame w={270} h={aus ? 156 : 124}>{Reiter(aktiv)}{MenuX({ x: aktiv === '3D' ? 4 : 40, y: 32, w: 200, items: [{ label: 'Maximieren' }, '-', { label: 'Als Vorschaubild', hi: !aus, off: aus }] })}
      {aus && <Frame position="absolute" x={130} y={116}>{Tip('Erst eine Karte wählen')}</Frame>}</Frame></Frame>;
  const PalZeile = (label, sub, icon, aus) => <Frame flex="row" items="center" gap={10} px={12} h={48} w="fill" rounded={4}>{I(icon, 18, aus ? 'd' : 'welt')}<Frame flex="col" grow={1}>{T(label, { c: aus ? 'text/deaktiviert' : 'text/primaer' })}{T(sub, { s: 12, c: 'text/sekundaer' })}</Frame></Frame>;
  const palette = <Frame flex="col" gap={8}>{T('Befehlspalette, ohne gezeigte 3D-Ansicht und ohne gewählte Karte', { s: 13, w: 600, c: 'text/sekundaer', width: 420 })}
    <Frame flex="col" p={4} w={420} rounded={6} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
      {PalZeile('Als Vorschaubild: 3D-Ansicht', 'Erst die 3D-Ansicht zeigen', 'ph:globe-hemisphere-west', true)}
      {PalZeile('Als Vorschaubild: Kartenbild der gewählten Karte', 'Erst eine Karte wählen (in 3D auf den Boden klicken)', 'ph:map-trifold', true)}
    </Frame></Frame>;
  const szene = Karte('Als Vorschaubild', 'Im Reitermenü der Schau unter „Maximieren“ und einem Trenner, nur an „3D“ (Bild der 3D-Ansicht) und „Weltkarte“ (Kartenbild der gewählten Karte). Gesperrt steht der Grund im Tooltip und für Screenreader in aria-describedby; die Palette zeigt ihn unter dem Befehl. Frei: „Das Bild der Ansicht auf die Projektkarte“, „Das Kartenbild auf die Projektkarte“.', [
    <Frame flex="row" gap={24} w="fill" items="start">
      {Reitermenue('Reiter „3D“', '3D', false)}
      {Reitermenue('Reiter „Weltkarte“, keine Karte gewählt', 'Weltkarte', true)}
      {palette}
    </Frame>,
  ]);
  const toasts = Karte('Toasts', 'Unten rechts; ein Fehler mit dem Satz seines Codes (preview.format, preview.too_large, preview.unreadable).', [
    <Frame flex="col" gap={8}>{Toast('Vorschaubild gesetzt')}{Toast('Vorschaubild entfernt')}{Toast('Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.', 'fehler')}</Frame>,
  ], { w: 420 });
  return <Frame name="Komponenten (App): Projekt" flex="col" w={1640} gap={32} p={56} bg={v('flaeche/gehaeuse')}>
    <Frame flex="col" w="fill" gap={12}>
      {T('Komponenten (App): Projekt', { f: Z, s: 32, w: 700, lh: 38 })}
      {T('Phase 4.0: die Projektkarte auf Start mit dem Vorschaubild 256 × 192 und die Wege, ein Bild zu setzen. Neue Tokens gibt es nicht. Die Beispielbilder sind aus Szenenfarben gezeichnet, keine Spielgrafik.', { c: 'text/sekundaer', width: 760 })}
    </Frame>
    {zustaende}
    <Frame flex="row" w="fill" gap={24}>{menue}{masse}</Frame>
    <Frame flex="row" w="fill" gap={24}>{szene}{toasts}</Frame>
  </Frame>;
