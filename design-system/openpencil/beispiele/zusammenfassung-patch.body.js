  // App: Welt bauen, Gebiet anlegen, Zusammenfassung mit Patch-Zeile (Phase 3f.3): Schritt 4 von 4 des Assistenten „Gebiet anlegen“ in Bauen, in einem
  // Projekt ohne den Spielcode für neue Orte. Die Zusammenfassung steht als Info-Hinweis (eine Zeile je Satz), darunter die Patch-Zeile: Plakette
  // „Nicht rückgängig“ und das Kästchen der einmaligen Zustimmung, hier angehakt, damit „Anlegen“ im Scharnier frei ist (ohne Haken ist es gesperrt;
  // WizardBar zeichnet keine gesperrte Taste). Quellen: apps/studio/src/workbench/wizards/{NewAreaWizard,PatchConsent,WizardFrame,wizardSpecs}.ts(x);
  // Plan docs/superpowers/plans/2026-10-26-phase-3f3-leichter-patch.md (Entscheidungen „Patch-Zeile“, „Wörter der Assistenten in Bauen“);
  // Gate docs/phase-3f-gate.md (3.4). Beispielwerte wie im Gate: Neuheim, 2 × 1 Karten aus 0005 und 0006 (Route 201).
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

  const DH = 832 - 44 - 286, SH = Math.round(DH * 548 / 788);
  const schau = DockGroup({ name: 'Gruppe Schau', tabs: ['3D', 'Weltkarte'], active: '3D', focused: false, h: SH, bg: 'flaeche/welt',
    children: Welt3D({ w: 740, h: SH - DI.reiter, sel: -1, children: [LayerBar(8, 8)] }) });
  const unten = DockGroup({ name: 'Gruppe Script und Probleme', tabs: ['Script', 'Probleme'], active: 'Script', focused: false, h: DH - SH, edge: 'top',
    children: Empty('Kein Script geöffnet.', 'Eine Scriptdatei öffnet sich über den Ort, ein Event oder ein Problem.') });
  const inspector = DockGroup({ name: 'Gruppe Inspector', tabs: ['Inspector'], active: 'Inspector', focused: false, w: 340, edge: 'left',
    children: [InspectorHead({ word: 'Gebiet', icon: 'ph:globe', title: 'Zweiblattdorf', ident: 'Zweiblattdorf · T01', badges: [['Ungespeichert', 'ungespeichert']] }),
      Section({ title: 'Karten', children: Row({ label: 'Ort', children: PlaceBtn('Zweiblattdorf', '#DEE79C') }) }),
      Section({ title: 'Aussehen', children: Row({ label: 'Musik Tag', changed: true, children: NumField('1005') }) }), Section({ title: 'Texte und Scripts', open: false })] });
  const dock = <Frame name="Dock Welt" flex="row" w="fill" h={DH}>
    <Frame name="Mitte" flex="col" grow={1} h="fill">{schau}{unten}</Frame>
    {inspector}
  </Frame>;

  // ----- Zusammenfassung: Info-Hinweis mit den Sätzen, darunter die Patch-Zeile -----
  const saetze = ['Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an.', 'Kopiert Karten 0005, 0006, mit Gebäuden.', 'Neuer Ort „Neuheim“.', 'Aussehen wie Route 201.', 'Eine neue, leere Liste der Figuren und Türen.', 'Anlegen lässt sich nicht rückgängig machen.'];
  const hinweis = <Frame name="Zusammenfassung" flex="row" items="start" gap={8} px={12} py={8} w="fill" rounded={6} bg={v('status/info-flaeche')}>{I('ph:info-fill', 18, 'info')}<Frame flex="col" gap={2} grow={1}>{saetze.map((s) => T(s))}</Frame></Frame>;
  const spalte = WizardCol([hinweis, PatchLine(true)], true);
  const hand = WizardHand({ name: 'Assistent: Zusammenfassung', cols: [spalte], foot: 'Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an.' });

  return Window({ name: 'App: Welt bauen, Gebiet anlegen, Zusammenfassung mit Patch-Zeile', active: 'welt', dark: DARK, title: { project: 'Sinnoh-Remix', game: 'Platin, DE', unsaved: 1 },
    status: Statusbar({ keys: [['↵', 'Anlegen'], ['⌘[', 'Zurück'], ['Esc', 'Abbrechen']], info: 'Schritt 4 von 4: Zusammenfassung' }),
    children: [liste, <Frame name="Werkbank" flex="col" grow={1} h="fill">{WizardBar({ title: 'Gebiet anlegen', steps: ['Quelle', 'Anordnung', 'Ort', 'Zusammenfassung'], step: 3, last: 'Anlegen', b: 'welt' })}{dock}{hand}</Frame>] });
