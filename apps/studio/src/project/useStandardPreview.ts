import { useEffect, useRef } from "react";
import { projectPreviewGet, projectPreviewSet } from "../api";
import { previewWritten } from "./previewMessages";
import { samePicture } from "./previewFromScene";

/** How long to wait where the webview has no `requestIdleCallback` (WebKit, the Tauri window on macOS). */
const FALLBACK_MS = 2000;

/** Runs `run` once the computer is calm; answers what cancels it. */
function whenIdle(run: () => void): () => void {
  if (typeof requestIdleCallback === "function") {
    const id = requestIdleCallback(run);
    return () => cancelIdleCallback(id);
  }
  const id = setTimeout(run, FALLBACK_MS);
  return () => clearTimeout(id);
}

/**
 * The standard picture of the open project (phase 4.0 §3.4): the start town's map picture, written to the slot "standard"
 * (`.g4/cache/preview-standard.png`), so Start shows it on the card of a project without an own picture.
 *
 * Once per opened project (`project` is a new object each time a project opens; null: none open), when the computer is calm after it
 * opened (`requestIdleCallback`, else after 2 s): reads the preview; with a start town (`standardMap`) draws its map picture with `render`
 * (base64 of a PNG) and writes it, unless it is the stored one. Which one is stored can only be seen while no own picture hides it:
 * - `source === "standard"`: the answer's `png` is the standard file; the picture is written only when it differs (`same`, by its colours:
 *   the kernel stores pictures anew, so their text never matches);
 * - `source === "eigen"`: the own picture hides the standard file; the picture is written (a write is cheap, and the file stays current
 *   for when the own picture is removed);
 * - `source === null`: there is no standard file yet; the picture is written.
 * Without a start town (no recipe: Diamant, Perl) nothing is drawn. Every failure (apicula missing, a refused write, a lost bridge) leaves
 * everything as it is, without a message: the map pictures already say in Technik when apicula is missing.
 */
export function useStandardPreview(
  project: object | null,
  render: (map: number, areaDataId: number) => Promise<string>,
  same: (a: string, b: string) => Promise<boolean> = samePicture,
): void {
  // The newest functions, read when the calm moment comes (the effect runs per project, not per render).
  const latest = useRef({ render, same });
  latest.current = { render, same };

  useEffect(() => {
    if (project === null) return;
    let alive = true;
    const cancel = whenIdle(() => {
      // A callback that was due as the project went is dropped too.
      if (!alive) return;
      void (async () => {
        const stored = await projectPreviewGet();
        const town = stored.standardMap;
        if (!alive || town === null) return;
        const png = await latest.current.render(town.map, town.areaDataId);
        if (!alive) return;
        if (stored.source === "standard" && stored.png !== null && (await latest.current.same(png, stored.png))) return;
        if (!alive) return;
        previewWritten.emit(await projectPreviewSet({ slot: "standard", png }));
      })().catch(() => {
        /* silent: no standard picture this time */
      });
    });
    return () => {
      alive = false;
      cancel();
    };
  }, [project]);
}
