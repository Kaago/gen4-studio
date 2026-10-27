  // App: Start, Projekte mit Vorschaubild (Phase 4.0): „Willkommen zurück“ mit vier Projektkarten, drei je Reihe (seit af493ba nimmt Start
  // die Breite bis 960 px). Von links: eigenes Bild mit „Zuletzt“ und offenem Menü „…“, Standardbild (Kartenbild der Startstadt), ohne Bild
  // der Name auf dem leeren Bildschirm, „nicht gefunden“. Vorlage: Test-Umgebung blick-start-hell.png. Die Bilder sind Beispielbilder aus
  // Szenenfarben (VorschauBild), keine Spielgrafik. Quellen: ui/frame/StartPane.tsx und .module.css, ProjectPreview.tsx.
  const W = 309;
  const pane = <Frame name="Start" flex="col" gap={16} w={1008} px={24} pt={48} pb={48}>
    {T('Willkommen zurück', { f: Z, s: 24, w: 700, lh: 30 })}
    <Frame name="Zuletzt geöffnete Projekte" flex="col" gap={16} w="fill">
      <Frame flex="row" gap={16} w="fill">
        {ProjKarte({ name: 'Sinnoh-Remix', game: 'Platin, DE', zeit: 'gerade eben', bild: 'eigen', zuletzt: true, w: W, menu: MENU_EIGEN, hover: true })}
        {ProjKarte({ name: 'Johto-Abenteuer', game: 'HeartGold/SoulSilver, DE', zeit: 'vor 2 Stunden', bild: 'standard', w: W })}
        {ProjKarte({ name: 'Neues-Sinnoh', game: 'Platin, DE', zeit: 'vor 3 Tagen', w: W })}
      </Frame>
      <Frame flex="row" gap={16} w="fill">
        {ProjKarte({ name: 'Alte-Kopie', game: 'Platin, DE', fehlt: true, w: W })}
      </Frame>
    </Frame>
    <Frame flex="row" gap={8}>{Btn({ label: 'Neues Spiel aus ROM…', art: 'Primär', icon: 'ph:plus' })}{Btn({ label: 'Projekt öffnen', icon: 'ph:folder-open', kbd: '⌘O' })}</Frame>
    <Frame flex="col" gap={2}>{T('Du brauchst deine eigene ROM-Datei (.nds).', { c: 'text/sekundaer' })}{T('Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM.', { c: 'text/sekundaer' })}</Frame>
    {T('Projekte sind entpackte DSPRE-Ordner.', { c: 'text/sekundaer' })}
  </Frame>;
  return Window({ name: 'App: Start, Projekte mit Vorschaubild', active: 'start', title: { project: null },
    status: Statusbar({ problems: null, info: 'Zuletzt geöffnete Projekte' }),
    children: [<Frame flex="col" items="center" grow={1} h="fill" bg={v('flaeche/gehaeuse')} overflow="hidden">{pane}</Frame>] });
