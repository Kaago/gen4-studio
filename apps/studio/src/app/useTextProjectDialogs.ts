import { useEffect, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { AdoptReport, AssetId, HeaderSummary, MigrateReport, ProjectInfo, RenameResult, RevertReport, TextProjectStatus } from "../api";
import { AGENT_SETTING_UP } from "../technik/TextProjectSurface";
import type { TextProjectView } from "../technik/TextProjectSurface";
import { adoptedText } from "../textproject/AdoptDialog";
import { liftedText } from "../textproject/MigrateDialog";
import { renamedText } from "../textproject/RenameIdDialog";
import { revertedText } from "../textproject/RevertDialog";
import { EntityIdCache } from "../textproject/TextProjectContext";
import { dismissHint, hintOf, shouldHint } from "../textproject/textProjectHint";
import { toast } from "../ui/frame/toasts";
import type { ExternalSyncApi } from "./useExternalSync";
import type { ProjectSession } from "./useProjectSession";
import { useStableCallback } from "./useStable";

/** The key of the open hint: one at a time, gone when the project closes, another opens or the folder is set up. */
export const HINT_TOAST = "text-project-hint";

/** Why „Einrichten…“ opens nothing while a build runs. */
export const AFTER_BUILD = "Erst wenn der Build fertig ist.";

/** Why „Einrichten…“ opens nothing while the folder's state as a text project is read. */
export const STILL_READING = "Das Projekt wird noch gelesen.";

interface TextProjectDialogsParams {
  session: ProjectSession;
  /** The open folder as a text project; null while it is read. */
  projectText: TextProjectStatus | null;
  sync: ExternalSyncApi;
  /** A conflict was resolved in Technik › Textprojekt. */
  onResolved: (assets: AssetId[]) => void;
  /** The agents' reload queue: a new session is read in it, after what an agent changed before. */
  reloads: MutableRefObject<Promise<void>>;
  /** Swaps the scene's and the editors' state for the new project's header list (`useProjectSession.onOpen`). */
  resetProject: (headers: HeaderSummary[]) => void;
  /** The saved project changed: Technik › Änderungen compares again. */
  bumpChanged: () => void;
  /** Counts up whenever the ids may have changed (a rename, an undo or redo, a sync that took something over). */
  idsRevision: number;
  bumpIds: () => void;
  /** Whether a dialog may open now (no wizard holds the frame). */
  canOpen: () => boolean;
  /** „Basis-ROM zuordnen…“ of the hint: Technik › Veröffentlichen. */
  showPublish: () => void;
  /** The project was closed (an older schema not lifted): the app shows Start. */
  onClosed: () => void;
  /** An agent sets the open folder up as text project (the hub's `busy`): set-up and take-back wait. */
  agentSetUp?: boolean;
}

/**
 * The text project's dialogs as the app holds them: „Als Textprojekt einrichten…“ (Technik, the open hint) and „Zurücknehmen…“, the lift of
 * an older schema („Abbrechen“ closes the project in the backend too), a rename's message, the cache of the ids the heads show, and the
 * open hint for a DSPRE folder.
 */
export function useTextProjectDialogs({
  session,
  projectText,
  sync,
  onResolved,
  reloads,
  resetProject,
  bumpChanged,
  idsRevision,
  bumpIds,
  canOpen,
  showPublish,
  onClosed,
  agentSetUp = false,
}: TextProjectDialogsParams) {
  const [opened, setDialog] = useState<"adopt" | "revert" | null>(null);
  // „Als Textprojekt einrichten?“ belongs to a DSPRE folder only: once the folder is a text project (an agent set it up, the session was
  // read anew) the dialog is gone, and a stale „Einrichten…“ opens nothing. Not while `onAdopted` finishes the dialog's own set-up: the
  // session is read anew (the folder is a text project) before it says so, and the dialog stays until then (it closes itself after).
  const [adoptRunning, setAdoptRunning] = useState(false);
  const dialog = opened === "adopt" && !adoptRunning && projectText?.state !== "dspre" ? null : opened;
  useEffect(() => {
    if (opened === "adopt" && !adoptRunning && projectText !== null && projectText.state !== "dspre") setDialog(null);
  }, [opened, adoptRunning, projectText]);
  const open = useStableCallback((which: "adopt" | "revert") => {
    if (session.info === null || !canOpen()) return;
    if (which === "adopt") {
      if (projectText === null) {
        toast.info(STILL_READING);
        return;
      }
      if (projectText.state !== "dspre") return;
    }
    if (agentSetUp) {
      toast.info(AGENT_SETTING_UP);
      return;
    }
    if (session.busy) {
      toast.info(AFTER_BUILD);
      return;
    }
    setDialog(which);
  });
  const textProject: TextProjectView | null =
    session.info === null ? null : { status: projectText, sync, busy: session.busy, onResolved, onSetUp: () => open("adopt"), onRevert: () => open("revert"), agentSetUp };

  /**
   * The backend opened the session anew after a set-up, a take-back or a lift: read again as after an agent's reopen (in the agents'
   * reload queue), then `said`.
   */
  const afterSwap = useStableCallback(async (said: string) => {
    const root = session.info?.root;
    if (root === undefined) return;
    const reloaded = reloads.current.then(() => session.reloadSession(root, resetProject)).then(bumpChanged);
    reloads.current = reloaded.catch(session.fail);
    await reloads.current;
    bumpIds();
    session.setMessage({ kind: "info", text: said });
  });
  const onAdopted = useStableCallback(async (report: AdoptReport) => {
    toast.dismissKey(HINT_TOAST);
    // The dialog's own set-up: it stays (with „Wird eingerichtet …“) until the session is read and the message said, then it closes itself.
    setAdoptRunning(true);
    try {
      await afterSwap(adoptedText(report));
    } finally {
      setAdoptRunning(false);
    }
  });
  const onReverted = useStableCallback((report: RevertReport) => afterSwap(revertedText(report)));
  const onLifted = useStableCallback(async (report: MigrateReport) => {
    const said = `${liftedText(report)}${report.snapshot === null ? "" : ` Sicherung: ${report.snapshot}`}`;
    const root = session.info?.root;
    // The files were lifted already (another Gen4 Studio did it): nothing changed and the backend kept the session it opened as older,
    // which still refuses every edit. Opening the folder anew reads the lifted files.
    if (report.changed.length === 0 && root !== undefined) {
      if (await session.openProjectAt(root, resetProject)) session.setMessage({ kind: "info", text: said });
      return;
    }
    await afterSwap(said);
  });
  // „Abbrechen“ of the lift: the project closes, in the backend too (unlifted it could only be read, and no agent may act on it unseen).
  const onCancelLift = useStableCallback(async () => {
    if (await session.closeProject(resetProject)) onClosed();
  });

  // A rename went in (one undo step that wrote files): the history and the ids are read again, Änderungen compares again.
  const onRenamed = useStableCallback((result: RenameResult) => {
    session.setMessage({ kind: "info", text: renamedText(result.plan) });
    for (const warning of result.warnings) toast.warning(warning);
    bumpIds();
    bumpChanged();
    void session.refreshDirty();
  });

  // The ids the heads show: a new cache (read anew per kind) for every session and every `idsRevision`; neither is read inside, they are
  // the triggers of a new cache.
  const info = session.info;
  const idCache = useMemo(() => new EntityIdCache(), [info, idsRevision]);

  // Another project (or none): the hint of the one before goes.
  const root = info?.root ?? null;
  useEffect(() => toast.dismissKey(HINT_TOAST), [root]);
  // A DSPRE folder that opened (not one an agent or a set-up opened anew) hints once per opening that it can be set up, unless the user
  // said not to ask again; without its base ROM the hint says it needs one and leads to where it is assigned. It stays until answered,
  // and follows the status while it is open: it goes once the folder is no DSPRE folder (set up, by an agent too), and offers
  // „Einrichten…“ once the base ROM is found (assigned meanwhile).
  const hinted = useRef<ProjectInfo | null>(null);
  useEffect(() => {
    if (info === null || projectText === null) return;
    if (projectText.state !== "dspre") {
      toast.dismissKey(HINT_TOAST);
      hinted.current = info;
      return;
    }
    if (hinted.current === info) {
      if (toast.shown(HINT_TOAST)) showHint(info.root, projectText);
      return;
    }
    hinted.current = info;
    if (session.isReopened(info) || !shouldHint(info.root, projectText)) return;
    showHint(info.root, projectText);
  }, [info, projectText]);
  /** Shows the hint of the DSPRE folder at `forget` (replacing the one open). */
  function showHint(forget: string, status: TextProjectStatus) {
    const { text, setUp } = hintOf(status);
    const first = setUp
      ? { label: "Einrichten…", onClick: () => open("adopt") }
      : {
          label: "Basis-ROM zuordnen…",
          onClick: () => {
            if (canOpen()) showPublish();
          },
        };
    toast.lastingInfo(text, { key: HINT_TOAST, actions: [first, { label: "Nicht mehr fragen", onClick: () => dismissHint(forget) }] });
  }

  return { textProject, dialog, closeDialog: () => setDialog(null), onAdopted, onReverted, onLifted, onCancelLift, onRenamed, idCache };
}
