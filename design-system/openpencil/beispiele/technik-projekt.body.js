  // App: Technik, Projekt mit Vorschaubild (Phase 4.0): Technik › Projekt mit dem Abschnitt „Vorschaubild“ im Zustand Standardbild
  // (Kartenbild der Startstadt, nur „Vorschaubild wählen…“, der Satz mit Zweiblattdorf). Vorlage: Test-Umgebung blick-technik-projekt-hell.png.
  // Quellen: technik/TechnikArea.tsx (ENTRIES, ProjectSurface, Surface) und .module.css. Das Bild ist ein Beispielbild aus Szenenfarben.
  const Eintrag = (title, sub, icon, sel) => ListRow({ title, sub, icon, sel });
  const liste = ListPane([
    PanelHead({ title: 'Technik' }),
    <Frame flex="col" gap={2} p={8} w="fill">
      {Eintrag('Projekt', 'Kartenbild der Startstadt', 'ph:folder-open', true)}
      {Eintrag('Prüfen und Bauen', 'Noch nicht gebaut', 'ph:hammer')}
      {Eintrag('Patches', 'Dynamische Header verfügbar', 'ph:bandaids')}
      {Eintrag('Emulator', 'Kein Emulator gewählt', 'ph:game-controller')}
      {Eintrag('Werkzeuge', '1 von 2 bereit', 'ph:toolbox')}
    </Frame>,
  ]);
  const flaeche = <Frame name="Fläche: Projekt" flex="col" gap={16} grow={1} h="fill" px={24} pt={32} bg={v('flaeche/deck')}>
    <Frame flex="col" gap={4}>{T('Projekt', { f: Z, s: 24, w: 700, lh: 30 })}{T('Sinnoh-Remix', { c: 'text/sekundaer' })}</Frame>
    <Frame name="Vorschaubild" flex="col" gap={8} p={16} w={880} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
      {T('Vorschaubild', { s: 15, w: 600 })}
      {VorschauBild('standard')}
      <Frame flex="row" gap={8}>{Btn({ label: 'Vorschaubild wählen…' })}</Frame>
      {T('Ohne eigenes Bild zeigt Start das Kartenbild von Zweiblattdorf.', { s: 12, c: 'text/sekundaer' })}
    </Frame>
  </Frame>;
  return Window({ name: 'App: Technik, Projekt mit Vorschaubild', active: 'technik', title: { project: 'Sinnoh-Remix' },
    status: Statusbar({}), children: [liste, flaeche] });
