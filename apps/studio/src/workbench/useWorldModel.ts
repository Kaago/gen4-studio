import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorText, getMatrix, worldMetaGet, worldMetaSet } from "../api";
import { useStableCallback } from "../app/useStable";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, WarpLink, WorldMeta } from "../api";
import { t } from "../i18n";
import { buildWorld, worldNames } from "./worldModel";
import type { World, WorldNames } from "./worldModel";

const WORLD_FILE = "g4-world.json";

/** The project file as it reads when there is none (or it is broken). */
export const EMPTY_WORLD_META: WorldMeta = { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} };

/** The kernel's refusal to write a broken world file, said before asking it. */
export const brokenWorldFile = (why: string) => t("workbench:useWorldModel.broken", { file: WORLD_FILE, why });

export interface WorldModel {
  /** Null until the cells and the project file of the open project are read. */
  world: World | null;
  /** What the world calls each header (`worldNames`); null with the world. */
  names: WorldNames | null;
  meta: WorldMeta;
  /** Why the project file could not be read (it is then left alone); null when it could. */
  broken: string | null;
  /** Writes the project file and builds the world with it; a broken file or a refused write is reported, nothing changes. */
  setMeta: (next: WorldMeta) => Promise<void>;
  /** Reads the cells again (matrix data moved); nothing renders unless they changed. Keeps its identity. */
  reread: () => void;
}

const sameList = (a: readonly (number | null)[] | null | undefined, b: readonly (number | null)[] | null | undefined): boolean =>
  a === b || (!!a && !!b && a.length === b.length && a.every((v, i) => v === b[i]));

/** Whether two reads of the cells found the same: then the world stays the same object. */
function sameCells(a: ReadonlyMap<number, MatrixDto>, b: ReadonlyMap<number, MatrixDto>): boolean {
  if (a.size !== b.size) return false;
  for (const [index, dto] of b) {
    const before = a.get(index);
    if (!before || before.width !== dto.width || before.height !== dto.height || !sameList(before.maps, dto.maps) || !sameList(before.headers, dto.headers)) return false;
  }
  return true;
}

/**
 * The world model of the open project (`buildWorld`): it reads the cells of every matrix with a header section once the
 * matrix listing is there (again for every `revision`, and for `reread`) and the project file `g4-world.json`
 * when the project opens. Answers for another project are dropped; a matrix that cannot be read is reported and left out.
 */
export function useWorldModel(args: {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  links: WarpLink[];
  loaded: boolean;
  revision: number;
  onError: (e: unknown) => void;
}): WorldModel {
  const { info, headers, matrices, links, loaded, revision } = args;
  // Reads run per project and revision, not per new error callback.
  const onError = useStableCallback(args.onError);
  const root = info?.root ?? null;
  const [cells, setCells] = useState<{ root: string; map: ReadonlyMap<number, MatrixDto> } | null>(null);
  const [file, setFile] = useState<{ root: string; meta: WorldMeta; broken: string | null } | null>(null);

  useEffect(() => {
    if (root === null) return;
    let current = true;
    worldMetaGet().then(
      ({ meta, broken }) => current && setFile({ root, meta, broken }),
      (e: unknown) => {
        if (!current) return;
        // Unread is treated as broken: nothing is written over a file that could not be read.
        setFile({ root, meta: EMPTY_WORLD_META, broken: errorText(e) });
        onError(e);
      },
    );
    return () => {
      current = false;
    };
  }, [root, onError]);

  // The matrices to read, as text: a new listing with the same matrices reads nothing.
  const sections = useMemo(
    () =>
      matrices
        .filter((m) => m.has_headers)
        .map((m) => m.index)
        .join(","),
    [matrices],
  );
  // Counts the reads: only the answer of the latest one counts, and a cleanup drops the one in flight.
  const reads = useRef(0);
  // The cells as last read, to compare a new read with before setting anything (the same cells set nothing, so nothing renders).
  const shownCells = useRef(cells);
  shownCells.current = cells;
  const reread = useStableCallback(() => {
    if (root === null || !loaded) return;
    const read = ++reads.current;
    const indices = sections === "" ? [] : sections.split(",").map(Number);
    void Promise.allSettled(indices.map((index) => getMatrix(index))).then((results) => {
      if (read !== reads.current) return;
      const map = new Map<number, MatrixDto>();
      let failure: unknown = undefined;
      for (const result of results) {
        if (result.status === "fulfilled") map.set(result.value.index, result.value);
        else failure ??= result.reason;
      }
      const prev = shownCells.current;
      if (prev === null || prev.root !== root || !sameCells(prev.map, map)) {
        shownCells.current = { root, map };
        setCells(shownCells.current);
      }
      if (failure !== undefined) onError(failure);
    });
  });
  useEffect(() => {
    reread();
    return () => {
      reads.current += 1;
    };
  }, [root, loaded, sections, revision, reread]);

  const ownFile = file !== null && file.root === root ? file : null;
  const ownCells = cells !== null && cells.root === root && loaded ? cells : null;
  const meta = ownFile?.meta ?? EMPTY_WORLD_META;
  const broken = ownFile?.broken ?? null;
  const family = info?.family ?? "";
  const world = useMemo(
    () => (ownFile === null || ownCells === null ? null : buildWorld({ family, headers, matrices, links, cells: ownCells.map, meta: ownFile.meta })),
    [family, headers, matrices, links, ownCells, ownFile],
  );

  const setMeta = useCallback(
    async (next: WorldMeta) => {
      if (root === null || ownFile === null) return;
      if (ownFile.broken !== null) {
        onError(new Error(brokenWorldFile(ownFile.broken)));
        return;
      }
      try {
        await worldMetaSet(next);
      } catch (e) {
        onError(e);
        return;
      }
      setFile((prev) => (prev !== null && prev.root === root ? { ...prev, meta: next } : prev));
    },
    [root, ownFile, onError],
  );

  const names = useMemo(() => (world === null ? null : worldNames(world)), [world]);
  return useMemo(() => ({ world, names, meta, broken, setMeta, reread }), [world, names, meta, broken, setMeta, reread]);
}
