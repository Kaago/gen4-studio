import { useEffect } from "react";
import { onThemeChange } from "../../ui/tokens/theme";
import { sceneBackground, selectionColor } from "../colors";
import { recolorMarkerOutline } from "../markers";
import type { SceneRefs } from "./types";

/**
 * Recolours the background of the scene and the selection outlines when the effective theme switches (also a system change
 * while the theme is "system"): one `setBackground` and one render per switch. The models and the content are not touched. Declared after
 * `useSceneSetup`, whose effect creates the host this one finds through `refs.handle`.
 */
export function useSceneTheme(refs: Pick<SceneRefs, "handle">) {
  useEffect(
    () =>
      onThemeChange((theme) => {
        const h = refs.handle.current;
        if (!h || h.disposed) return;
        h.setBackground(sceneBackground(theme));
        // The selection outlines that are drawn now change their colour; they are not rebuilt.
        const outline = selectionColor(theme);
        h.buildings?.recolorOutlines(outline);
        if (h.markers) recolorMarkerOutline(h.markers, outline);
        h.render();
      }),
    [refs],
  );
}
