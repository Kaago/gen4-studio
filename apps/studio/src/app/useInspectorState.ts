import { useRef, useState } from "react";
import type { InspectorModel, SceneHover } from "../workbench/Inspector";

/**
 * The selection and the scene's hover, apart: the inspector shows the selection (the pin), the middle of the status line names
 * the tile or marker under the pointer.
 */
export function useInspectorState() {
  const [pinned, setPinned] = useState<InspectorModel>({ kind: "none" });
  const [hover, setHover] = useState<SceneHover | null>(null);
  // The key handler is registered once and reads the pin through a ref.
  const pinnedRef = useRef(pinned);
  pinnedRef.current = pinned;
  return { pinned, setPinned, hover, setHover, pinnedRef };
}

export type InspectorState = ReturnType<typeof useInspectorState>;
