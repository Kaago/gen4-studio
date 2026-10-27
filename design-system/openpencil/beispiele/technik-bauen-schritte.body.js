  // App: Technik, Prüfen und Bauen mit Schritten (Phase 4.2): die Fläche „Prüfen und Bauen“ nach einem Build, zwei Frames aus einer Vorlage; MODE wählt
  // den Zustand (in der erzeugten .jsx umstellen):
  //   'schritte'  „App: Technik, Prüfen und Bauen mit Schritten“: Tasten „Bauen“, „Neu bauen (ohne Cache)“, „Starten“, „Letzter Build“ und „Pfad“,
  //               die Überschrift „Schritte“ mit der Tabelle der gelaufenen Schritte und „Übersprungen (38)“ eingeklappt, darunter „Probleme“
  //   'konflikt'  „App: Technik, Texte auf zwei Seiten geändert“: derselbe Screen vor dem Build (noch ohne Schritte), darüber der Dialog der Rückfrage
  // Quellen: apps/studio/src/technik/{TechnikArea,BuildSteps}.tsx und .module.css, technik/stepWords.ts (Wörter), app/TextConflictDialog.tsx,
  // ui/primitives/{Table,Section,Segmented,Dialog}.module.css; Plan docs/superpowers/plans/2026-10-27-phase-4-2-build-graph.md (Entscheidungen
  // „Anzeige“, „Neu bauen“, „Konflikt beim Bauen“); Texte aus design-system/README.md, Abschnitt „Nachtrag 4.2“. Beispielwerte aus HeartGold DE
  // nach einer Kartenänderung: „Karten packen: 1 Member eingesetzt“, „Overlays: 127 aus dem Cache“, „ROM schreiben 0,05 s“, „Übersprungen (38)“;
  // ein DSPRE-Ordner hat keine Basis-Marke, darum „Prüfsummen: keine Basis-Marke“.
  const MODE = 'schritte';
  if (HX_DUNKEL) Object.assign(HX, HX_DUNKEL);
  const KONFLIKT = MODE === 'konflikt';

  // ----- Liste „Technik“: vier Einträge mit Symbol und Zustand -----
  const eintraege = [
    ['Prüfen und Bauen', 'ph:hammer', KONFLIKT ? 'Noch nicht gebaut' : 'gebaut gerade eben', true],
    ['Patches', 'ph:bandaids', 'Dynamische Header: angewendet'],
    ['Emulator', 'ph:game-controller', '/Applications/melonDS.app'],
    ['Werkzeuge', 'ph:toolbox', 'Alle bereit'],
  ];
  const liste = ListPane([PanelHead({ title: 'Technik' }), <Frame name="Einträge" flex="col" gap={2} p={8} w="fill">
    {eintraege.map(([title, icon, sub, sel]) => ListRow({ title, icon, sub, sel }))}
  </Frame>]);

  // ----- Fläche „Prüfen und Bauen“: höchstens 880 breit, 32 oben, 24 seitlich, Abstand 16 -----
  const BW = 880;
  const Rahmen = (name, children) => <Frame name={name} flex="col" w={BW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/leise')} strokeWidth={1} overflow="hidden">{children}</Frame>;
  // Spalten: Schritt und Ergebnis teilen den Rest, Dauer 96 rechtsbündig; je Zelle 8 innen, Zeile dichte/zeile mit Linie linie/leise unten
  const SW = Math.round((BW - 96) / 2);
  const Zelle = (s, w, o = {}) => <Frame flex="row" items="center" gap={8} px={8} w={w} h="fill" justify={o.end ? 'end' : 'start'}>{T(s, { s: o.s || 13, w: o.w, c: o.c, one: true })}</Frame>;
  const KopfZeile = <Frame name="Kopfzeile" flex="row" w="fill" h={DI.steuer} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/standard')}>
    {Zelle('Schritt', SW, { s: 12, w: 600, c: 'text/sekundaer' })}{Zelle('Ergebnis', SW, { s: 12, w: 600, c: 'text/sekundaer' })}{Zelle('Dauer', 96, { s: 12, w: 600, c: 'text/sekundaer', end: true })}
  </Frame>;
  const Zeile = ([schritt, ergebnis, dauer]) => <Frame name={schritt} flex="row" w="fill" h={DI.zeile} strokeWeights={{ top: 0, right: 0, bottom: 1, left: 0 }} stroke={v('linie/leise')}>
    {Zelle(schritt, SW)}{Zelle(ergebnis, SW)}{Zelle(dauer, 96, { end: true })}
  </Frame>;
  const GELAUFEN = [
    ['Karten packen', '1 Member eingesetzt', '0,02 s'],
    ['Spielcode (ARM9)', '1 aus dem Cache', '< 0,01 s'],
    ['Overlays', '127 aus dem Cache', '0,01 s'],
    ['ROM schreiben', 'geschrieben', '0,05 s'],
    ['Prüfsummen', 'keine Basis-Marke', '< 0,01 s'],
  ];
  const schritte = <Frame name="Build-Schritte" flex="col" gap={8} w={BW}>
    {Rahmen('Schritte des letzten Builds', [KopfZeile, ...GELAUFEN.map(Zeile)])}
    {Rahmen('Übersprungen', <Frame flex="row" items="center" gap={6} px={8} h={DI.steuer} w="fill">{I('ph:caret-right', 18)}{T('Übersprungen (38)', { w: 600 })}</Frame>)}
  </Frame>;

  const Fakt = (label, wert, mono) => <Frame flex="col" gap={2}>{T(label, { s: 12, c: 'text/sekundaer' })}{T(wert, { f: mono ? M : F, s: 13 })}</Frame>;
  const flaeche = <Frame name="Prüfen und Bauen" flex="col" gap={16} pt={32} px={24} grow={1} h="fill" bg={v('flaeche/deck')} overflow="hidden">
    <Frame name="Kopf" flex="col" gap={4}>{T('Prüfen und Bauen', { f: Z, s: 24, lh: 30, w: 700 })}</Frame>
    <Frame name="Tasten" flex="row" items="center" gap={8}>
      {Btn({ label: 'Bauen', art: 'Primär', icon: 'ph:hammer' })}{Btn({ label: 'Neu bauen (ohne Cache)', icon: 'ph:hammer' })}{Btn({ label: 'Starten', icon: 'ph:play' })}
    </Frame>
    <Frame name="Letzter Build" flex="row" gap={24}>
      {Fakt('Letzter Build', KONFLIKT ? 'Noch nicht gebaut' : 'gerade eben')}
      {!KONFLIKT && Fakt('Pfad', '/Users/du/Roms/_testkopien/4-2-browser-hgde/out.nds', true)}
    </Frame>
    {T('Schritte', { w: 600, s: 15 })}
    {KONFLIKT ? T('Noch kein Build in dieser Sitzung.') : schritte}
    {T('Probleme', { w: 600, s: 15 })}
    {T('Keine Probleme. Die Prüfungen laufen nach jeder Änderung; beim Speichern und Bauen landen Fehler und Warnungen hier.', { width: 640 })}
  </Frame>;

  const win = Window({ name: KONFLIKT ? 'Fenster' : 'App: Technik, Prüfen und Bauen mit Schritten', active: 'technik', title: { project: 'Johto-Remix', game: 'HeartGold, DE' },
    status: Statusbar({ problems: 'Keine Probleme', probArt: 'ok', build: KONFLIKT ? 'Noch nicht gebaut' : 'Gebaut gerade eben', built: !KONFLIKT }),
    children: [liste, flaeche] });
  if (!KONFLIKT) return win;

  // ----- Dialog „Texte auf zwei Seiten geändert“: 480 breit wie DialogX, je Archiv eine Zeile mit Segmenten der Größe S -----
  const DW = 480;
  const Archiv = (n, wahl) => <Frame name={'Textarchiv ' + n} flex="row" items="center" w="fill" minH={DI.zeile}>{T('Textarchiv ' + n, { w: 600, grow: 1 })}{SegS(['Text (JSON)', 'Spieldatei'], wahl)}</Frame>;
  const dialog = <Frame name="Dialog" flex="col" gap={12} p={20} w={DW} rounded={12} bg={v('flaeche/blatt')} stroke={v('linie/standard')} strokeWidth={1} shadow={SCHATTEN}>
    <Frame name="Schließen" position="absolute" x={DW - 8 - DI.steuer} y={8} w={DI.steuer} h={DI.steuer} rounded={6} flex="row" items="center" justify="center">{I('ph:x', DI.symbol, 's')}</Frame>
    {T('Texte auf zwei Seiten geändert', { s: 18, w: 600, width: DW - 72 })}
    {T('Diese Textarchive wurden als Text (JSON) und als Spieldatei geändert, seit das Studio sie zuletzt gebaut hat, etwa in DSPRE oder durch einen Git-Wechsel. Wähle je Archiv, welche Fassung gilt; die andere wird überschrieben.', { c: 'text/sekundaer', width: DW - 40 })}
    <Frame name="Archive" flex="col" gap={4} w="fill">{Archiv('0012', 'Text (JSON)')}{Archiv('0433', 'Spieldatei')}</Frame>
    <Frame name="Tasten" flex="row" justify="end" gap={8} w="fill">{Btn({ label: 'Abbrechen' })}{Btn({ label: 'Übernehmen und bauen', art: 'Primär' })}</Frame>
  </Frame>;
  return MitDialog('App: Technik, Texte auf zwei Seiten geändert', win, dialog);
