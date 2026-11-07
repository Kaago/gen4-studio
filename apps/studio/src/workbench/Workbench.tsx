import type { MutableRefObject, ReactNode } from "react";
import type { Live } from "../app/useLive";
import { PaintProvider } from "./paint/PaintContext";
import type { PaintModel } from "./paint/PaintContext";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData, WorkbenchLive } from "./WorkbenchContext";

/**
 * The providers the panels read: `data` (what changes rarely), `live` (hover and message) and `liveMode` (play mode), each in its own
 * context, and the paint state of „Gelände malen“ (`usePaint`; `paint` hands it to the app's shortcuts). It wraps the list column and the
 * deck, which App renders inside the shell. The docks themselves are `ui/frame/Deck`.
 */
export default function Workbench({
  data,
  live,
  liveMode,
  paint,
  children,
}: {
  data: WorkbenchData;
  live: WorkbenchLive;
  liveMode?: Live;
  paint?: MutableRefObject<PaintModel | null>;
  children: ReactNode;
}) {
  return (
    <WorkbenchProviders data={data} live={live} liveMode={liveMode}>
      <PaintProvider bridge={paint}>{children}</PaintProvider>
    </WorkbenchProviders>
  );
}
