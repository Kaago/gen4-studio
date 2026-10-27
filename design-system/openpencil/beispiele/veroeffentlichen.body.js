  // App: Technik, Veröffentlichen (Phase 4.1a), drei Frames aus einer Vorlage; MODE wählt den Zustand (in der erzeugten .jsx umstellen):
  //   'gefunden'  „App: Technik, Veröffentlichen“: Basis-ROM gefunden (Platin DE (Testprojekt), d40f19ac), Patch erstellt (4,7 KiB)
  //   'fehlt'     „App: Technik, Basis-ROM fehlt“: Warnung mit Grund, „Basis-ROM suchen…“, „Patch erstellen…“ gesperrt mit Grund daneben
  //   'zuordnen'  „App: Technik, Basis-ROM zuordnen“: Vorschlag der .nds neben dem Ordner, darüber der offene Dialog mit einer Warnung
  // Quellen: apps/studio/src/technik/PublishSurface.tsx und .module.css (Fläche und Karten wie TechnikArea.module.css: Fläche flaeche/deck,
  // Innenabstand 32 / 24, Lesebreite 880, Karten flaeche/blatt mit Kante linie/standard, radius/fenster, dichte/panel-innen, Abstand 8),
  // technik/TechnikArea.tsx (Liste mit fünf Einträgen und Untertiteln), technik/useBase.ts. Texte aus design-system/README.md, „Nachtrag 4.1a“.
  // Projekt „Sinnoh-Remix“, Platin DE; Vergleich mit den Screenshots unter target/agent/gate-2026-10-27/4-1a/ (Hell, Dunkel, Kompakt).
  const MODE = 'gefunden';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const NAMES = { gefunden: 'App: Technik, Veröffentlichen', fehlt: 'App: Technik, Basis-ROM fehlt', zuordnen: 'App: Technik, Basis-ROM zuordnen' };
  const GRUND = 'Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).';

  // ----- Liste „Technik“: sechs Einträge (mit „Projekt“ aus 4.0) mit Symbol und Zustand, „Veröffentlichen“ gewählt; nach dem Patch ist seine ROM der letzte Build -----
  const sub = { gefunden: 'Patch erstellt', fehlt: 'Basis-ROM fehlt', zuordnen: 'Keine Basis-ROM zugeordnet' }[MODE];
  const liste = ListPane([
    PanelHead({ title: 'Technik' }),
    <Frame flex="col" gap={2} p={8} w="fill">
      {ListRow({ title: 'Projekt', sub: 'Kartenbild der Startstadt', icon: 'ph:folder-open' })}
      {ListRow({ title: 'Prüfen und Bauen', sub: MODE === 'gefunden' ? 'gebaut gerade eben' : 'Noch nicht gebaut', icon: 'ph:hammer' })}
      {ListRow({ title: 'Veröffentlichen', sub, icon: 'ph:upload-simple', sel: true })}
      {ListRow({ title: 'Patches', sub: 'Dynamische Header verfügbar', icon: 'ph:bandaids' })}
      {ListRow({ title: 'Emulator', sub: 'Kein Emulator gewählt', icon: 'ph:game-controller' })}
      {ListRow({ title: 'Werkzeuge', sub: '1 von 2 bereit', icon: 'ph:toolbox' })}
    </Frame>,
  ]);

  // ----- Karten (wie die Karte „Dynamische Header“ in Patches) -----
  const Karte = (name, title, badge, children) => <Frame name={name} flex="col" gap={8} p={16} w="fill" rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>
    <Frame flex="row" items="center" gap={8}>{T(title, { s: 15, w: 600 })}{badge}</Frame>
    {children}
  </Frame>;
  const Satz = (s) => T(s, { width: 600 });
  const Notiz = (s) => T(s, { s: 12, c: 'text/sekundaer' });
  const Angabe = (label, value, o = {}) => <Frame flex="col" gap={2} grow={o.grow}>{T(label, { s: 12, c: 'text/sekundaer' })}{o.mono ? Mono(value, { c: 'text/primaer', s: 13, one: o.one }) : T(value)}</Frame>;
  const Reihe = (children) => <Frame flex="row" items="center" gap={8} w="fill">{children}</Frame>;

  const plakette = {
    gefunden: Badge('Gefunden', 'ok', 'ph:check'),
    fehlt: Badge('Fehlt', 'warn', 'ph:warning'),
    zuordnen: Badge('Nicht zugeordnet', 'neutral'),
  }[MODE];
  const basisInhalt = {
    gefunden: [
      <Frame flex="row" gap={24} w="fill">
        {Angabe('ROM', 'Platin DE (Testprojekt)')}
        {Angabe('SHA-1', 'd40f19ac', { mono: true })}
        {Angabe('Datei', '/Users/du/Spiele/Sinnoh-Remix.nds', { mono: true, one: true, grow: 1 })}
      </Frame>,
    ],
    fehlt: [Notice(GRUND, 'warn'), Reihe(Btn({ label: 'Basis-ROM suchen…', art: 'Primär' }))],
    zuordnen: [Notice('Neben dem Projekt liegt „Sinnoh-Remix.nds“.', 'info'), Reihe([Btn({ label: 'Zuordnen…', art: 'Primär' }), Btn({ label: 'Andere Datei wählen…' })])],
  }[MODE];
  const basis = Karte('Karte: Basis-ROM', 'Basis-ROM', plakette, [
    Satz('Ein Patch beschreibt den Unterschied zu deiner Basis-ROM. Die ROM bleibt außerhalb des Projekts; Gen4 Studio merkt sich nur, wo sie liegt.'),
    ...basisInhalt,
  ]);

  const gesperrt = MODE !== 'gefunden';
  const ergebnis = <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v('status/erfolg-flaeche')}>
    {I('ph:check-circle-fill', 18, 'ok')}
    <Frame flex="col" gap={4} grow={1}>
      {T('Patch erstellt: 4,7 KiB', { w: 600, c: 'status/erfolg' })}
      {Mono('/Users/du/Spiele/Sinnoh-Remix_DSPRE_contents/build/Sinnoh-Remix-CPUD.xdelta', { c: 'text/primaer', s: 13 })}
      {T('Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.')}
    </Frame>
  </Frame>;
  const patch = Karte('Karte: Patch', 'Patch', undefined, [
    Satz('Der Patch enthält nur deine Änderungen. Wer dieselbe Basis-ROM hat, wendet ihn mit xdelta3 oder Rom Patcher JS an.'),
    Reihe([Btn({ label: 'Patch erstellen…', art: 'Primär', off: gesperrt }), gesperrt && Notiz(MODE === 'fehlt' ? GRUND : 'Erst eine Basis-ROM zuordnen.')]),
    MODE === 'gefunden' && ergebnis,
  ]);

  const flaeche = <Frame name="Fläche: Veröffentlichen" flex="col" gap={16} px={24} py={32} grow={1} h="fill" bg={v('flaeche/deck')}>
    {T('Veröffentlichen', { s: 24, w: 700, f: Z })}
    <Frame flex="col" gap={16} w={880}>{basis}{patch}</Frame>
  </Frame>;

  const win = Window({ name: NAMES[MODE], active: 'technik', title: { project: 'Sinnoh-Remix', game: 'Platin, DE' }, status: Statusbar(MODE === 'gefunden' ? { build: 'Gebaut gerade eben', built: true } : {}), children: [liste, flaeche] });
  if (MODE !== 'zuordnen') return win;

  // ----- Dialog „Basis-ROM zuordnen?“ (Dialog-Primitive wie DialogX, mit Inhalt): Beschreibung, eine Warnung (Meldung wie Notice, mit
  // Zeilenbreite, damit der Satz umbricht), Satz mit „Nicht rückgängig“. Ohne Bekannte Basis-ROM, weil die SHA-1 unbekannt ist. -----
  const W = 480;
  const dialog = <Frame name="Dialog" flex="col" gap={12} p={20} w={W} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={W - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T('Basis-ROM zuordnen?', { s: 18, w: 600, width: W - 72 })}
    {T('POKEMON PL · CPUD · SHA-1 5e1f0b7a', { c: 'text/sekundaer', width: W - 40 })}
    <Frame flex="col" gap={8} w="fill">
      <Frame flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v('status/warnung-flaeche')}>
        {I('ph:warning-fill', 18, 'w')}{T('Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.', { width: W - 40 - 24 - 26 })}
      </Frame>
      <Frame flex="row" items="center" gap={8}>{T('Schreibt die Basismarke .g4/base.json ins Projekt.')}{Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')}</Frame>
    </Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Zuordnen', art: 'Primär' })}</Frame>
  </Frame>;
  return MitDialog(NAMES.zuordnen, win, dialog);
