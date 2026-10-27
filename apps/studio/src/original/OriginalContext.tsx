import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { AssetId, AssetKind, HeaderFamily, OriginalStatus, ResetResult, ResetTarget } from "../api";
import { ResetDialog } from "./ResetDialog";

export interface ResetApi {
  /** The base ROM as resetting needs it (`resetEntry` greys out by it); null while it is read and without a project. */
  status: OriginalStatus | null;
  /** Opens „… auf Original zurücksetzen?“ for `target`; `scriptDirty`: the script editor holds unsaved changes of that script file. */
  open: (target: ResetTarget, opts?: { scriptDirty?: boolean }) => void;
}

const NONE: ResetApi = { status: null, open: () => {} };
const ResetContext = createContext<ResetApi>(NONE);

/** What every place offering „Auf Original zurücksetzen…“ needs: the status for its entry and `open` for the dialog. */
export function useReset(): ResetApi {
  return useContext(ResetContext);
}

/** Gives `value` to every `useReset` below: a part shown on its own (the gallery, a test) without the app's dialog. */
export function ResetProvider({ value, children }: { value: ResetApi; children: ReactNode }) {
  return <ResetContext.Provider value={value}>{children}</ResetContext.Provider>;
}

export type OriginalProviderProps = {
  children: ReactNode;
  /** The reset went in (the dialog has closed): its result and the entity's name („Karte 0003“). */
  onReset: (result: ResetResult, label: string) => void | Promise<void>;
  /** The base ROM as resetting needs it, from `useOriginal` (the app holds it, so an agent's base change can read it again). */
  status: OriginalStatus | null;
  /** The open project's game family (`ProjectInfo.family`: `Dp`, `Pt`, `Hgss`), for the names of the header fields; null without a project. */
  family: string | null;
  /** The store's assets with unsaved edits: the dialog says they are replaced. */
  dirty: readonly AssetId[];
};

/** The header family of the project's game family (`ProjectInfo.family` is the kernel's `Dp`, `Pt`, `Hgss`). */
export function headerFamilyOf(family: string): HeaderFamily | null {
  switch (family.toLowerCase()) {
    case "dp":
      return "DP";
    case "pt":
      return "Pt";
    case "hgss":
      return "HGSS";
    default:
      return null;
  }
}

const SINGLE: Partial<Record<ResetTarget["kind"], AssetKind>> = {
  map: "Map",
  event_file: "Event",
  text_archive: "TextArchive",
  trainer: "Trainer",
  encounter: "Encounter",
};

/** The store assets a reset of `target` sets (the kernel's `ResetTarget::assets`); none for the file targets, which are no store values. */
function storeAssets(target: ResetTarget): AssetId[] {
  switch (target.kind) {
    case "text_message":
      return [{ kind: "TextArchive", index: target.archive }];
    case "header":
      return [{ kind: "HeaderTable", index: 0 }];
    case "species":
      return (["Personal", "Learnset", "Evolution"] as const).map((kind) => ({ kind, index: target.index }));
    case "script":
      return [];
    default: {
      const kind = SINGLE[target.kind];
      return kind ? [{ kind, index: target.index }] : [];
    }
  }
}

/** One `open`; `serial` tells a dialog's close from that of the one before (a slow reset must not close a dialog opened meanwhile). */
type Request = { target: ResetTarget; scriptDirty: boolean; serial: number };

/**
 * Shares the status of the base ROM and holds the one reset dialog: `useReset().open(target)` shows it; after a reset `onReset` gets the
 * result (the app reloads the changed assets and says so).
 */
export function OriginalProvider({ children, onReset, status, family, dirty }: OriginalProviderProps) {
  const [request, setRequest] = useState<Request | null>(null);
  // Counts every `open` for as long as the provider lives, so no two dialogs share a serial.
  const serials = useRef(0);
  const open = useCallback((target: ResetTarget, opts?: { scriptDirty?: boolean }) => {
    serials.current += 1;
    setRequest({ target, scriptDirty: opts?.scriptDirty ?? false, serial: serials.current });
  }, []);
  const api = useMemo<ResetApi>(() => ({ status, open }), [status, open]);
  const unsaved = request !== null && storeAssets(request.target).some((a) => dirty.some((d) => d.kind === a.kind && d.index === a.index));
  return (
    <ResetContext.Provider value={api}>
      {children}
      {request !== null && (
        <ResetDialog
          key={request.serial}
          target={request.target}
          baseLabel={status?.state === "ready" ? status.label : null}
          family={family === null ? null : headerFamilyOf(family)}
          scriptDirty={request.scriptDirty}
          dirty={unsaved}
          onClose={() => setRequest((shown) => (shown?.serial === request.serial ? null : shown))}
          onReset={onReset}
        />
      )}
    </ResetContext.Provider>
  );
}
