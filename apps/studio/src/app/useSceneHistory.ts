import { useCallback, useEffect, useRef, useState } from "react";
import { useStableCallback } from "./useStable";

/** How many scenes „Zurück“ can lead through. */
const LIMIT = 50;

interface Trail {
  /** The headers shown, oldest first. */
  scenes: number[];
  /** The scene that shows now (-1: none yet). */
  at: number;
}

const EMPTY: Trail = { scenes: [], at: -1 };

/**
 * The history of the scenes shown (header ids, at most `LIMIT`): every change of `selected` by a click, a door, the picker, the palette or a
 * link is kept once; „Zurück“ and „Vor“ show a scene of it through `show` (the same way a click shows one) without recording that jump
 * again. A new scene after „Zurück“ drops what was ahead; another `project` starts over.
 */
export function useSceneHistory(selected: number | null, show: (header: number) => void, project: string | null = null) {
  // The trail is read by the jumps at once (a ref), the render learns of it through `moved`.
  const trail = useRef<Trail>(EMPTY);
  const [, moved] = useState(0);
  const shownIn = useRef(project);
  const jump = useStableCallback(show);

  useEffect(() => {
    if (shownIn.current !== project) {
      shownIn.current = project;
      trail.current = EMPTY;
      moved((n) => n + 1);
    }
    if (selected === null) return;
    const { scenes, at } = trail.current;
    // The scene a jump of the history just showed (or the one shown again) is not a new one.
    if (scenes[at] === selected) return;
    const kept = [...scenes.slice(0, at + 1), selected].slice(-LIMIT);
    trail.current = { scenes: kept, at: kept.length - 1 };
    moved((n) => n + 1);
  }, [selected, project]);

  const step = useCallback(
    (by: -1 | 1) => {
      const { scenes, at } = trail.current;
      const to = at + by;
      if (to < 0 || to >= scenes.length) return;
      // Moved before the scene shows, so the change of `selected` that follows finds it recorded.
      trail.current = { scenes, at: to };
      moved((n) => n + 1);
      jump(scenes[to]);
    },
    [jump],
  );
  const back = useCallback(() => step(-1), [step]);
  const forward = useCallback(() => step(1), [step]);

  const { scenes, at } = trail.current;
  return { back, forward, canBack: at > 0, canForward: at >= 0 && at < scenes.length - 1 };
}
