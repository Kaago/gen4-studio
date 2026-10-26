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
 * Once per opened project (`project` is a new object each time a project opens, `root` its folder; null: none open), when the computer is
 * calm after it opened (`requestIdleCallback`, else after 2 s). Reading and writing go by the project's folder (`path`), so a job of one
 * project can never write into another one's slot:
 * 1. Read the preview by the folder. Its start town (`standardMap`) is only named while that folder is the open project; none: stop.
 * 2. Draw the town's map picture with `render` (base64 of a PNG; the models come from the open project).
 * 3. Read again by the folder: if it no longer names the same start town, another project was opened meanwhile (the models may have been
 *    its) and the picture is dropped.
 * 4. Write it, unless it is the stored one. Which one is stored can only be seen while no own picture hides it:
 *    - `source === "standard"`: the answer's `png` is the standard file; written only when it differs (`same`, by its colours: the
 *      kernel stores pictures anew, so their text never matches);
 *    - `source === "eigen"`: the own picture hides the standard file; written (a write is cheap, and the file stays current for when the
 *      own picture is removed);
 *    - `source === null`: there is no standard file yet; written.
 * Without a start town (no recipe: Diamant, Perl) nothing is drawn. Every failure (apicula missing, a refused write, a lost bridge) leaves
 * everything as it is, without a message: the map pictures already say in Technik when apicula is missing.
 */
export function useStandardPreview(
  project: { root: string } | null,
  render: (map: number, areaDataId: number) => Promise<string>,
  same: (a: string, b: string) => Promise<boolean> = samePicture,
): void {
  // The newest functions, read when the calm moment comes (the effect runs per project, not per render).
  const latest = useRef({ render, same });
  latest.current = { render, same };

  useEffect(() => {
    if (project === null) return;
    const path = project.root;
    let alive = true;
    const cancel = whenIdle(() => {
      // A callback that was due as the project went is dropped too.
      if (!alive) return;
      void (async () => {
        const town = (await projectPreviewGet(path)).standardMap;
        if (!alive || town === null) return;
        const png = await latest.current.render(town.map, town.areaDataId);
        if (!alive) return;
        const stored = await projectPreviewGet(path);
        const still = stored.standardMap;
        if (!alive || still === null || still.header !== town.header || still.map !== town.map || still.areaDataId !== town.areaDataId) return;
        if (stored.source === "standard" && stored.png !== null && (await latest.current.same(png, stored.png))) return;
        if (!alive) return;
        const written = await projectPreviewSet({ path, slot: "standard", png });
        // The project went while it was written: nobody is told of a project no longer open (Start reads the file when it shows).
        if (alive) previewWritten.emit(written, path);
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
