import { useEffect, useState } from "react";
import type { DraftDto, MatrixDto } from "../../api";
import { setDraft } from "./draft";
import type { SceneRefs } from "./types";

/**
 * Draws `draft` on the scene of `matrix` (see `setDraft`) and returns how many cells, tiles and markers it drew. It draws again when the
 * draft changes and after every build of the scene (`sceneBuilt`): the heights of the frames follow the altitudes of the shown matrix.
 * While the scene still shows another matrix, the layer is empty. Renders.
 */
export function useDraft(refs: SceneRefs, draft: DraftDto | null, matrix: MatrixDto, sceneBuilt: number): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    setCount(setDraft(h, draft, h.matrix === matrix ? matrix : null));
    h.render();
  }, [draft, matrix, sceneBuilt]);
  return count;
}
