  // Marken über den Zielbildern, Stand 27.10.2026 (4.0, 4.1a, 4.2). Je Marke ein render mit replace_id auf „Stand: <Zielbild>“; MODE wählt sie
  // (in der erzeugten .jsx umstellen). Maße wie die Marken vom 25.10.: Zeile 36 hoch, Plakette 32 hoch mit Radius 16, Symbol 18, Text 13/20.
  //   'start'            „Stand: Start: Willkommen zurück“ (Seite „Zielbild: Start und Welt“)
  //   'bauen'            „Stand: Prüfen und Bauen“ (Seite „Zielbild: Gegenstände, Texte, Testen“)
  //   'veroeffentlichen' „Stand: Veröffentlichen: Patch erstellen“ (dieselbe Seite)
  const MODE = 'start';
  const MK = {
    start: ['Start: Willkommen zurück', 'teilweise', 'Siehe „App: Start, Willkommen zurück“ und „App: Start, Projekte mit Vorschaubild“. Vorschaubilder auf den Karten (4.0) sind neu, nicht im Zielbild; ohne „Neues Spiel“.'],
    bauen: ['Prüfen und Bauen', 'teilweise', 'Siehe „App: Technik, Prüfen und Bauen mit Schritten“. Befunde ja; die Build-Schritte (4.2) sind neu, nicht im Zielbild.'],
    veroeffentlichen: ['Veröffentlichen: Patch erstellen', 'teilweise', 'Siehe „App: Technik, Veröffentlichen“. Technik › Veröffentlichen statt Assistent, Ziel fest build/, keine Version und keine Änderungsnotizen, keine Taste in der Titelleiste.'],
  }[MODE];
  const art = { umgesetzt: ['status/erfolg-flaeche', 'status/erfolg', 'ph:check-circle-fill', 'ok', 'Umgesetzt'], teilweise: ['status/warnung-flaeche', 'status/warnung', 'ph:circle-half-fill', 'w', 'Teilweise umgesetzt'], offen: ['flaeche/deck', 'text/sekundaer', 'ph:circle-dashed', 's', 'Zielbild, noch offen'] }[MK[1]];
  return <Frame name={'Stand: ' + MK[0]} flex="row" items="center" gap={12} h={36}>
    <Frame flex="row" items="center" gap={6} px={12} h={32} rounded={16} bg={v(art[0])}>{I(art[2], 18, art[3])}{T(art[4], { w: 600, lh: 20, c: art[1] })}</Frame>
    {T(MK[2], { lh: 20, c: 'text/sekundaer' })}
  </Frame>;
