import type { ReactNode } from "react";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData, WorkbenchLive } from "./WorkbenchContext";

/**
 * The providers the panels read: `data` (what changes rarely) and `live` (hover and message), each in its own context. It wraps the
 * list column and the deck, which App renders inside the shell. The docks themselves are `ui/frame/Deck`.
 */
export default function Workbench({ data, live, children }: { data: WorkbenchData; live: WorkbenchLive; children: ReactNode }) {
  return (
    <WorkbenchProviders data={data} live={live}>
      {children}
    </WorkbenchProviders>
  );
}
