import { useMemo } from "react";
import type { HeaderSummary, MatrixSummary, MemberCounts, WarpLink } from "../api";
import { buildPlaces } from "../workbench/places";
import { NewAreaWizard } from "../workbench/wizards/NewAreaWizard";
import { NewRoomWizard } from "../workbench/wizards/NewRoomWizard";
import { areaResult, roomResult } from "../workbench/wizards/wizardSpecs";
import type { Wizards } from "./useWizards";

interface Props {
  wizards: Pick<Wizards, "request" | "close" | "created">;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
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
export function WizardHost({ wizards, headers, matrices, matricesLoaded, matricesFailed, links, counts, selected, shownMatrix, lastTile, onHeaderPatch, onHeaderPatchRepaired, onError, onPlay }: Props) {
  const { request, close, created } = wizards;
  // Only the room wizard reads the places, so no other request (nor a closed host) groups the headers.
  const wantsPlaces = request?.kind === "room";
  const places = useMemo(() => (wantsPlaces ? buildPlaces(headers, matrices) : []), [wantsPlaces, headers, matrices]);
  if (!request) return null;
  switch (request.kind) {
    case "area":
      return (
        <NewAreaWizard
          headers={headers}
          matrices={matrices}
          matricesLoaded={matricesLoaded}
          matricesFailed={matricesFailed}
          links={links}
          counts={counts}
          prefill={areaPrefill(request.prefill, selected)}
          onClose={close}
          onCreated={(report, spec) => created({ matrix: report.matrix, header: report.header, announcement: areaResult(report, spec).lines[0] })}
          onHeaderPatch={onHeaderPatch}
          onHeaderPatchRepaired={onHeaderPatchRepaired}
          onError={onError}
          onPlay={onPlay}
        />
      );
    case "room":
      return (
        <NewRoomWizard
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
          onCreated={(report, spec) => created({ matrix: report.matrix, header: spec.header.id, announcement: roomResult(report, spec).lines[0] })}
          onHeaderPatch={onHeaderPatch}
          onHeaderPatchRepaired={onHeaderPatchRepaired}
          onError={onError}
          onPlay={onPlay}
        />
      );
  }
}
