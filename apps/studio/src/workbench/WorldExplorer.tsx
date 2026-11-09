import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HeaderSummary, ProjectInfo } from "../api";
import type { MarkSeverity } from "../checks/FindingMark";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth, WorldSort } from "../ui/frame/shellState";
import { Icon, IconButton, Menu, PanelHead, Picker, Segmented } from "../ui/primitives";
import type { IconName, MenuEntry, PickerItem } from "../ui/primitives";
import { t } from "../i18n";
import { HeaderList } from "./HeaderList";
import type { HeaderIdentity } from "./headerIdentity";
import type { RenameTarget } from "./RoomActions";
import { id4, termPlural } from "./terms";
import type { WizardRequest } from "./WorkbenchContext";
import { readRecent, touchRecent } from "./worldSort";
import type { ListSort, TreeSort } from "./worldSort";
import type { Area, Room, World } from "./worldModel";
import { WorldTree } from "./WorldTree";
import styles from "./WorldExplorer.module.css";

export interface WorldExplorerProps {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  /** The world model (`useWorldModel`); null while it is read. */
  world: World | null;
  /** What a header is and how it is called, as the world tree says (`WorkbenchWorld.identify`): the folded row names the shown place by it. */
  identify?: (header: HeaderSummary) => HeaderIdentity;
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
  /** The places are folded to one row that names the shown place and opens a picker (the shell remembers it). */
  folded: boolean;
  onFold: (folded: boolean) => void;
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

/** The sorts of each mode; their words are `worldExplorer.sort.<mode>.<value>`. */
const TREE_SORTS: readonly TreeSort[] = ["game", "name", "recent"];
const LIST_SORTS: readonly ListSort[] = ["id", "name", "code"];
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

/** The groups of the picker of the folded row, as the tree has them. */
const more = () => t("workbench:worldTree.groups.more");
const unconnected = () => t("workbench:worldTree.groups.unconnected");
const other = () => t("workbench:worldTree.groups.other");

/**
 * What the picker „Ort oder Raum wählen“ lists: the areas and their rooms of every world map, „Weitere Gebiete“, „Nicht verbunden“ and
 * (Tüfteln) „Sonstiges“, each by the id of its header (an area by its first), so a number finds it as in the tree; a room names its place
 * in a second line. Tüfteln adds number and code. Until the world is read, the headers by id.
 */
function placeItems(world: World | null, headers: readonly HeaderSummary[], tinker: boolean): PickerItem[] {
  const tech = (h: HeaderSummary) => (tinker ? `${id4(h.id)} · ${h.internal_name}` : undefined);
  const header = (h: HeaderSummary, label: string, icon: IconName, group?: string, description?: string): PickerItem => ({ id: h.id, label, icon, group, description, tech: tech(h) });
  if (world === null) return headers.map((h) => header(h, h.location_name || h.internal_name, "place"));
  const rooms = (list: readonly Room[], group: string) => list.map((r) => header(r.header, r.name, "room", group, r.placeName));
  const areas = (list: readonly Area[], group: string) => list.flatMap((a) => [header(a.headers[0]!, a.name, "outside", group), ...rooms(a.rooms, group)]);
  return [
    ...world.worldMaps.flatMap((m) => areas(m.areas, m.name)),
    ...areas(world.moreAreas, more()),
    ...rooms(world.unconnected, unconnected()),
    ...(tinker ? world.other.map((h) => header(h, h.location_name || h.internal_name, "tinker", other())) : []),
  ];
}

/** The shown place as the folded row names it: an area by its name, a room after its place („Zweiblattdorf › Haus 1“). */
function shownPlace(identity: HeaderIdentity | null, header: HeaderSummary | undefined): { label: string; icon: IconName } {
  if (header === undefined) return { label: t("workbench:worldExplorer.choose"), icon: "place" };
  if (identity === null) return { label: header.location_name || header.internal_name, icon: "place" };
  if (identity.kind === "room") return { label: identity.place === identity.name ? identity.name : `${identity.place} › ${identity.name}`, icon: "room" };
  return { label: identity.name, icon: identity.kind === "area" ? "outside" : "place" };
}

/** The folded places: one row with the shown place, a symbol and an arrow, as a button that opens the picker. */
function FoldedPlaces({
  world,
  headers,
  selected,
  depth,
  identify,
  onSelect,
  onTouch,
}: Pick<WorldExplorerProps, "world" | "headers" | "selected" | "depth" | "identify" | "onSelect"> & { onTouch: (areaKey: string) => void }) {
  const { t } = useTranslation("workbench");
  const [open, setOpen] = useState(false);
  const header = headers.find((h) => h.id === selected);
  const { label, icon } = shownPlace(header && identify ? identify(header) : null, header);
  // Built again only when the world, the headers or the depth change (the row renders with every selection).
  const items = useMemo(() => placeItems(world, headers, depthAtLeast(depth, "tuefteln")), [world, headers, depth]);
  const pick = (item: PickerItem) => {
    const chosen = headers.find((h) => h.id === item.id);
    if (!chosen) return;
    onSelect(chosen);
    // As a click in the tree: the area of the pick (a room's owner) is the one chosen last.
    const areaKey = world?.areaOf.get(chosen.id);
    if (areaKey !== undefined) onTouch(areaKey);
  };
  return (
    <>
      <button type="button" className={styles.foldedRow} data-places-row="" title={t("worldExplorer.choose")} onClick={() => setOpen(true)}>
        <Icon name={icon} />
        <span className={styles.foldedLabel}>{label}</span>
        <Icon name="caretDown" />
      </button>
      <Picker open={open} onOpenChange={setOpen} title={t("worldExplorer.choose")} items={items} onPick={pick} placeholder={t("worldExplorer.pickPlaceholder")} emptyText={t("worldTree.noMatch")} />
    </>
  );
}

/**
 * The section „Orte“ of the list „Welt“: the world tree or the flat header list, with the menus for the view, the sort and the
 * wizards and the fold in its head. Folded, one row names the shown place and opens a picker; the tree stays mounted (hidden), so
 * its search and open branches survive. Memoised like the list: it renders when the headers, the world, the selection, the matrix
 * shown, the reveal token, the sort, the fold or a callback change, not when the scene is hovered or a message arrives. The callbacks
 * must keep their identity for that.
 */
export const WorldExplorer = memo(function WorldExplorer({ onOpenWizard, folded, onFold, sort, onSort, identify, ...tree }: WorldExplorerProps) {
  const { t } = useTranslation("workbench");
  const [mode, setMode] = useState<Mode>(readMode);
  // „Gelöschte zeigen“, the same for the tree and the list; off by default.
  const [showDeleted, setShowDeleted] = useState(false);
  const changeMode = (next: Mode) => {
    setMode(next);
    writeMode(next);
  };
  const [recent, touch] = useRecent(tree.info?.root ?? null);
  const sortItems: MenuEntry[] =
    mode === "tree"
      ? TREE_SORTS.map((value) => ({ label: t(`worldExplorer.sort.tree.${value}`), checked: sort.tree === value, onSelect: () => onSort({ tree: value }) }))
      : LIST_SORTS.map((value) => ({ label: t(`worldExplorer.sort.list.${value}`), checked: sort.list === value, onSelect: () => onSort({ list: value }) }));
  const actions = (
    <>
      {!folded && (
        <>
          <Segmented<Mode>
            size="s"
            iconOnly
            ariaLabel={t("worldExplorer.mode.label")}
            value={mode}
            onChange={changeMode}
            options={[
              { value: "tree", label: t("worldExplorer.mode.tree"), icon: "tree" },
              { value: "list", label: t("worldExplorer.mode.list"), icon: "list" },
            ]}
          />
          <Menu label={t("worldExplorer.sort.label")} trigger={<Icon name="sort" />} items={sortItems} />
        </>
      )}
      <Menu
        label={t("worldExplorer.create.label")}
        trigger={<Icon name="plus" />}
        items={[
          { label: t("worldExplorer.create.area"), onSelect: () => onOpenWizard({ kind: "area" }) },
          { label: t("worldExplorer.create.room"), onSelect: () => onOpenWizard({ kind: "room" }) },
        ]}
      />
      <IconButton label={folded ? t("worldExplorer.expand") : t("worldExplorer.collapse")} icon={folded ? "caretRight" : "caretDown"} onClick={() => onFold(!folded)} />
    </>
  );
  return (
    <section className={folded ? `${styles.section} ${styles.folded}` : styles.section} aria-label={termPlural("place")}>
      <PanelHead title={termPlural("place")} actions={actions} />
      {folded && <FoldedPlaces world={tree.world} headers={tree.headers} selected={tree.selected} depth={tree.depth} identify={identify} onSelect={tree.onSelect} onTouch={touch} />}
      <div className={styles.body} hidden={folded}>
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
            showDeleted={showDeleted}
            onShowDeleted={setShowDeleted}
          />
        ) : (
          // The recent choice reorders only the sort that asks for it: another sort keeps its rows when an area is touched.
          <WorldTree {...tree} sort={sort.tree} recent={sort.tree === "recent" ? recent : NO_RECENT} onTouch={touch} showDeleted={showDeleted} onShowDeleted={setShowDeleted} />
        )}
      </div>
    </section>
  );
});
