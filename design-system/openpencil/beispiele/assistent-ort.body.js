  // App: Welt bauen, Gebiet anlegen, Schritt „Ort“ (Phase 3f.3): der Assistent „Gebiet anlegen“ in Bauen, Schritt 3 von 4. Die Schrittleiste im
  // Scharnier heißt „Quelle, Anordnung, Ort, Zusammenfassung“; der Schritt liegt als Streifen (286) unter dem Dock, das gemountet bleibt und niedriger
  // wird. Links „Ort“ mit „Vorhandener Ort | Neuer Ort“ (Vorgabe für ein Gebiet: „Neuer Ort“ mit dem Gebietsnamen), rechts „Aussehen“ mit dem Bild der
  // Vorlage und „Aussehen wie Route 201“. Der Fuß trägt den ersten Satz der Zusammenfassung und die Plakette „Nicht rückgängig“.
  // Quellen: apps/studio/src/workbench/wizards/{NewAreaWizard,PlaceStep,LookPicker,WizardFrame,wizardSpecs}.ts(x), workbench/places/PlacePicker.tsx;
  // Plan docs/superpowers/plans/2026-10-26-phase-3f3-leichter-patch.md (Entscheidungen „Schritt in Bauen“, „Inhalt „Ort““); Gate docs/phase-3f-gate.md
  // (3.2), (3.3). Beispielwerte wie im Gate: Quelle Sinnoh, Zellen 3,26 und 4,26 (Karten 0005, 0006, Route 201), Name „Neuheim“.
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);

  const orte = [
    TreeRowX({ label: 'Sinnoh', icon: 'ph:map-trifold', open: true, count: 72 }),
    TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', depth: 1, open: false, sel: true, shown: true, count: 6 }),
    TreeRowX({ label: 'Route 201', icon: 'ph:globe', depth: 1, count: 0 }),
    TreeRowX({ label: 'Sandgemme', icon: 'ph:globe', depth: 1, open: false, count: 7 }),
  ];
  const szene = [
    TreeRowX({ label: 'Zweiblattdorf', icon: 'ph:globe', open: true }),
    TreeRowX({ label: 'Karten', icon: 'ph:mountains', depth: 1, open: false, count: 1 }),
    TreeRowX({ label: 'Figuren', icon: 'ph:person-simple-walk', depth: 1, open: false, count: 8 }),
    TreeRowX({ label: 'Türen', icon: 'ph:door', depth: 1, open: false, count: 4 }),
  ];
  const liste = WeltListe({ orte, szene, orteH: 220 });

  // ----- Dock, niedriger: Schau (3D vorn) und Script/Probleme im Verhältnis von 548 : 240, rechts der Inspector mit dem gewählten Gebiet -----
  const DH = 832 - 44 - 286, SH = Math.round(DH * 548 / 788);
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: SH, bg: 'flaeche/welt',
    children: Welt3D({ w: 740, h: SH - DI.reiter, sel: -1, children: [LayerBar(8, 8)] }) });
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: DH - SH, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
    children: [InspectorHead({ word: 'Gebiet', icon: 'ph:globe', title: 'Zweiblattdorf', ident: 'Zweiblattdorf · T01' }),
      Section({ title: 'Karten', children: Row({ label: 'Ort', children: PlaceBtn('Zweiblattdorf', '#DEE79C') }) }),
      Section({ title: 'Aussehen', open: false }), Section({ title: 'Texte und Scripts', open: false })] });
  const dock = <Frame name="Dock Welt" flex="row" w="fill" h={DH}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;

  // ----- Schritt „Ort“: zwei Spalten -----
  const ort = WizardCol([
    T('Ort', { w: 600 }),
    Seg(['Vorhandener Ort', 'Neuer Ort'], 'Neuer Ort'),
    <Frame flex="col" gap={4} w={360}>{T('Name des neuen Orts', { c: 'text/sekundaer' })}{Field('Neuheim')}</Frame>,
  ], true);
  const aussehen = WizardCol([
    T('Aussehen', { w: 600 }),
    <Frame name="Aussehen wie …" flex="row" items="center" gap={8}><Frame name="Bild der Vorlage" w={64} h={64} rounded={4} bg="#7B9C31" stroke={v('linie/standard')} strokeWidth={1} />{Btn({ label: 'Aussehen wie Route 201', small: true })}</Frame>,
  ]);
  const hand = WizardHand({ name: 'Assistent: Ort', cols: [ort, aussehen], foot: 'Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an.' });

  return Window({ name: 'App: Welt bauen, Gebiet anlegen, Schritt Ort', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE' },
    status: Statusbar({ keys: [['↵', 'Weiter'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 3 von 4: Ort' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{WizardBar({ title: 'Gebiet anlegen', steps: ['Quelle', 'Anordnung', 'Ort', 'Zusammenfassung'], step: 2, last: 'Anlegen', b: 'welt' })}{dock}{hand}</Frame>] });
