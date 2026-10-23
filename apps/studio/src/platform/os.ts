import { inTauri } from "./transport";

/** The app window on macOS: the title bar overlays the content and leaves room for the traffic lights. */
export function isMacTauri(): boolean {
  return inTauri() && /Mac/.test(navigator.platform);
}
