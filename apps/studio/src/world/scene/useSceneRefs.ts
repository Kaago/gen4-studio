import { useRef, useState } from "react";
import type { HeaderSummary } from "../../api";
import type { SceneHost } from "./SceneHost";
import type { SceneCallbacks, SceneRefs } from "./types";

/** The latest values the scene reads when it runs (the setup effect, loaders and refreshes), assigned while rendering; the returned object never changes. */
export function useSceneRefs(latest: { showPermissions: boolean; showEvents: boolean; eventFileId: number | null; headers: HeaderSummary[]; areaDataId: number; callbacks: SceneCallbacks }): SceneRefs {
  const handle = useRef<SceneHost | null>(null);
  const showPermissions = useRef(latest.showPermissions);
  const showEvents = useRef(latest.showEvents);
  const eventFile = useRef(latest.eventFileId);
  eventFile.current = latest.eventFileId;
  const headers = useRef(latest.headers);
  headers.current = latest.headers;
  const area = useRef(latest.areaDataId);
  area.current = latest.areaDataId;
  const callbacks = useRef(latest.callbacks);
  callbacks.current = latest.callbacks;
  // Created once (a lazy state initialiser, not a memo that React may recompute): the scene effects capture this object.
  const [refs] = useState(() => ({ handle, callbacks, showPermissions, showEvents, eventFile, headers, area }));
  return refs;
}
