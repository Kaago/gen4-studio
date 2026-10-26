  // App: Einstellungen, Agenten (Phase 3h.3): die Spalte Agenten im Zustand „an, mit Projekt“, Freigabe „Lesen“.
  // Quellen: apps/studio/src/einstellungen/{SettingsArea,AgentsSettings}.tsx und .module.css. Bereich höchstens 1040 (innen 992), Titel titel/s,
  // Spalten im Raster minmax(280, 1fr) mit Abstand 16: Darstellung, Emulator, Tastenkürzel in der ersten Reihe; Agenten beginnt die zweite und
  // ist dort zwei Spalten breit (Container-Abfrage ab 576 px). Rechts daneben zwei kleine Zustände der Spalte (Aus, Noch nicht festgelegt).
  const COL = 320, GAP = 16;
  const Spalte = (title, children, o = {}) => <Frame name={o.name || title} flex="col" gap={o.gap || 12} p={16} w={o.w || COL} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1}>{T(title, { s: 15, w: 600 })}{children}</Frame>;
  const Feld = (label, children, gap = 6) => <Frame flex="col" items="start" gap={gap} w="fill">{label && T(label, { w: 600, c: 'text/sekundaer' })}{children}</Frame>;
  const Hint = (s, width) => T(s, { s: 12, c: 'text/sekundaer', width });

  const darstellung = Spalte('Darstellung', [
    Feld('Thema', Seg([['Hell', 'ph:sun'], ['Dunkel', 'ph:moon'], ['Wie System', 'ph:monitor']], 'Wie System')),
    Feld('Dichte', [Seg(['Komfort', 'Kompakt'], 'Komfort'), Hint('Kompakt: Zeilen 24 statt 32 px hoch')]),
  ]);
  const emulator = Spalte('Emulator', <Frame flex="row" items="center" gap={8} w="fill">{T('Kein Emulator gewählt', { c: 'text/sekundaer', grow: 1 })}{Btn({ label: 'Emulator wählen', small: true })}</Frame>);
  const KEYS = [['Projekt öffnen', '⌘O'], ['Befehlspalette öffnen', '⌘K'], ['Speichern', '⌘S'], ['Rückgängig', '⌘Z'], ['Wiederholen', '⇧⌘Z'], ['Hier spielen (nur in der App)', '⌘R'], ['Pokédex (im Bereich Pokémon)', '⌘1'], ['Trainer (im Bereich Pokémon)', '⌘2'], ['Begegnungen (im Bereich Pokémon)', '⌘3'], ['Attacken (im Bereich Pokémon)', '⌘4']];
  const kuerzel = Spalte('Tastenkürzel', <Frame flex="col" gap={6} w="fill">{KEYS.map(([a, k]) => <Frame flex="row" items="center" w="fill">{T(a, { grow: 1 })}{Kbd(k)}</Frame>)}</Frame>);

  // Spalte Agenten: zwei Spalten breit
  const AW = 2 * COL + GAP;
  const Punkt = (ready) => <Frame w={8} h={8} rounded={4} bg={ready ? v('status/erfolg') : undefined} stroke={v(ready ? 'status/erfolg' : 'linie/stark')} strokeWidth={1} />;
  const Server = (ready, text) => <Frame name="Server" flex="row" items="center" gap={6}>{Punkt(ready)}{T(text, { c: 'text/sekundaer' })}</Frame>;
  const SETUP = 'claude mcp add gen4-studio -- "/Applications/Gen4 Studio.app/Contents/MacOS/studio" mcp';
  const Setup = (w) => <Frame name="Einrichtungszeile" flex="row" px={8} py={6} w={w} rounded={4} bg={v('flaeche/deck')} stroke={v('linie/leise')} strokeWidth={1}>{T(SETUP, { f: M, s: 13, lh: 20, width: w - 16 })}</Frame>;
  const Grant = (path, level) => <Frame name="Freigabe" flex="row" items="center" gap={8} w="fill">{T(path, { f: M, one: true, grow: 1 })}{T(level, { c: 'text/sekundaer' })}{IconBtn('ph:x')}</Frame>;
  const agenten = (o) => Spalte('Agenten', [
    Feld(null, [Toggle(o.on, 'Lokale Agenten zulassen'), Server(o.on, o.on ? 'Bereit auf 127.0.0.1:52814' : 'Aus')]),
    Feld('Einrichtung in Claude Code', [Setup(o.w - 32), Btn({ label: 'Einrichtungszeile kopieren', icon: 'ph:copy', small: true })]),
    Feld('Dieses Projekt', [Seg(['Nicht', 'Lesen', 'Lesen und bearbeiten'], o.level), o.level ? null : Hint('Noch nicht festgelegt: Gen4 Studio fragt beim ersten Zugriff eines Agenten', o.w - 32)]),
    o.others ? Feld('Andere Projekte', <Frame flex="col" gap={2} w="fill">{Grant('/Users/du/Documents/Gen4 Studio/Johto-Remix_DSPRE_contents', 'Lesen und bearbeiten')}</Frame>) : null,
    Feld(null, [Toggle(false, 'Expertenwerkzeuge (g4_call)'), Hint('Gibt Agenten jeden Befehl des Kernels, auch ungeprüfte')]),
  ], { name: o.name || 'Agenten', w: o.w, gap: 16 });

  const bereich = <Frame name="Einstellungen" flex="col" gap={24} pt={48} px={24} w={1040}>
    {T('Einstellungen', { f: Z, s: 24, lh: 30, w: 700 })}
    <Frame name="Spalten" flex="col" gap={GAP} w="fill">
      <Frame flex="row" items="start" gap={GAP} w="fill">{darstellung}{emulator}{kuerzel}</Frame>
      {agenten({ on: true, level: 'Lesen', others: true, w: AW })}
    </Frame>
  </Frame>;
  // Zwei kleine Zustände der Spalte (Aus; an, aber noch nicht festgelegt) als eigener Frame neben dem Screen: NUR_ZUSTAENDE = true
  const NUR_ZUSTAENDE = false;
  if (NUR_ZUSTAENDE) return <Frame name="App: Einstellungen, Agenten (Zustände der Spalte)" flex="row" items="start" gap={24} p={24} bg={v('flaeche/gehaeuse')} rounded={10}>
    <Frame flex="col" gap={8}>{Mono('Aus')}{agenten({ on: false, level: 'Lesen', others: false, w: AW, name: 'Agenten, Aus' })}</Frame>
    <Frame flex="col" gap={8}>{Mono('An, noch nicht festgelegt')}{agenten({ on: true, level: null, others: false, w: AW, name: 'Agenten, Noch nicht festgelegt' })}</Frame>
  </Frame>;
  return Window({ name: 'App: Einstellungen, Agenten', active: 'einstellungen', title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ info: 'Änderungen gelten sofort und für alle Projekte' }),
    children: [<Frame name="Inhalt" flex="col" items="center" grow={1} h="fill" overflow="hidden">{bereich}</Frame>] });
