import type { ReactNode } from "react";
import type { Live } from "../app/useLive";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData, WorkbenchLive } from "./WorkbenchContext";

/**
 * The providers the panels read: `data` (what changes rarely), `live` (hover and message) and `liveMode` (play mode), each in its own
 * context. It wraps the list column and the deck, which App renders inside the shell. The docks themselves are `ui/frame/Deck`.
 */
export default function Workbench({ data, live, liveMode, children }: { data: WorkbenchData; live: WorkbenchLive; liveMode?: Live; children: ReactNode }) {
  return (
    <WorkbenchProviders data={data} live={live} liveMode={liveMode}>
      {children}
    </WorkbenchProviders>
  );
}
