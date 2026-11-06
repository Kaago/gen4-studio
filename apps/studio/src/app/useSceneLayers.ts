import { useCallback, useState } from "react";

/** Which layers of the scene are shown; the setters keep their identity. */
export interface SceneLayers {
  showPermissions: boolean;
  showHeights: boolean;
  showEvents: boolean;
  setShowPermissions(on: boolean): void;
  setShowHeights(on: boolean): void;
  setShowEvents(on: boolean): void;
}

/**
 * The switches of the scene's layers „Gelände“, „Höhen“ and „Ereignisse“. „Höhen“ and „Gelände“ exclude each other (two fills over the
 * same tiles would mix their colours): switching one on switches the other off; both may be off. The scene opens with „Gelände“ and
 * „Ereignisse“.
 */
export function useSceneLayers(): SceneLayers {
  const [fill, setFill] = useState<"permissions" | "heights" | null>("permissions");
  const [showEvents, setShowEventsState] = useState(true);
  const setShowPermissions = useCallback((on: boolean) => setFill((now) => (on ? "permissions" : now === "permissions" ? null : now)), []);
  const setShowHeights = useCallback((on: boolean) => setFill((now) => (on ? "heights" : now === "heights" ? null : now)), []);
  const setShowEvents = useCallback((on: boolean) => setShowEventsState(on), []);
  return { showPermissions: fill === "permissions", showHeights: fill === "heights", showEvents, setShowPermissions, setShowHeights, setShowEvents };
}
