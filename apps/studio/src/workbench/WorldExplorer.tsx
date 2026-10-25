import { memo, useCallback, useState } from "react";
import type { HeaderSummary, ProjectInfo } from "../api";
import { gameLabel } from "../app/gameLabel";
import type { MarkSeverity } from "../checks/FindingMark";
import { ListPane } from "../ui/frame/ListPane";
import type { Depth, WorldSort } from "../ui/frame/shellState";
import { Icon, Menu, Segmented } from "../ui/primitives";
import type { MenuEntry } from "../ui/primitives";
import { HeaderList } from "./HeaderList";
import type { RenameTarget } from "./RoomActions";
import type { WizardRequest } from "./WorkbenchContext";
import { readRecent, touchRecent } from "./worldSort";
import type { ListSort, TreeSort } from "./worldSort";
import type { World } from "./worldModel";
import { WorldTree } from "./WorldTree";

export interface WorldExplorerProps {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  /** The world model (`useWorldModel`); null while it is read. */
  world: World | null;
  /** False until the overview's first matrix listing arrived. */
  matricesLoaded: boolean;
  /** The first matrix listing failed (and nothing is loaded): the header line says the overview is not available instead of loading. */
  matricesFailed: boolean;
  selected: number | null;
  /** The matrix the scene shows: headers using it are marked. */
  shownMatrix: number | null;
  /** Bumped by every link to a header: the tree scrolls to the selection again, also when it was selected already. */
  revealToken: number;
  /** Tüfteln writes the identifiers (id, code) into every row; Bauen speaks of the game only. */
  depth: Depth;
  onSelect: (h: HeaderSummary) => void;
  onOpenWizard: (request: WizardRequest) => void;
  /** Whether rotom has taken the scripts over; without it the script entries only show a hint. */
  scriptsAdopted: boolean;
  /** Opens a script file in the script panel (at a 1-based line). Must keep its identity (the tree is memoised). */
  onOpenScript: (file: number, line?: number) => void;
  /** Whether the list column shows or is folded to its grip (the shell remembers it per area). */
  listOpen: boolean;
  onToggleList: () => void;
  /** The worst error or warning the checks found per header (`headerMarks`): a mark at the end of its row. None: no marks. */
  marks?: ReadonlyMap<number, MarkSeverity>;
  /** How the tree and the list are sorted (the shell remembers it). */
  sort: WorldSort;
  onSort: (sort: Partial<WorldSort>) => void;
  /** Renames a room or a world map; gives a room to an area (null: its entrance decides again). Both write `g4-world.json`. */
  onRename: (target: RenameTarget, name: string) => void;
  onOwner: (room: number, areaKey: string | null) => void;
}

type Mode = "tree" | "list";
const MODE_KEY = "g4.explorer.mode";

function readMode(): Mode {
  try {
    return localStorage.getItem(MODE_KEY) === "list" ? "list" : "tree";
  } catch {
    return "tree";
  }
}

function writeMode(mode: Mode): void {
  try {
    localStorage.setItem(MODE_KEY, mode);
  } catch {
    /* the mode is simply not remembered */
  }
}

const TREE_SORTS: readonly { value: TreeSort; label: string }[] = [
  { value: "game", label: "Spielreihenfolge" },
  { value: "name", label: "Name A–Z" },
  { value: "recent", label: "Zuletzt gewählt" },
];
const LIST_SORTS: readonly { value: ListSort; label: string }[] = [
  { value: "id", label: "Nummer" },
  { value: "name", label: "Name" },
  { value: "code", label: "Kürzel" },
];
const NO_RECENT: readonly string[] = [];

/** The area keys chosen last in the open project (`g4.world.recent.<path>`), read again for another project; and the touch that puts one first. */
function useRecent(project: string | null): [readonly string[], (areaKey: string) => void] {
  const read = (p: string | null) => ({ project: p, list: p === null ? NO_RECENT : readRecent(p) });
  const [recent, setRecent] = useState<{ project: string | null; list: readonly string[] }>(() => read(project));
  if (recent.project !== project) setRecent(read(project));
  const touch = useCallback(
    (areaKey: string) => {
      if (project !== null) setRecent({ project, list: touchRecent(project, areaKey) });
    },
    [project],
  );
  return [recent.project === project ? recent.list : NO_RECENT, touch];
}

/**
 * The list pane of the world: the world tree or the flat header list, with the menus for the sort and the wizards in its
 * head. Memoised like the list: it renders when the headers, the world, the selection, the matrix shown, the reveal token,
 * the sort or a callback change, not when the scene is hovered or a message arrives. The callbacks must keep their identity for that.
 */
export const WorldExplorer = memo(function WorldExplorer({ onOpenWizard, listOpen, onToggleList, sort, onSort, ...tree }: WorldExplorerProps) {
  const [mode, setMode] = useState<Mode>(readMode);
  const changeMode = (next: Mode) => {
    setMode(next);
    writeMode(next);
  };
  const [recent, touch] = useRecent(tree.info?.root ?? null);
  const sortItems: MenuEntry[] =
    mode === "tree"
      ? TREE_SORTS.map(({ value, label }) => ({ label, checked: sort.tree === value, onSelect: () => onSort({ tree: value }) }))
      : LIST_SORTS.map(({ value, label }) => ({ label, checked: sort.list === value, onSelect: () => onSort({ list: value }) }));
  const actions = (
    <>
      <Segmented<Mode>
        size="s"
        iconOnly
        ariaLabel="Ansicht"
        value={mode}
        onChange={changeMode}
        options={[
          { value: "tree", label: "Baum", icon: "tree" },
          { value: "list", label: "Liste", icon: "list" },
        ]}
      />
      <Menu label="Sortierung" trigger={<Icon name="sort" />} items={sortItems} />
      <Menu
        label="Neu anlegen"
        trigger={<Icon name="plus" />}
        items={[
          { label: "Gebiet anlegen…", onSelect: () => onOpenWizard({ kind: "area" }) },
          { label: "Raum anlegen…", onSelect: () => onOpenWizard({ kind: "room" }) },
        ]}
      />
    </>
  );
  return (
    <ListPane title="Orte" subtitle={tree.info ? gameLabel(tree.info) : undefined} actions={actions} open={listOpen} onToggle={onToggleList}>
      {mode === "list" ? (
        <HeaderList
          info={tree.info}
          headers={tree.headers}
          selected={tree.selected}
          shownMatrix={tree.shownMatrix}
          revealToken={tree.revealToken}
          depth={tree.depth}
          sort={sort.list}
          onSelect={tree.onSelect}
          marks={tree.marks}
        />
      ) : (
        // The recent choice reorders only the sort that asks for it: another sort keeps its rows when an area is touched.
        <WorldTree {...tree} sort={sort.tree} recent={sort.tree === "recent" ? recent : NO_RECENT} onTouch={touch} />
      )}
    </ListPane>
  );
});
