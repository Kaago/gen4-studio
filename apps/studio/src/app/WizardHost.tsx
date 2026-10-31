import { useMemo } from "react";
import type { World } from "../workbench/worldModel";
import type { AdoptProgressDto, AdoptReportDto, AreaReport, AreaSpec, DraftDto, HeaderSummary, MatrixSummary, MemberCounts, RegionReport, RoomReport, RoomSpec, ScriptsStatusDto, WarpLink } from "../api";
import { ClearWorldWizard, clearedAnnouncement } from "../project/ClearWorldWizard";
import { useShell } from "../ui/frame/ShellContext";
import { buildPlaces } from "../workbench/places";
import { recipeRequest } from "../workbench/recipes";
import { NewAreaWizard } from "../workbench/wizards/NewAreaWizard";
import { NewRoomWizard } from "../workbench/wizards/NewRoomWizard";
import { areaResult, roomResult } from "../workbench/wizards/wizardSpecs";
import type { Created, Wizards } from "./useWizards";

interface Props {
  wizards: Pick<Wizards, "request" | "open" | "close" | "created">;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  /** The world model (null: not read yet): the area wizard in Bauen offers its world maps and areas as sources. It is passed in because the host lives beside the workbench, outside the context `useWorld` reads. */
  world: World | null;
  /** The matrix overview was read (false: `matrices` is empty because nothing arrived yet, not because there are none). */
  matricesLoaded: boolean;
  /** The latest read of the overview failed. */
  matricesFailed: boolean;
  links: WarpLink[];
  counts: MemberCounts;
  /** The selected header and its matrix: the area wizard copies from that matrix unless the request names another, and selects the header's cells in it. */
  selected: { header: number; matrix: number } | null;
  /** The matrix the scene shows (null: none): `lastTile` is a tile of it. */
  shownMatrix: number | null;
  /** The tile the pointer was last over in the shown matrix (null: none yet): the room wizard starts its outside door there. */
  lastTile: () => { x: number; z: number } | null;
  /** The Dynamic Headers patch was applied from a wizard: the header table and the header list are read again. */
  onHeaderPatch: () => Promise<void>;
  onHeaderPatchRepaired?: () => void;
  onError: (e: unknown) => void;
  /** "Hier spielen" on a wizard's result (after it showed what it made): save, build, run. */
  onPlay: () => void;
  /** The project's script status (null: not read yet); "Welt leeren" needs adopted scripts. */
  scripts: ScriptsStatusDto | null;
  /** Adopts the project's scripts (the wizard "Welt leeren" offers it). */
  onAdoptScripts: (onProgress: (progress: AdoptProgressDto) => void) => Promise<AdoptReportDto>;
  /** Unsaved edits of the store and the open scripts; "Welt leeren" needs none. */
  unsaved: number;
  /** Saves everything ("Welt leeren" offers it; a wizard locks ⌘S). */
  onSave: () => Promise<void>;
  /** "Welt leeren" went in: the world is read again as after ⌘Z. */
  onRegionApplied: (report: RegionReport) => Promise<void>;
  /** The open wizard's draft (null: none, or the wizard closed or created): the app hands it to the scene and the world map. */
  onDraft: (draft: DraftDto | null) => void;
}

/** The source matrix: the request's, else the selected header's (and then that header's cells are selected). */
function areaPrefill(requested: { matrix?: number } | undefined, selected: Props["selected"]): { matrix?: number; header?: number } {
  if (requested?.matrix !== undefined) return { matrix: requested.matrix };
  return selected ? { matrix: selected.matrix, header: selected.header } : {};
}

/**
 * The open wizard. Its frame puts the step bar into the world's hinge and the step into the world's hand (portals into the deck's
 * slots); the host lives beside the workbench, not in it, so opening, typing and closing a wizard re-renders none of the workbench's panels.
 */
export function WizardHost(props: Props) {
  const { wizards, world, headers, matrices, matricesLoaded, matricesFailed, links, counts, selected, shownMatrix, lastTile, onHeaderPatch, onHeaderPatchRepaired, onError, onPlay, onDraft } = props;
  const { request, open, close, created } = wizards;
  // Bauen gives a new area or room a place and a look; Tüfteln lets the user choose its header.
  const depth = useShell().viewDepth;
  // Only the room wizard reads the places, so no other request (nor a closed host) groups the headers.
  const wantsPlaces = request?.kind === "room";
  const places = useMemo(() => (wantsPlaces ? buildPlaces(headers, matrices) : []), [wantsPlaces, headers, matrices]);
  if (!request) return null;
  // A next step of a result: show what the wizard made (as „Zeigen“ does), then open the step's wizard; „Raum anlegen“ puts its door
  // outside into the header just made. A step without a wizard does nothing (its button is locked anyway).
  const step = (id: string, made: Created) => {
    const next = recipeRequest(id, made.header);
    if (next) void created(made).then(() => open(next));
  };
  const areaMade = (report: AreaReport, spec: AreaSpec): Created => ({ matrix: report.matrix, header: report.header, announcement: areaResult(report, spec, depth).lines[0], name: spec.name });
  const roomMade = (report: RoomReport, spec: RoomSpec): Created => ({ matrix: report.matrix, header: spec.header.id, announcement: roomResult(report, spec, depth).lines[0], name: spec.name });
  // The town is a header of matrix 0 (the recipe looks for it there).
  const regionMade = (report: RegionReport): Created => ({ matrix: 0, header: report.plan.start_header, announcement: clearedAnnouncement(report) });
  switch (request.kind) {
    case "area":
      return (
        <NewAreaWizard
          depth={depth}
          world={world}
          headers={headers}
          matrices={matrices}
          matricesLoaded={matricesLoaded}
          matricesFailed={matricesFailed}
          links={links}
          counts={counts}
          prefill={areaPrefill(request.prefill, selected)}
          onClose={close}
          onCreated={(report, spec) => created(areaMade(report, spec))}
          onStep={(id, report, spec) => step(id, areaMade(report, spec))}
          onHeaderPatch={onHeaderPatch}
          onHeaderPatchRepaired={onHeaderPatchRepaired}
          onError={onError}
          onPlay={onPlay}
          onDraft={onDraft}
        />
      );
    case "room":
      return (
        <NewRoomWizard
          depth={depth}
          headers={headers}
          links={links}
          counts={counts}
          matrices={matrices}
          matricesLoaded={matricesLoaded}
          matricesFailed={matricesFailed}
          shownMatrix={shownMatrix}
          places={places}
          prefill={request.prefill}
          lastTile={lastTile}
          onClose={close}
          onCreated={(report, spec) => created(roomMade(report, spec))}
          onStep={(id, report, spec) => step(id, roomMade(report, spec))}
          onHeaderPatch={onHeaderPatch}
          onHeaderPatchRepaired={onHeaderPatchRepaired}
          onError={onError}
          onPlay={onPlay}
          onDraft={onDraft}
        />
      );
    case "clearWorld":
      return (
        <ClearWorldWizard
          scripts={props.scripts}
          onAdopt={props.onAdoptScripts}
          unsaved={props.unsaved}
          onSave={props.onSave}
          onApplied={props.onRegionApplied}
          onClose={close}
          onShow={(report) => created(regionMade(report))}
          onStep={(id, report) => step(id, regionMade(report))}
          onPlay={onPlay}
          onDraft={onDraft}
        />
      );
  }
}
