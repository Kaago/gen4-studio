// Ladezeit einer Szene im Browser-Modus messen (Gate 3f, „Ladezeit von Zweiblattdorf“).
//
// In das Browser-Fenster der Test-Umgebung einfügen (javascript_tool), nachdem die Seite mit `?project=<Kopie>` geladen ist,
// dann `await measureSceneLoad('[data-area-key="w0:1"]')` (Platin: Zweiblattdorf; HeartGold: Neuborkia `w0:126`).
// Gemessen wird vom Klick auf die Baumzeile bis zur letzten Antwort der Brücke (`/__bridge/`), wenn 4 s lang keine neue
// Anfrage mehr kam. Die erste Messung nach dem Start der Brücke ist kalt (Kartenmodelle noch nicht im Cache der Brücke),
// jede weitere nach Neuladen der Seite warm. Ergebnis: Millisekunden, Zahl der Anfragen, `get_map` und `get_map_model_glb`.
window.measureSceneLoad = async (selector, quietMs = 4000) => {
  performance.setResourceTimingBufferSize(20000);
  for (let i = 0; i < 120 && !document.querySelector(selector); i++) await new Promise((r) => setTimeout(r, 250));
  await new Promise((r) => setTimeout(r, 3000));
  performance.clearResourceTimings();
  const row = document.querySelector(selector);
  if (!row) return { error: `keine Zeile ${selector}` };
  const t0 = performance.now();
  row.click();
  const bridge = () => performance.getEntriesByType("resource").filter((e) => e.name.includes("/__bridge/"));
  let count = -1;
  let quietSince = performance.now();
  while (performance.now() - quietSince < quietMs) {
    await new Promise((r) => setTimeout(r, 250));
    const n = bridge().length;
    if (n !== count) {
      count = n;
      quietSince = performance.now();
    }
  }
  const entries = bridge();
  const byCommand = {};
  for (const e of entries) {
    const command = e.name.split("/").pop().split("?")[0];
    byCommand[command] = (byCommand[command] ?? 0) + 1;
  }
  const end = Math.max(...entries.map((e) => e.responseEnd));
  return { ms: Math.round(end - t0), requests: entries.length, maps: byCommand.get_map ?? 0, models: byCommand.get_map_model_glb ?? 0 };
};
