  // App: Start, Leere Region wählen (Phase 3g.2): Neues Spiel aus ROM, Schritt 2 „Vorlage“, die Karte „Leere Region mit Startstadt“ gewählt.
  // Quellen: apps/studio/src/project/NewProjectWizard.tsx (CARDS, TemplateCards) und .module.css. Rahmen wie projektstart.body.js.
  // Die Zusammenfassung („… Scripts werden übernommen.“) steht erst im Schritt 4 im Fuß, nicht hier; „Nicht geprüft“ erscheint nur bei einer ROM
  // mit dieser Freigabe (hier: Platin DE, Freigegeben, also keine Plakette). Abweichung: Das Scharnier zeigt die Linie der Welt (bereich/welt-punkt).
  const ZIEL = '/Users/du/Documents/Gen4 Studio/Sinnoh-Remix_DSPRE_contents';
  const Fact = (label, value, o = {}) => <Frame flex="row" items="start" gap={12} w="fill">{T(label, { s: 13, c: 'text/sekundaer', width: 80 })}{typeof value === 'string' ? T(value, { f: o.mono ? M : F, s: o.mono ? 12 : 13, grow: 1 }) : value}</Frame>;
  // Schau: der Titel der ROM auf dem DS-Bildschirm im dunklen Rahmen; die Zeile Vorlage zeigt die Wahl („Leere Region“)
  const schau = <Frame name="Vorschau" flex="row" items="center" justify="center" gap={24} p={24} w="fill" grow={1}>
    <Frame flex="col" p={8} rounded={8} bg={v('flaeche/bildschirm-rahmen')}>
      {DSScreen({ scale: 2, children: <Frame position="absolute" x={0} y={0} w={512} h={384} flex="row" items="center" justify="center">{T('POKEMON PL', { f: PX, s: 32, lh: 40, c: 'ds/textbox/flaeche', align: 'center' })}</Frame> })}
    </Frame>
    <Frame flex="col" gap={6} w={360}>
      {Fact('Spiel', 'Platin (Deutsch)')}
      {Fact('Vorlage', 'Leere Region')}
      {Fact('Ziel', ZIEL, { mono: true })}
    </Frame>
  </Frame>;
  // Auswahlkarte: Kopf mit Zeichen (gewählt: gefülltes Häkchen, sonst offener Kreis) und Titel, darunter der Satz; gewählt trägt auswahl/flaeche
  // und die doppelte Kante auswahl/kante (nie nur Farbe)
  const Karte = (title, text, on) => <Frame flex="col" items="start" gap={6} p={12} grow={1} minH={112} rounded={4} bg={v(on ? 'auswahl/flaeche' : 'flaeche/blatt')} stroke={v(on ? 'auswahl/kante' : 'linie/standard')} strokeWidth={on ? 2 : 1}>
    <Frame flex="row" items="center" gap={8}>{I(on ? 'ph:check-circle-fill' : 'ph:circle', 18, on ? 'at' : 's')}{T(title, { s: 15, w: 600 })}</Frame>
    {T(text, { c: 'text/sekundaer', width: 220 })}
  </Frame>;
  const hand = <Frame name="Assistent: Vorlage" flex="col" gap={8} px={16} py={12} w="fill" h="fill" bg={v('flaeche/deck')}>
    {T('Womit startet das Projekt?', { w: 600 })}
    <Frame name="Karten (radiogroup)" flex="row" gap={8} w="fill">
      {Karte('Original', 'Das Spiel, wie es ist.', false)}
      {Karte('Leere Region mit Startstadt', 'Nur die Startstadt mit Spielerhaus bleibt, der Rest ist eine leere Region. Ein Start-Script gibt dir deinen ersten Partner.', true)}
      {Karte('Nur Daten', 'Für Balancing: Werte, Lernsets, Trainer. Startet im Bereich Pokémon.', false)}
    </Frame>
  </Frame>;
  return Window({ name: 'App: Start, Leere Region wählen', active: 'start', title: { project: null },
    status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 2 von 4: Vorlage', problems: null }),
    children: [Deck({ schau, scharnier: WizardBar({ title: 'Neues Spiel aus ROM', steps: ['ROM', 'Vorlage', 'Name und Ziel', 'Zusammenfassung'], step: 1, last: 'Anlegen', b: 'welt' }), hand })] });
