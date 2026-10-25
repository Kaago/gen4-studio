  // App: Start, Neues Spiel aus ROM (Phase 3g.1): der Assistent im eigenen kleinen Deck des Bereichs Start, ohne offenes Projekt.
  // Quellen: apps/studio/src/project/{ProjectStart,NewProjectWizard}.tsx und .module.css, ui/frame/Hinge.
  // Schritt 1 „ROM“ mit einer erkannten Platin-ROM (Deutsch, Freigegeben). Der Name „Sinnoh-Remix“ steht im Ziel der Schau
  // (der Nutzer hat ihn schon eingetippt und ist zu Schritt 1 zurückgegangen).
  // Abweichung: Das Scharnier im Assistentenmodus zeigt die Linie der Welt (bereich/welt-punkt), weil Start keine eigene Bereichsfarbe hat.
  const ROM_PATH = '/Users/du/Roms/Platin.nds';
  const ZIEL = '/Users/du/Documents/Gen4 Studio/Sinnoh-Remix_DSPRE_contents';
  // Label und Wert in zwei Spalten (dl): Label sekundär, Wert primär
  const Fact = (label, value, o = {}) => <Frame flex="row" items="start" gap={12} w="fill">{T(label, { s: 13, c: 'text/sekundaer', width: o.lw || 80 })}{typeof value === 'string' ? T(value, { f: o.mono ? M : F, s: o.mono ? 12 : 13, grow: 1 }) : value}</Frame>;
  // Schau: der Titel der ROM auf dem DS-Bildschirm im dunklen Rahmen (wie das Etikett eines Moduls), daneben, was bisher gewählt ist
  const schau = <Frame name="Vorschau" flex="row" items="center" justify="center" gap={24} p={24} w="fill" grow={1}>
    <Frame flex="col" p={8} rounded={8} bg={v('flaeche/bildschirm-rahmen')}>
      {DSScreen({ scale: 2, children: <Frame position="absolute" x={0} y={0} w={512} h={384} flex="row" items="center" justify="center">{T('POKEMON PL', { f: PX, s: 32, lh: 40, c: 'ds/textbox/flaeche', align: 'center' })}</Frame> })}
    </Frame>
    <Frame flex="col" gap={6} w={360}>
      {Fact('Spiel', 'Platin (Deutsch)', { lw: 56 })}
      {Fact('Vorlage', 'Original', { lw: 56 })}
      {Fact('Ziel', ZIEL, { lw: 56, mono: true })}
    </Frame>
  </Frame>;
  // Hand, Schritt 1: links Pfad und Hinweis, rechts die Karte der ROM
  const colL = <Frame flex="col" gap={8} px={16} py={12} grow={1} h="fill">
    <Frame flex="col" gap={4} w={480}>{T('ROM-Datei', { w: 600 })}{Field(ROM_PATH, { mono: false })}</Frame>
    {Btn({ label: 'Datei wählen…', art: 'Sekundär', icon: 'ph:folder-open' })}
    {T('Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM. Die ROM bleibt unverändert.', { c: 'text/sekundaer', width: 420 })}
  </Frame>;
  const colR = <Frame flex="row" gap={16} px={16} py={12} grow={2} h="fill" strokeWeights={{ top: 0, right: 0, bottom: 0, left: 1 }} stroke={v('linie/leise')}>
    <Frame w={256} h={192}>{DSScreen({ scale: 1, lines: ['POKEMON PL'] })}</Frame>
    <Frame flex="col" gap={4} grow={1}>
      {Fact('Spiel', 'Platin', { lw: 76 })}
      {Fact('Sprache', 'Deutsch', { lw: 76 })}
      {Fact('Spielcode', 'CPUD', { lw: 76, mono: true })}
      {Fact('Freigabe', Badge('Freigegeben', 'ok'), { lw: 76 })}
      {Fact('Basis', 'Bekannte Basis-ROM: Platin DE (Testprojekt)', { lw: 76 })}
    </Frame>
  </Frame>;
  const hand = <Frame name="Assistent: ROM" flex="row" w="fill" h="fill" bg={v('flaeche/deck')}>{colL}{colR}</Frame>;
  return Window({ name: 'App: Start, Neues Spiel aus ROM', active: 'start', title: { project: null },
    status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 1 von 4: ROM', problems: null }),
    children: [Deck({ schau, scharnier: WizardBar({ title: 'Neues Spiel aus ROM', steps: ['ROM', 'Vorlage', 'Name und Ziel', 'Zusammenfassung'], step: 0, last: 'Anlegen', b: 'welt' }), hand })] });
