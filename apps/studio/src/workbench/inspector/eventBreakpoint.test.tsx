// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { OFF_STATUS } from "../../api.live";
import type { LiveState, LiveStatusDto } from "../../api.live";
import { liveValuesOf } from "../../app/liveStore";
import { liveHingeOf } from "../../app/useLive";
import type { Live } from "../../app/useLive";
import { useEventLive } from "./eventBreakpoint";
import type { EventLive } from "./eventBreakpoint";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const live = (state: LiveState): Live => {
  const s: LiveStatusDto = { ...OFF_STATUS, state };
  return {
    status: s,
    values: liveValuesOf(s),
    active: state !== "off",
    hinge: liveHingeOf(s),
    setFlag: vi.fn(),
    setVar: vi.fn(),
    resume: vi.fn(),
    stop: vi.fn(),
    breakpointOf: () => null,
    toggleBreakpoint: vi.fn(),
  };
};

describe("useEventLive", () => {
  it("is_active_only_in_play_mode_as_the_hinge_shows_it", async () => {
    const container = document.createElement("div");
    const root = createRoot(container);
    let seen: EventLive | undefined;
    function Probe({ mode }: { mode: Live | undefined }) {
      seen = useEventLive(mode);
      return null;
    }
    const activeIn = async (mode: Live | undefined) => {
      await act(async () => root.render(<Probe mode={mode} />));
      return seen?.active;
    };
    expect(await activeIn(undefined)).toBeUndefined();
    // melonDS unavailable: the hinge stays dark, no breakpoint row.
    expect(await activeIn(live("off"))).toBe(false);
    expect(await activeIn(live("unavailable"))).toBe(false);
    for (const state of ["starting", "connecting", "waiting_for_save", "live", "halted", "lost"] as const) expect(await activeIn(live(state))).toBe(true);
    act(() => root.unmount());
  });
});
