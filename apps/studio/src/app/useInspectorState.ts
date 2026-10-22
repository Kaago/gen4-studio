import { useRef, useState } from "react";
import type { InspectorModel } from "../workbench/Inspector";
import { shownInspector } from "../workbench/pin";

/**
 * What the inspector shows: the selected header (or building, event, link target) is pinned, and
 * the scene's hover overrides it only while the pointer is over a cell or marker.
 */
export function useInspectorState() {
  const [pinned, setPinned] = useState<InspectorModel>({ kind: "none" });
  const [hover, setHover] = useState<InspectorModel | null>(null);
  // A selected building or event stays in the inspector while the pointer is over the scene (it is
  // being dragged there); other event markers still take over.
  const inspector = shownInspector(pinned, hover);
  // The key handler is registered once and reads the pin through a ref.
  const pinnedRef = useRef(pinned);
  pinnedRef.current = pinned;
  return { pinned, setPinned, hover, setHover, inspector, pinnedRef };
}

export type InspectorState = ReturnType<typeof useInspectorState>;
