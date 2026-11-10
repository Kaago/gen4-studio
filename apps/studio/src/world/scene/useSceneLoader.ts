import { useEffect } from "react";
import { errorText, getMap, mapModelTextureStatus } from "../../api";
import { t } from "../../i18n";
import type { MatrixDto, TextureIssue, TileBehavior } from "../../api";
import type { Message } from "../../app/message";
import type { CellRect } from "../../workbench/sceneScope";
import { fetchCurrent } from "../../reload";
import { MODEL_CONCURRENCY, limitConcurrency, textureNote } from "../load";
import { disposeTree } from "../sceneUtil";
import { reportBuildings } from "./buildingReport";
import { buildingModelJob } from "./buildingModels";
import { showCellFrame } from "./cellFrame";
import { frameCells } from "./camera";
import { createGround } from "../ground";
import { addModel, buildContent, removeMapModel, removeModels, reportTerrainPatterns, setMarkers } from "./content";
import { mapModel } from "./worldMesh";
import { fetchEvents } from "./eventFetch";
import { setHeightsShown } from "./heightLayer";
import type { SceneHost } from "./SceneHost";
import { loadNames } from "./tileNames";
import type { Loaded, ReportEvents, SceneRefs } from "./types";

/**
 * Fetches the maps (and the event file, and the tile names) of `matrix` and puts the overlay,
 * buildings and markers into the scene once all are there; the camera then frames the cells
 * `focus`. Resolves true when the scene was built, false when this load was superseded (`stale`)
 * meanwhile.
 */
async function showMaps(
  h: SceneHost,
  matrix: MatrixDto,
  focus: CellRect,
  refs: SceneRefs,
  history: { readonly current: number },
  stale: () => boolean,
  reportEvents: ReportEvents,
  onSceneBuilt: () => void,
): Promise<boolean> {
  // Without names the overlay is uncoloured by class and the hover shows raw values only.
  const problems: SceneHost["loadProblems"] = { names: null, maps: [] };
  const namesPending = loadNames().catch((e) => {
    problems.names = t("world:scene.tileNamesNotLoaded", { error: errorText(e) });
    return new Map<number, TileBehavior>();
  });
  const ids = [...new Set(matrix.maps.filter((m): m is number => m !== null))];
  // Maps and events are fetched together; an undo or redo that lands meanwhile makes them
  // be fetched again. A failure of the events only costs the markers, never the scene.
  // The names are awaited inside, so no undo can land between the history check and the build.
  const { loaded, fetchedEvents, mapProblems, names } = await fetchCurrent(
    async () => {
      const loaded = new Map<number, Loaded>();
      const mapProblems: string[] = [];
      const [fetchedEvents, , names] = await Promise.all([
        fetchEvents(refs.eventFile.current),
        Promise.all(
          ids.map(async (id) => {
            try {
              loaded.set(id, { map: await getMap(id) });
            } catch (e) {
              mapProblems.push(t("world:scene.mapProblem", { id, error: errorText(e) }));
            }
          }),
        ),
        namesPending,
      ]);
      return { loaded, fetchedEvents, mapProblems, names };
    },
    () => history.current,
  );
  problems.maps = mapProblems;
  if (stale()) return false;
  h.ground = createGround(matrix, loaded);
  const { content, overlay, tiles, bases, buildings } = buildContent(matrix, loaded, names, h.ground);
  h.scene.remove(h.content);
  h.buildings?.dispose();
  disposeTree(h.content);
  h.content = content;
  h.buildings = buildings;
  buildings.select(h.selection);
  h.models = [];
  h.loadProblems = problems;
  h.overlay = overlay;
  h.matrix = matrix;
  h.loaded = loaded;
  h.tiles = tiles;
  h.bases = bases;
  h.names = names;
  overlay.visible = refs.showPermissions.current;
  reportTerrainPatterns(h);
  // The old layer went with the old content; a new one is built only while „Höhen“ is on (else on its first switch-on).
  h.heights = null;
  setHeightsShown(h, refs.showHeights.current);
  h.scene.add(content);
  // The selected cell is framed at its height in this matrix.
  showCellFrame(h);
  setMarkers(h, fetchedEvents.events, refs.showEvents.current);
  reportEvents(fetchedEvents);

  frameCells(h, focus, matrix);
  h.refreshHover?.();
  reportBuildings(h, refs);
  onSceneBuilt();
  return true;
}

/**
 * Loads the terrain models of the loaded maps, then the building models the layer lacks, a few at
 * a time, each hung into the scene as it arrives; reports progress and, at the end, what failed.
 * A task that has not started when the load is superseded never starts; a started one finishes,
 * but its scene is disposed instead of added. So is a map's model when a one-map reload of that
 * map (`useMapModelRefresh`) was asked for after the task started; a reload still in flight when
 * this load starts is dropped in turn.
 */
export async function loadModels(h: SceneHost, matrix: MatrixDto, area: number, stale: () => boolean, report: (message: Message | null) => void) {
  const modelIds = [...h.loaded.keys()];
  // Each map task reads the generation when it starts, after this bump: only the reloads lose.
  for (const id of modelIds) h.mapModelGen.set(id, (h.mapModelGen.get(id) ?? 0) + 1);
  const textureIssues: TextureIssue[] = [];
  const mapTasks = modelIds.map((id) => async () => {
    if (stale()) return;
    const gen = h.mapModelGen.get(id) ?? 0;
    const model = await mapModel(h, id, area);
    // The backend falls back to an untextured model when the tileset is unusable; ask why.
    // A failing status call only costs the note.
    await mapModelTextureStatus(id, area)
      .then((issue) => issue && textureIssues.push(issue))
      .catch(() => undefined);
    if (stale() || (h.mapModelGen.get(id) ?? 0) !== gen) {
      disposeTree(model);
      return;
    }
    // A one-map reload may have shown this map already: its instances give way, one set stays.
    removeMapModel(h, id);
    addModel(h, matrix, id, model);
    h.queueRender();
  });
  // Building models come after the terrain, through the same bounded loader.
  const layer = h.buildings;
  const buildingIds = layer ? layer.missingModelIds() : [];
  const buildingTasks = layer ? buildingIds.map((id) => buildingModelJob(h, layer, id, area, stale)) : [];
  const labels = [...modelIds.map((id) => t("world:scene.mapModel", { id })), ...buildingIds.map((id) => t("world:scene.buildingModel", { id }))];
  const tasks = [...mapTasks, ...buildingTasks];
  let lastReport = 0;
  const progress = (done: number, total: number) => {
    // The status bar re-renders the whole workbench: at most ten updates per second.
    const now = performance.now();
    if (done < total && now - lastReport < 100) return;
    lastReport = now;
    report({ kind: "info", text: t("world:scene.modelsProgress", { done, total }) });
  };
  progress(0, tasks.length);
  const results = await limitConcurrency(tasks, MODEL_CONCURRENCY, progress);
  const modelErrors = results.flatMap((r, i) => (r.status === "rejected" ? [`${labels[i]}: ${errorText(r.reason)}`] : []));
  const { names: namesError, maps: mapErrors } = h.loadProblems;
  const problems = [
    namesError,
    mapErrors.length ? t("world:scene.mapsNotLoaded", { count: mapErrors.length, first: mapErrors[0] }) : null,
    modelErrors.length ? t("world:scene.modelsNotLoaded", { count: modelErrors.length, first: modelErrors[0] }) : null,
  ].filter((p): p is string => p !== null);
  const note = textureNote(textureIssues);
  if (problems.length) report({ kind: "error", text: [...problems, note].filter((p) => p !== null).join(" · ") });
  else report(note ? { kind: "info", text: note } : null);
}

/** What the loader needs besides the refs; `onSceneBuilt` and `clearEventStatus` are called while a full load runs. */
export interface SceneLoaderParams {
  matrix: MatrixDto;
  /** The cells the camera frames after a full load (it belongs to `matrix`); a reload of the models alone leaves the camera. */
  focus: CellRect;
  areaDataId: number;
  modelsToken: number;
  /** Bumped by every undo and redo (see `MatrixScene`'s `history` prop). */
  history: { readonly current: number };
  refs: SceneRefs;
  reportEvents: ReportEvents;
  /** A full load starts: the note about the previous matrix's events goes. */
  clearEventStatus: () => void;
  /** A full load put its content into the scene. */
  onSceneBuilt: () => void;
}

/**
 * (Re)loads the scene. A new matrix loads the maps first (overlay and markers appear as soon as
 * they are there), then the models, a few at a time, each hung into the scene as it arrives.
 * The same matrix with another area data (or `modelsToken`) reloads only the models. Every run
 * supersedes the one before it: that one's pending work is dropped when it comes back.
 */
export function useSceneLoader({ matrix, focus, areaDataId, modelsToken, history, refs, reportEvents, clearEventStatus, onSceneBuilt }: SceneLoaderParams) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    let cancelled = false;
    const area = areaDataId;
    // The scene shows this matrix's maps already when only the models are to be replaced.
    const modelsOnly = h.matrix === matrix;
    /** Status updates of a superseded load are dropped. */
    const report = (message: Message | null) => {
      if (!cancelled) refs.callbacks.current.onStatus(message);
    };
    if (!modelsOnly) {
      report({ kind: "info", text: t("world:scene.loadingMaps") });
      clearEventStatus();
    }
    const stale = () => cancelled || refs.handle.current !== h;

    (async () => {
      if (modelsOnly) {
        removeModels(h);
        // The building models are textured by the area data too: they show boxes until reloaded.
        h.buildings?.resetModels();
        h.render();
      } else if (!(await showMaps(h, matrix, focus, refs, history, stale, reportEvents, onSceneBuilt))) {
        return;
      }
      await loadModels(h, matrix, area, stale, report);
    })().catch((e) => report({ kind: "error", text: errorText(e) }));

    return () => {
      cancelled = true;
    };
  }, [matrix, areaDataId, modelsToken]);
}
