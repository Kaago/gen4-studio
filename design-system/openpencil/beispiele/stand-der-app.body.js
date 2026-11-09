  // Brett „Stand der App“ auf der Seite „Titel“ (x 1720, y 0), Stand 29.10.2026 nach 4.0, 4.1a, 4.1b, 4.2, 4.3b und 4.4; am 09.11.2026 nachgetragen: 6.2, 6.3b, 6.5a, 6.4a.
  // Ersetzt das Brett vom 25.10. per render mit replace_id (Einzeländerungen lehnt OpenPencil bei dieser Datei ab). Die Höhe wächst mit den Listen; vorher war sie fest 1060
  // und die mittlere Liste lief in „Die größten Abweichungen“. Eine Zeile ist [Screen, Notiz?]; die Marken wie Stand() über den Zielbildern.
  const Marke = (art) => { const m = { umgesetzt: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok', 'Umgesetzt'], teilweise: ['status/warnung-flaeche', 'status/warnung', 'ph:circle-half-fill', 'w', 'Teilweise umgesetzt'], offen: ['flaeche/deck', 'text/sekundaer', 'ph:circle-dashed', 's', 'Zielbild, noch offen'] }[art];
    return <Frame flex="row" items="center" gap={6} px={10} h={28} rounded={14} bg={v(m[0])}>{I(m[2], 16, m[3])}{T(m[4], { w: 600, c: m[1] })}</Frame>; };
  const Zeile = ([titel, notiz], i) => <Frame flex="col" gap={2} px={14} py={8} w="fill" stroke={i ? v('linie/leise') : undefined} strokeWeights={i ? { top: 1, right: 0, bottom: 0, left: 0 } : undefined}>
    {T(titel, { w: 600, width: 450 })}{notiz && T(notiz, { s: 12, c: 'text/sekundaer', width: 450 })}</Frame>;
  const Spalte = (art, anzahl, zeilen) => <Frame flex="col" gap={12} w={480}>
    <Frame flex="row" items="center" gap={10}>{Marke(art)}{T(anzahl, { s: 15, c: 'text/sekundaer' })}</Frame>
    <Frame flex="col" w="fill" rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1} overflow="hidden">{zeilen.map(Zeile)}</Frame>
  </Frame>;
  const umgesetzt = [['Raster: Matrix 0'], ['Pokédex: Staravia'], ['Attacken: Flügelschlag'], ['Gegenstände: Trank'], ['Texte: Archiv, Zweiblattdorf'], ['Befehlspalette'], ['Welt im dunklen Thema']];
  const teilweise = [
    ['Start: Willkommen zurück', 'Marke und Vorschaubild (4.0): Bilder auf den Projektkarten, Technik › Projekt; nicht im Zielbild'],
    ['Weltkarte'], ['Ort anlegen (Gebiet anlegen)'],
    ['Gelände malen (6.4a)', 'Sieben Unterwerkzeuge, Palette, Ebenen und Tileset-Editor im Inspector, Dialog „Neue Karte malen“; „App: Werkbank“, Reihe 6, Brett „Malen“ auf „Komponenten“'],
    ['Skript (Reiter der Welt-Hand)'], ['Dialog bearbeiten (über Texte)'],
    ['Entwicklung und Lernset'], ['Trainer'], ['Begegnungen'],
    ['Prüfen und Bauen', 'Build-Schritte und „Texte auf zwei Seiten geändert“ (4.2), neu, nicht im Zielbild'],
    ['Veröffentlichen: Patch erstellen', 'Technik › Veröffentlichen (4.1a): zwei Karten statt Assistent, Ziel fest build/, keine Taste in der Titelleiste'],
    ['Technik: Patches und Plugins'], ['Neues Spiel (aus ROM)'],
    ['Welt bauen: Figur bearbeiten', 'Werkbank (3e), Seite „App: Werkbank“'],
    ['Einstellungen', 'Agenten (3h), neu, nicht im Zielbild'],
    ['Weltmodell (3f)', 'Raumplan, Schritt „Ort“, Patch-Zeile, geteilter Raum; Seite „App: Werkbank“, Reihe 2'],
    ['Auf Original zurücksetzen (4.1b)', 'Menü „Aktionen“, Kontextmenü, Dialog mit Vorher und Nachher, Technik › Änderungen; „App: Werkbank“ und „App: Texte, Technik, Dialoge“, Reihe 0; kein Zielbild'],
    ['Textprojekt (4.3b)', 'Technik › Textprojekt mit Konflikt, Dialoge „Als Textprojekt einrichten“ und „ID umbenennen“, ID-Zeile im Kopf; „App: Texte, Technik, Dialoge“, Reihe 0 und 1; kein Zielbild'],
    ['Git minimal (4.4)', 'Technik › Änderungen gegen den letzten Commit (Feld und Nachricht), Dialog „Git einrichten“, Git in der Statusleiste; „App: Texte, Technik, Dialoge“, Reihe 0 und 1; kein Zielbild'],
    ['Höhen und Budget (6.2)', 'Schicht „Höhen“ der Schau, Abschnitt „Budget“ im Inspector einer Karte; „App: Werkbank“, Reihe 3, Brett „Budget und Höhenstufen“ auf „Komponenten“; kein Zielbild'],
    ['Backen und Build (6.3b)', 'Abschnitt „Quelle“ einer Karte (Welt-Karte und binär), Welt-Konflikt in Technik › Textprojekt; „App: Werkbank“, Reihe 4, und „App: Texte, Technik, Dialoge“, Reihe 3; kein Zielbild'],
    ['Kartenteile importieren', 'OBJ, glTF und PNG (6.5a): Dialog „Teile importieren…“ mit „Umwandeln“, Vorschau und Budget, hell und dunkel; „App: Werkbank“, Reihe 5; kein Zielbild'],
  ];
  const offen = [['Welt erzählen, Neue Figur', '3d'], ['Geschichten, Graph, Flags', '3d'], ['Neuer Trainer', '3d'], ['Läden', 'später'], ['Spielen live', 'Phase 4']];
  const Punkt = (n, titel, text) => <Frame flex="row" gap={10} w={464}>{T(String(n), { w: 600, c: 'text/sekundaer', width: 14 })}<Frame flex="col" gap={0} w={440}>{T(titel, { w: 600 })}{T(text, { s: 12, c: 'text/sekundaer', width: 440 })}</Frame></Frame>;
  const Besser = (titel, text) => <Frame flex="row" gap={10} w={468}>{I('ph:check-circle-fill', 18, 'ok')}<Frame flex="col" w={440}>{T(titel, { w: 600 })}{T(text, { s: 12, c: 'text/sekundaer', width: 440 })}</Frame></Frame>;
  return <Frame name="Stand der App: 29.10.2026" flex="col" w={1640} gap={28} p={64} bg={v('flaeche/gehaeuse')}>
    <Frame flex="row" w="fill" gap={24} items="end">
      <Frame flex="col" gap={8} grow={1}>
        {T('Stand der App', { f: Z, s: 48, lh: 56, w: 700 })}
        {T('29.10.2026, nach Phase 3 (3c, 3b.3, 3b.4, 3g, 3e, 3h, 3f) und 4.0, 4.1a, 4.1b, 4.2, 4.3b, 4.4; nachgetragen am 09.11.2026: 6.2, 6.3b, 6.5a, 6.4a. Die Seiten „Zielbild: …“ zeigen die Idee vom 23.10. mit einer Marke je Screen. Die Seiten „App: …“ zeichnen die App so nach, wie sie heute aussieht. Komponenten (App) stehen auf der Seite Komponenten unter den Zielbild-Brettern.', { s: 15, lh: 22, c: 'text/sekundaer', width: 1000 })}
      </Frame>
      <Frame flex="col" gap={4} px={16} py={12} rounded={8} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1}>
        {T('Abweichungen im Detail', { w: 600 })}{Mono('design-system/Audit-Design-System.md')}
      </Frame>
    </Frame>
    <Frame flex="row" w="fill" gap={24} items="start">
      {Spalte('umgesetzt', '7 Screens', umgesetzt)}
      {Spalte('teilweise', teilweise.length + ' Screens', teilweise)}
      {Spalte('offen', '8 Screens', offen)}
    </Frame>
    <Frame flex="row" w="fill" gap={48} items="start">
      <Frame flex="col" gap={10} w={984}>
        {T('Die größten Abweichungen', { s: 15, w: 600 })}
        <Frame flex="row" gap={24} w="fill">
          <Frame flex="col" gap={8} w={480}>{Punkt(1, 'Tiefe Erzählen fehlt', 'Nur Bauen und Tüfteln, keine Rezepte (3d).')}{Punkt(2, 'Schau zeigt Daten statt Spielerblick', 'Datenkarten statt DS-Profil, Kampfvorschau, Laden (3d).')}{Punkt(3, 'Kein Spielmodus', '„Hier spielen“ startet den Emulator extern (Phase 4).')}</Frame>
          <Frame flex="col" gap={8} w={480}>{Punkt(4, 'Zweite Sprachebene halb', 'Nummern statt Symbole, Räume nach Kürzel, Grafik als Zahl.')}{Punkt(5, 'Szene nur Farbe', 'Keine Overlay-Muster (Phase 6), Marker als Quader.')}{Punkt(6, 'Assistenten als Formular', 'In der Hand, ohne Vorschau in der Schau (3d).')}</Frame>
        </Frame>
      </Frame>
      <Frame flex="col" gap={10} w={480}>
        {T('Besser als entworfen', { s: 15, w: 600 })}
        {Besser('Prüflicht mit drei Zählern', 'Fehler, Warnungen, Hinweise; „Nicht geprüft“ statt falschem „Alles stimmig“.')}
        {Besser('Verlauf als Menü', 'Die letzten 10 Schritte mit Labels, eigene Taste „Wiederholen“.')}
        {Besser('Prüfen und Beheben', 'Panel Probleme, Fix-Dialog mit Vorher und Nachher, Ausblenden.')}
        {Besser('Daten-Bausteine', 'Typ, Wertebalken, Tabelle, Filter, Hexblock, Pokémon-Karte.')}
      </Frame>
    </Frame>
  </Frame>;
