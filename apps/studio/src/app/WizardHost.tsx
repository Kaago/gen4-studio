import { useEffect, useMemo, useState } from "react";
import type { World } from "../workbench/worldModel";
import type { AdoptProgressDto, AdoptReportDto, AreaReport, AreaSpec, AssetId, DraftDto, HeaderSummary, MatrixSummary, MemberCounts, ProjectInfo, RegionReport, RoomReport, RoomSpec, ScriptsStatusDto, WarpLink } from "../api";
import { errorText, releases } from "../api";
import type { ReleasesDto } from "../api";
import { PublishWizard } from "../publish/PublishWizard";
import type { PublishWizardProps } from "../publish/PublishWizard";
import { ClearWorldWizard, clearedAnnouncement } from "../project/ClearWorldWizard";
import { useShell } from "../ui/frame/ShellContext";
import { buildPlaces } from "../workbench/places";
import { recipeRequest } from "../workbench/recipes";
import { NewAreaWizard } from "../workbench/wizards/NewAreaWizard";
import { NewRoomWizard } from "../workbench/wizards/NewRoomWizard";
import { ConnectDoorsWizard } from "../workbench/wizards/ConnectDoorsWizard";
import { NewFigureWizard } from "../workbench/wizards/NewFigureWizard";
import { NewTrainerHereWizard } from "../workbench/wizards/NewTrainerHereWizard";
import { dialogTitle } from "../workbench/dialog/dialogModel";
import type { DialogTarget } from "../workbench/dialog/dialogModel";
import type { ScriptFiles } from "../data/useDataArea";
import type { StoryRecipeRequest } from "../stories/recipes/storyRecipeModel";
import { headerLabel } from "../workbench/headers";
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
  /** „Veröffentlichen“: what the wizard needs of the app; the host reads the releases list itself when the wizard opens. */
  publish: PublishHostProps;
  /** The open wizard's draft (null: none, or the wizard closed or created): the app hands it to the scene and the world map. */
  onDraft: (draft: DraftDto | null) => void;
  /** The open project (null: none): the recipes of Erzählen take the family's default looks from it, the flag field its names. */
  project?: ProjectInfo | null;
  /** A recipe of Erzählen went in (one undo step): the app reads the assets it changed again, as after ⌘Z. */
  onRecipeApplied?: (changed: AssetId[]) => Promise<void>;
  /** „Dialog bearbeiten“ after „Neue Figur“: the app opens the dialog editor of the new figure. */
  onOpenDialog?: (target: DialogTarget) => void;
  /** The script panel's documents: „Neue Figur“ appends to the place's script file, so typing there is saved first (as before a split). */
  scriptFiles?: ScriptFiles;
  /** „Zu einer Geschichte“ after „Neue Figur“: the app opens „Gespräch schaltet weiter“ with the new figure. */
  onOpenStoryRecipe?: (request: StoryRecipeRequest) => void;
  /** „Gelände anpassen“ on a result that made a map in a cell (`Created.cell`): the app selects it and starts painting. */
  startPainting?: (cell: { matrix: number; x: number; y: number }) => void;
}

type PublishHostProps = Omit<PublishWizardProps, "defaults" | "defaultsError">;

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
    // „Gelände anpassen“ is the app's step, no wizard: show what was made, then paint its cell.
    const cell = made.cell;
    if (id === "adjust_terrain" && cell && props.startPainting) {
      const paint = props.startPainting;
      void created(made).then(() => paint({ matrix: made.matrix, x: cell.x, y: cell.y }));
      return;
    }
    const next = recipeRequest(id, made.header);
    if (next) void created(made).then(() => open(next));
  };
  const areaMade = (report: AreaReport, spec: AreaSpec): Created => ({ matrix: report.matrix, header: report.header, announcement: areaResult(report, spec, depth).lines[0], name: spec.name });
  const roomMade = (report: RoomReport, spec: RoomSpec): Created => ({ matrix: report.matrix, header: spec.header.id, announcement: roomResult(report, spec, depth).lines[0], name: spec.name });
  // The town is a header of matrix 0 (the recipe looks for it there).
  const regionMade = (report: RegionReport): Created => ({ matrix: 0, header: report.plan.start_header, announcement: clearedAnnouncement(report) });
  // The recipes of Erzählen start in the request's place, else the selected one, on the request's tile, else the tile last hovered
  // when the scene shows that place's matrix.
  const headerOf = (id: number) => headers.find((h) => h.id === id);
  const sceneTile = (header: number) => (headerOf(header)?.matrix_id === shownMatrix ? (lastTile() ?? undefined) : undefined);
  const startAt = (prefill: { header?: number; tile?: { x: number; z: number } } | undefined) => {
    const header = prefill?.header ?? selected?.header;
    if (header === undefined) return undefined;
    const tile = prefill?.tile ?? sceneTile(header);
    return tile ? { header, tile } : { header };
  };
  const placeName = (id: number) => {
    const h = headerOf(id);
    return h ? `„${headerLabel(h)}“` : "dem Ort";
  };
  // What a recipe made, shown as „Zeigen“ shows it: the place it went into (the one of door A for two doors).
  const recipeMade = (header: number, announcement: string): Created => ({ matrix: headerOf(header)?.matrix_id ?? shownMatrix ?? 0, header, announcement });
  const applied = async (changed: AssetId[]) => {
    await props.onRecipeApplied?.(changed);
  };
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
          depth={depth}
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
    case "publish":
      return <PublishHost {...props.publish} />;
    case "new_figure": {
      const figureMade = (input: { header: number }) => recipeMade(input.header, `Figur in ${placeName(input.header)} aufgestellt`);
      return (
        <NewFigureWizard
          depth={depth}
          headers={headers}
          matrices={matrices}
          project={props.project ?? null}
          prefill={startAt(request.prefill)}
          onDraft={onDraft}
          onDone={(outcome) => applied(outcome.changed)}
          onShow={(_, input) => void created(figureMade(input))}
          onStep={(id, outcome, input) => {
            // „Dialog bearbeiten“: show the figure's place, then its dialog editor (an app dialog, not a wizard).
            if (id === "edit_dialog") {
              const { event_file, object } = outcome.report;
              const target: DialogTarget = { header: input.header, eventFile: event_file, kind: "object", index: object, title: dialogTitle("object", object) };
              void created(figureMade(input)).then(() => props.onOpenDialog?.(target));
            } else if (id === "story_join") {
              // „Zu einer Geschichte“: the figure's place, then „Gespräch schaltet weiter“ with the new figure (its story chosen there).
              const figure = { header: input.header, local_id: outcome.report.local_id };
              void created(figureMade(input)).then(() => props.onOpenStoryRecipe?.({ recipe: "talk_advances", prefill: { figure } }));
            } else step(id, figureMade(input));
          }}
          onCancel={close}
          onError={onError}
          beforeApply={async (input) => {
            // The recipe appends the figure's slot to the place's script file: typing there would overwrite it on the next save.
            const file = headerOf(input.header)?.script_file_id;
            if (file !== undefined) await props.scriptFiles?.saveIfDirty(file);
          }}
        />
      );
    }
    case "connect_doors": {
      const doorsMade = (input: { a: { header: number }; b: { header: number } }) => recipeMade(input.a.header, `Türen verbunden: ${placeName(input.a.header)} und ${placeName(input.b.header)}`);
      // Door A from the scene: the tile last hovered when it shows that place (a door there is offered by the wizard).
      const a = request.prefill?.a;
      const startA = a ? (a.warp !== undefined || a.tile !== undefined ? a : startAt({ header: a.header })) : startAt(undefined);
      return (
        <ConnectDoorsWizard
          depth={depth}
          headers={headers}
          matrices={matrices}
          prefill={startA ? { a: startA } : undefined}
          onDraft={onDraft}
          onDone={(outcome) => applied(outcome.changed)}
          onShow={(_, input) => void created(doorsMade(input))}
          onStep={(id, _, input) => step(id, doorsMade(input))}
          onCancel={close}
          onError={onError}
        />
      );
    }
    case "new_trainer_here": {
      const trainerMade = (input: { header: number; name: string }) => recipeMade(input.header, `Trainer „${input.name}“ in ${placeName(input.header)} aufgestellt`);
      return (
        <NewTrainerHereWizard
          depth={depth}
          headers={headers}
          matrices={matrices}
          project={props.project ?? null}
          prefill={startAt(request.prefill)}
          onDraft={onDraft}
          onDone={(outcome) => applied(outcome.changed)}
          onShow={(_, input) => void created(trainerMade(input))}
          onStep={(id, _, input) => step(id, trainerMade(input))}
          onCancel={close}
          onError={onError}
        />
      );
    }
  }
}

/** „Veröffentlichen“ with the releases list, read once when it opens (its proposal, the stem, the default place, the versions taken). */
function PublishHost(props: PublishHostProps) {
  const [list, setList] = useState<{ defaults: ReleasesDto | null; error: string | null }>({ defaults: null, error: null });
  useEffect(() => {
    let live = true;
    releases().then(
      (defaults) => live && setList({ defaults, error: null }),
      (e) => live && setList({ defaults: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, []);
  return <PublishWizard {...props} defaults={list.defaults} defaultsError={list.error} />;
}
