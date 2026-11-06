import { useRef, useState } from "react";
import type { SceneHost } from "./SceneHost";
import type { SceneCallbacks, SceneRefs } from "./types";

/** The latest values the scene reads when it runs (the setup effect, loaders and refreshes), assigned while rendering; the returned object never changes. */
export function useSceneRefs(latest: { showPermissions: boolean; showHeights: boolean; showEvents: boolean; eventFileId: number | null; destName: (headerId: number) => string | undefined; areaDataId: number; callbacks: SceneCallbacks }): SceneRefs {
  const handle = useRef<SceneHost | null>(null);
  const showPermissions = useRef(latest.showPermissions);
  const showHeights = useRef(latest.showHeights);
  const showEvents = useRef(latest.showEvents);
  const eventFile = useRef(latest.eventFileId);
  eventFile.current = latest.eventFileId;
  const destName = useRef(latest.destName);
  destName.current = latest.destName;
  const area = useRef(latest.areaDataId);
  area.current = latest.areaDataId;
  const callbacks = useRef(latest.callbacks);
  callbacks.current = latest.callbacks;
  // Created once (a lazy state initialiser, not a memo that React may recompute): the scene effects capture this object.
  const [refs] = useState(() => ({ handle, callbacks, showPermissions, showHeights, showEvents, eventFile, destName, area }));
  return refs;
}
